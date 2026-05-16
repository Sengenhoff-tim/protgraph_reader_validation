use std::fs::File;
use std::io::{BufReader};
use std::thread;
use std::sync::{Arc};
use crossbeam_channel::{Receiver, Sender, bounded};
use anyhow::{Result, anyhow};
use std::thread::JoinHandle;
use rayon::ThreadPoolBuilder;
use rayon::scope;

use crate::traversal::{Entry, Interval, MetaData, ProteinGraph, TraversalData, TraversalStatus};
use crate::io::{start_protein_graph_reader, spawn_writer_manager, bin_reader_manager};
use crate::utils::Config;

const GB: u64 = 1024*1024*1024;

pub fn process_graphs_deduplicated(
    config: Config
) -> Result<()> {

    let cli = config.cli;

    let file = File::open(cli.graph_input_path)?;
    let reader = BufReader::new(file);


     //setup global channels
    let (tx_graph, rx_graph) = bounded::<Result<ProteinGraph>>(2);

    let reader_handle = thread::spawn(|| start_protein_graph_reader(reader, tx_graph));

    let (tx_entry, writer_handle) = spawn_writer_manager(
        &cli.output_path,
        cli.hash_bits,
        cli.max_handles,
        cli.avail_processors
    )?;
    

    let intervals = Arc::new(config.intervals);

    let graph_handle = spawn_graph_processor(
        rx_graph,
        tx_entry,
        intervals,
        cli.max_vars,
        cli.avail_processors as usize,
        (cli.avail_memory as u64 *GB) as usize,
        cli.job_splits,
        cli.job_split_depth
    )?;

    for h in graph_handle {
        h.join()
            .map_err(|_| anyhow!("graph processor thread panicked"))??;
    }
    
    reader_handle.join().unwrap();

    let result = writer_handle.join().unwrap()?;

    bin_reader_manager(result, cli.avail_processors as usize, &cli.output_path)?;

    Ok(())
}

fn spawn_graph_processor(
    protein_graphs: Receiver<Result<ProteinGraph>>,
    tx_entry: Sender<Entry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    limit: usize,
    n_splits: u8,
    max_depth:u8

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
                limit,
                n_splits,
                max_depth
            )?;
        }

        Ok(())
    });

    Ok(vec![handle])
}

fn process_protein_graph(
    protein_graph: ProteinGraph,
    tx_entry: Sender<Entry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    limit: usize,
    n_splits: u8,
    max_depth: u8
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
        limit,
        n_splits,
        max_depth
    )?;
    Ok(())
}

pub fn spawn_workers(
    traversal_data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    intervals: &[Interval],
    max_vars: u8,
    tx_entry: Sender<Entry>,
    num_threads: usize,
    limit: usize,
    n_splits: u8,
    max_depth: u8
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
                        0,
                        max_depth,
                        max_vars,
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
    tx_entry: Sender<Entry>,
    interval: Interval,
    depth: u8,
    max_depth: u8,
    max_vars: u8,
    limit: usize,
    n_splits: u8,
) {
    // -------------------------
    // DEPTH TERMINATION
    // -------------------------
    if depth >= max_depth {
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
                        max_depth,
                        max_vars,
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
                    let _ = tx_entry.send(entry);
                }
            }
        }

        Err(e) => {
            eprintln!("traverse error: {e:?}");
        }
    }
}