use std::{error::Error, fmt};

use bitcoincore_rpc::{Auth, Client, Error as RpcError};

use satoshi_suite_config::Config;

#[derive(Debug)]
pub enum ClientError {
    CannotConnect(RpcError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::CannotConnect(err) => write!(f, "Cannot connect to Bitcoin Core: {}", err),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::CannotConnect(err) => Some(err),
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
    // TODO: allow for other authentication
    let auth = Auth::UserPass(
        config.bitcoin_rpc_username.clone(),
        config.bitcoin_rpc_password.clone(),
    );

    // let auth = bitcoincore_rpc::Auth::CookieFile("/Users/alex/Library/Application Support/Bitcoin/regtest/.cookie".to_string().parse().unwrap());

    let url = match wallet_name {
        None => format!("{}", config.bitcoin_rpc_url),
        Some(name) => format!("{}/wallet/{}", config.bitcoin_rpc_url, name),
    };

    Client::new(&url, auth.clone()).map_err(ClientError::CannotConnect)
}
