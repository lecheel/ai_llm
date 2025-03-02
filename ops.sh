#! /bin/bash
if [[ $1 == "k" ]]; then 
    echo -e "\033[1;32m -= llm build =-\033[0m"
    llm build
else
    cargo build --release
fi
cp ./target/release/llm .
cp llm ~/bin
