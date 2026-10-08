use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};

use crate::completer::{Completer, Reply};
use crate::completions::{self, Endpoint};

const OPENROUTER_URL: &str = "https://openrouter.ai/api";

pub struct OpenRouterModel {
    endpoint: Endpoint,
}

impl OpenRouterModel {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            endpoint: Endpoint {
                name: "openrouter",
                api_key,
                model,
                client: reqwest::Client::new(),
                base_url: OPENROUTER_URL.to_owned(),
            },
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.endpoint.base_url = base_url;
        self
    }
}

impl Model for OpenRouterModel {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        completions::next_action(&self.endpoint, task, observation, history).await
    }
}

impl Completer for OpenRouterModel {
    async fn complete(
        &mut self,
        text: &str,
        png: Option<&[u8]>,
    ) -> Result<Reply, OpenCompCoreError> {
        completions::complete(&self.endpoint, text, png).await
    }
}
