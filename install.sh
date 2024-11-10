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

# Construct download URL
VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/satoshi-suite"

echo "⬇️  Downloading satoshi-suite $VERSION..."
curl -L "$DOWNLOAD_URL" -o $BINARY_NAME

echo "📦 Installing satoshi-suite..."
chmod +x $BINARY_NAME
sudo mv $BINARY_NAME /usr/local/bin/

echo "✅ Successfully installed satoshi-suite!"
echo "Run 'satoshi-suite --help' to get started"