use::anyhow::Result;
use crate::protgraph_types::{ProteinGraph, Interval};

type StateId = usize;


#[derive(Debug, Clone)]
struct State {
    node: u32,
    parent: Option<StateId>,
    edge_id: Option<usize>,
    tv: i64,
    var: u8,
}

pub struct Traversal {
    arena: Vec<State>,
    states_at_node: Vec<Vec<StateId>>,
}

impl Traversal {
    pub fn reconstruct_trace(
        &self,
        mut state_id: StateId,
    ) -> Result<Vec<(u32, Option<usize>)>> {
        let mut trace = Vec::new();

        while let Some(state) = self.arena.get(state_id) {
            trace.push((state.node, state.edge_id));

            match state.parent {
                Some(p) => state_id = p,
                None => break,
            }
        }

        trace.reverse();
        Ok(trace)
    }
}

impl ProteinGraph {
    pub fn traverse_varcount(
        &self,
        interval: &Interval,
        max_vars: u8,
    ) -> Result<Traversal> {
let mut arena: Vec<State> = Vec::new();
let mut states_at_node: Vec<Vec<StateId>> = vec![Vec::new(); self.nodes.len() as usize];

// initial state at node 0
arena.push(State {
    node: 0,
    parent: None,
    edge_id: None,
    tv: 0,
    var: 0,
});

states_at_node[0].push(0);

for i in 0..self.nodes.len() as usize - 1 {
    if states_at_node[i].is_empty() {
        continue;
    }
    
    let (e_b, e_e) = if i == 0 {
        (0 as usize, self.nodes[0] as usize)
    } else {
        (self.nodes[i - 1] as usize, self.nodes[i] as usize)
    };

    let current_states = states_at_node[i].clone();  // Clone here

    for state_id in current_states {
        for k in e_b..e_e {
            let target_node = self.edges[k] as u32;

            let (achieved, new_var) = {
                let state = &arena[state_id];
                (
                    state.tv + self.mono_weight[target_node as usize],
                    state.var + self.variant_count[k],
                )
            };
            
            let shifted = Interval {
                lower: interval.lower - achieved,
                upper: interval.upper - achieved,
            };

            let overlap = self.has_overlapping_interval(
                target_node as usize,
                &shifted,
            )?;

            if new_var > max_vars || !overlap {
                continue;
            }

            let new_state_id = arena.len();

            arena.push(State {
                node: target_node,
                parent: Some(state_id),
                edge_id: Some(k),
                tv: achieved,
                var: new_var,
            });

            states_at_node[target_node as usize].push(new_state_id);
        }
    }
    
    states_at_node[i].clear();
}

            Ok(Traversal {
                arena,
                states_at_node,
            })
        }

    pub fn traverse_and_build_traces(
        &self,
        interval: &Interval,
        max_vars: u8,
    ) -> Result<Vec<Vec<(u32, Option<usize>)>>> {
        let traversal = self.traverse_varcount(interval, max_vars)?;

        let final_states =
            &traversal.states_at_node[(self.nodes.len() - 1) as usize];

        let mut traces = Vec::new();

        for &state_id in final_states {
            traces.push(
                traversal.reconstruct_trace(state_id)?
            );
        }

        Ok(traces)
    }
}