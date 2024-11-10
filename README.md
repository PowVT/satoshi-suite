# Satoshi Suite: A command line tool for performing Bitcoin RPC calls

## Overview

Satoshi Suite is designed to automate Bitcoin RPC calls. The suite comes with a full test environment for tinkering and testing Bitcoin applications. Using the suite you can create wallets, send transactions, interact with the Ordinals and Runes protocols, and much more. 

Inspiration for this repository came from the [taproot-wizards/purrfect_vault](https://github.com/taproot-wizards/purrfect_vault).

## Prerequisites

- A C++ compiler for building bitcoin-core
- Rust installation for building the Satoshi Suite and ord services
- [Just](https://github.com/casey/just) command runner

For reference, check the bitcoin core build [docs](https://github.com/bitcoin/bitcoin/blob/master/doc/build-unix.md) for required dependencies.

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

2. Copy the binary to a location in your PATH.
   ```bash
   sudo cp target/release/satoshi-suite /usr/local/bin/
   ```

## Configuration Options

When using the Satoshi Suite, you have two primary configuration options. Using a Internal or external Bitcoin Core node.

### Internal Bitcoin Core (Default)

- Uses a local Bitcoin Core instance managed by the suite
- Default RPC credentials (user/password)
- Data stored in ./data/bitcoin
- Ideal for development and testing

#### Using with Internal Bitcoin Core

The suite comes with built-in Bitcoin Core support. To use it:

1. Start the Bitcoin daemon:
   ```bash
   just start-bitcoind
   ```

2. In another terminal, execute commands against the local Bitcoin node:
   ```bash
   # Create a BTC wallet named "satoshi"
   satoshi-suite create-wallet satoshi

   # Get a new bech32m address for a wallet
   satoshi-suite get-new-address satoshi bech32m
   ```

3. To stop all services and delete data cache:
   ```bash
   just kill-all
   ```

### External Bitcoin Core

- Connect to any Bitcoin Core instance
- Support for both username/password and cookie authentication
- Flexible RPC URL configuration
- Useful for connecting to existing nodes or custom setups

#### Using with External Bitcoin Core

You can also use Satoshi Suite with your own Bitcoin Core instance. There are several ways to connect:

1. RPC URL configuration:
   ```bash
   satoshi-suite --rpc-url "http://localhost:8332" --rpc-username "myuser" --rpc-password "mypass" get-balance wallet1
   ```

3. Using cookie-based authentication:
   ```bash
   satoshi-suite --rpc-url "http://localhost:8332" --cookie-file "~/.bitcoin/.cookie" get-balance wallet1
   ```

### Network Configuration

- Supports mainnet, testnet, and regtest networks
- Network type can be specified with --network flag
- Default network is regtest and primarily used for spinning up test environments and generating quick feedback loops.

## Available Commands

**View all possible commands and their inputs using `just help` and `just -l`.**

### Wallet

| Command | Inputs | Description |
|---------|--------|-------------|
| `create-wallet` | `<wallet_name>` | Create a new Bitcoin wallet |
| `get-balance` | `<wallet_name>` | Get the balance of your Bitcoin wallet |
| `get-new-address` | `<wallet_name>` | Generate a new receive address |
| `list-unspent` | `<wallet_name>` | List all UTXOs for the specified wallet |
| `get-wallet-info` | `<wallet_name>` | Retrieve information related to the specified wallet |
| `get-address-info` | `<wallet_name> <wallet_address>` | Retrieve information related to a specific address |
| `sign-tx` | `<wallet_name> <recipient_address> <amount_in_btc> <fee_amount_in_btc> <utxo_selection_strategy>` | Sign a transaction |
| `send-btc` | `<wallet_name> <recipient_address> <amount_in_btc>` | Create, sign, and broadcast a BTC transaction |

### Multisig

| Command | Inputs | Description |
|---------|--------|-------------|
| `new-multisig` | `<num_required_signatures> <comma_separated_wallet_names> <multisig_name>` | Create a new multisig wallet |
| `create-psbt` | `<multisig_wallet_name> <recipient_address> <amount_in_btc> <fee_amount_in_btc> <utxo_selection_strategy>` | Create a multisig transaction |
| `decode-psbt` | `<psbt_hash>` | Retrieve the inputs and outputs for a specific PSBT |
| `analyze-psbt` | `<psbt_hash>` | Retrieve network-related information for a PSBT |
| `combine-psbts` | `<signed_psbt_1,signed_psbt_2,...>` | Combine multiple partially signed Bitcoin transactions |
| `finalize-psbt` | `<combined_psbt_hex>` | Finalize a fully signed PSBT |
| `finalize-psbt-and-broadcast` | `<combined_psbt_hex>` | Finalize and broadcast a fully signed PSBT |

### Network

| Command | Inputs | Description |
|---------|--------|-------------|
| `start-bitcoind` | - | Start a local Regtest Bitcoin network |
| `start-ord` | - | Start the ord server. View the ord explorer at `http://localhost:80`|
| `kill-all` | - | Terminate all services and clear cached data |
| `mine-blocks` | `<wallet_name> <number_of_blocks_to_mine>` | Mine blocks on the Regtest network |
| `get-tx` | `<tx_hash>` | Get information about a specific transaction |
| `get-tx-out` | `<tx_hash> <vout_index> <num_confirmations>` | Get transaction outputs |
| `broadcast-tx` | `<signed_tx_hash> <max-fee-rate>` | Broadcast a signed transaction |
| `get-spendable-balance` | `<address>` | Sum all UTXO amounts with 6+ confirmations |
| `bootstrap-env` | - | Init a fresh bitcoin test environment with ten wallets and 50 BTC in each wallet |

### Ordinal

| Command | Inputs | Description |
|---------|--------|-------------|
| `inscribe-ordinal` | `<wallet_name>` | Inscribe a new ordinal, using the pre-existing inscription data |
| `etch-rune` | `<wallet_name>` | Etch a rune, using the pre-existing inscription and runestone data |

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