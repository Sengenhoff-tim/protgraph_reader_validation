
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::fs::File;
use std::io::{BufWriter};
use std::io::Write;

use crate::protgraph_types::{ProteinGraph, Interval};

impl ProteinGraph {
    pub fn traverse_varcount(
        &self,
        interval: &Interval,
        max_vars: u8,
        trace: Arc<Mutex<BufWriter<File>>>
    ) -> anyhow::Result<Vec<Vec<u32>>> {
        // State information
        let mut tv_vals: HashMap<u32, Vec<i64>> = HashMap::new();
        let mut var_count: HashMap<u32, Vec<u8>> = HashMap::new();
        let mut paths: HashMap<u32, Vec<Vec<u32>>> = HashMap::new();

        // Initial values for traversal
        tv_vals.insert(0, vec![0]);
        var_count.insert(0, vec![0]);
        paths.insert(0, vec![vec![0]]);

        // For every node (in top order)
        for i in 0..self.num_n.saturating_sub(1) {
            let i = i as u32;

            // Skip if tv_vals is already empty (no paths!)
            if tv_vals.get(&i).map_or(true, |v| v.is_empty()) {
                continue;
            }

            // Get beginning and ending of edge-ids
            let (e_b, e_e) = if i == 0 {
                (0, self.nodes[0] as usize)
            } else {
                (
                    self.nodes[(i - 1) as usize] as usize,
                    self.nodes[i as usize] as usize,
                )
            };

            // Clone once to avoid borrow checker issues
            let tv_vals_i = tv_vals.get(&i).unwrap().clone();
            let var_count_i = var_count.get(&i).unwrap().clone();
            let paths_i = paths.get(&i).unwrap().clone();

            // For every possible path
            for j in 0..tv_vals_i.len() {
                // For every outgoing edge of the node
                for k in e_b..e_e {
                    // Calculate the achieved weight and target_node
                    let achieved = tv_vals_i[j] + self.mono_weight[self.edges[k] as usize] as i64;
                    let shifted = Interval {
                        lower: interval.lower - achieved,
                        upper: interval.upper - achieved,
                    };
                    let target_node = self.edges[k] as u32;

                    // Additionally count the variants
                    let current_var_count = var_count_i[j] as u16 + self.variant_count[k] as u16;
                    let achieved = tv_vals_i[j] + self.mono_weight[self.edges[k] as usize] as i64;

                    let overlap = self.has_overlapping_interval(target_node as usize, &shifted)?;

                    let accepted = (current_var_count as u8) <= max_vars && overlap;
                    
                    let mut t = trace.lock().unwrap();
                    writeln!(
                        t,
                        "{} {} {} {} {} {} {} {} {} {} {}",
                        i,
                        j,
                        k,
                        target_node,
                        achieved,
                        shifted.lower,
                        shifted.upper,
                        current_var_count,
                        max_vars,
                        overlap,
                        accepted
                    )?;

                    if accepted
                    {
                        // CASE: Expanding
                        // Add new tv_val
                        tv_vals
                            .entry(target_node)
                            .or_insert_with(Vec::new)
                            .push(achieved);

                        // Count up used variants
                        var_count
                            .entry(target_node)
                            .or_insert_with(Vec::new)
                            .push(current_var_count as u8);

                        // Add new path how we achieved it
                        let mut new_path = paths_i[j].clone();
                        new_path.push(target_node);

                        paths
                            .entry(target_node)
                            .or_insert_with(Vec::new)
                            .push(new_path);
                    }
                }
            }

            // Free memory during traversal, since older results can be removed (-> dag)!
            tv_vals.remove(&i);
            var_count.remove(&i);
            paths.remove(&i);
        }

        // Return results
        Ok(paths
            .get(&((self.num_n - 1) as u32))
            .cloned()
            .unwrap_or_default())
    }
}
/* 
#[derive(Clone)]
pub struct TraversalState {
    tv_vals: Vec<i64>,
    var_count: Vec<u8>,
    paths: Vec<Vec<u32>>
}

impl TraversalState {
    fn new() -> Self {
        TraversalState {
            tv_vals: Vec::new(),
            var_count: Vec::new(),
            paths: Vec::new(),
        }
    }
}

impl ProteinGraph {
    pub fn traverse_varcount(
        &self,
        interval: &Interval,
        max_vars: u8,
    ) -> anyhow::Result<Vec<Vec<u32>>> {
        let n_nodes = self.num_n as usize;
        if n_nodes == 0 {
            return Ok(Vec::new());
        }

        let mut results: HashMap<usize, TraversalState> = HashMap::new();

        results.insert(
            0,
            TraversalState {
                tv_vals: vec![0],
                var_count: vec![0],
                paths: vec![vec![0]],
            },
        );

        // Process each node in topological order
        for node_index in 0..n_nodes.saturating_sub(1) {
            let state = match results.get(&node_index) {
                Some(s) if !s.tv_vals.is_empty() => s,
                _ => continue,
            };

            // Snapshot ONCE (critical fix)
            let tv_vals = state.tv_vals.clone();
            let var_counts = state.var_count.clone();
            let paths = state.paths.clone();

            // Get edge range for this node
            let (edge_start_idx, edge_end_idx) = if node_index == 0 {
                (0, self.nodes[0] as usize)
            } else {
                (
                    self.nodes[node_index - 1] as usize,
                    self.nodes[node_index] as usize,
                )
            };

            // For every possible path at node i
            for tv_val_idx in 0..tv_vals.len() {
                for edge_idx in edge_start_idx..edge_end_idx {
                    let target_node = self.edges[edge_idx] as usize;

                    let achieved =
                        tv_vals[tv_val_idx] + self.mono_weight[target_node];

                    let shifted = Interval {
                        lower: interval.lower - achieved,
                        upper: interval.upper - achieved,
                    };

                    let current_var_count =
                        var_counts[tv_val_idx] + self.variant_count[edge_idx];

                    if current_var_count <= max_vars
                        && self.has_overlapping_interval(target_node, &shifted)?
                    {
                        let mut new_path = paths[tv_val_idx].clone();
                        new_path.push(target_node as u32);

                        let entry = results
                            .entry(target_node)
                            .or_insert_with(TraversalState::new);

                        entry.tv_vals.push(achieved);
                        entry.var_count.push(current_var_count);
                        entry.paths.push(new_path);
                    }
                }
            }

            // Safe to remove after full snapshot usage
            results.remove(&node_index);
        }

        Ok(
            results
                .get(&(n_nodes - 1))
                .cloned()
                .unwrap()
                .paths,
        )
    }
}
*/
