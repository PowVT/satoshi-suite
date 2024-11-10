use std::error::Error;

use clap::Parser;

use tracing_subscriber::EnvFilter;

pub mod cli;
use cli::Cli;

mod commands;
use commands::handler;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let args = Cli::parse();

    let config = args.options.make_config();
    handler(&args, &config)
}
