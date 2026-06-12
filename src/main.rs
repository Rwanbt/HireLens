mod cli;
mod core;
mod export;
mod llm;
mod parser;
mod utils;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    let args = cli::Args::parse();
    cli::run(args).await
}
