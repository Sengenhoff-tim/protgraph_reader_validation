use serde::{Serialize};
use serde::ser::SerializeTuple;
// bookkeeping for traversal. parent and edge use MAX as sentiel
// IMPORTANT: in reference implementation, u32::MAX is a valid value

pub type StateId = usize;

#[derive(Debug, Clone)]
pub struct State {
    pub parent: StateId,
    pub node: u32,
    pub edge: u32,
    pub var: u8,
    pub tv: i64,
}



pub struct TraversalState {
    pub final_state_idx: usize,
    pub arena: Vec<State>,
    pub states_at_node: Vec<Vec<StateId>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerializableState {
    pub node: u32,
    pub edge: u32,
    pub parent: StateId,
}

#[derive(Debug, Clone)]
pub struct SerializedTraversalState {
    pub arena: Vec<SerializableState>,
    pub final_states: Vec<StateId>,
}

impl Serialize for State {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut t = serializer.serialize_tuple(3)?;
        t.serialize_element(&self.node)?;
        t.serialize_element(&self.edge)?;
        t.serialize_element(&self.parent)?;
        t.end()
    }
}

impl TraversalState {
    pub fn new(num_nodes: usize, max_vars: u8) -> Self {
        let mut arena = Vec::with_capacity(num_nodes*max_vars as usize);
        let mut states_at_node = vec![Vec::new(); num_nodes];

        arena.push(State {
            parent: usize::MAX,
            node: 0,
            edge: u32::MAX,
            var: 0,
            tv: 0,
        });

        states_at_node[0].push(0 as StateId);

        let final_state_idx = num_nodes-1;

        Self {
            final_state_idx,
            arena,
            states_at_node,
        }
    }

    pub fn reconstruct_trace( &self, mut state_id: StateId, ) -> Vec<(u32, u32)> { 
        let mut trace = Vec::new(); 
        
        while state_id != usize::MAX { 
            let state = &self.arena[state_id]; 
            trace.push((state.node, state.edge)); state_id = state.parent; 
        } 
        
        trace.reverse(); 
        trace 
    }

    #[inline]
    pub fn push_state(
        &mut self,
        parent: StateId,
        node: u32,
        edge: u32,
        var: u8,
        tv: i64,
        target_node: usize,
    ) -> () {
        let id = self.arena.len();

        self.arena.push(State {
            parent: parent,
            node: node,
            edge: edge,
            var: var,
            tv: tv,
        });

        self.states_at_node[target_node].push(id);
    }
}
/* 
    impl Serialize for TraversalState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("TraversalState", 2)?;

        let mut seq = serializer.serialize_seq(Some(self.arena.len()))?;

        for s in &self.arena {
            seq.serialize_element(&(
                s.node,
                s.edge,
                s.parent,
            ))?;
        }

        seq.end()?;

        st.serialize_field("arena", &seq)?;
        st.serialize_field("states_at_node", &self.states_at_node)?;

        st.end()
    }
}
}
*/

impl SerializedTraversalState {
    pub fn reconstruct_trace(&self, mut state_id: StateId) -> Vec<(u32, u32)> {

    let mut trace = Vec::new();

    while state_id != usize::MAX {
        let state = &self.arena[state_id];
        trace.push((state.node, state.edge));
        state_id = state.parent;
    }

    trace.reverse();
    trace
    }
}