#!/usr/bin/bash
cargo build --release
sudo cp ./target/release/raw-package-manager /usr/local/bin
