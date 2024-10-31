use std::error::Error;

use bitcoin::{
    address::{NetworkChecked, NetworkUnchecked},
    Address, Network, ScriptBuf,
};

pub fn string_to_address(address: &str, network: Network) -> Result<Address, Box<dyn Error>> {
    let addr_unchecked: Address<NetworkUnchecked> = address
        .parse()
        .map_err(|_| format!("Cannot parse address: {}", address))?;
    let addr_checked: Address<NetworkChecked> = addr_unchecked.require_network(network)?;

    Ok(addr_checked)
}

pub fn get_scriptpubkey_from_address(
    address: &str,
    network: Network,
) -> Result<String, Box<dyn Error>> {
    let addr: Address<NetworkChecked> = string_to_address(address, network)?;

    let script: ScriptBuf = addr.script_pubkey();

    // Convert to hex with 0x prefix
    Ok(format!("0x{}", hex::encode(script.as_bytes())))
}
