use crate::flow::{Flow, FlowError, FlowState, ToolCallSpec};

pub struct FlowRunner {
    flow: Flow,
    current_index: usize,
}

impl FlowRunner {
    pub fn new(flow: Flow) -> Result<Self, FlowError> {
        let current_index = flow
            .states
            .iter()
            .position(|state| state.name == flow.initial_state)
            .ok_or_else(|| FlowError::UnknownInitialState(flow.initial_state.clone()))?;
        Ok(Self {
            flow,
            current_index,
        })
    }

    pub fn flow_name(&self) -> &str {
        &self.flow.name
    }

    pub fn current_state(&self) -> &FlowState {
        &self.flow.states[self.current_index]
    }

    pub fn on_enter_tool_call(&self) -> Option<&ToolCallSpec> {
        self.current_state().on_enter_tool_call.as_ref()
    }

    /// Advances to the state reached by the first transition matching `trigger` from the
    /// current state. Returns `true` if a transition fired, `false` if the current state has
    /// no matching transition (the flow stays put).
    pub fn advance(&mut self, trigger: &str) -> bool {
        let Some(transition) = self
            .current_state()
            .transitions
            .iter()
            .find(|transition| transition.on == trigger)
        else {
            return false;
        };
        let to_state = transition.to_state.clone();
        // `to_state` is guaranteed to exist by `validate_semantics` at parse time.
        self.current_index = self
            .flow
            .states
            .iter()
            .position(|state| state.name == to_state)
            .expect("transition target validated at parse time");
        true
    }
}
