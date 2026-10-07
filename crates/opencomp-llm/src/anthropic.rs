use base64::Engine;
use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};
use serde::Deserialize;

use crate::prompt;

const ANTHROPIC_URL: &str = "https://api.anthropic.com";

pub struct AnthropicModel {
    api_key: String,
    model: String,
    client: reqwest::Client,
    base_url: String,
}

impl AnthropicModel {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
            base_url: ANTHROPIC_URL.to_owned(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

impl Model for AnthropicModel {
    async fn next_action(
        &mut self,
        task: &str,
        observation: &Observation,
        history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        let text = prompt::user_text(task, observation.width, observation.height, history)?;
        let png = base64::engine::general_purpose::STANDARD.encode(&observation.png);
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": png,
                        }
                    },
                    { "type": "text", "text": text }
                ]
            }]
        });

        let url = format!(
            "{}/v1/messages",
            self.base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        let response = response
            .error_for_status()
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        let payload: MessagesResponse = response
            .json()
            .await
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        let text = payload.text().ok_or_else(|| {
            OpenCompCoreError::Model("anthropic response had no text".to_owned())
        })?;
        let action = prompt::parse_action(text)?;
        Ok(Turn {
            action,
            reasoning: None,
        })
    }
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

impl MessagesResponse {
    fn text(&self) -> Option<&str> {
        self.content.iter().find_map(|block| {
            if block.kind == "text" {
                block.text.as_deref()
            } else {
                None
            }
        })
    }
}
