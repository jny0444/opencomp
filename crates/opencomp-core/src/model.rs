use crate::{action::Action, error::OpenCompCoreError, observation::Observation};

#[derive(Debug)]
pub struct Turn {
    pub action: Action,
    pub reasoning: Option<String>,
    pub output_bytes: usize,
    pub output_tokens: Option<u32>,
}

pub trait Model {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError>;
}
