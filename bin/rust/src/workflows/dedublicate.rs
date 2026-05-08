use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::{Arc};
use crossbeam_channel::{bounded, Sender, Receiver};
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::thread::JoinHandle;


use crate::traversal::{Interval, MetaData, TraversalData, Entry, ProteinGraph};
use crate::io::{start_protein_graph_reader, writer_thread};



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
    let (tx_entry, rx_entry) = bounded::<Entry>(t_count);

    let (tx_graph, rx_graph) = bounded::<Result<ProteinGraph>>(2);

    let reader_handle = thread::spawn(|| start_protein_graph_reader(reader, tx_graph));

    
    //setup seq writer
    let seq_writer = setup_output_file(&output_path)?;

    let writer_handle = thread::spawn(move || {

        let mut writer = seq_writer;

        writer_thread(rx_entry, &mut writer)
    });

    let intervals = Arc::new(intervals);

    let graph_handle = spawn_graph_processor(
        rx_graph,
        tx_entry,
        intervals,
        max_vars,
        t_count,
        1,
    )?;

    for h in graph_handle {
        h.join()
            .map_err(|_| anyhow!("graph processor thread panicked"))??;
    }
    
    reader_handle.join().unwrap();

    writer_handle.join().unwrap()?;

    Ok(())
}

fn spawn_graph_processor(
    protein_graphs: Receiver<Result<ProteinGraph>>,
    tx_entry: Sender<Entry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    _num_threads: usize,
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
            )?;
        }

        Ok(())
    });

    Ok(vec![handle])
}

/* 
fn process_protein_graph(
    protein_graph: ProteinGraph,
    tx_entry: Sender<Entry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
) -> Result<()> {
    let (tx_trace, rx_trace) = bounded::<Vec<(u32, u32)>>(t_count);

    let traversal_data = Arc::new(protein_graph.traversal_data);
    let meta_data = Arc::new(protein_graph.meta_data);

    let peptide_handles = spawn_peptide_builders(
        &meta_data,
        tx_entry,
        rx_trace,
        t_count,
    )?;

    let trace_handles = spawn_trace_builders(
        traversal_data, 
        &intervals, 
        max_vars, 
        tx_trace.clone(), 
        t_count
    )?;

    
    for h in trace_handles {
    h.join()
        .map_err(|_| anyhow!("trace thread panicked"))??;
    }

    drop(tx_trace);

    for h in peptide_handles {
        h.join()
            .map_err(|_| anyhow!("peptide thread panicked"))??;
    }


    Ok(())
}
*/
fn process_protein_graph(
    protein_graph: ProteinGraph,
    tx_entry: Sender<Entry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
) -> Result<()> {

    let traversal_data = Arc::new(protein_graph.traversal_data);
    let meta_data = Arc::new(protein_graph.meta_data);


    let worker_handles = spawn_workers(
        traversal_data, 
        meta_data,
        &intervals, 
        max_vars, 
        tx_entry.clone(), 
        t_count
    )?;

    
    for h in worker_handles {
    h.join()
        .map_err(|_| anyhow!("worker thread panicked"))??;
    }

    Ok(())
}
/* 
fn spawn_trace_builders(
    traversal_data: Arc<TraversalData>,
    intervals: &[Interval],
    max_vars: u8,
    tx: Sender<Vec<(u32, u32)>>,
    num_threads: usize
) -> Result<Vec<JoinHandle<Result<()>>>> {

    let (job_tx, job_rx) = bounded::<Interval>(num_threads);

    // ---- spawn workers ----
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let data = Arc::clone(&traversal_data);
        let job_rx = job_rx.clone();
        let tx = tx.clone();

        let handle = thread::spawn(move || -> Result<()> {
            for interval in job_rx {
                if let Err(e) = data.traverse_and_stream_traces(
                    &interval, 
                    max_vars, 
                    &tx
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

pub fn spawn_peptide_builders(
    meta: &Arc<MetaData>,
    tx_entry: Sender<Entry>,
    rx_trace: Receiver<Vec<(u32, u32)>>,
    num_threads: usize,
) -> Result<Vec<JoinHandle<Result<()>>>> {
    
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let rx_trace = rx_trace.clone();
        let tx_entry = tx_entry.clone();
        let meta = Arc::clone(&meta);

        let handle = thread::spawn(move || -> Result<()> {
            for trace in rx_trace {
                let peptide= meta.build_peptide(
                    &trace,
                );
                if let Ok(Some(entry)) = peptide {
                    tx_entry.send(entry)
                        .map_err(|e| anyhow!("send failed: {e}"))?;
                } else {
                    continue;
                }        
            }

            Ok(())
        });

        handles.push(handle);
    }
    Ok(handles)
}
    */

pub fn setup_output_file(output_path: &PathBuf) -> Result<BufWriter<File>> {
    let output_path = if output_path.is_absolute() {
        output_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(output_path)
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)?;

    Ok(BufWriter::new(file))
}
fn spawn_workers(
    traversal_data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    intervals: &[Interval],
    max_vars: u8,
    tx_entry: Sender<Entry>,
    num_threads: usize,
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
                    &meta
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
