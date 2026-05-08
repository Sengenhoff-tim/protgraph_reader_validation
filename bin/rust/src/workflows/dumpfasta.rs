use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::{Arc, RwLock};
use crossbeam_channel::{bounded, Sender, Receiver};
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::thread::JoinHandle;

use crate::traversal::{Interval, MetaData, StringTable, TraversalData};
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
        BufReader::new(file)
    );

    let output_path = if output_path.is_absolute() {
        output_path
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

    let writer = BufWriter::new(file);

    let (tx_pep, rx_pep) = bounded::<String>(t_count);

    let writer_handle = thread::spawn(move || {

        let mut writer = writer;

        writer_thread(rx_pep, &mut writer)
    });

    let global_sequences = Arc::new(RwLock::new(Vec::new()));

    

    for (graph_idx, graph) in reader.enumerate() {
        match graph {
            Ok(protein_graph) => {
                let (tx_trace, rx_trace) = bounded::<Vec<(u32, u32)>>(t_count);

                {
                    let mut seqs = global_sequences.write().unwrap();
                    seqs.push(protein_graph.sequences);
                }
                
                let seq_arc = Arc::clone(&global_sequences);


                let traversal_data = Arc::new(protein_graph.traversal_data);
                let meta_data = Arc::new(protein_graph.meta_data);

                let fragment_handles = spawn_fragment_builders(
                    &meta_data,
                    &seq_arc, 
                    tx_pep.clone(), 
                    t_count, 
                    rx_trace,
                    graph_idx
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

                for h in fragment_handles {
                    h.join()
                        .map_err(|_| anyhow!("fragment thread panicked"))??;
                }

            }
            Err(e) => {
                eprintln!("error reading graph: {}", e);
                break;
            }
        }
        
    }

    drop(tx_pep);

    writer_handle.join().unwrap()?;

    Ok(())
}

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

pub fn spawn_fragment_builders(
    meta: &Arc<MetaData>,
    sequences: &Arc<RwLock<Vec<StringTable>>>,
    tx_out: Sender<String>,
    num_threads: usize,
    rx_trace: Receiver<Vec<(u32, u32)>>,
    graph_idx: usize
) -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut handles = Vec::new();

    for _ in 0..num_threads {
        let rx_trace = rx_trace.clone();
        let tx_out = tx_out.clone();
        let meta = Arc::clone(&meta);
        let arc_seq = sequences.clone();

        let handle = thread::spawn(move || -> Result<()> {
            for trace in rx_trace {
                let fragment= &meta.build_fragment(
                    &arc_seq,
                    &trace,
                    graph_idx
                );

                if let Ok(Some((meta, sequence))) = fragment {
                    tx_out.send(meta.to_string())
                        .map_err(|e| anyhow!("send failed: {e}"))?;
                    tx_out.send(sequence.to_string())
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