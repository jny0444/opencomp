use crate::{action::Action, error::OpenCompCoreError, observation::Observation};

pub trait Computer {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError>;
    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError>;
}
