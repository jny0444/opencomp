use opencomp_core::{action::Action, computer::Computer, error::OpenCompCoreError, model::Model};

pub struct Agent<C, M> {
    computer: C,
    model: M,
    max_steps: usize,
}

impl<C, M> Agent<C, M> {
    pub fn new(computer: C, model: M, max_steps: usize) -> Self {
        Self {
            computer,
            model,
            max_steps,
        }
    }
}

impl<C: Computer, M: Model> Agent<C, M> {
    pub async fn run(&mut self, task: &str) -> Result<String, OpenCompCoreError> {
        let mut history = Vec::new();

        loop {
            if history.len() >= self.max_steps {
                return Err(OpenCompCoreError::Model(format!(
                    "step limit of {} was hit",
                    self.max_steps
                )));
            }

            let observation = self
                .computer
                .screenshot()
                .await
                .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
            let turn = self.model.next_action(task, &observation, &history).await?;

            tracing::info!(step = history.len() + 1, action = ?turn.action);

            match turn.action {
                Action::Done { result } => return Ok(result),
                action => {
                    history.push(action.clone());
                    let scaled = opencomp_core::scale::scale_action(
                        &action,
                        observation.width,
                        observation.height,
                        observation.screen_width,
                        observation.screen_height,
                    );
                    self.computer.act(&scaled).await?;
                }
            }
        }
    }
}
