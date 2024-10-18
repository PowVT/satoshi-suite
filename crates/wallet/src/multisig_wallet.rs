use std::{collections::HashMap, error::Error};

use log::info;

use serde_json::json;

use bitcoin::{Address, Amount};
use bitcoincore_rpc::{json::{AddressType, CreateRawTransactionInput, WalletCreateFundedPsbtResult}, RpcApi};

use satoshi_suite_client::create_rpc_client;
use satoshi_suite_config::Config;
use satoshi_suite_utxo_selection::{strat_handler, UTXOStrategy};

use crate::Wallet;

#[derive(Debug)]
pub struct MultisigWallet {
    pub name: String,
    pub nrequired: u32,
    pub signers: Vec<String>,
}

impl MultisigWallet {
    pub fn new(wallet_names: &Vec<String>, nrequired: u32, multisig_name: &str, config: &Config) -> Result<Self, Box<dyn Error>> {
        if wallet_names.len() < nrequired as usize {
            return Err("More required signers than wallets".into());
        }

        let mut xpubs: HashMap<String, String> = HashMap::new();

        for wallet_name in wallet_names {
            Wallet::new(wallet_name, config)?;
        }

        for (i, wallet_name) in wallet_names.iter().enumerate() {
            let client = create_rpc_client(config, Some(wallet_name))?;
            let descriptors: serde_json::Value = client.call("listdescriptors", &[])?;
            let descriptors_array: &Vec<serde_json::Value> = descriptors["descriptors"].as_array()
                .ok_or_else(|| format!("Invalid descriptor format for wallet {}", wallet_name))?;
            xpubs = extract_int_ext_xpubs(xpubs, descriptors_array.clone(), i)?;
        }

        let num_signers = nrequired.to_string();
        let external_desc = format!(
            "wsh(sortedmulti({}, {}, {}, {}))",
            num_signers, xpubs["external_xpub_1"], xpubs["external_xpub_2"], xpubs["external_xpub_3"]
        );
        let internal_desc = format!(
            "wsh(sortedmulti({}, {}, {}, {}))",
            num_signers, xpubs["internal_xpub_1"], xpubs["internal_xpub_2"], xpubs["internal_xpub_3"]
        );

        let client = create_rpc_client(config, None)?;

        let external_desc_info = client.get_descriptor_info(&external_desc)?;
        let internal_desc_info = client.get_descriptor_info(&internal_desc)?;

        let external_descriptor = external_desc_info.descriptor;
        let internal_descriptor = internal_desc_info.descriptor;

        let multisig_ext_desc = json!({
            "desc": external_descriptor,
            "active": true,
            "internal": false,
            "timestamp": json!("now")
        });

        let multisig_int_desc = json!({
            "desc": internal_descriptor,
            "active": true,
            "internal": true,
            "timestamp": json!("now")
        });

        let multisig_desc = json!([multisig_ext_desc, multisig_int_desc]);

        client.create_wallet(multisig_name, Some(true), Some(true), None, None)?;

        let multisig_desc_vec: Vec<serde_json::Value> = serde_json::from_value(multisig_desc)?;
        let client2 = create_rpc_client(config, Some(multisig_name))?;
        client2.call::<serde_json::Value>("importdescriptors", &[json!(multisig_desc_vec)])?;

        let wallet = Wallet::new(multisig_name, config)?;
        let info = wallet.get_wallet_info()?;
        info!("{:#?}", info);

        Ok(Self {
            name: multisig_name.to_string(),
            nrequired,
            signers: wallet_names.clone(),
        })
    }

    pub fn create_psbt(wallet_name: &str, recipient: &Address, amount: Amount, fee_amount: Amount, utxo_strat: UTXOStrategy, config: &Config) -> Result<WalletCreateFundedPsbtResult, Box<dyn Error>> {
        let wallet: Wallet = Wallet::new(wallet_name, config)?;
    
        // Ensure the wallet is a multisig wallet
        if wallet.get_wallet_info()?.private_keys_enabled {
            return Err("Wallet is not a multisig wallet".into());
        }
    
        let bal = wallet.get_balances()?;
        if bal.mine.trusted.to_sat() < amount.to_sat() {
            return Err("Insufficient balance".into());
        }
    
        let unspent_txs = wallet.list_all_unspent(None)?;
        if unspent_txs.is_empty() {
            return Err("No unspent transactions".into());
        }
    
        // Based on the strategy, select UTXOs
        let selected_utxos = strat_handler(&unspent_txs, amount, fee_amount, utxo_strat)
            .map_err(|e| format!("Error selecting UTXOs: {}", e))?;
    
        let mut tx_inputs = Vec::new();
        let mut total_amount = Amount::from_sat(0);
        for utxo in &selected_utxos {
            tx_inputs.push(CreateRawTransactionInput {
                txid: utxo.txid,
                vout: utxo.vout,
                sequence: None,
            });
            total_amount += utxo.amount;
        }
    
        let mut tx_outputs: HashMap<String, Amount> = HashMap::new();
        tx_outputs.insert(recipient.to_string(), amount);
    
        // Add change output if there's any remaining amount
        let change_amount = total_amount - amount - fee_amount;
        if change_amount.to_sat() > 0 {
            let change_address = wallet.new_address(&AddressType::Bech32)?;
            tx_outputs.insert(change_address.to_string(), change_amount);
        }
    
        let locktime = None;
        // TODO: can optionally specify the fee rate here, otherwise it will have the wallet estimate it
        let options = None;
        let bip32derivs = None;
        let client = create_rpc_client(config, Some(wallet_name))?;
        let psbt = client
            .wallet_create_funded_psbt(&tx_inputs[..], &tx_outputs, locktime, options, bip32derivs)?;
    
        Ok(psbt)
    }
}

pub fn extract_int_ext_xpubs(
    mut xpubs: HashMap<String, String>,
    descriptors_array: Vec<serde_json::Value>,
    i: usize,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    // Find the correct descriptors for external and internal xpubs
    let external_xpub = descriptors_array
        .iter()
        .find(|desc| {
            desc["desc"]
                .as_str()
                .unwrap_or_default()
                .starts_with("wpkh")
                && desc["desc"].as_str().unwrap_or_default().contains("/0/*")
        })
        .ok_or(format!("External xpub not found for wallet {}", i + 1))?
        .as_str()
        .ok_or(format!("External xpub cannot be parsed to string {}", i + 1))?
        .to_string();

    let internal_xpub = descriptors_array
        .iter()
        .find(|desc| {
            desc["desc"]
                .as_str()
                .unwrap_or_default()
                .starts_with("wpkh")
                && desc["desc"].as_str().unwrap_or_default().contains("/1/*")
        })
        .ok_or(format!("Internal xpub not found for wallet {}", i + 1))?
        .as_str()
        .ok_or(format!("Internal xpub cannot be parsed to string {}", i + 1))?
        .to_string();

    // formatting notes: https://bitcoincoredocs.com/descriptors.html
    // split at "]" and take the last part
    let external_xpub_no_path = external_xpub.split("]").last().unwrap().to_string();
    let internal_xpub_no_path = internal_xpub.split("]").last().unwrap().to_string();

    // split at ")" and take the first part
    let external_xpub_no_path = external_xpub_no_path.split(")").next().unwrap().to_string();
    let internal_xpub_no_path = internal_xpub_no_path.split(")").next().unwrap().to_string();

    xpubs.insert(format!("internal_xpub_{}", i + 1), internal_xpub_no_path);
    xpubs.insert(format!("external_xpub_{}", i + 1), external_xpub_no_path);

    Ok(xpubs)
}