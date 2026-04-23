#!/bin/bash
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
PROJECT_NAME="dock"

build() {
    echo "Building $PROJECT_NAME..."
    cargo build --release
}

install_binary() {
    local src="target/release/$PROJECT_NAME"
    local dst="$INSTALL_DIR/$PROJECT_NAME"

    if [ ! -f "$src" ]; then
        echo "Error: Binary not found at $src"
        exit 1
    fi

    echo "Installing to $dst..."
    sudo cp "$src" "$dst"
    sudo chmod +x "$dst"
    echo "Installed $PROJECT_NAME to $dst"
}

uninstall() {
    local dst="$INSTALL_DIR/$PROJECT_NAME"
    if [ -f "$dst" ]; then
        echo "Removing $dst..."
        sudo rm "$dst"
        echo "Uninstalled $PROJECT_NAME"
    fi
}

case "${1:-install}" in
    install)
        build
        install_binary
        ;;
    build)
        build
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 {install|build|uninstall}"
        exit 1
        ;;
esac