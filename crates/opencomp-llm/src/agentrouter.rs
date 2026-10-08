use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};

use crate::completions::{self, Endpoint};

const AGENTROUTER_URL: &str = "https://agentrouter.org";
const USER_AGENT: &str = "claude-cli/1.0.0 (external, cli)";

pub struct AgentRouterModel {
    endpoint: Endpoint,
}

impl AgentRouterModel {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            endpoint: Endpoint {
                name: "agentrouter",
                api_key,
                model,
                client: client(),
                base_url: AGENTROUTER_URL.to_owned(),
            },
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.endpoint.base_url = base_url;
        self
    }
}

impl Model for AgentRouterModel {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        completions::next_action(&self.endpoint, task, observation, history).await
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("agentrouter http client")
}
