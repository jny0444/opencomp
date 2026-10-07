pub mod anthropic;
mod completions;
pub mod groq;
pub mod openrouter;
pub mod prompt;
pub mod scripted;

pub use anthropic::AnthropicModel;
pub use groq::GroqModel;
pub use openrouter::OpenRouterModel;
pub use scripted::ScriptedModel;
