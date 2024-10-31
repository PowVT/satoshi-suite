use std::{error::Error, fmt};

use log::info;

use serde::Deserialize;

use bitcoin::key::UntweakedKeypair;
use bitcoin::script::Builder as ScriptBuilder;
use bitcoin::secp256k1::{rand, Secp256k1};
use bitcoin::{Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxOut, Txid};
use bitcoincore_rpc::json::{
    AddressType, GetAddressInfoResult, GetBalancesResult, GetWalletInfoResult,
    ListUnspentQueryOptions, ListUnspentResultEntry, WalletProcessPsbtResult,
};
use bitcoincore_rpc::jsonrpc::serde_json::{json, Value};
use bitcoincore_rpc::{Client, Error as RpcError, RpcApi};

use ord::Chain;

use ordinals::{Etching, Runestone};

use satoshi_suite_client::{create_rpc_client, ClientError};
use satoshi_suite_config::Config;
use satoshi_suite_ordinals::InscriptionData;
use satoshi_suite_utxo_selection::{strat_handler, UTXOStrategy};

use crate::{build_commit_transaction, build_reveal_transaction, create_taproot_info};

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

// Commit / reveal transaction data
#[derive(Debug)]
pub struct CommitRevealTxPair {
    pub commit_txid: Txid,
    pub reveal_txid: Txid,
    pub total_fees: u64,
}

// Inscription-specific data
#[derive(Debug)]
pub struct InscriptionTransactions {
    pub base: CommitRevealTxPair,
}

// Etching-specific data
#[derive(Debug)]
pub struct EtchingTransactions {
    pub base: CommitRevealTxPair,
    pub rune_id: ordinals::Rune,
}

pub struct Wallet {
    pub client: Client,
    pub network: Network,
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
        commit_fee: Amount,
        reveal_fee: Amount,
        file_path: &str,
        config: &Config,
    ) -> Result<InscriptionTransactions, Box<dyn Error>> {
        let secp = Secp256k1::new();
        let key_pair = UntweakedKeypair::new(&secp, &mut rand::thread_rng());

        // Create inscription
        let inscription = InscriptionData::new(Chain::Regtest, file_path)?;
        let reveal_script = inscription.reveal_script_as_scriptbuf(ScriptBuilder::new())?;

        // Create taproot info
        let (taproot_spend_info, commit_script) =
            create_taproot_info(&secp, &key_pair, reveal_script.clone())?;

        let postage = Amount::from_sat(*postage);

        // Get unspent outputs for funding
        let utxos = self.list_all_unspent(None)?;
        if utxos.is_empty() {
            return Err("No unspent outputs available for inscription".into());
        }

        // Select a single UTXO for the commit transaction
        let selected_utxos = strat_handler(&utxos, postage, commit_fee, UTXOStrategy::SingleUTXO)?;

        if selected_utxos.is_empty() {
            return Err("No UTXOs selected for inscription".into());
        }

        // Build commit transaction
        let (commit_tx, commit_vout) = build_commit_transaction(
            &self,
            &secp,
            selected_utxos[0].clone(),
            postage,
            commit_fee,
            commit_script,
        )?;

        // Get recipient address for reveal tx
        let recipient_address = self.new_address(&AddressType::Bech32m)?;

        // Change output
        let reveal_outputs = vec![TxOut {
            value: postage.checked_sub(reveal_fee).unwrap_or(Amount::ZERO),
            script_pubkey: recipient_address.script_pubkey(),
        }];

        // Create and sign reveal transaction
        let reveal_tx = build_reveal_transaction(
            &secp,
            &key_pair,
            &reveal_script,
            &taproot_spend_info,
            OutPoint {
                txid: commit_tx.txid(),
                vout: commit_vout,
            },
            postage,
            Sequence::ENABLE_RBF_NO_LOCKTIME,
            reveal_outputs,
        )?;

        // Send commit transaction
        let commit_txid = self.client.send_raw_transaction(&commit_tx)?;

        // mine 6 blocks to confirm the commit transaction
        let miner = Wallet::new("miner", &config)?;
        let _ = miner.mine_blocks(&AddressType::Bech32, 6)?;

        // Send reveal transaction
        let reveal_txid = self.client.send_raw_transaction(&reveal_tx)?;

        Ok(InscriptionTransactions {
            base: CommitRevealTxPair {
                commit_txid,
                reveal_txid,
                total_fees: commit_fee.to_sat() + reveal_fee.to_sat(),
            },
        })
    }

    pub fn etch_rune(
        &self,
        etching: Etching,
        postage: &u64,
        commit_fee: Amount,
        reveal_fee: Amount,
        premine_tx_amount: Amount,
        file_path: &str,
        config: &Config,
    ) -> Result<EtchingTransactions, Box<dyn Error>> {
        let secp = Secp256k1::new();
        let key_pair = UntweakedKeypair::new(&secp, &mut rand::thread_rng());

        // Calculate premine amount
        let premine = etching.premine.unwrap_or(0);

        // Create inscription
        let mut inscription = InscriptionData::new(Chain::Regtest, file_path)?;
        inscription.pointer = Some(vec![]);
        inscription.rune = Some(
            etching
                .rune
                .ok_or("Invalid etching data; rune is missing")?
                .commitment(),
        );

        // Create reveal script
        let reveal_script = inscription.reveal_script_as_scriptbuf(ScriptBuilder::new())?;

        // Create taproot info
        let (taproot_spend_info, commit_script) =
            create_taproot_info(&secp, &key_pair, reveal_script.clone())?;

        // Get unspent outputs for funding
        let utxos = self.list_all_unspent(None)?;
        if utxos.is_empty() {
            return Err("No unspent outputs available for etching".into());
        }

        let postage = Amount::from_sat(*postage);

        // Select UTXOs
        let selected_utxos = strat_handler(&utxos, postage, commit_fee, UTXOStrategy::SingleUTXO)?;
        if selected_utxos.is_empty() {
            return Err("No UTXOs selected for etching".into());
        }

        // Create and sign commit transaction
        let (commit_tx, commit_vout) = build_commit_transaction(
            &self,
            &secp,
            selected_utxos[0].clone(),
            postage,
            commit_fee,
            commit_script,
        )?;

        // Get new addresses for deploy and mint txs
        let recipient_address = self.new_address(&AddressType::Bech32m)?;
        println!("Recipient address: {}", recipient_address);

        // Create reveal outputs
        let mut reveal_outputs = vec![TxOut {
            value: postage - premine_tx_amount - reveal_fee,
            script_pubkey: recipient_address.script_pubkey(),
        }];

        // Add premine output if applicable
        if premine > 0 {
            reveal_outputs.push(TxOut {
                value: premine_tx_amount,
                script_pubkey: recipient_address.script_pubkey(),
            });
        }

        // Add runestone output
        let runestone = Runestone {
            edicts: Vec::new(), // No edicts for initial etching
            etching: Some(etching),
            mint: None,
            pointer: (premine > 0).then_some(1), // Points to premine output
        };
        reveal_outputs.push(TxOut {
            value: Amount::ZERO,
            script_pubkey: ScriptBuf::from_bytes(runestone.encipher().to_bytes()),
        });

        // Create and sign reveal transaction
        let reveal_tx = build_reveal_transaction(
            &secp,
            &key_pair,
            &reveal_script,
            &taproot_spend_info,
            OutPoint {
                txid: commit_tx.txid(),
                vout: commit_vout,
            },
            postage,
            Sequence::from_height(Runestone::COMMIT_CONFIRMATIONS - 1),
            reveal_outputs,
        )?;

        // Broadcast transactions
        let commit_txid = self.client.send_raw_transaction(&commit_tx)?;

        let miner = Wallet::new("miner", &config)?;
        let _ = miner.mine_blocks(&AddressType::Bech32, 6)?;

        let reveal_txid = self.client.send_raw_transaction(&reveal_tx)?;

        Ok(EtchingTransactions {
            base: CommitRevealTxPair {
                commit_txid,
                reveal_txid,
                total_fees: commit_fee.to_sat() + reveal_fee.to_sat(),
            },
            rune_id: etching.rune.unwrap(),
        })
    }
}
