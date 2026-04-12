#!/bin/bash
# SPDX-License-Identifier: MIT

set -e

REPO="ininids/aam-cli"
BINARY_NAME="aam"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$ARCH" == "x86_64" ]; then ARCH="amd64"; fi
if [ "$ARCH" == "aarch64" ] || [ "$ARCH" == "arm64" ]; then ARCH="arm64"; fi

ASSET_NAME="${BINARY_NAME}-${OS}-${ARCH}"
if [ "$OS" == "windows" ]; then ASSET_NAME="${ASSET_NAME}.exe"; fi

URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest \
  | grep "browser_download_url" \
  | grep "$ASSET_NAME" \
  | cut -d '"' -f 4)

echo "Downloading $BINARY_NAME from $URL..."
curl -L "$URL" -o /tmp/$BINARY_NAME

chmod +x /tmp/$BINARY_NAME
sudo mv /tmp/$BINARY_NAME /usr/local/bin/
echo "Done! Try running '$BINARY_NAME --version'"