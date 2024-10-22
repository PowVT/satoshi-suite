use std::error::Error;

use bitcoin::absolute::LockTime;
use bitcoin::key::{TapTweak, UntweakedKeypair};
use bitcoin::secp256k1::{All, Message, Secp256k1};
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::{LeafVersion, TaprootSpendInfo};
use bitcoin::transaction::{Sequence, Version};
use bitcoin::{
    Amount, OutPoint, ScriptBuf, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Witness,
};
use bitcoincore_rpc::json::{AddressType, ListUnspentResultEntry};

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
) -> Result<Transaction, Box<dyn Error>> {
    let reveal_amount = Amount::from_sat(
        amount
            .to_sat()
            .checked_sub(fee_amount.to_sat())
            .ok_or("Insufficient amount for reveal transaction fee")?,
    );

    let mut reveal_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: commit_outpoint,
            script_sig: ScriptBuf::default(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: reveal_amount,
            script_pubkey: wallet.new_address(&AddressType::Bech32m)?.script_pubkey(),
        }],
    };

    // Sign the reveal transaction
    let mut sighash_cache = SighashCache::new(&reveal_tx);
    let leaf_hash = TapLeafHash::from_script(reveal_script, LeafVersion::TapScript);
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[TxOut {
                value: amount, // Keep as Amount
                script_pubkey: ScriptBuf::new_p2tr(
                    secp,
                    taproot_spend_info.internal_key(),
                    taproot_spend_info.merkle_root(),
                ),
            }]),
            leaf_hash,
            TapSighashType::Default,
        )
        .expect("Failed to construct sighash");

    let signature = secp.sign_schnorr(
        &Message::from_digest_slice(sighash.as_ref())?,
        &key_pair
            .tap_tweak(secp, taproot_spend_info.merkle_root())
            .to_inner(),
    );

    reveal_tx.input[0].witness.push(signature.as_ref().to_vec());
    reveal_tx.input[0].witness.push(reveal_script.clone());
    reveal_tx.input[0].witness.push(
        &taproot_spend_info
            .control_block(&(reveal_script.clone(), LeafVersion::TapScript))
            .expect("Failed to create control block")
            .serialize(),
    );

    Ok(reveal_tx)
}
