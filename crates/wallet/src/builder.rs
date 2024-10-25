use std::error::Error;

use bitcoin::absolute::LockTime;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::{All, Message, Secp256k1};
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::{LeafVersion, TaprootSpendInfo};
use bitcoin::transaction::{Sequence, Version};
use bitcoin::{
    Amount, OutPoint, ScriptBuf, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Witness,
};
use bitcoincore_rpc::json::{AddressType, ListUnspentResultEntry};
use ordinals::Runestone;

use crate::Wallet;

pub fn build_commit_transaction(
    wallet: &Wallet,
    _secp: &Secp256k1<All>,
    utxo: ListUnspentResultEntry,
    amount: Amount,
    fee_amount: Amount,
    commit_script: ScriptBuf,
) -> Result<(Transaction, u32), Box<dyn Error>> {
    let total_needed = amount
        .to_sat()
        .checked_add(fee_amount.to_sat())
        .ok_or("Amount overflow")?;

    if total_needed > utxo.amount.to_sat() {
        return Err("Insufficient funds for commit transaction".into());
    }

    let change_amount = Amount::from_sat(
        utxo.amount
            .to_sat()
            .checked_sub(total_needed)
            .ok_or("Amount underflow")?,
    );

    // Create commit transaction
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: utxo.txid,
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::default(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: vec![
            TxOut {
                value: amount,
                script_pubkey: commit_script,
            },
            TxOut {
                value: change_amount,
                script_pubkey: wallet.new_address(&AddressType::Bech32m)?.script_pubkey(),
            },
        ],
    };

    let signed_tx = wallet.sign_tx(&tx)?;
    Ok((signed_tx, 0))
}

pub fn build_reveal_transaction(
    wallet: &Wallet,
    secp: &Secp256k1<All>,
    key_pair: &UntweakedKeypair,
    reveal_script: &ScriptBuf,
    taproot_spend_info: &TaprootSpendInfo,
    commit_outpoint: OutPoint,
    amount: Amount,
    fee_amount: Amount,
    is_rune: bool,
    additional_outputs: Vec<TxOut>,
) -> Result<Transaction, Box<dyn Error>> {
    // Calculate the total amount needed for additional outputs
    let additional_output_total: u64 = additional_outputs.iter().map(|output| output.value.to_sat()).sum();

    // Calculate the remaining amount after subtracting fees and additional outputs
    let remaining_amount = amount
        .to_sat()
        .checked_sub(fee_amount.to_sat())
        .and_then(|amt| amt.checked_sub(additional_output_total))
        .ok_or("Insufficient amount for reveal transaction")?;

    let sequence = if is_rune {
        // Sequence::from_height(Runestone::COMMIT_CONFIRMATIONS - 1)
        Sequence::ENABLE_RBF_NO_LOCKTIME
    } else {
        Sequence::ENABLE_RBF_NO_LOCKTIME
    };

    let mut outputs = Vec::new();

    if remaining_amount > 0 {
        outputs.push(TxOut {
            value: Amount::from_sat(remaining_amount),
            script_pubkey: wallet.new_address(&AddressType::Bech32m)?.script_pubkey(),
        });
    }

    outputs.extend(additional_outputs);

    println!("Outputs: {:?}", outputs);

    let mut reveal_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: commit_outpoint,
            script_sig: ScriptBuf::default(),
            sequence,
            witness: Witness::default(),
        }],
        output: outputs,
    };

    // Create the input's previous output
    let prev_tx_out = TxOut {
        value: amount,
        script_pubkey: ScriptBuf::new_p2tr(
            secp,
            taproot_spend_info.internal_key(),
            taproot_spend_info.merkle_root(),
        ),
    };

    // Sign the reveal transaction
    let mut sighash_cache = SighashCache::new(&mut reveal_tx);
    let leaf_hash = TapLeafHash::from_script(reveal_script, LeafVersion::TapScript);
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[prev_tx_out]),
            leaf_hash,
            TapSighashType::Default,
        )
        .expect("Failed to construct sighash");

    // Use the untweaked keypair for signing
    let signature = secp.sign_schnorr(
        &Message::from_digest_slice(sighash.as_ref())?,
        key_pair,
    );

    let witness = sighash_cache
        .witness_mut(0)
        .expect("getting mutable witness reference should work");

    witness.push(signature.as_ref());
    witness.push(reveal_script);
    witness.push(
        &taproot_spend_info
            .control_block(&(reveal_script.clone(), LeafVersion::TapScript))
            .expect("Failed to create control block")
            .serialize(),
    );

    Ok(reveal_tx)
}
