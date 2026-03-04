#!/usr/bin/bash
cargo build --release
sudo cp ./target/release/github-repository-manager ~/.local/bin/
