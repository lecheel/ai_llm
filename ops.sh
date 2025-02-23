#! /bin/bash
cargo build --release
cp ./target/release/llm .
cp llm ~/bin
