use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
};

use anyhow::Result;
use crossbeam_channel::Sender;
use rayon::{ThreadPoolBuilder, scope};

use crate::process_graphs::{
    graph::{MetaData, ProteinGraph, TraversalData},
    utilities::{Interval, TraversalStatus},
};
use crate::shared::BinEntry;

pub fn spawn_workers(
    protein_graph: ProteinGraph,
    intervals: Arc<Vec<Interval>>,
    max_vars: u8,
    tx_entry: Sender<BinEntry>,
    num_threads: usize,
    limit: usize,
    n_splits: u8,
    max_depth: u8,
    incomplete: Arc<AtomicBool>,
) -> Result<()> {
    let traversal_data = Arc::new(protein_graph.traversal_data);
    let meta_data = Arc::new(protein_graph.meta_data);

    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    pool.install(|| {
        scope(|s| {
            for interval in intervals.iter().cloned() {
                let data = Arc::clone(&traversal_data);
                let meta = Arc::clone(&meta_data);
                let tx_entry = tx_entry.clone();
                let incomplete = Arc::clone(&incomplete);

                s.spawn(move |_| {
                    traversal_thread(
                        data, meta, tx_entry, interval, 0, max_depth, max_vars, limit, n_splits,
                        incomplete,
                    );
                });
            }
        });
    });

    Ok(())
}

fn traversal_thread(
    data: Arc<TraversalData>,
    meta: Arc<MetaData>,
    tx_entry: Sender<BinEntry>,
    interval: Interval,
    depth: u8,
    max_depth: u8,
    max_vars: u8,
    limit: usize,
    n_splits: u8,
    incomplete: Arc<AtomicBool>,
) {
    // depth termination
    if depth >= max_depth {
        incomplete.store(true, Relaxed);
        return;
    }

    match data.traverse(&interval, max_vars, limit) {
        Ok(TraversalStatus::Overflow()) => {
            let splits = interval.split_to_n(n_splits);

            for sub in splits {
                let data = Arc::clone(&data);
                let meta = Arc::clone(&meta);
                let tx_entry = tx_entry.clone();
                let incomplete = Arc::clone(&incomplete);

                rayon::spawn(move || {
                    traversal_thread(
                        data,
                        meta,
                        tx_entry,
                        sub,
                        depth + 1,
                        max_depth,
                        max_vars,
                        limit,
                        n_splits,
                        incomplete,
                    );
                });
            }
        }

        Ok(TraversalStatus::Complete(state)) => {
            let final_states = &state.states_at_node[((data.nodes.len() - 1))];

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
