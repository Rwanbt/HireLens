mod auth;
mod cli;
mod core;
mod export;
mod gui;
mod llm;
mod parser;
mod utils;
mod web;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    if args.is_gui() {
        return gui::run();
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(cli::run(args))
}
