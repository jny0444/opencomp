use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Run {
        #[arg(long)]
        task: String,
        #[arg(long, default_value = "scripted")]
        model: String,
        #[arg(long, default_value_t = 25)]
        max_steps: usize,
        #[arg(long, default_value = "desktop")]
        computer: String,
    },
}
