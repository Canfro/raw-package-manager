#!/usr/bin/bash
cargo build --release
cp ./target/release/grm ~/.local/bin/
