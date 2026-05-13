use::anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use crossbeam_deque::Injector;
use xxhash_rust::xxh64::xxh64;
use std::{mem::take, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use crate::traversal::{Entry, Interval, MetaData, Pdbs, TraversalState, TraversalStatus};

const SEED: u64 = 0xC0111DE;

const N_SPLITS: usize = 4;
const MAX_DEPTH: u8 = 4;


pub struct TraversalData {
    pub nodes: Box<[u32]>,
    pub edges: Box<[u32]>,
    pub mono_weight: Box<[i64]>,
    pub variant_count: Box<[u8]>,
    pub pdbs: Pdbs,
}

#[inline(always)]
fn build_edge_ranges(
    nodes: &[u32],
    edge_ranges: &mut Vec<(usize, usize)>,
) {
    for i in 0..nodes.len() {
        let begin = if i == 0 { 0 } else { nodes[i - 1] as usize };
        let end = nodes[i] as usize;
        edge_ranges.push((begin, end));
    }
}

impl TraversalData {
    pub fn traverse_varcount(
        &self,
        interval: &Interval,
        max_vars: u8,
        limit: usize
    ) -> Result<TraversalStatus> {
        let mut edge_ranges = Vec::with_capacity(self.nodes.len());

        let mut traversal_state = TraversalState::new(self.nodes.len(), max_vars, limit);
        
        build_edge_ranges(&self.nodes, &mut edge_ranges);

        for node_idx in 0..self.nodes.len() - 1 {
            if traversal_state.states_at_node[node_idx].is_empty() {
                continue;
            }

            let (edge_begin, edge_end) = edge_ranges[node_idx];

            let current_states = take(&mut traversal_state.states_at_node[node_idx]);

            for state_id in current_states {
                let state = &traversal_state.arena[state_id];
                let tv = state.tv;
                let var = state.var;

                for edge_idx in edge_begin..edge_end {
                    let new_var = var + self.variant_count[edge_idx];
                    
                    if new_var > max_vars {
                        continue;
                    }

                    let target_node = self.edges[edge_idx] as usize;

                    let achieved = tv + self.mono_weight[target_node];

                    let lower = interval.lower - achieved;
                    let upper = interval.upper - achieved;
                    
                    if !self.has_overlapping_interval(target_node, lower, upper) {
                        continue;
                    }
                    if !traversal_state.push_state(
                        state_id,
                        self.edges[edge_idx],
                        edge_idx as u32,
                        new_var,
                        achieved,
                        target_node,
                    ) {
                        return Ok(TraversalStatus::Overflow());
                    }
                }           
            }
        }
        Ok(TraversalStatus::Complete(traversal_state))
    }

    /* 
    pub fn traverse_and_stream_traces(
        &self,
        interval: &Interval,
        max_vars: u8,
        tx: &Sender<Vec<(u32, u32)>>,
    ) -> anyhow::Result<()> {
        let traversal_state = &self.traverse_varcount(interval, max_vars)?;

        let final_states =
            &traversal_state.states_at_node[(self.nodes.len() - 1) as usize];

        for &state_id in final_states {
            let trace = traversal_state.reconstruct_trace(state_id);

            tx.send(trace)
                .map_err(|e| anyhow!("channel send failed: {e}"))?;
        }

        Ok(())
    }
    */

    pub fn traverse_and_stream_traces(
    &self,
    interval: &Interval,
    max_vars: u8,
    tx: &Sender<Vec<(u32, u32)>>,
    limit: usize
) -> anyhow::Result<()> {
    self.traverse_and_stream_traces_inner(
        interval,
        max_vars,
        tx,
        0, 
        limit// depth starts here
    )
}

fn traverse_and_stream_traces_inner(
    &self,
    interval: &Interval,
    max_vars: u8,
    tx: &Sender<Vec<(u32, u32)>>,
    depth: u8,
    limit: usize
) -> anyhow::Result<()> {
    // =========================
    // HARD TERMINATION CONDITION
    // =========================
    if depth >= MAX_DEPTH {
        // placeholder (you said you will log here later)
        return Ok(());
    }

    let traversal_state = self.traverse_varcount(interval, max_vars, limit)?;

    match traversal_state {
        TraversalStatus::Overflow() => {
            let splits = interval.split(N_SPLITS);

            for sub in splits {
                self.traverse_and_stream_traces_inner(
                    &sub,
                    max_vars,
                    tx,
                    depth + 1,
                    limit
                )?;
            }
        }

        TraversalStatus::Complete(state) => {
            let final_states =
                &state.states_at_node[(self.nodes.len() - 1) as usize];

            for &state_id in final_states {
                let trace = state.reconstruct_trace(state_id);

                tx.send(trace)
                    .map_err(|e| anyhow!("channel send failed: {e}"))?;
            }
        }
    }

    Ok(())
}

#[inline]
pub fn has_overlapping_interval(
    &self,
    node: usize,
    lower: i64,
    upper: i64,
) -> bool {
    let slice = match self.pdbs.get_node_intervals(node) {
        Some(s) => s,
        None => return false,
    };

    let mut i = 0;
    while i < slice.len() {
        let iv = unsafe { slice.get_unchecked(i) };

        if iv.lower > upper {
            break;
        }

        if iv.lower <= upper && iv.upper >= lower {
            return true;
        }

        i += 1;
    }

    false
}

}
