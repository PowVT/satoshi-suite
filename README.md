# Satoshi Suite: A Collection of Bitcoin Development Tools

## Overview

Satoshi Suite is designed to automate Bitcoin development tasks. It is ideal for developers who want to test Bitcoin applications without using real Bitcoin on the mainnet and need a bootstrapped BTC execution environment in which to send test transactions.

Inspiration for this repository came from the [taproot-wizards/purrfect_vault](https://github.com/taproot-wizards/purrfect_vault).

## Prerequisites

- A C++ compiler for building bitcoin-core
- Rust installation for building the Satoshi Suite and ord services
- [Just](https://github.com/casey/just) command runner

For reference, check the [bitcoin core build docs](https://github.com/bitcoin/bitcoin/blob/master/doc/build-unix.md) for required dependencies.

## How to Run

### Clone the Repository

   ```bash
    git clone https://github.com/powvt/satoshi-suite.git
    cd satoshi-suite
   ```

### Building and Running

Use `just` as a command wrapper. See the `justfile` for executing commands directly.

1. Install dependencies and build:
   ```bash
   just install-deps
   just build
   ```

2. Start the Bitcoin daemon:
   ```bash
   just start-bitcoind
   ```

3. In another terminal, execute commands against the local Bitcoin node:
   ```bash
   # Create a BTC wallet named "satoshi"
   just create-wallet satoshi

   # Get a new address for the wallet
   just get-new-address satoshi
   ```

4. To stop all services:
   ```bash
   just kill-all
   ```

## Available Commands

> View all possible commands and their inputs with `just -l`

### Wallet Commands

| Command | Inputs | Description |
|---------|--------|-------------|
| `just create-wallet` | `<wallet_name>` | Create a new Bitcoin wallet |
| `just get-balance` | `<wallet_name>` | Get the balance of your Bitcoin wallet |
| `just get-new-address` | `<wallet_name>` | Generate a new receive address |
| `just list-unspent` | `<wallet_name>` | List all UTXOs for the specified wallet |
| `just get-wallet-info` | `<wallet_name>` | Retrieve information related to the specified wallet |
| `just get-address-info` | `<wallet_name> <wallet_address>` | Retrieve information related to a specific address |
| `just sign-tx` | `<wallet_name> <recipient_address> <amount_in_btc> <fee_amount_in_btc> <utxo_selection_strategy>` | Sign a transaction |
| `just send-btc` | `<wallet_name> <recipient_address> <amount_in_btc>` | Create, sign, and broadcast a BTC transaction |

### Multisig Commands

| Command | Inputs | Description |
|---------|--------|-------------|
| `just new-multisig` | `<num_required_signatures> <comma_separated_wallet_names> <multisig_name>` | Create a new multisig wallet |
| `just create-psbt` | `<multisig_wallet_name> <recipient_address> <amount_in_btc> <fee_amount_in_btc> <utxo_selection_strategy>` | Create a multisig transaction |
| `just decode-psbt` | `<psbt_hash>` | Retrieve the inputs and outputs for a specific PSBT |
| `just analyze-psbt` | `<psbt_hash>` | Retrieve network-related information for a PSBT |
| `just combine-psbts` | `<signed_psbt_1,signed_psbt_2,...>` | Combine multiple partially signed Bitcoin transactions |
| `just finalize-psbt` | `<combined_psbt_hex>` | Finalize a fully signed PSBT |
| `just finalize-psbt-and-broadcast` | `<combined_psbt_hex>` | Finalize and broadcast a fully signed PSBT |

### Network Commands

| Command | Inputs | Description |
|---------|--------|-------------|
| `just start-bitcoind` | - | Start a local Regtest Bitcoin network |
| `just start-ord` | - | Start the ord server. View the ord explorer at `http://localhost:80`|
| `just kill-all` | - | Terminate all services and clear cached data |
| `just mine-blocks` | `<wallet_name> <number_of_blocks_to_mine>` | Mine blocks on the Regtest network |
| `just get-tx` | `<tx_hash>` | Get information about a specific transaction |
| `just get-tx-out` | `<tx_hash> <vout_index> <num_confirmations>` | Get transaction outputs |
| `just broadcast-tx` | `<signed_tx_hash> <max-fee-rate>` | Broadcast a signed transaction |
| `just get-spendable-balance` | `<address>` | Sum all UTXO amounts with 6+ confirmations |
| `just bootstrap-env` | - | Init a fresh bitcoin test environment with ten wallets and 50 BTC in each wallet |

### Ordinal Commands

| Command | Inputs | Description |
|---------|--------|-------------|
| `just inscribe-ordinal` | `<wallet_name>` | Inscribe a new ordinal, using the pre-existing inscription data |
| `just etch-rune` | `<wallet_name>` | Etch a rune, using the pre-existing inscription and runestone data |

## UTXO Selection Strategies

When generating a signed transaction, you have four options for selecting which UTXOs to spend. These strategies can result in different outcomes, especially if you have many UTXOs in your wallet. Here are the available strategies and some considerations for choosing the right one:

1. **`branch-and-bound`**:
   - **Description**: This strategy exhaustively searches for the optimal combination of UTXOs to minimize change.
   - **Pros**: Can provide the most efficient use of UTXOs by finding the best combination.
   - **Cons**: Computationally intensive and may be slow when dealing with a large number of UTXOs.

2. **`fifo` (First In, First Out)**:
   - **Description**: Selects the oldest UTXOs first.
   - **Pros**: Simple and efficient; can help reduce the number of UTXOs over time.
   - **Cons**: May result in larger transaction sizes if older UTXOs are small.

3. **`largest-first`**:
   - **Description**: Selects the largest UTXOs first.
   - **Pros**: Results in smaller transaction payloads, potentially reducing transaction fees.
   - **Cons**: May leave a large number of small UTXOs in your wallet, leading to inefficiencies later.

4. **`smallest-first`**:
   - **Description**: Selects the smallest UTXOs first.
   - **Pros**: Helps consolidate many small UTXOs, which can be useful for cleanup.
   - **Cons**: Leads to larger transaction sizes, increasing transaction fees.

#### Choosing a Strategy

When selecting a UTXO strategy, consider the following factors:

- **Transaction Size and Fees**: Smaller UTXOs result in larger transaction data and higher fees, while larger UTXOs minimize transaction size and fees.
- **Wallet Cleanup**: Using strategies like `smallest-first` can help clean up many small UTXOs.
- **Performance**: The `branch-and-bound` strategy, while optimal, can be slow with many UTXOs. Simpler strategies like `fifo` and `largest-first` are faster.

By carefully choosing your UTXO selection strategy, you can optimize your transactions for size, fees, or performance based on your specific needs.

## License

This project is licensed under the MIT License.