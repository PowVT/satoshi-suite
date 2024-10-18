use bitcoin::Network;

pub struct Config {
    pub network: Network,
    pub bitcoin_rpc_url: String,
    pub bitcoin_rpc_username: String,
    pub bitcoin_rpc_password: String,
    pub create_wallets: bool,
}

impl Config {
    pub fn new(
        network: Network,
        rpc_url: String,
        bitcoin_rpc_username: String,
        bitcoin_rpc_password: String,
        create_wallets: bool,
    ) -> Self {
        let port = match network {
            Network::Bitcoin => 8332,
            Network::Testnet => 18332,
            Network::Regtest => 18443,
            Network::Signet => 38332,
            _ => panic!("Unsupported network"),
        };

        let bitcoin_rpc_url = format!("{}:{}", rpc_url, port);

        let config = Config {
            network,
            bitcoin_rpc_url,
            bitcoin_rpc_username,
            bitcoin_rpc_password,
            create_wallets,
        };

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            Network::Regtest,
            "http://127.0.0.1".to_string(),
            "user".to_string(),
            "password".to_string(),
            true,
        )
    }
}
