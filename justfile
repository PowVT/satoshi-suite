# list all commands
help:
    RUST_LOG=info ./target/release/satoshi-suite --help

##############################
### Development Variables  ###
##############################

# Default settings for local development
bitcoin_datadir := "./data/bitcoin"
bitcoin_rpc_user := "user"
bitcoin_rpc_pass := "password"

# Bitcoin CLI command builder for local node
bitcoin_cli := "./bitcoin-core/src/bitcoin-cli -regtest -rpcuser=" + bitcoin_rpc_user + " -rpcpassword=" + bitcoin_rpc_pass
bitcoind := "./bitcoin-core/src/bitcoind -regtest -rpcuser=" + bitcoin_rpc_user + " -rpcpassword=" + bitcoin_rpc_pass

# Ord settings
ord_datadir := "./data/ord"
ord := "./ord/target/release/ord --regtest --bitcoin-rpc-username=" + bitcoin_rpc_user + " --bitcoin-rpc-password=" + bitcoin_rpc_pass

############################
### Development Commands ###
############################

# build rust binary
alias b := build

build:
    cargo build --release

# install bitcoin and ord dependencies
install-deps:
    bash ./scripts/build_bitcoincore_and_ord.sh
    just build

# start local Bitcoind server
start-bitcoind *ARGS:
    mkdir -p {{ bitcoin_datadir }}
    {{ bitcoind }} -timeout=15000 -server=1 -txindex=1 -fallbackfee=1.0 -maxtxfee=1.1 -deprecatedrpc=warnings -datadir={{bitcoin_datadir}} {{ ARGS }}

# stop local Bitcoind server
stop-bitcoind:
    @if lsof -ti :18443 >/dev/null 2>&1; then \
        {{ bitcoin_cli }} stop; \
        echo "bitcoind server on port 18443 stopped."; \
    else \
        echo "No bitcoind server found running on port 18443."; \
    fi

# remove local Bitcoind data
clean-bitcoin-data:
    rm -rf {{ bitcoin_datadir }}

# bootstrap local BTC chain
bootstrap-btc:
    just clean-bitcoin-data
    just stop-bitcoind
    just start-bitcoind

# start local Ordinal server
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

# stop local Ordinal server
stop-ord:
    @if lsof -ti :80 >/dev/null 2>&1; then \
        kill $(lsof -t -i:80); \
        echo "ord server on port 80 killed."; \
    else \
        echo "No ord server found running on port 80."; \
    fi

# remove local Ordinals data
clean-ord-data:
    rm -rf {{ ord_datadir }}

# stop all local services and remove all cached data
kill-all:
    just stop-bitcoind
    just stop-ord
    just clean-bitcoin-data
    just clean-ord-data