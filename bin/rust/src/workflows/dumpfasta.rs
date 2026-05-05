use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::Arc;
use crossbeam_channel::{bounded, Sender};
use std::path::PathBuf;
use anyhow::{Result, anyhow};

use crate::traversal::{ProteinGraph, Interval};
use crate::io::{ProteinGraphReader, writer_thread};

pub fn process_graphs_to_fasta(
    graph_input_path: PathBuf,
    output_path: PathBuf, 
    intervals: &Vec<Interval>, 
    max_vars: u8, 
    t_count: usize
) -> anyhow::Result<()> {

    let file = File::open(&graph_input_path)?;
    let reader = ProteinGraphReader::new(
        BufReader::new(file),
        false
    );

    for graph in reader {
        match graph {
            Ok(protein_graph) => { 
                let protein_graph = std::sync::Arc::new(protein_graph);
                process_single_graph(
                    protein_graph, 
                    &intervals, 
                    max_vars, 
                    output_path.clone(), 
                    t_count
                )?; 
            }
            Err(e) => {
                eprintln!("error reading graph: {}", e);
                break;
            }
        }
        
    }
    Ok(())
}

fn process_single_graph(
    graph: Arc<ProteinGraph>,
    intervals: &Vec<Interval>,
    max_vars: u8,
    output_path: PathBuf,
    num_threads: usize
) -> Result<()> {

    let (tx, rx) = bounded::<Vec<(u32, u32)>>(128);

    let graph_for_writer = Arc::clone(&graph);

    let writer_handle = thread::spawn(move || {
        writer_thread(rx, output_path, graph_for_writer)
    });

    spawn_workers(
        graph,
        intervals,
        max_vars,
        tx,
        num_threads
    )?;

    writer_handle
        .join()
        .map_err(|_| anyhow!("Writer thread panicked"))??;

    Ok(())
}

fn spawn_workers(
    graph: Arc<ProteinGraph>,
    intervals: &Vec<Interval>,
    max_vars: u8,
    tx: Sender<Vec<(u32, u32)>>,
    num_threads: usize
) -> Result<()> {

    let (job_tx, job_rx) = bounded::<Interval>(num_threads);

    // ---- spawn workers ----
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let graph = Arc::clone(&graph);
        let job_rx = job_rx.clone();
        let tx = tx.clone();

        let handle = thread::spawn(move || {
            for interval in job_rx {
                if let Err(e) = graph.traversal_data.traverse_and_stream_traces(
                    &interval, 
                    max_vars, 
                    &tx
                ) {
                    eprintln!("interval failed: {e:?}");
                }
            }
        });

        handles.push(handle);
    }

    // ---- feed jobs ----
    for interval in intervals.iter() {
        job_tx.send(interval.clone())
            .map_err(|e| anyhow!("job send failed: {e}"))?;
    }

    drop(job_tx);
    drop(tx);

    // ---- join workers ----
    for handle in handles {
        handle.join()
            .map_err(|_| anyhow!("worker panicked"))?;
    }

    Ok(())
}