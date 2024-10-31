use std::{error::Error, str::FromStr};

use log::info;

use ordinals::{Etching, Rune, Terms};
use serde_json::json;

use bitcoin::{Amount, Txid};
use bitcoincore_rpc::{json::AddressType, RawTx, RpcApi};

use satoshi_suite_client::create_rpc_client;
use satoshi_suite_config::Config;
use satoshi_suite_signing::{sign_tx, verify_signed_tx};
use satoshi_suite_utxo_selection::UTXOStrategy;
use satoshi_suite_wallet::{
    get_scriptpubkey_from_address, string_to_address, MultisigWallet, Wallet,
};

use crate::cli::{Action, Cli};

pub fn handler(args: &Cli, config: &Config) -> Result<(), Box<dyn Error>> {
    match &args.action {
        Action::BootstrapEnv { address_type } => bootstrap_env(&address_type, config),
        Action::GetBlockHeight => get_block_height(config),
        Action::NewWallet { wallet_name } => new_wallet(wallet_name.as_str(), config),
        Action::NewMultisig {
            wallet_names,
            nrequired,
            multisig_name,
        } => new_multisig_wallet(&wallet_names, *nrequired, multisig_name.as_str(), config),
        Action::GetWalletInfo { wallet_name } => get_wallet_info(wallet_name.as_str(), config),
        Action::ListDescriptors { wallet_name } => list_descriptors(wallet_name.as_str(), config),
        Action::GetNewAddress {
            wallet_name,
            address_type,
        } => get_new_address(wallet_name.as_str(), &address_type, config),
        Action::GetAddressInfo {
            wallet_name,
            address,
        } => get_address_info(wallet_name.as_str(), &address, config),
        Action::DeriveAddresses {
            descriptor,
            start,
            end,
        } => derive_addresses(descriptor.as_str(), *start, *end, config),
        Action::RescanBlockchain { start } => rescan_blockchain(*start, config),
        Action::GetBalance { wallet_name } => get_balance(wallet_name.as_str(), config),
        Action::ListUnspent { wallet_name } => list_unspent(wallet_name.as_str(), config),
        Action::GetTx { wallet_name, txid } => get_tx(wallet_name.as_str(), &txid, config),
        Action::GetTxOut { txid, vout } => get_tx_out(&txid, *vout, config),
        Action::SendBtc {
            wallet_name,
            recipient,
            amount,
        } => send_btc(wallet_name.as_str(), &recipient, *amount, config),
        Action::SignTx {
            wallet_name,
            recipient,
            amount,
            fee_amount,
            utxo_strat,
        } => sign_transaction(
            wallet_name.as_str(),
            &recipient,
            *amount,
            *fee_amount,
            *utxo_strat,
            config,
        ),
        Action::DecodeRawTx { tx_hex } => decode_raw_tx(tx_hex.as_str(), config),
        Action::VerifySignedTx { tx_hex } => verify_signed_transaction(tx_hex.as_str(), config),
        Action::BroadcastTx { tx_hex } => broadcast_tx(tx_hex.as_str(), config),
        Action::CreatePsbt {
            wallet_name,
            recipient,
            amount,
            fee_amount,
            utxo_strat,
        } => create_psbt(
            wallet_name.as_str(),
            &recipient,
            *amount,
            *fee_amount,
            *utxo_strat,
            config,
        ),
        Action::ProcessPsbt {
            wallet_name,
            psbt_hex,
        } => process_psbt(wallet_name.as_str(), psbt_hex.as_str(), config),
        Action::DecodePsbt { psbt_hex } => decode_psbt(psbt_hex.as_str(), config),
        Action::AnalyzePsbt { psbt_hex } => analyze_psbt(psbt_hex.as_str(), config),
        Action::CombinePsbts { psbts } => combine_psbts(&psbts, config),
        Action::FinalizePsbt { psbt_hex } => finalize_psbt(psbt_hex.as_str(), config),
        Action::FinalizePsbtAndBroadcast { psbt_hex } => {
            finalize_psbt_and_broadcast(&psbt_hex, config)
        }
        Action::InscribeOrdinal {
            wallet_name,
            postage,
            file_path,
        } => inscribe_ordinal(wallet_name.as_str(), &postage, &file_path, config),
        Action::EtchRune {
            wallet_name,
            postage,
            file_path,
        } => etch_rune(wallet_name.as_str(), &postage, &file_path, config),
        Action::MineBlocks {
            wallet_name,
            blocks,
            address_type,
        } => wallet_mine_blocks(wallet_name.as_str(), *blocks, &address_type, config),
    }
}

pub fn bootstrap_env(address_type: &AddressType, config: &Config) -> Result<(), Box<dyn Error>> {
    for i in 1..11 {
        let wallet = Wallet::new(&format!("wallet{}", i), &config)?;
        let _ = wallet.mine_blocks(&address_type, 1)?;
    }

    let miner = Wallet::new("miner", &config)?;
    let _ = miner.mine_blocks(&address_type, 100)?;

    for i in 1..11 {
        let wallet = Wallet::new(&format!("wallet{}", i), &config)?;
        let balance = wallet.get_balances()?;

        let expected_balance = bitcoin::Amount::from_btc(50.0).unwrap();
        if !balance.mine.trusted.eq(&expected_balance) {
            return Err(format!(
                "Wallet {} balance mismatch. Expected: {}, Actual: {}",
                i, expected_balance, balance.mine.trusted
            )
            .into());
        }
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
    Ok(())
}

pub fn new_multisig_wallet(
    wallet_names: &Vec<String>,
    nrequired: u32,
    multisig_name: &str,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
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
    address: &String,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let addr = string_to_address(address, config.network)?;

    let pubkey = get_scriptpubkey_from_address(address, config.network)?;
    info!("Address PubKey: {}", pubkey);

    let address_info = wallet.get_address_info(&addr)?;
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
    info!("{:?}", tx);
    Ok(())
}

pub fn get_tx_out(txid: &str, vout: u32, config: &Config) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let txid_converted =
        bitcoin::Txid::from_str(txid).map_err(|_| Box::<dyn Error>::from("Invalid TxID"))?;
    let tx_out = client
        .get_tx_out(&txid_converted, vout, None)?
        .ok_or_else(|| Box::<dyn Error>::from("TxOut not found"))?;
    info!("{:#?}", tx_out);
    Ok(())
}

pub fn send_btc(
    wallet_name: &str,
    recipient: &String,
    amount: bitcoin::Amount,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let recipient_addr = string_to_address(recipient, config.network)?;

    let outpoint = wallet.send(&recipient_addr, amount)?;
    info!("Sent: {}", outpoint);
    Ok(())
}

pub fn sign_transaction(
    wallet_name: &str,
    recipient: &String,
    amount: bitcoin::Amount,
    fee_amount: bitcoin::Amount,
    utxo_strat: UTXOStrategy,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let client = create_rpc_client(config, None)?;
    let wallet = Wallet::new(wallet_name, config)?;
    let recipient_addr = string_to_address(recipient, config.network)?;

    let tx = sign_tx(
        &client,
        &wallet,
        &recipient_addr,
        amount,
        fee_amount,
        utxo_strat,
    )?;
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

pub fn create_psbt(
    wallet_name: &str,
    recipient: &String,
    amount: bitcoin::Amount,
    fee_amount: bitcoin::Amount,
    utxo_strat: UTXOStrategy,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let psbt = MultisigWallet::create_psbt(
        wallet_name,
        recipient,
        amount,
        fee_amount,
        utxo_strat,
        config,
    )?;
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
    let raw_hex: String = res
        .hex
        .ok_or_else(|| Box::<dyn Error>::from("Cannot get hex from finalized PSBT"))?
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    info!("FinalizedPSBT: {}", raw_hex);

    let tx = client.send_raw_transaction(raw_hex)?;
    info!("Broadcasted transaction: {}", tx);
    Ok(())
}

pub fn inscribe_ordinal(
    wallet_name: &str,
    postage: &u64,
    file_path: &str,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;

    // For mainnet/testnet: these fees should be dynamically fetched
    let commit_fee = Amount::from_sat(20000);
    let reveal_fee = Amount::from_sat(20000);

    if Amount::from_sat(*postage) < reveal_fee + Amount::from_sat(546) {
        return Err("postage must be greater than reveal fee + min dust".into());
    }

    let inscription_info =
        wallet.inscribe_ordinal(postage, commit_fee, reveal_fee, file_path, config)?;
    info!("Inscription info: {:#?}", inscription_info);
    Ok(())
}

pub fn etch_rune(
    wallet_name: &str,
    postage: &u64,
    file_path: &str,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;

    let rune = "ZZZZZZZZZZZZZAAAA".parse::<Rune>().unwrap();

    // validation checks
    if rune.is_reserved() {
        return Err(format!("rune `{rune}` is reserved").into());
    }

    let divisibility = 2;
    if divisibility > 38 {
        return Err("divisibility must be less than or equal 38".into());
    }

    // Create the etching with proper supply calculations
    let premine = 100000; // 1000.00
    let terms_amount = 10000; // 100.00
    let terms_cap = 90;

    // Validate supply
    let supply = premine + (terms_cap as u128 * terms_amount as u128);
    if supply == 0 {
        return Err("supply must be greater than zero".into());
    }

    let etching = Etching {
        divisibility: Some(divisibility),
        premine: Some(premine),
        rune: Some(rune),
        spacers: Some(0),
        symbol: Some('$'),
        terms: Some(Terms {
            amount: Some(terms_amount),
            cap: Some(terms_cap),
            height: (None, None),
            offset: (None, None),
        }),
        turbo: true,
    };

    // more validation checks
    // let current_height = u32::try_from(wallet.client.get_block_count()?).unwrap();
    // let reveal_height = current_height + u32::from(Runestone::COMMIT_CONFIRMATIONS);

    // let first_rune_height = Rune::first_rune_height(bitcoin::Network::Regtest);
    // if reveal_height < first_rune_height {
    //     return Err(format!(
    //         "rune reveal height below rune activation height: {reveal_height} < {first_rune_height}"
    //     ).into());
    // }

    if let Some(ref terms) = etching.terms {
        if terms.cap == Some(0) {
            return Err("terms.cap must be greater than zero".into());
        }
        if terms.amount.unwrap_or(0) == 0 {
            return Err("terms.amount must be greater than zero".into());
        }
    }

    let commit_fee = Amount::from_sat(20000);
    let reveal_fee = Amount::from_sat(20000);
    let premine_tx_amount = if premine > 0 {
        Amount::from_sat(10000)
    } else {
        Amount::ZERO
    };

    if Amount::from_sat(*postage) < reveal_fee + premine_tx_amount {
        return Err("postage must be greater than reveal fee + min dust".into());
    }

    let rune_info = wallet.etch_rune(
        etching,
        postage,
        commit_fee,
        reveal_fee,
        premine_tx_amount,
        file_path,
        config,
    )?;
    info!("Etching Info: {:#?}", rune_info);
    Ok(())
}

pub fn wallet_mine_blocks(
    wallet_name: &str,
    blocks: u64,
    address_type: &AddressType,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let wallet = Wallet::new(wallet_name, config)?;
    let coinbase_recipient = wallet.mine_blocks(address_type, blocks)?;
    info!("Mined {} blocks to {}", blocks, coinbase_recipient);
    Ok(())
}
