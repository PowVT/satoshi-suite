#!/usr/bin/env bash

set -e


BITCOIN_REPO="git@github.com:bitcoin/bitcoin.git"
ORD_REPO="git@github.com:ordinals/ord.git"
BITCOIN_VERSION="v27.0"

if [ ! -d "bitcoin-core" ]; then
    echo "Building and setting up Bitcoin Core..."

    git clone $BITCOIN_REPO bitcoin-core
    pushd bitcoin-core
    git checkout $BITCOIN_VERSION
    ./autogen.sh
    ./configure
    make -j4
    popd
fi

if [ ! -d "ord" ]; then
    echo "Cloning and setting up ord repo..."

    git clone $ORD_REPO ord

    pushd ord
    cargo build --release
    popd
fi

echo "Setup complete."