use std::{error::Error, fmt};

use bitcoincore_rpc::{Client, Error as RpcError};

use satoshi_suite_config::Config;

#[derive(Debug)]
pub enum ClientError {
    CannotConnect(RpcError),
    InvalidConfiguration(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::CannotConnect(err) => write!(f, "Cannot connect to Bitcoin Core: {}", err),
            ClientError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::CannotConnect(err) => Some(err),
            ClientError::InvalidConfiguration(_) => None,
        }
    }
}

impl From<RpcError> for ClientError {
    fn from(err: RpcError) -> Self {
        ClientError::CannotConnect(err)
    }
}

pub fn create_rpc_client(
    config: &Config,
    wallet_name: Option<&str>,
) -> Result<Client, ClientError> {
    let url = config.bitcoin_rpc.format_url(wallet_name);
    let auth = config.bitcoin_rpc.auth();

    Client::new(&url, auth).map_err(ClientError::CannotConnect)
}
