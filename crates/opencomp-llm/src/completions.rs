use base64::Engine;
use image::ImageEncoder;
use opencomp_core::{
    action::Action,
    error::OpenCompCoreError,
    model::Turn,
    observation::Observation,
};
use serde::Deserialize;

use crate::prompt;

pub(crate) struct Endpoint {
    pub name: &'static str,
    pub api_key: String,
    pub model: String,
    pub client: reqwest::Client,
    pub base_url: String,
}

pub(crate) async fn next_action(
    endpoint: &Endpoint,
    task: &str,
    observation: &Observation,
    history: &[Action],
) -> Result<Turn, OpenCompCoreError> {
    let text = prompt::user_text(task, observation.width, observation.height, history)?;
    let image = image_data_url(&observation.png)?;
    let body = serde_json::json!({
        "model": endpoint.model,
        "max_tokens": 1024,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": text },
                {
                    "type": "image_url",
                    "image_url": { "url": image }
                }
            ]
        }]
    });

    let url = format!(
        "{}/v1/chat/completions",
        endpoint.base_url.trim_end_matches('/')
    );
    let response = endpoint
        .client
        .post(url)
        .bearer_auth(&endpoint.api_key)
        .header("x-title", "opencomp")
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
        return Err(OpenCompCoreError::Model(http_failure(status, &body)));
    }
    let payload: ChatResponse = serde_json::from_str(&body)
        .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
    if let Some(message) = payload.error_message() {
        return Err(OpenCompCoreError::Model(message));
    }
    let text = payload.text().ok_or_else(|| {
        OpenCompCoreError::Model(format!("{} response had no text", endpoint.name))
    })?;
    Ok(Turn {
        action: prompt::parse_action(text)?,
        reasoning: None,
    })
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    error: Option<ProviderError>,
}

#[derive(Deserialize)]
struct ProviderError {
    message: String,
}

fn http_failure(status: reqwest::StatusCode, body: &str) -> String {
    let detail = provider_message(body).unwrap_or_else(|| body.trim().to_owned());
    if detail.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status}: {detail}")
    }
}

fn provider_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(value.get("error")?.get("message")?.as_str()?.to_owned())
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: Option<MessageContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Deserialize)]
struct ContentPart {
    text: Option<String>,
}

const JPEG_BYTE_BUDGET: usize = 700 * 1024;

fn image_data_url(bytes: &[u8]) -> Result<String, OpenCompCoreError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let jpeg = jpeg_under_budget(bytes)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(jpeg);
        return Ok(format!("data:image/jpeg;base64,{encoded}"));
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

fn jpeg_under_budget(png: &[u8]) -> Result<Vec<u8>, OpenCompCoreError> {
    let image = image::load_from_memory(png)
        .map_err(|error| OpenCompCoreError::Model(error.to_string()))?
        .into_rgb8();
    let mut jpeg = encode_jpeg(&image, 75)?;
    if jpeg.len() > JPEG_BYTE_BUDGET {
        jpeg = encode_jpeg(&image, 60)?;
    }
    if jpeg.len() > JPEG_BYTE_BUDGET {
        jpeg = encode_jpeg(&image, 45)?;
    }
    Ok(jpeg)
}

fn encode_jpeg(image: &image::RgbImage, quality: u8) -> Result<Vec<u8>, OpenCompCoreError> {
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
    Ok(jpeg.into_inner())
}

impl ChatResponse {
    fn error_message(&self) -> Option<String> {
        if !self.choices.is_empty() {
            return None;
        }
        self.error.as_ref().map(|error| error.message.clone())
    }

    fn text(&self) -> Option<&str> {
        let content = self.choices.first()?.message.content.as_ref()?;
        match content {
            MessageContent::Text(text) => Some(text.as_str()),
            MessageContent::Parts(parts) => parts.iter().find_map(|part| part.text.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use super::image_data_url;

    #[test]
    fn a_non_png_stays_a_png_data_url() {
        let url = image_data_url(&[0]).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn a_png_is_sent_as_jpeg_under_the_budget() {
        let mut image = image::RgbaImage::new(1280, 800);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgba([
                (x % 48) as u8,
                (y % 48) as u8,
                ((x / 8 + y / 8) % 255) as u8,
                255,
            ]);
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();

        let url = image_data_url(&png.into_inner()).unwrap();
        let encoded = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let jpeg = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .unwrap();
        assert!(jpeg.len() <= super::JPEG_BYTE_BUDGET, "{}", jpeg.len());
        assert_eq!(&jpeg[..2], b"\xff\xd8");
    }
}
