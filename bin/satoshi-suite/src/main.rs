use std::error::Error;

use clap::Parser;

pub mod cli;
use cli::Cli;

mod commands;
use commands::handler;

use satoshi_suite_config::Config;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Cli::parse();

    let config = Config::new(
        args.network,
        args.rpc_url.clone(),
        args.rpc_username.clone(),
        args.rpc_password.clone(),
        args.create_wallets,
    );

    handler(&args, &config)
}
