#!/usr/bin/env bash

set -e

BITCOIN_REPO="git@github.com:bitcoin/bitcoin.git"
ORD_REPO="git@github.com:ordinals/ord.git"
BITCOIN_VERSION="v27.0"
ORD_VERSION="0.21.0"

# Detect architecture
ARCH=$(uname -m)
echo "Detected architecture: $ARCH"

# Set architecture-specific flags
CONFIGURE_FLAGS=""
if [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "arm64" ]]; then
    echo "Configuring for ARM architecture..."
    # ARM-specific optimizations
    export CXXFLAGS="${CXXFLAGS} -march=armv8-a"
fi

if [ ! -d "bitcoin-core" ]; then
    echo "Building and setting up Bitcoin Core..."

    git clone $BITCOIN_REPO bitcoin-core
    pushd bitcoin-core
    git checkout $BITCOIN_VERSION
    ./autogen.sh
    
    # Configure with architecture awareness
    if [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "arm64" ]]; then
        ./configure --enable-hardening $CONFIGURE_FLAGS
    else
        ./configure $CONFIGURE_FLAGS
    fi
    
    # Use all available cores for compilation
    make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
    popd
fi

if [ ! -d "ord" ]; then
    echo "Cloning and setting up ord repo..."

    git clone $ORD_REPO ord

    pushd ord
    git checkout $ORD_VERSION
    
    # Build with optimizations for the current architecture
    if [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "arm64" ]]; then
        RUSTFLAGS="-C target-cpu=native" cargo build --release
    else
        cargo build --release
    fi
    
    popd
fi

echo "Setup complete for $ARCH architecture."