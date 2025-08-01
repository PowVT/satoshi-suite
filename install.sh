#!/bin/bash

set -e

REPO="PowVT/satoshi-suite"
BINARY_NAME="satoshi-suite"

# Determine OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Get latest release version
VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

# Determine platform-specific asset name
case $OS in
    darwin)
        ASSET_NAME="satoshi-suite-${VERSION}-${ARCH}-apple-darwin.tar.gz"
        ;;
    linux)
        ASSET_NAME="satoshi-suite-${VERSION}-${ARCH}-unknown-linux-gnu.tar.gz"
        ;;
    *)
        echo "Unsupported operating system: $OS"
        exit 1
        ;;
esac

# Construct download URL
DOWNLOAD_URL="https://github.com/$REPO/releases/download/${VERSION}/${ASSET_NAME}"

echo "⬇️  Downloading satoshi-suite ${VERSION}..."
curl -L "$DOWNLOAD_URL" -o "$ASSET_NAME"

echo "📦 Extracting..."
tar xzf "$ASSET_NAME"

echo "🔧 Installing..."
chmod +x satoshi-suite
sudo mv satoshi-suite /usr/local/bin/

echo "🧹 Cleaning up..."
rm -f "$ASSET_NAME"

echo "✅ Successfully installed satoshi-suite!"
echo "Run 'satoshi-suite --help' to get started"