use::anyhow::{Result};
use std::{mem::take};
use crate::traversal::{Interval, Pdbs, TraversalState, TraversalStatus};

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
