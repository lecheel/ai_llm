#! /bin/bash
if [[ $1 == "k" ]]; then 
    llm build
else
    cargo build --release
fi
cp ./target/release/llm .
cp llm ~/bin
