mod cli;
mod demo;

use clap::Parser;
use cli::{Cli, Command};
use opencomp_agent::Agent;
use opencomp_computer::Desktop;
use opencomp_llm::ScriptedModel;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let Cli { command } = Cli::parse();
    let Command::Run {
        task,
        model,
        max_steps,
    } = command;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if model != "scripted" {
        eprintln!("unknown model `{model}`; expected `scripted`");
        std::process::exit(1);
    }

    let computer = match Desktop::new() {
        Ok(computer) => computer,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let model = ScriptedModel::new(demo::script());
    let mut agent = Agent::new(computer, model, max_steps);

    match agent.run(&task).await {
        Ok(result) => println!("{result}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
