use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::{Arc};
use crossbeam_channel::{Receiver, Sender, bounded};
use xxhash_rust::xxh64::xxh64;
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::thread::JoinHandle;

const SEED: u64 = 0xC0111DE;

//use std::io::Write;

const MAX_DEPTH: u16 = 100;


use crate::io::tmp_files::BinWriterParams;
use crate::traversal::{Entry, Interval, MetaData, ProteinGraph, TraversalData, TraversalStatus};
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
        entry_channel_size: 50
    };

    let n_splits = 2;
    let limit = (1024*1024*1024)/t_count;
    
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
        limit,
        n_splits
    )?;

    for h in graph_handle {
        h.join()
            .map_err(|_| anyhow!("graph processor thread panicked"))??;
    }
    
    reader_handle.join().unwrap();

    let result = writer_handle.join().unwrap()?;

    bin_reader_manager(result, t_count, output_path)?;

    Ok(())
}

fn spawn_graph_processor(
    protein_graphs: Receiver<Result<ProteinGraph>>,
    tx_entry: Sender<(u64, Entry)>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    do_hash: bool,
    limit: usize,
    n_splits: usize

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
                do_hash,
                limit,
                n_splits
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
    do_hash: bool,
    limit: usize,
    n_splits: usize
) -> Result<()> {

    let traversal_data = Arc::new(protein_graph.traversal_data);
    let meta_data = Arc::new(protein_graph.meta_data);


    spawn_workers(
    traversal_data,
    meta_data,
    &intervals,
    max_vars,
    tx_entry.clone(),
    t_count,
    do_hash,
    limit,
    n_splits,
)?;
    Ok(())
}

use rayon::ThreadPoolBuilder;
use rayon::scope;

pub fn spawn_workers(
    traversal_data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    intervals: &[Interval],
    max_vars: u8,
    tx_entry: Sender<(u64, Entry)>,
    num_threads: usize,
    do_hash: bool,
    limit: usize,
    n_splits: usize,
) -> Result<()> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    pool.install(|| {
        scope(|s| {
            for interval in intervals.iter().cloned() {
                let data = Arc::clone(&traversal_data);
                let meta = Arc::clone(&meta);
                let tx_entry = tx_entry.clone();

                s.spawn(move |_| {
                    traverse_rayon(
                        data,
                        meta,
                        tx_entry,
                        interval,
                        0, // depth
                        max_vars,
                        do_hash,
                        limit,
                        n_splits,
                    );
                });
            }
        });
    });

    Ok(())
}

fn traverse_rayon(
    data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    tx_entry: Sender<(u64, Entry)>,
    interval: Interval,
    depth: u16,
    max_vars: u8,
    do_hash: bool,
    limit: usize,
    n_splits: usize,
) {
    // -------------------------
    // DEPTH TERMINATION
    // -------------------------
    if depth >= MAX_DEPTH {
        return;
    }

    match data.traverse_varcount(&interval, max_vars, limit) {
        Ok(TraversalStatus::Overflow()) => {
            let splits = interval.split(n_splits);

            for sub in splits {
                let data = Arc::clone(&data);
                let meta = Arc::clone(&meta);
                let tx_entry = tx_entry.clone();

                rayon::spawn(move || {
                    traverse_rayon(
                        data,
                        meta,
                        tx_entry,
                        sub,
                        depth + 1,
                        max_vars,
                        do_hash,
                        limit,
                        n_splits,
                    );
                });
            }
        }

        Ok(TraversalStatus::Complete(state)) => {
            let final_states =
                &state.states_at_node[(data.nodes.len() - 1) as usize];

            for &state_id in final_states {
                let trace = state.reconstruct_trace(state_id);

                if let Ok(Some(entry)) = meta.build_peptide(&trace) {
                    let mut pep_hash = 0;
                    if do_hash {
                        pep_hash = xxh64(entry.pep.as_bytes(), SEED);
                    }

                    let _ = tx_entry.send((pep_hash, entry));
                }
            }
        }

        Err(e) => {
            eprintln!("traverse error: {e:?}");
        }
    }
}