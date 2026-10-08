use opencomp_core::{
    action::Action,
    model::{Model, Turn},
};

pub struct ScriptedModel {
    actions: Vec<Action>,
}

impl ScriptedModel {
    pub fn new(actions: Vec<Action>) -> Self {
        Self { actions }
    }
}

impl Model for ScriptedModel {
    async fn next_action(
        &mut self,
        _task: &str,
        _observation: &opencomp_core::observation::Observation,
        _history: &[Action],
    ) -> Result<opencomp_core::model::Turn, opencomp_core::error::OpenCompCoreError> {
        if self.actions.is_empty() {
            return Err(opencomp_core::error::OpenCompCoreError::Model(
                "no scripted action left".to_owned(),
            ));
        }

        let action = self.actions.remove(0);
        Ok(Turn {
            action,
            reasoning: None,
        })
    }
}
