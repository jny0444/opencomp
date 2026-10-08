use base64::Engine;
use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::{Model, Turn},
    observation::Observation,
};
use serde::Deserialize;

use crate::completer::{Completer, Reply};
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
        let reply = self.reply(&text, Some(&observation.png)).await?;
        Ok(Turn {
            action: prompt::parse_action(&reply.text)?,
            follow: Vec::new(),
            reasoning: None,
            output_bytes: reply.output_bytes,
            output_tokens: reply.output_tokens,
        })
    }
}

impl Completer for AnthropicModel {
    async fn complete(
        &mut self,
        text: &str,
        png: Option<&[u8]>,
    ) -> Result<Reply, OpenCompCoreError> {
        self.reply(text, png).await
    }
}

impl AnthropicModel {
    async fn reply(&self, text: &str, png: Option<&[u8]>) -> Result<Reply, OpenCompCoreError> {
        let mut content = Vec::new();
        if let Some(png) = png {
            let png = base64::engine::general_purpose::STANDARD.encode(png);
            content.push(serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/png",
                    "data": png,
                }
            }));
        }
        content.push(serde_json::json!({ "type": "text", "text": text }));
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": content
            }]
        });

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        if !status.is_success() {
            return Err(OpenCompCoreError::Model(format!(
                "HTTP {status}: {}",
                body.trim()
            )));
        }
        let output_bytes = body.len();
        let payload: MessagesResponse = serde_json::from_str(&body)
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        let text = payload
            .text()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| OpenCompCoreError::Model("anthropic response had no text".to_owned()))?;
        Ok(Reply {
            text: text.to_owned(),
            output_bytes,
            output_tokens: payload.usage.map(|usage| usage.output_tokens),
        })
    }
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    output_tokens: u32,
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
