use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::{Arc};
use crossbeam_channel::{bounded, Sender, Receiver};
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::thread::JoinHandle;

use std::io::Write;


use crate::io::tmp_files::BinWriterParams;
use crate::traversal::{Interval, MetaData, TraversalData, Entry, ProteinGraph};
use crate::io::{start_protein_graph_reader, spawn_writer_manager, bin_reader_manager};

pub fn process_graphs_dedublicated(
    graph_input_path: PathBuf,
    output_path: PathBuf, 
    intervals: Vec<Interval>, 
    max_vars: u8, 
    t_count: usize
) -> Result<()> {

    let file = File::open(&graph_input_path)?;
    let reader = BufReader::new(file);


     //setup global channels
    let (tx_graph, rx_graph) = bounded::<Result<ProteinGraph>>(2);

    let reader_handle = thread::spawn(|| start_protein_graph_reader(reader, tx_graph));

    let writer_params = BinWriterParams{
        total_entries: 6*1000000,
        avg_entry_size: 80,
        overhead: 2.5,
        skew: 2.0,
        max_memory: 2 * 1024 * 1024 * 1024,
        entry_channel_size: 100
    };
    
    let (tx_entry, writer_handle) = spawn_writer_manager(
        "./shards",
        writer_params,
        128,
    )?;

    let intervals = Arc::new(intervals);

    let do_hash = true;

    let graph_handle = spawn_graph_processor(
        rx_graph,
        tx_entry,
        intervals,
        max_vars,
        t_count,
        do_hash,
    )?;

    for h in graph_handle {
        h.join()
            .map_err(|_| anyhow!("graph processor thread panicked"))??;
    }
    
    reader_handle.join().unwrap();

    let result = writer_handle.join().unwrap()?;

    let mut file = File::create("./test")?;

    // Write some dummy content
    writeln!(file, "generated {} shard files",
        result.filenames.len())?;
    writeln!(file, "still-open cached handles: {}",
        result.handles.len())?;
    

    bin_reader_manager(result, t_count, output_path)?;

    

    Ok(())
}

fn spawn_graph_processor(
    protein_graphs: Receiver<Result<ProteinGraph>>,
    tx_entry: Sender<(u64, Entry)>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    do_hash: bool
) -> anyhow::Result<Vec<JoinHandle<anyhow::Result<()>>>> {
    
    let handle = thread::spawn(move || -> anyhow::Result<()> {
        for graph in protein_graphs {
            let protein_graph = graph?;

            process_protein_graph(
                protein_graph,
                tx_entry.clone(),
                intervals.clone(),
                max_vars,
                t_count,
                do_hash
            )?;
        }

        Ok(())
    });

    Ok(vec![handle])
}

fn process_protein_graph(
    protein_graph: ProteinGraph,
    tx_entry: Sender<(u64, Entry)>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    do_hash: bool
) -> Result<()> {

    let traversal_data = Arc::new(protein_graph.traversal_data);
    let meta_data = Arc::new(protein_graph.meta_data);


    let worker_handles = spawn_workers(
        traversal_data, 
        meta_data,
        &intervals, 
        max_vars, 
        tx_entry.clone(), 
        t_count,
        do_hash
    )?;

    
    for h in worker_handles {
    h.join()
        .map_err(|_| anyhow!("worker thread panicked"))??;
    }

    Ok(())
}

fn spawn_workers(
    traversal_data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    intervals: &[Interval],
    max_vars: u8,
    tx_entry: Sender<(u64, Entry)>,
    num_threads: usize,
    do_hash: bool
) -> Result<Vec<JoinHandle<Result<()>>>> {
    let (job_tx, job_rx) = bounded::<Interval>(num_threads);
    let mut handles = Vec::new();

    // ---- spawn workers ----
    for _ in 0..num_threads {
        let data = Arc::clone(&traversal_data);
        let meta = Arc::clone(&meta);
        let job_rx = job_rx.clone();
        let tx_entry = tx_entry.clone();

        let handle = thread::spawn(move || -> Result<()> {
            for interval in job_rx {
                if let Err(e) = data.traverse_and_stream_entries(
                    &interval,
                    max_vars,
                    &tx_entry,
                    &meta,
                    do_hash,
                ) {
                    eprintln!("interval failed: {e:?}");
                }
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

    Ok(handles)
}