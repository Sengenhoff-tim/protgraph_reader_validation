use::anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use std::mem::{take};
use crate::protgraph_types::{Interval, TraversalState, Pdbs};

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
    ) -> Result<TraversalState> {
        let mut edge_ranges = Vec::with_capacity(self.nodes.len());

        let mut traversal_state = TraversalState::new(self.nodes.len(), max_vars);
        
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
                    traversal_state.push_state(
                        self.edges[edge_idx],
                        state_id,
                        edge_idx as u32,
                        achieved,
                        new_var,
                        target_node,
                    );
                }           
            }
        }
        Ok(traversal_state)
    }

    pub fn traverse_and_stream_traces(
        &self,
        interval: &Interval,
        max_vars: u8,
        tx: &Sender<Vec<(u32, u32)>>
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


#[inline]
pub fn has_overlapping_interval(
    &self,
    node: usize,
    lower: i64,
    upper: i64,
) -> bool {
    let slice = match self.pdbs.get_node_intervals(node) {
        Some(s) => s,
        None => return false, // or debug_assert! depending on invariants
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