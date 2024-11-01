##################
# Chain Commands #
##################

# list all commands
help:
    RUST_LOG=info ./target/release/satoshi-suite --help

# get current block height
get-block-height:
    RUST_LOG=info ./target/release/satoshi-suite get-block-height

# Rescan the local blockchain for wallet related transactions. Use to import multisig wallet balances
rescan-blockchain start="0":
    RUST_LOG=info ./target/release/satoshi-suite rescan-blockchain -s {{ start }}

# broadcast a signed BTC transaction
broadcast-tx tx_hex:
    RUST_LOG=info ./target/release/satoshi-suite broadcast-tx -t {{ tx_hex }}

###################
# Wallet Commands #
###################

# get new wallet
new-wallet wallet_name:
    RUST_LOG=info ./target/release/satoshi-suite new-wallet -w {{ wallet_name }}

# get wallet info
get-wallet-info wallet_name:
    RUST_LOG=info ./target/release/satoshi-suite get-wallet-info -w {{ wallet_name }}

# list descriptors
list-descriptors wallet_name:
    RUST_LOG=info ./target/release/satoshi-suite list-descriptors -w {{ wallet_name }}

# create a new multisig wallet
new-multisig wallet_names multisig_name required_signatures="2":
    RUST_LOG=info ./target/release/satoshi-suite new-multisig -v {{ wallet_names }} -m {{ multisig_name }} -n {{ required_signatures }}

# get new wallet address
get-new-address wallet_name address_type="bech32m":
    RUST_LOG=info ./target/release/satoshi-suite get-new-address -w {{ wallet_name }} -z {{ address_type }}

# get address info
get-address-info wallet_name address:
    RUST_LOG=info ./target/release/satoshi-suite get-address-info -w {{ wallet_name }} -a {{ address }}

# derive address
derive-addresses descriptor="wpkh([0000000]/84'/1'/0'/0/*)" start="0" end="2":
    RUST_LOG=info ./target/release/satoshi-suite derive-addresses -d {{ descriptor }} -s {{ start }} -e {{ end }}

# get wallet balance
get-balance wallet_name:
    RUST_LOG=info ./target/release/satoshi-suite get-balance -w {{ wallet_name }}

# mine blocks to a particular wallet
mine-blocks wallet_name blocks="20" address_type="bech32":
    RUST_LOG=info ./target/release/satoshi-suite mine-blocks -w {{ wallet_name }} -b {{ blocks }} -z {{ address_type }}

# list unspent transactions
list-unspent wallet_name:
    RUST_LOG=info ./target/release/satoshi-suite list-unspent -w {{ wallet_name }}

# get transaction data from transaction ID
get-tx wallet_name txid:
    RUST_LOG=info ./target/release/satoshi-suite get-tx -w {{ wallet_name }} -i {{ txid }}

# get details about an unspent transaction output
get-tx-out txid vout="0":
    RUST_LOG=info ./target/release/satoshi-suite get-tx-out -i {{ txid }} -o {{ vout }}

# decode raw transaction
decode-raw-tx tx_hex:
    RUST_LOG=info ./target/release/satoshi-suite decode-raw-tx -t {{ tx_hex }}

# create a signed BTC transaction
sign-tx wallet_name recipient amount fee_amount="0.01" utxo_strat="fifo":
    RUST_LOG=info ./target/release/satoshi-suite sign-tx -w {{ wallet_name }} -r {{ recipient }} -x {{ amount }} -f {{ fee_amount }} -y {{ utxo_strat }}

# send BTC to recipient address
send-btc wallet_name recipient amount:
    RUST_LOG=info ./target/release/satoshi-suite send-btc -w {{ wallet_name }} -r {{ recipient }} -x {{ amount }}

# create partially signed BTC transaction
create-psbt wallet_name recipient amount fee_amount="0.01" utxo_strat="fifo":
    RUST_LOG=info ./target/release/satoshi-suite create-psbt -w {{ wallet_name }} -r {{ recipient }} -x {{ amount }} -f {{ fee_amount }} -y {{ utxo_strat }}

# process PSBT with wallet
process-psbt wallet_name psbt_hex:
    RUST_LOG=info ./target/release/satoshi-suite process-psbt -w {{ wallet_name }} -p {{ psbt_hex }}

# decode partially signed BTC transaction
decode-psbt psbt_hex:
    RUST_LOG=info ./target/release/satoshi-suite decode-psbt -p {{ psbt_hex }}

# analyze partially signed BTC transaction
analyze-psbt psbt_hex:
    RUST_LOG=info ./target/release/satoshi-suite analyze-psbt -p {{ psbt_hex }}

# combine partially signed BTC transactions
combine-psbts psbts:
    RUST_LOG=info ./target/release/satoshi-suite combine-psbts -l {{ psbts }}

# finalize partially signed BTC transaction
finalize-psbt psbt_hex:
    RUST_LOG=info ./target/release/satoshi-suite finalize-psbt -p {{ psbt_hex }}

# finalize and broadcast PSBT
finalize-psbt-and-broadcast psbt_hex:
    RUST_LOG=info ./target/release/satoshi-suite finalize-psbt-and-broadcast -p {{ psbt_hex }}

# verify signed transaction
verify-signed-tx tx_hex:
    RUST_LOG=info ./target/release/satoshi-suite verify-signed-tx -t {{ tx_hex }}

#########################
### Ordinal Commands ####
#########################

# Inscribe an ordinal using satoshi-suite
inscribe-ordinal wallet_name file_path postage="50000":
    RUST_LOG=info ./target/release/satoshi-suite inscribe-ordinal -w {{ wallet_name }} -p {{ postage }} -f {{ file_path }}

# Etch a rune using satoshi-suite
etch-rune wallet_name file_path postage="50000":
    RUST_LOG=info ./target/release/satoshi-suite etch-rune -w {{ wallet_name }} -p {{ postage }} -f {{ file_path }}

###################################
# Build and Bootstrap Commands #
###################################

bitcoin_datadir := "./data/bitcoin"
bitcoin_cli := "./bitcoin-core/src/bitcoin-cli -regtest -rpcuser=user -rpcpassword=password"
bitcoind := "./bitcoin-core/src/bitcoind -regtest -rpcuser=user -rpcpassword=password"

ord_datadir := "./data/ord"
ord := "./ord/target/release/ord --regtest --bitcoin-rpc-username=user --bitcoin-rpc-password=password"

# bootstrap a testing environment. Gives you 10 wallets (wallet1, wallet2, etc.) with 50 BTC each.
bootstrap-env address_type="bech32":
    RUST_LOG=info ./target/release/satoshi-suite bootstrap-env -z {{ address_type }}

# start Bitcoind server
start-bitcoind *ARGS:
    mkdir -p {{ bitcoin_datadir }}
    {{ bitcoind }} -timeout=15000 -server=1 -txindex=1 -fallbackfee=1.0 -maxtxfee=1.1 -deprecatedrpc=warnings -datadir={{bitcoin_datadir}} {{ ARGS }}

# stop Bitcoind server
stop-bitcoind:
    @if lsof -ti :18443 >/dev/null 2>&1; then \
        {{ bitcoin_cli }} stop; \
        echo "bitcoind server on port 18443 stopped."; \
    else \
        echo "No bitcoind server found running on port 18443."; \
    fi

# remove Bitcoind data
clean-bitcoin-data:
    rm -rf {{ bitcoin_datadir }}

# bootstrap BTC chain
bootstrap-btc:
    just clean-bitcoin-data
    just stop-bitcoind
    just start-bitcoind

# start the Ordinal server
start-ord *ARGS:
    mkdir -p {{ ord_datadir }}
    @if lsof -ti :18443 >/dev/null 2>&1; then \
        {{ ord }} \
            --data-dir={{ord_datadir}} \
            --index-addresses \
            --index-runes \
            --index-sats \
            --index-transactions \
            --commit-interval=1 \
            {{ ARGS }} \
            server; \
    else \
        echo "run just bootstrap-btc before starting ord server."; \
    fi

# kill the Ordinal server
stop-ord:
    @if lsof -ti :80 >/dev/null 2>&1; then \
        kill $(lsof -t -i:80); \
        echo "ord server on port 80 killed."; \
    else \
        echo "No ord server found running on port 80."; \
    fi

# remove Ordinals data
clean-ord-data:
    rm -rf {{ ord_datadir }}

# stop all services and remove all cached data
kill-all:
    just stop-bitcoind
    just stop-ord
    just clean-bitcoin-data
    just clean-ord-data

alias b := build

# build rust binary
build:
    cargo build --release

# install bitcoin and ord dependencies
install-deps:
    bash ./scripts/build_bitcoincore_and_ord.sh
    just build