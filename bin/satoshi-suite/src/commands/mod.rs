use std::{error::Error, str::FromStr};

use log::info;

use serde_json::json;

use bitcoin::{Address, Txid};
use bitcoincore_rpc::{json::AddressType, RawTx, RpcApi};

use satoshi_suite_client::create_rpc_client;
use satoshi_suite_config::Config;
use satoshi_suite_signing::{sign_tx, verify_signed_tx};
use satoshi_suite_utxo_selection::UTXOStrategy;
use satoshi_suite_wallet::{MultisigWallet, Wallet};

use crate::cli::{Action, Cli};

pub fn handler(args: &Cli, config: &Config) -> Result<(), Box<dyn Error>> {
    match args.action {
        Action::BootstrapEnv => bootstrap_env(&args.address_type, &config),
        Action::GetBlockHeight => get_block_height(&config),
        Action::NewWallet => new_wallet(&args.wallet_name, &config),
        Action::NewMultisig => new_multisig_wallet( &args.wallet_names, args.nrequired, &args.multisig_name, &config),
        Action::GetWalletInfo => get_wallet_info(&args.wallet_name, &config),
        Action::ListDescriptors => list_descriptors(&args.wallet_name, &config),
        Action::GetNewAddress => get_new_address(&args.wallet_name, &args.address_type, &config),
        Action::GetAddressInfo => get_address_info(&args.wallet_name, &args.address, &config),
        Action::DeriveAddresses => {
            derive_addresses(&args.descriptor, args.start, args.end, &config)
        }
        Action::RescanBlockchain => rescan_blockchain(args.start, &config),
        Action::GetBalance => get_balance(&args.wallet_name, &config),
        Action::ListUnspent => list_unspent(&args.wallet_name, &config),
        Action::GetTx => get_tx(&args.wallet_name, &args.txid, &config),
        Action::GetTxOut => get_tx_out(&args.txid, args.vout, &config),
        Action::SendBtc => send_btc(&args.wallet_name, &args.recipient, args.amount, &config),
        Action::SignTx => sign_transaction(
            &args.wallet_name,
            &args.recipient,
            args.amount,
            args.fee_amount,
            args.utxo_strat,
            &config,
        ),
        Action::DecodeRawTx => decode_raw_tx(&args.tx_hex, &config),
        Action::VerifySignedTx => verify_signed_transaction(&args.tx_hex, &config),
        Action::BroadcastTx => broadcast_tx(&args.tx_hex, &config),
        Action::CreatePsbt => create_psbt(&args.wallet_name, &args.recipient, args.amount, args.fee_amount, args.utxo_strat, &config),
        Action::ProcessPsbt => process_psbt(&args.wallet_name, &args.psbt_hex, &config),
        Action::DecodePsbt => decode_psbt(&args.psbt_hex, &config),
        Action::AnalyzePsbt => analyze_psbt(&args.psbt_hex, &config),
        Action::CombinePsbts => combine_psbts(&args.psbts, &config),
        Action::FinalizePsbt => finalize_psbt(&args.psbt_hex, &config),
        Action::FinalizePsbtAndBroadcast => finalize_psbt_and_broadcast(&args.psbt_hex, &config),
        Action::MineBlocks => {
            mine_blocks(&args.wallet_name, args.blocks, &args.address_type, &config)
        }
    }
}

pub fn bootstrap_env(address_type: &AddressType, config: &Config) -> Result<(), Box<dyn Error>> {
    for i in 1..11 {
        new_wallet(&format!("wallet{}", i), &config)?;
        mine_blocks(&format!("wallet{}", i), 1, &address_type, &config)?;
    }

    new_wallet("miner", &config)?;
    mine_blocks("miner", 100, &address_type, &config)?;

    for i in 1..11 {
        let wallet = Wallet::new(&format!("wallet{}", i), &config)?;
        let balance = wallet.get_balances()?;
        assert!(balance
            .mine
            .trusted
            .eq(&bitcoin::Amount::from_btc(50.0).unwrap()));
    }

    Ok(())
}

pub fn get_block_height(config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let height = client.get_block_count()?;
    info!("Current block height: {}", height);
    Ok(())
}

pub fn new_wallet(wallet_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let _ = Wallet::new(wallet_name, config)?;
    info!("Wallet created: {}", wallet_name);
    Ok(())
}

pub fn new_multisig_wallet(wallet_names: &Vec<String>, nrequired: u32, multisig_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let multisig = MultisigWallet::new(wallet_names, nrequired, multisig_name, config)?;
    info!("Multisig wallet created");
    info!("Multisig wallet name: {}", multisig.name);
    info!("Multisig wallet signers: {}", multisig.signers.join(", "));
    info!("Multisig wallet nrequired: {}", multisig.nrequired);
    Ok(())
}

pub fn get_wallet_info(wallet_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let info = wallet.get_wallet_info()?;
    info!("{:#?}", info);
    Ok(())
}

pub fn rescan_blockchain(start_height: u32, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    client.rescan_blockchain(Some(start_height as usize), None)?;
    info!("Blockchain rescanned");
    Ok(())
}

pub fn list_descriptors(wallet_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, Some(wallet_name))?;
    let descriptors: serde_json::Value = client.call("listdescriptors", &[])?;
    info!("{:#?}", descriptors);
    Ok(())
}

pub fn get_new_address(
    wallet_name: &str,
    address_type: &AddressType,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let address = wallet.new_address(address_type)?;
    info!("New address: {}", address);
    Ok(())
}

pub fn get_address_info(
    wallet_name: &str,
    address: &Address,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let address_info = wallet.get_address_info(address)?;
    info!("{:#?}", address_info);
    Ok(())
}

pub fn derive_addresses(
    descriptor: &str,
    start: u32,
    end: u32,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let range: [u32; 2] = [start, end];

    let addresses = client.derive_addresses(&descriptor.to_string(), Some(range))?;
    info!("Derived addresses:");
    for (i, address) in addresses.iter().enumerate() {
        info!("  {}: {:#?}", i + start as usize, address);
    }
    Ok(())
}

pub fn get_balance(wallet_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let balance = wallet.get_balances()?;
    info!("Balance: {:#?}", balance);
    Ok(())
}

pub fn list_unspent(wallet_name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let unspent = wallet.list_all_unspent(None)?;
    info!("Unspent: {:#?}", unspent);
    Ok(())
}

pub fn get_tx(wallet_name: &str, txid: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, Some(wallet_name))?;
    let txid = Txid::from_str(txid)?;
    let tx = client.get_transaction(&txid, None)?;
    info!("{:#?}", tx);
    Ok(())
}

pub fn get_tx_out(txid: &str, vout: u32, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let txid_converted = bitcoin::Txid::from_str(txid).map_err(|_| Box::<dyn Error>::from("Invalid TxID"))?;
    let tx_out = client.get_tx_out(&txid_converted, vout, None)?
        .ok_or_else(|| Box::<dyn Error>::from("TxOut not found"))?;
    info!("{:#?}", tx_out);
    Ok(())
}

pub fn send_btc(
    wallet_name: &str,
    recipient: &Address,
    amount: bitcoin::Amount,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let outpoint = wallet.send(recipient, amount)?;
    info!("Sent: {}", outpoint);
    Ok(())
}

pub fn sign_transaction(
    wallet_name: &str,
    recipient: &Address,
    amount: bitcoin::Amount,
    fee_amount: bitcoin::Amount,
    utxo_strat: UTXOStrategy,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let wallet = Wallet::new(wallet_name, config)?;
    let tx = sign_tx(&client, &wallet, &recipient, amount, fee_amount, utxo_strat)?;
    info!("Signed transaction: {}", tx.raw_hex());
    Ok(())
}

pub fn decode_raw_tx(tx_hex: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let tx = client.decode_raw_transaction(tx_hex, None)?;
    info!("{:#?}", tx);
    Ok(())
}

pub fn verify_signed_transaction(tx_hex: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    verify_signed_tx(&client, tx_hex)?;
    info!("Transaction is valid");
    Ok(())
}

pub fn broadcast_tx(tx_hex: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let tx = client.send_raw_transaction(tx_hex)?;
    info!("Broadcasted transaction: {}", tx);
    Ok(())
}

pub fn create_psbt(wallet_name: &str, recipient: &Address, amount: bitcoin::Amount, fee_amount: bitcoin::Amount, utxo_strat: UTXOStrategy, config: &Config) -> Result<(), Box<dyn Error>> {
    let psbt = MultisigWallet::create_psbt(wallet_name, recipient, amount, fee_amount, utxo_strat, config)?;
    info!("PSBT: {:#?}", psbt);
    Ok(())
}

pub fn process_psbt(wallet_name: &str, psbt: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let psbt = wallet.process_psbt(psbt)?;
    info!("PSBT: {:#?}", psbt);
    Ok(())
}

pub fn decode_psbt(psbt: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let psbt: serde_json::Value = client.call("decodepsbt", &[json!(psbt)])?;
    info!("PSBT: {:#?}", psbt);
    Ok(())
}

pub fn analyze_psbt(psbt: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let psbt: serde_json::Value = client.call("analyzepsbt", &[json!(psbt)])?;
    info!("PSBT: {:#?}", psbt);
    Ok(())
}

pub fn combine_psbts(psbts: &Vec<String>, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let res = client.combine_psbt(&psbts[..])?;
    info!("CombinedPSBT: {:#?}", res);
    Ok(())
}

pub fn finalize_psbt(psbt: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let res = client.finalize_psbt(psbt, None)?;
    info!("FinalizedPSBT: {:#?}", res);
    Ok(())
}

pub fn finalize_psbt_and_broadcast(psbt: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let res = client.finalize_psbt(psbt, None)?;
    if !res.complete {
        return Err("Incomplete PSBT".into());
    }
    let raw_hex: String = res.hex.ok_or_else(|| Box::<dyn Error>::from("Cannot get hex from finalized PSBT"))?
        .iter().map(|b| format!("{:02x}", b)).collect();

    info!("FinalizedPSBT: {}", raw_hex);

    let tx = client.send_raw_transaction(raw_hex)?;
    info!("Broadcasted transaction: {}", tx);
    Ok(())
}

pub fn mine_blocks(
    wallet_name: &str,
    blocks: u64,
    address_type: &AddressType,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let coinbase_recipient = wallet.new_address(address_type)?;
    let client = create_rpc_client(config, None)?;
    let _ = client.generate_to_address(blocks, &coinbase_recipient)?;
    info!("Mined {} blocks to {}", blocks, coinbase_recipient);
    Ok(())
}
