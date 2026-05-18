use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Arc, atomic::AtomicBool},
    thread,
    thread::JoinHandle,
};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};

use crate::process_graphs::threading::graph_workers::spawn_workers;
use crate::process_graphs::{graph::ProteinGraph, utilities::Interval};
use crate::shared::BinEntry;

pub fn spawn_graph_dispatcher(
    protein_graphs: Receiver<Result<ProteinGraph>>,
    tx_entry: Sender<BinEntry>,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    t_count: usize,
    limit: usize,
    n_splits: u8,
    max_depth: u8,
    log_writer: BufWriter<File>,
) -> anyhow::Result<JoinHandle<anyhow::Result<BufWriter<File>>>> {
    let handle = thread::spawn(move || -> anyhow::Result<BufWriter<File>> {
        let mut log_writer = log_writer;

        for graph in protein_graphs {
            let protein_graph = graph?;

            // for logging
            let accession = protein_graph.meta_data.accessions[0].clone();
            let incomplete = Arc::new(AtomicBool::new(false));

            spawn_workers(
                protein_graph,
                intervals.clone(),
                max_vars,
                tx_entry.clone(),
                t_count,
                limit,
                n_splits,
                max_depth,
                Arc::clone(&incomplete),
            )?;

            if incomplete.load(std::sync::atomic::Ordering::Relaxed) {
                writeln!(
                    log_writer,
                    "{},Incomplete traversal due to memory limit",
                    accession
                )?;
            }
        }

        log_writer.flush()?;
        Ok(log_writer)
    });

    Ok(handle)
}
