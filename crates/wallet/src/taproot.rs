use std::error::Error;

use bitcoin::{
    key::{Secp256k1, UntweakedKeypair},
    secp256k1::All,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    ScriptBuf,
};

pub fn create_taproot_info(
    secp: &Secp256k1<All>,
    key_pair: &UntweakedKeypair,
    reveal_script: ScriptBuf,
) -> Result<(TaprootSpendInfo, ScriptBuf), Box<dyn Error>> {
    let (public_key, _parity) = bitcoin::key::XOnlyPublicKey::from_keypair(key_pair);

    let taproot_builder = TaprootBuilder::new()
        .add_leaf(0, reveal_script.clone())
        .expect("adding leaf should work");

    let taproot_spend_info = taproot_builder
        .finalize(secp, public_key)
        .expect("finalizing taproot builder should work");

    let commit_script = ScriptBuf::new_p2tr(
        secp,
        taproot_spend_info.internal_key(),
        taproot_spend_info.merkle_root(),
    );

    Ok((taproot_spend_info, commit_script))
}
