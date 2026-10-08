use std::time::Instant;

use opencomp_core::{
    action::Action, computer::Computer, error::OpenCompCoreError, model::Model,
    observation::Observation,
};

use crate::policy;

pub struct Agent<C, M> {
    computer: C,
    model: M,
    max_steps: usize,
    frame_hash: fn(&[u8]) -> u64,
}

impl<C, M> Agent<C, M> {
    pub fn new(computer: C, model: M, max_steps: usize) -> Self {
        Self {
            computer,
            model,
            max_steps,
            frame_hash: hash_bytes,
        }
    }

    pub fn with_frame_hash(mut self, frame_hash: fn(&[u8]) -> u64) -> Self {
        self.frame_hash = frame_hash;
        self
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

            let step = history.len() + 1;
            let started = Instant::now();
            let observation = self
                .computer
                .screenshot()
                .await
                .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
            let capture_ms = started.elapsed().as_millis();
            let before = (self.frame_hash)(&observation.png);

            let started = Instant::now();
            let turn = self.model.next_action(task, &observation, &history).await?;
            let model_ms = started.elapsed().as_millis();

            let action = turn.action;
            if let Action::Done { result } = &action {
                log_step(
                    step,
                    &observation,
                    capture_ms,
                    model_ms,
                    0,
                    false,
                    &action,
                    turn.output_bytes,
                    turn.output_tokens,
                );
                return Ok(result.clone());
            }
            if let Err(error) = policy::check(&action, observation.width, observation.height) {
                tracing::warn!(
                    step,
                    action = ?action,
                    %error,
                    "refused action"
                );
                log_step(
                    step,
                    &observation,
                    capture_ms,
                    model_ms,
                    0,
                    false,
                    &action,
                    turn.output_bytes,
                    turn.output_tokens,
                );
                return Err(error);
            }
            history.push(action.clone());
            let scaled = opencomp_core::scale::scale_action(
                &action,
                observation.width,
                observation.height,
                observation.screen_width,
                observation.screen_height,
            );
            let started = Instant::now();
            self.computer.act(&scaled).await?;
            let act_ms = started.elapsed().as_millis();
            let frame_changed = match self.computer.screenshot().await {
                Ok(after) => (self.frame_hash)(&after.png) != before,
                Err(error) => {
                    tracing::warn!(step, %error, "could not hash the frame after act");
                    false
                }
            };
            log_step(
                step,
                &observation,
                capture_ms,
                model_ms,
                act_ms,
                frame_changed,
                &action,
                turn.output_bytes,
                turn.output_tokens,
            );
        }
    }
}

fn log_step(
    step: usize,
    observation: &Observation,
    capture_ms: u128,
    model_ms: u128,
    act_ms: u128,
    frame_changed: bool,
    action: &Action,
    output_bytes: usize,
    output_tokens: Option<u32>,
) {
    let output_tokens = output_tokens
        .map(|count| count.to_string())
        .unwrap_or_else(|| "-".to_owned());
    tracing::info!(
        step,
        capture_ms = capture_ms as u64,
        image_bytes = observation.png.len(),
        width = observation.width,
        height = observation.height,
        model_ms = model_ms as u64,
        output_bytes,
        output_tokens = %output_tokens,
        act_ms = act_ms as u64,
        frame_changed,
        action = ?action,
    );
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
