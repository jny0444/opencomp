mod cli;
mod demo;
mod spotlight;

use clap::Parser;
use cli::{Cli, Command};
use opencomp_agent::Agent;
use opencomp_browser::Session;
use opencomp_computer::Desktop;
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::model::Model;
use opencomp_llm::{
    AgentRouterModel, AnthropicModel, GroqModel, OpenRouterModel, ScriptedModel, SplitModel,
};
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
            Ok(mut computer) => {
                let task = match spotlight::prepare(&mut computer, &task).await {
                    Ok(task) => task,
                    Err(error) => {
                        report(Err(error));
                        return;
                    }
                };
                run(computer, &task, &model, max_steps).await
            }
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
            let mut agent = measured(computer, ScriptedModel::new(demo::script()), max_steps);
            agent.run(task).await
        }
        "anthropic" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("ANTHROPIC_API_KEY");
            let model_name =
                std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-5".to_owned());
            let mut agent = measured(
                computer,
                SplitModel::new(
                    AnthropicModel::new(api_key.clone(), model_name.clone()),
                    AnthropicModel::new(api_key, grounder_model(&model_name)),
                    opencomp_vision::frame_hash,
                ),
                max_steps,
            );
            agent.run(task).await
        }
        "groq" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("GROQ_API_KEY");
            let model_name =
                std::env::var("GROQ_MODEL").unwrap_or_else(|_| "qwen/qwen3.8-27b".to_owned());
            let mut agent = measured(
                computer,
                SplitModel::new(
                    GroqModel::new(api_key.clone(), model_name.clone()),
                    GroqModel::new(api_key, grounder_model(&model_name)),
                    opencomp_vision::frame_hash,
                ),
                max_steps,
            );
            agent.run(task).await
        }
        "openrouter" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("OPENROUTER_API_KEY");
            let model_name =
                std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "qwen/qwen3.8-27b".to_owned());
            let mut agent = measured(
                computer,
                SplitModel::new(
                    OpenRouterModel::new(api_key.clone(), model_name.clone()),
                    OpenRouterModel::new(api_key, grounder_model(&model_name)),
                    opencomp_vision::frame_hash,
                ),
                max_steps,
            );
            agent.run(task).await
        }
        "agentrouter" => {
            dotenvy::dotenv().ok();
            let api_key = require_env("AGENTROUTER_API_KEY");
            let model_name =
                std::env::var("AGENTROUTER_MODEL").unwrap_or_else(|_| "gpt-4o".to_owned());
            let mut agent = measured(
                computer,
                SplitModel::new(
                    AgentRouterModel::new(api_key.clone(), model_name.clone()),
                    AgentRouterModel::new(api_key, grounder_model(&model_name)),
                    opencomp_vision::frame_hash,
                ),
                max_steps,
            );
            agent.run(task).await
        }
        _ => {
            eprintln!(
                "unknown model `{model}`; expected `scripted`, `anthropic`, `groq`, `openrouter`, or `agentrouter`"
            );
            std::process::exit(1);
        }
    }
}

fn grounder_model(planner: &str) -> String {
    match std::env::var("GROUNDER_MODEL") {
        Ok(value) if !value.is_empty() => value,
        _ => planner.to_owned(),
    }
}

fn measured<C: Computer, M: Model>(computer: C, model: M, max_steps: usize) -> Agent<C, M> {
    Agent::new(computer, model, max_steps).with_frame_hash(opencomp_vision::frame_hash)
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
