
// bookkeeping for traversal. parent and edge use MAX as sentiel
// IMPORTANT: in reference implementation, u32::MAX is a valid value

pub type StateId = usize;

const STATE_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct State {
    pub parent: StateId,
    pub node: u32,
    pub edge: u32,
    pub var: u8,
    pub tv: i64,
}

pub enum TraversalStatus {
    Complete(TraversalState),
    Overflow(),
}

pub struct TraversalState {
    pub limit: usize,
    pub arena: Vec<State>,
    pub states_at_node: Vec<Vec<StateId>>,
}

impl TraversalState {
    pub fn new(num_nodes: usize, max_vars: u8, limit: usize) -> Self {
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

        //let final_state_idx = num_nodes-1;

        Self {
            limit,
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
    ) -> bool {
        let len: usize = self.arena.len();

        if self.arena.len()*STATE_SIZE >= self.limit {
            return false;
        }

        self.arena.push(State {
            parent: parent,
            node: node,
            edge: edge,
            var: var,
            tv: tv,
        });

        self.states_at_node[target_node].push(len);

        true
    }
}