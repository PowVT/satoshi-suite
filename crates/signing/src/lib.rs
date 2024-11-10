use std::{collections::HashMap, error::Error};

use tracing::{info, warn};

use bitcoin::{
    consensus::{deserialize, serialize},
    Address, Amount, OutPoint, Transaction, TxOut,
};

use bitcoincore_rpc::{
    json::{AddressType, CreateRawTransactionInput, ListUnspentResultEntry},
    Client, RpcApi,
};

use satoshi_suite_utxo_selection::{strat_handler, UTXOStrategy};
use satoshi_suite_wallet::Wallet;

pub fn sign_tx(
    client: &Client,
    wallet: &Wallet,
    recipient: &Address,
    amount: Amount,
    fee_amount: Amount,
    utxo_strat: UTXOStrategy,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let balances = wallet.get_balances()?;

    if balances.mine.trusted.to_sat() < amount.to_sat() {
        return Err("Insufficient balance".into());
    }

    let unspent_txs: Vec<ListUnspentResultEntry> = wallet.list_all_unspent(None)?;
    if unspent_txs.is_empty() {
        return Err("No unspent transactions".into());
    }

    let selected_utxos =
        strat_handler(&unspent_txs, amount, fee_amount, utxo_strat).map_err(|e| e.to_string())?;

    let mut utxo_inputs: Vec<CreateRawTransactionInput> = Vec::new();
    let mut total_amount = Amount::from_sat(0);
    for utxo in &selected_utxos {
        utxo_inputs.push(CreateRawTransactionInput {
            txid: utxo.txid,
            vout: utxo.vout,
            sequence: Some(0),
        });
        total_amount += utxo.amount;
    }

    let mut outputs: HashMap<String, Amount> = HashMap::new();
    outputs.insert(recipient.to_string(), amount);

    let change_amount = total_amount - amount - fee_amount;
    if change_amount.to_sat() > 0 {
        let change_address: Address = wallet.new_address(&AddressType::Bech32)?;
        outputs.insert(change_address.to_string(), change_amount);
    }

    let tx = client.create_raw_transaction(&utxo_inputs[..], &outputs, None, None)?;

    let signed_tx = wallet.sign_tx(&tx)?;
    let raw_tx = serialize(&signed_tx);

    Ok(raw_tx)
}

pub fn verify_signed_tx(client: &Client, tx_hex: &str) -> Result<(), Box<dyn Error>> {
    let tx: Transaction = deserialize(&hex::decode(tx_hex)?)?;

    info!("Verifying transaction: {}", tx.txid());
    info!("Number of inputs: {}", tx.input.len());

    // Check if UTXOs are still unspent
    for (index, input) in tx.input.iter().enumerate() {
        info!("Checking UTXO for input {}", index);
        match is_utxo_unspent(client, &input.previous_output) {
            Ok(true) => info!("UTXO for input {} is unspent", index),
            Ok(false) => return Err(format!("UTXO for input {} is already spent", index).into()),
            Err(e) => return Err(format!("Error checking UTXO for input {}: {}", index, e).into()),
        }
    }

    // Closure to fetch previous transaction output (TxOut) for each input
    let mut spent = |outpoint: &OutPoint| -> Option<TxOut> {
        match client.get_raw_transaction(&outpoint.txid, None) {
            Ok(raw_tx) => raw_tx.output.get(outpoint.vout as usize).cloned(),
            Err(e) => {
                warn!("Failed to fetch raw transaction {}: {}", outpoint.txid, e);
                None
            }
        }
    };

    // Verify the transaction
    match tx.verify(&mut spent) {
        Ok(_) => {
            info!("Transaction verified successfully");
            Ok(())
        }
        Err(e) => Err(format!("Transaction verification failed: {}", e).into()),
    }
}

fn is_utxo_unspent(client: &Client, outpoint: &OutPoint) -> Result<bool, Box<dyn Error>> {
    match client.get_tx_out(&outpoint.txid, outpoint.vout, Some(false))? {
        Some(_) => Ok(true), // UTXO exists and is unspent
        None => Ok(false),   // UTXO doesn't exist (already spent)
    }
}
