mod cli;
mod demo;

use clap::Parser;
use cli::{Cli, Command};
use opencomp_agent::Agent;
use opencomp_browser::Session;
use opencomp_computer::Desktop;
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_llm::{AnthropicModel, GroqModel, OpenRouterModel, ScriptedModel};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let Cli { command } = Cli::parse();
    let Command::Run {
        task,
        model,
        max_steps,
        computer,
    } = command;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let result = match computer.as_str() {
        "desktop" => match Desktop::new() {
            Ok(computer) => run(computer, &task, &model, max_steps).await,
            Err(error) => Err(error),
        },
        "browser" => match Session::launch().await {
            Ok(computer) => run(computer, &task, &model, max_steps).await,
            Err(error) => Err(error),
        },
        _ => {
            eprintln!("unknown computer `{computer}`; expected `desktop` or `browser`");
            std::process::exit(1);
        }
    };

    report(result);
}

async fn run<C: Computer>(
    computer: C,
    task: &str,
    model: &str,
    max_steps: usize,
) -> Result<String, OpenCompCoreError> {
    match model {
        "scripted" => {
            let mut agent = Agent::new(computer, ScriptedModel::new(demo::script()), max_steps);
            agent.run(task).await
        }
        "anthropic" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("ANTHROPIC_API_KEY");
            let model_name =
                std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-5".to_owned());
            let mut agent = Agent::new(
                computer,
                AnthropicModel::new(api_key, model_name),
                max_steps,
            );
            agent.run(task).await
        }
        "groq" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("GROQ_API_KEY");
            let model_name =
                std::env::var("GROQ_MODEL").unwrap_or_else(|_| "qwen/qwen3.8-27b".to_owned());
            let mut agent = Agent::new(computer, GroqModel::new(api_key, model_name), max_steps);
            agent.run(task).await
        }
        "openrouter" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("OPENROUTER_API_KEY");
            let model_name =
                std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "qwen/qwen3.8-27b".to_owned());
            let mut agent = Agent::new(
                computer,
                OpenRouterModel::new(api_key, model_name),
                max_steps,
            );
            agent.run(task).await
        }
        _ => {
            eprintln!(
                "unknown model `{model}`; expected `scripted`, `anthropic`, `groq`, or `openrouter`"
            );
            std::process::exit(1);
        }
    }
}

fn require_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!("{name} is not set");
            std::process::exit(1);
        }
    }
}

fn report(result: Result<String, OpenCompCoreError>) {
    match result {
        Ok(result) => println!("{result}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
