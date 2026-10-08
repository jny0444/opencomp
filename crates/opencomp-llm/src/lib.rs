pub mod agentrouter;
pub mod anthropic;
mod completer;
mod completions;
pub mod groq;
pub mod openrouter;
pub mod prompt;
pub mod scripted;
pub mod split;

pub use agentrouter::AgentRouterModel;
pub use anthropic::AnthropicModel;
pub use completer::{Completer, Reply};
pub use groq::GroqModel;
pub use openrouter::OpenRouterModel;
pub use scripted::ScriptedModel;
pub use split::SplitModel;
