use bitcoin::amount::Denomination::Bitcoin;
use bitcoin::{Amount, Network};
use bitcoincore_rpc::json::AddressType;
use clap::{Parser, Subcommand};

use satoshi_suite_utxo_selection::UTXOStrategy;

#[derive(Parser, Debug)]
#[command(name = "satoshi-suite")]
#[command(about = "Bitcoin wallet and transaction management suite", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub options: Options,
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Parser, Debug)]
pub struct Options {
    /// Network to use (mainnet, testnet, regtest)
    #[arg(long, value_parser = parse_network_type, default_value = "regtest")]
    pub network: Network,

    /// RPC URL for Bitcoin Core
    #[arg(long, default_value = "http://127.0.0.1")]
    pub rpc_url: String,

    /// RPC Username for Bitcoin Core
    #[arg(long, default_value = "user")]
    pub rpc_username: String,

    /// RPC Password for Bitcoin Core
    #[arg(long, default_value = "password")]
    pub rpc_password: String,

    /// Whether to create wallets if they don't exist
    #[arg(long, default_value = "true")]
    pub create_wallets: bool,
}

#[derive(Subcommand, Debug)]
pub enum Action {
    /// Bootstrap a testing environment
    BootstrapEnv {
        /// Address type for generated addresses
        #[arg(short='z', long, value_parser = parse_address_type, default_value = "bech32")]
        address_type: AddressType,
    },

    /// Get the current block height
    GetBlockHeight,

    /// Create a new wallet
    NewWallet {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
    },

    /// Create a new multisig wallet
    NewMultisig {
        /// List of wallet names to use in multisig
        #[arg(short = 'v', long, value_delimiter = ',')]
        wallet_names: Vec<String>,
        /// Number of required signatures
        #[arg(short = 'n', long)]
        nrequired: u32,
        /// Name for the multisig wallet
        #[arg(short = 'm', long)]
        multisig_name: String,
    },

    /// Get wallet information
    GetWalletInfo {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
    },

    /// List wallet descriptors
    ListDescriptors {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
    },

    /// Get a new address from wallet
    GetNewAddress {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Address type
        #[arg(short='z', long, value_parser = parse_address_type, default_value = "bech32")]
        address_type: AddressType,
    },

    /// Get address information
    GetAddressInfo {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Address to get info for
        #[arg(short = 'a', long)]
        address: String,
    },

    /// Derive addresses from a descriptor
    DeriveAddresses {
        /// Descriptor to derive from
        #[arg(short = 'd', long)]
        descriptor: String,
        /// Start index
        #[arg(short = 's', long, default_value = "0")]
        start: u32,
        /// End index
        #[arg(short = 'e', long, default_value = "1")]
        end: u32,
    },

    /// Rescan the blockchain
    RescanBlockchain {
        /// Start height for rescan
        #[arg(short = 's', long, default_value = "0")]
        start: u32,
    },

    /// Get wallet balance
    GetBalance {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
    },

    /// List unspent transactions
    ListUnspent {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
    },

    /// Get transaction information
    GetTx {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Transaction ID
        #[arg(short = 'i', long)]
        txid: String,
    },

    /// Get transaction output information
    GetTxOut {
        /// Transaction ID
        #[arg(short = 'i', long)]
        txid: String,
        /// Output index
        #[arg(short = 'o', long, default_value = "0")]
        vout: u32,
    },

    /// Send BTC to an address
    SendBtc {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Recipient address
        #[arg(short = 'r', long)]
        recipient: String,
        /// Amount to send
        #[arg(short='x', long, value_parser = parse_amount)]
        amount: Amount,
    },

    /// Sign a transaction
    SignTx {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Recipient address
        #[arg(short = 'r', long)]
        recipient: String,
        /// Amount to send
        #[arg(short='x', long, value_parser = parse_amount)]
        amount: Amount,
        /// Fee amount
        #[arg(short='f', long, value_parser = parse_amount)]
        fee_amount: Amount,
        /// UTXO selection strategy
        #[arg(short='y', long, value_parser = parse_utxo_strategy, default_value = "fifo")]
        utxo_strat: UTXOStrategy,
    },

    /// Decode a raw transaction
    DecodeRawTx {
        /// Raw transaction hex
        #[arg(short = 't', long)]
        tx_hex: String,
    },

    /// Verify a signed transaction
    VerifySignedTx {
        /// Signed transaction hex
        #[arg(short = 't', long)]
        tx_hex: String,
    },

    /// Broadcast a transaction
    BroadcastTx {
        /// Transaction hex to broadcast
        #[arg(short = 't', long)]
        tx_hex: String,
    },

    /// Create a PSBT
    CreatePsbt {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Recipient address
        #[arg(short = 'r', long)]
        recipient: String,
        /// Amount to send
        #[arg(short='x', long, value_parser = parse_amount)]
        amount: Amount,
        /// Fee amount
        #[arg(short='f', long, value_parser = parse_amount)]
        fee_amount: Amount,
        /// UTXO selection strategy
        #[arg(short='y', long, value_parser = parse_utxo_strategy, default_value = "fifo")]
        utxo_strat: UTXOStrategy,
    },

    /// Process a PSBT
    ProcessPsbt {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// PSBT hex
        #[arg(short = 'p', long)]
        psbt_hex: String,
    },

    /// Decode a PSBT
    DecodePsbt {
        /// PSBT hex
        #[arg(short = 'p', long)]
        psbt_hex: String,
    },

    /// Analyze a PSBT
    AnalyzePsbt {
        /// PSBT hex
        #[arg(short = 'p', long)]
        psbt_hex: String,
    },

    /// Combine multiple PSBTs
    CombinePsbts {
        /// List of PSBT hexes
        #[arg(short = 'l', long, value_delimiter = ',')]
        psbts: Vec<String>,
    },

    /// Finalize a PSBT
    FinalizePsbt {
        /// PSBT hex
        #[arg(short = 'p', long)]
        psbt_hex: String,
    },

    /// Finalize and broadcast a PSBT
    FinalizePsbtAndBroadcast {
        /// PSBT hex
        #[arg(short = 'p', long)]
        psbt_hex: String,
    },

    /// Inscribe an ordinal
    InscribeOrdinal {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Postage amount in sats
        #[arg(short = 'p', long, default_value = "10000")]
        postage: u64,
        /// File path for inscription
        #[arg(short = 'f', long)]
        file_path: String,
    },

    /// Etch a rune
    EtchRune {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Postage amount in sats
        #[arg(short = 'p', long, default_value = "10000")]
        postage: u64,
        /// File path for etching data
        #[arg(short = 'f', long)]
        file_path: String,
    },

    /// Mine blocks
    MineBlocks {
        /// Name of the wallet
        #[arg(short = 'w', long, default_value = "default_wallet")]
        wallet_name: String,
        /// Number of blocks to mine
        #[arg(short = 'b', long, default_value = "1")]
        blocks: u64,
        /// Address type for coinbase
        #[arg(short='z', long, value_parser = parse_address_type, default_value = "bech32")]
        address_type: AddressType,
    },
}

fn parse_amount(s: &str) -> Result<Amount, &'static str> {
    Amount::from_str_in(s, Bitcoin).map_err(|_| "invalid amount")
}

fn parse_network_type(s: &str) -> Result<Network, &'static str> {
    match s {
        "regtest" => Ok(Network::Regtest),
        "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Bitcoin),
        _ => Err("Unknown network type"),
    }
}

fn parse_address_type(s: &str) -> Result<AddressType, &'static str> {
    match s {
        "legacy" => Ok(AddressType::Legacy),
        "p2sh-segwit" => Ok(AddressType::P2shSegwit),
        "bech32" => Ok(AddressType::Bech32),
        "bech32m" => Ok(AddressType::Bech32m),
        _ => Err("Unknown address type"),
    }
}

fn parse_utxo_strategy(s: &str) -> Result<UTXOStrategy, &'static str> {
    match s {
        "branch-and-bound" => Ok(UTXOStrategy::BranchAndBound),
        "fifo" => Ok(UTXOStrategy::Fifo),
        "largest-first" => Ok(UTXOStrategy::LargestFirst),
        "smallest-first" => Ok(UTXOStrategy::SmallestFirst),
        _ => Err("Unknown UTXO selection strategy"),
    }
}
