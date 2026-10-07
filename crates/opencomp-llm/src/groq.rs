use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};

use crate::completions::{self, Endpoint};

const GROQ_URL: &str = "https://api.groq.com/openai";

pub struct GroqModel {
    endpoint: Endpoint,
}

impl GroqModel {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            endpoint: Endpoint {
                name: "groq",
                api_key,
                model,
                client: reqwest::Client::new(),
                base_url: GROQ_URL.to_owned(),
            },
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.endpoint.base_url = base_url;
        self
    }
}

impl Model for GroqModel {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        completions::next_action(&self.endpoint, task, observation, history).await
    }
}
