use std::path::PathBuf;

use bitcoin::Network;

#[derive(Clone, Debug)]
pub enum BitcoinRpcConfig {
    // Built-in Bitcoin Core configuration
    Internal {
        network: Network,
        rpc_url: String,
        rpc_username: String,
        rpc_password: String,
        data_dir: PathBuf,
    },
    // External Bitcoin Core configuration
    External {
        network: Network,
        rpc_url: String,
        rpc_username: String,
        rpc_password: String,
        cookie_file: Option<PathBuf>,
    },
}

impl BitcoinRpcConfig {
    pub fn network(&self) -> Network {
        match self {
            BitcoinRpcConfig::Internal { network, .. } => *network,
            BitcoinRpcConfig::External { network, .. } => *network,
        }
    }

    pub fn rpc_url(&self) -> &str {
        match self {
            BitcoinRpcConfig::Internal { rpc_url, .. } => rpc_url,
            BitcoinRpcConfig::External { rpc_url, .. } => rpc_url,
        }
    }

    pub fn auth(&self) -> bitcoincore_rpc::Auth {
        match self {
            BitcoinRpcConfig::Internal {
                rpc_username,
                rpc_password,
                ..
            } => bitcoincore_rpc::Auth::UserPass(rpc_username.clone(), rpc_password.clone()),
            BitcoinRpcConfig::External {
                rpc_username,
                rpc_password,
                cookie_file,
                ..
            } => {
                if let Some(cookie_path) = cookie_file {
                    bitcoincore_rpc::Auth::CookieFile(cookie_path.clone())
                } else {
                    bitcoincore_rpc::Auth::UserPass(rpc_username.clone(), rpc_password.clone())
                }
            }
        }
    }

    pub fn format_url(&self, wallet_name: Option<&str>) -> String {
        let base_url = self.rpc_url();
        match wallet_name {
            None => base_url.to_string(),
            Some(name) => format!("{}/wallet/{}", base_url, name),
        }
    }

    pub fn data_dir(&self) -> Option<&PathBuf> {
        match self {
            BitcoinRpcConfig::Internal { data_dir, .. } => Some(data_dir),
            BitcoinRpcConfig::External { .. } => None,
        }
    }
}

pub struct Config {
    pub bitcoin_rpc: BitcoinRpcConfig,
    pub create_wallets: bool,
}

impl Config {
    pub fn new_internal(
        network: Network,
        rpc_url: String,
        rpc_username: String,
        rpc_password: String,
        data_dir: PathBuf,
        create_wallets: bool,
    ) -> Self {
        let port = match network {
            Network::Bitcoin => 8332,
            Network::Testnet => 18332,
            Network::Regtest => 18443,
            Network::Signet => 38332,
            _ => panic!("Unsupported network"),
        };

        let bitcoin_rpc = BitcoinRpcConfig::Internal {
            network,
            rpc_url: format!("{}:{}", rpc_url, port),
            rpc_username,
            rpc_password,
            data_dir,
        };

        Config {
            bitcoin_rpc,
            create_wallets,
        }
    }

    pub fn new_external(
        network: Network,
        rpc_url: String,
        rpc_username: Option<String>,
        rpc_password: Option<String>,
        cookie_file: Option<PathBuf>,
        create_wallets: bool,
    ) -> Self {
        let port = match network {
            Network::Bitcoin => 8332,
            Network::Testnet => 18332,
            Network::Regtest => 18443,
            Network::Signet => 38332,
            _ => panic!("Unsupported network"),
        };

        let bitcoin_rpc = BitcoinRpcConfig::External {
            network,
            rpc_url: format!("{}:{}", rpc_url, port),
            rpc_username: rpc_username.unwrap_or_default(),
            rpc_password: rpc_password.unwrap_or_default(),
            cookie_file,
        };

        Config {
            bitcoin_rpc,
            create_wallets,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new_internal(
            Network::Regtest,
            "http://127.0.0.1".to_string(),
            "user".to_string(),
            "password".to_string(),
            PathBuf::from("./data/bitcoin"),
            true,
        )
    }
}

pub fn config_to_network(config: &Config) -> Network {
    config.bitcoin_rpc.network()
}
