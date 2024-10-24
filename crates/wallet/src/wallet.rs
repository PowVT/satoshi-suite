use std::{error::Error, fmt};

use log::info;

use serde::Deserialize;

use bitcoin::key::{UntweakedKeypair, XOnlyPublicKey};
use bitcoin::script::Builder as ScriptBuilder;
use bitcoin::secp256k1::{rand, Secp256k1};
use bitcoin::taproot::TaprootBuilder;
use bitcoin::{Address, Amount, Network, OutPoint, ScriptBuf, Transaction, Txid};
use bitcoincore_rpc::json::{
    AddressType, GetAddressInfoResult, GetBalancesResult, GetWalletInfoResult,
    ListUnspentQueryOptions, ListUnspentResultEntry, WalletProcessPsbtResult,
};
use bitcoincore_rpc::jsonrpc::serde_json::{json, Value};
use bitcoincore_rpc::{Client, Error as RpcError, RpcApi};

use ord::Chain;

use satoshi_suite_client::{create_rpc_client, ClientError};
use satoshi_suite_config::Config;
use satoshi_suite_ordinals::InscriptionData;
use satoshi_suite_utxo_selection::{strat_handler, UTXOStrategy};

use crate::{build_commit_transaction, build_reveal_transaction};

#[derive(Debug)]
pub enum WalletError {
    ClientError(ClientError),
    WalletCreationDisabled(String),
    AddressNetworkMismatch,
    SigningFailed(String),
    RpcError(RpcError),
    AddressNotFound,
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::ClientError(err) => write!(f, "Client error: {}", err),
            WalletError::WalletCreationDisabled(name) => {
                write!(f, "Wallet creation disabled: {}", name)
            }
            WalletError::AddressNetworkMismatch => write!(f, "Address network mismatch"),
            WalletError::SigningFailed(err) => write!(f, "Signing failed: {}", err),
            WalletError::RpcError(err) => write!(f, "RPC error: {}", err),
            WalletError::AddressNotFound => write!(f, "Address not found in transaction details"),
        }
    }
}

impl Error for WalletError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WalletError::ClientError(err) => Some(err),
            WalletError::RpcError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ClientError> for WalletError {
    fn from(err: ClientError) -> Self {
        WalletError::ClientError(err)
    }
}

impl From<RpcError> for WalletError {
    fn from(err: RpcError) -> Self {
        WalletError::RpcError(err)
    }
}

#[derive(Deserialize)]
struct SendResult {
    txid: Txid,
}

#[derive(Debug)]
pub struct InscriptionTransactions {
    pub commit_tx: Transaction,
    pub commit_txid: Txid,
    pub reveal_tx: Transaction,
    pub reveal_txid: Txid,
    pub total_fees: u64,
}

pub struct Wallet {
    client: Client,
    network: Network,
}

impl Wallet {
    pub fn new(name: &str, config: &Config) -> Result<Self, WalletError> {
        let name = name.to_string();
        let client = create_rpc_client(config, None)?;

        let wallet_list = client.list_wallet_dir()?;

        if wallet_list.contains(&name) {
            let loaded_wallets = client.list_wallets()?;

            if !loaded_wallets.contains(&name) {
                info!("Loading wallet {}", name);
                client.load_wallet(&name)?;
            } else {
                info!("Wallet {} already loaded", name);
            }
        } else {
            if !config.create_wallets {
                return Err(WalletError::WalletCreationDisabled(name));
            }
            info!("Creating wallet {}", name);
            client.create_wallet(&name, None, None, None, None)?;
        }

        Ok(Wallet {
            client: create_rpc_client(config, Some(&name))?,
            network: config.network,
        })
    }

    pub fn new_address(&self, address_type: &AddressType) -> Result<Address, WalletError> {
        let address = self.client.get_new_address(None, Some(*address_type))?;
        address
            .require_network(self.network)
            .map_err(|_| WalletError::AddressNetworkMismatch)
    }

    pub fn get_balances(&self) -> Result<GetBalancesResult, WalletError> {
        self.client.get_balances().map_err(WalletError::from)
    }

    pub fn send(&self, address: &Address, amount: Amount) -> Result<OutPoint, WalletError> {
        let output = json!([{
            address.to_string(): amount.to_btc()
        }]);
        let send_result: SendResult = self
            .client
            .call("send", &[output, Value::Null, "unset".into(), 1.into()])?;
        let txid = send_result.txid;

        let transaction_info = self.client.get_transaction(&txid, None)?;
        let target_vout = transaction_info
            .details
            .iter()
            .find_map(|details| {
                details
                    .address
                    .as_ref()
                    .filter(|&tx_address| tx_address == address)
                    .map(|_| details.vout)
            })
            .ok_or(WalletError::AddressNotFound)?;

        Ok(OutPoint {
            txid,
            vout: target_vout,
        })
    }

    pub fn sign_tx(&self, tx: &Transaction) -> Result<Transaction, WalletError> {
        let signed = self
            .client
            .sign_raw_transaction_with_wallet(tx, None, None)?;
        signed
            .transaction()
            .map_err(|e| WalletError::SigningFailed(e.to_string()))
    }

    pub fn get_wallet_info(&self) -> Result<GetWalletInfoResult, WalletError> {
        self.client.get_wallet_info().map_err(WalletError::from)
    }

    pub fn get_address_info(&self, address: &Address) -> Result<GetAddressInfoResult, WalletError> {
        self.client
            .get_address_info(address)
            .map_err(WalletError::from)
    }

    pub fn list_all_unspent(
        &self,
        query_options: Option<ListUnspentQueryOptions>,
    ) -> Result<Vec<ListUnspentResultEntry>, WalletError> {
        self.client
            .list_unspent(Some(1), Some(9999999), None, None, query_options)
            .map_err(WalletError::from)
    }

    pub fn process_psbt(&self, psbt: &str) -> Result<WalletProcessPsbtResult, WalletError> {
        self.client
            .wallet_process_psbt(psbt, None, None, None)
            .map_err(WalletError::from)
    }

    pub fn mine_blocks(
        &self,
        address_type: &AddressType,
        blocks: u64,
    ) -> Result<Address, Box<dyn Error>> {
        // require network to be regtest
        if self.network != Network::Regtest {
            return Err("Network must be regtest".into());
        }

        let coinbase_recipient = self.new_address(address_type)?;
        let _ = self
            .client
            .generate_to_address(blocks, &coinbase_recipient)?;
        Ok(coinbase_recipient)
    }

    pub fn inscribe_ordinal(
        &self,
        postage: &u64,
        file_path: &str,
        config: &Config,
    ) -> Result<InscriptionTransactions, Box<dyn Error>> {
        let secp = Secp256k1::new();

        // Create inscription
        let inscription =
            InscriptionData::new(Chain::Regtest, file_path)?;
        let reveal_script = inscription.reveal_script_as_scriptbuf(ScriptBuilder::new())?;

        // Generate key pair for taproot
        let key_pair = UntweakedKeypair::new(&secp, &mut rand::thread_rng());
        let (public_key, _parity) = XOnlyPublicKey::from_keypair(&key_pair);

        // Build taproot tree
        let taproot_builder = TaprootBuilder::new()
            .add_leaf(0, reveal_script.clone())
            .expect("adding leaf should work");

        let taproot_spend_info = taproot_builder
            .finalize(&secp, public_key)
            .expect("finalizing taproot builder should work");

        // Create commit transaction output
        let commit_script = ScriptBuf::new_p2tr(
            &secp,
            taproot_spend_info.internal_key(),
            taproot_spend_info.merkle_root(),
        );

        // Get unspent outputs for funding
        let utxos = self.list_all_unspent(None)?;
        if utxos.is_empty() {
            return Err("No unspent outputs available for inscription".into());
        }

        let amount = Amount::from_sat(*postage); // Inscription amount
        let commit_fee = Amount::from_sat(20000); // Commit fee
        let reveal_fee = Amount::from_sat(20000); // Reveal fee

        // Select a single UTXO for the commit transaction
        let selected_utxos = strat_handler(&utxos, amount, commit_fee, UTXOStrategy::SingleUTXO)?;

        if selected_utxos.is_empty() {
            return Err("No UTXOs selected for inscription".into());
        }

        // Build commit transaction
        let (commit_tx, commit_vout) = build_commit_transaction(
            &self,
            &secp,
            selected_utxos[0].clone(),
            amount,
            commit_fee,
            commit_script,
        )?;

        // Create and sign reveal transaction
        let reveal_tx = build_reveal_transaction(
            &self,
            &secp,
            &key_pair,
            &reveal_script,
            &taproot_spend_info,
            OutPoint {
                txid: commit_tx.txid(),
                vout: commit_vout,
            },
            amount,
            reveal_fee,
        )?;

        // Calculate total fees
        let total_fees = commit_fee.to_sat() + reveal_fee.to_sat();

        // Send commit transaction
        let commit_txid = self.client.send_raw_transaction(&commit_tx)?;

        // mine 6 blocks to confirm the commit transaction
        let miner = Wallet::new("miner", &config)?;
        let _ = miner.mine_blocks(&AddressType::Bech32, 6)?;

        // Send reveal transaction
        let reveal_txid = self.client.send_raw_transaction(&reveal_tx)?;

        Ok(InscriptionTransactions {
            commit_tx,
            commit_txid,
            reveal_tx,
            reveal_txid,
            total_fees,
        })
    }
}
