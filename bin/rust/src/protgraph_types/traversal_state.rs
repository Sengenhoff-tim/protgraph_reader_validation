// bookkeeping for traversal. parent and edge use MAX as sentiel
// IMPORTANT: in reference implementation, u32::MAX is a valid value

type StateId = usize;

#[derive(Debug, Clone)]
pub struct State {
    node: u32,
    edge: u32,
    pub var: u8,
    parent: StateId,
    pub tv: i64,
}

pub struct TraversalState {
    pub arena: Vec<State>,
    pub states_at_node: Vec<Vec<StateId>>,
}

impl TraversalState {
    pub fn new(num_nodes: usize, max_vars: u8) -> Self {
        let mut arena = Vec::with_capacity(num_nodes*max_vars as usize);
        let mut states_at_node = vec![Vec::new(); num_nodes];

        arena.push(State {
            node: 0,
            parent: usize::MAX,
            edge: u32::MAX,
            tv: 0,
            var: 0,
        });

        states_at_node[0].push(0 as StateId);

        Self {
            arena,
            states_at_node,
        }
    }

    pub fn reconstruct_trace(
        &self,
        mut state_id: StateId,
    ) -> Vec<(u32, u32)> {
        let mut trace = Vec::new();

        while state_id != usize::MAX {
            let state = &self.arena[state_id];
            trace.push((state.node, state.edge));
            state_id = state.parent;
        }

        trace.reverse();
        trace
    }

    #[inline]
    pub fn push_state(
        &mut self,
        node: u32,
        parent: StateId,
        edge: u32,
        tv: i64,
        var: u8,
        target_node: usize,
    ) -> () {
        let id = self.arena.len();

        self.arena.push(State {
            node,
            parent,
            edge,
            tv,
            var,
        });

        self.states_at_node[target_node].push(id);
    }
}