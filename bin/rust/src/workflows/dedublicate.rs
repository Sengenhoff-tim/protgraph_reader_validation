use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::Arc;
use crossbeam_channel::{bounded, Sender};
use std::path::PathBuf;
use anyhow::{Result, anyhow};

use crate::traversal::{Interval, TraversalData};
use crate::io::{ProteinGraphReader, tmp_bin_writer, Incoming};

pub fn process_graphs_dedublicated(
    graph_input_path: PathBuf,
    intervals: &Vec<Interval>, 
    max_vars: u8, 
    t_count: usize
) -> anyhow::Result<()> {

    let file = File::open(&graph_input_path)?;
    let reader = ProteinGraphReader::new(
        BufReader::new(file),
        true
    );

    let file = File::create("out.bin")?;

// then you'd need a different API that accepts BinBufferedWriter directly

    let (tx, rx) = bounded::<Incoming>(10);

    tmp_bin_writer(rx, file)?;

    for graph in reader {
        match graph {
            Ok(protein_graph) => { 
                //let protein_graph = std::sync::Arc::new(protein_graph);
                tx.send(
                    Incoming::Meta(protein_graph.meta_data)
                )?;
                let traversal_data = std::sync::Arc::new(protein_graph.traversal_data);
                spawn_workers(
                    traversal_data, 
                    &intervals, 
                    max_vars, 
                    tx.clone(), 
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

fn spawn_workers(
    traversal_data: Arc<TraversalData>,
    intervals: &[Interval],
    max_vars: u8,
    tx: Sender<Incoming>,
    num_threads: usize
) -> Result<()> {

    let (job_tx, job_rx) = bounded::<Interval>(num_threads);

    // ---- spawn workers ----
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let data = Arc::clone(&traversal_data);
        let job_rx = job_rx.clone();
        let tx = tx.clone();

        let handle = thread::spawn(move || -> Result<()> {
            for interval in job_rx {
                let traversal_state = data.traverse_varcount(&interval, max_vars)?;
                tx.send(Incoming::Traversal(traversal_state))
                    .map_err(|e| anyhow!("channel send failed: {e}"))?;
            }

            Ok(())
        });

        handles.push(handle);
    }

    // ---- feed jobs ----
    for interval in intervals.iter() {
        job_tx.send(interval.clone())
            .map_err(|e| anyhow!("job send failed: {e}"))?;
    }

    drop(job_tx);

    // ---- join workers ----
    for handle in handles {
        handle.join()
            .map_err(|_| anyhow!("worker panicked"))??;
    }

    Ok(())
}