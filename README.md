# Anoma

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE)
[![Drone CI build status](https://ci.heliax.dev/api/badges/anomanetwork/anoma/status.svg)](https://ci.heliax.dev/anomanetwork/anoma)

## Overview

[Anoma](https://anoma.network/) is a sovereign, proof-of-stake blockchain protocol that enables private, asset-agnostic cash and private bartering among any number of parties.

This is an implementation of the Anoma ledger in Rust.

## Docs

- [docs](https://anomanetwork.github.io/anoma/): built from [tech-specs mdBook](./tech-specs/)
- [rustdoc](https://anomanetwork.github.io/anoma/rustdoc/anoma/): built from the source

## Warning

> Here lay dragons: this codebase is still experimental, try at your own risk!

## Installing

### Dependencies

The ledger currently requires that [Tendermint version 0.34.x](https://github.com/tendermint/tendermint) is installed and available on path. [The pre-built binaries and the source for 0.34.8 are here](https://github.com/tendermint/tendermint/releases/tag/v0.34.8), also directly available in some package managers.

### Notes

The transaction code can currently be built from [tx_template](wasm/txs/tx_template) and validity predicates from [vp_template](wasm/vps/vp_template), which is Rust code compiled to wasm.

The transaction template calls functions from the host environment. The validity predicate template can validate a transaction and the storage key changes that is has performed.

The matchmaker template receives intents with the borsh encoding define in `data_template` and crafts data to be sent with `tx_intent_template` to the ledger. It matches only two intents that are the exact opposite.

### Instructions

```shell
# Build the validity predicate, transaction and matchmaker wasm modules
docker build -t anoma-wasm wasm
make build-wasm-scripts-docker

# Build Anoma
make

```

## Running an Anoma node

```shell
# Run Anoma node (this will also initialize and run Tendermint node)
make run-ledger

# Reset the state (resets Tendermint too)
make reset-ledger

# Submit a custom transaction with a wasm code and arbitrary data in `tx.data` file.
# Note that you have to have a `tx.data` file for this to work, albeit it can be empty.
cargo run --bin anoma -- tx --code-path wasm/txs/tx_template/tx.wasm --data-path tx.data

# Setup temporary addresses aliases until we have a better client support:

# User addresses
export ALBERT=a1qq5qqqqqg4znssfsgcurjsfhgfpy2vjyxy6yg3z98pp5zvp5xgersvfjxvcnx3f4xycrzdfkak0xhx
export BERTHA=a1qq5qqqqqxv6yydz9xc6ry33589q5x33eggcnjs2xx9znydj9xuens3phxppnwvzpg4rrqdpswve4n9
export CHRISTEL=a1qq5qqqqqxsuygd2x8pq5yw2ygdryxs6xgsmrsdzx8pryxv34gfrrssfjgccyg3zpxezrqd2y2s3g5s

# Fungible token addresses
export XAN=a1qq5qqqqqxuc5gvz9gycryv3sgye5v3j9gvurjv34g9prsd6x8qu5xs2ygdzrzsf38q6rss33xf42f3
export BTC=a1qq5qqqqq8q6yy3p4xyurys3n8qerz3zxxeryyv6rg4pnxdf3x3pyv32rx3zrgwzpxu6ny32r3laduc
export ETH=a1qq5qqqqqx3z5xd3ngdqnzwzrgfpnxd3hgsuyx3phgfry2s3kxsc5xves8qe5x33sgdprzvjptzfry9
export DOT=a1qq5qqqqqxq652v3sxap523fs8pznjse5g3pyydf3xqurws6ygvc5gdfcxyuy2deeggenjsjrjrl2ph

# Bite-sized tokens
export SCHNITZEL=a1qq5qqqqq8prrzv6xxcury3p4xucygdp5gfprzdfex9prz3jyg56rxv69gvenvsj9g5enswpcl8npyz
export APFEL=a1qq5qqqqqgfp52de4x56nqd3ex56y2wph8pznssjzx5ersw2pxfznsd3jxeqnjd3cxapnqsjz2fyt3j
export KARTOFFEL=a1qq5qqqqqxs6yvsekxuuyy3pjxsmrgd2rxuungdzpgsmyydjrxsenjdp5xaqn233sgccnjs3eak5wwh
```

## Interacting with Anoma

```shell
# Submit a token transfer
cargo run --bin anomac -- transfer --source $BERTHA --target $ALBERT --token $XAN --amount 10.1 --code-path wasm/txs/tx_transfer/tx.wasm

# Submit a transaction to update an account's validity predicate
cargo run --bin anomac -- update --address $BERTHA --code-path wasm/vps/vp_user/vp.wasm

# run gossip node with intent gossip system and rpc server (use default config)
cargo run --bin anoma -- run-gossip --rpc "127.0.0.1:39111"

# run gossip node with intent gossip system, matchmaker and rpc (use default config)
cargo run --bin anoman -- run-gossip --rpc "127.0.0.1:39111" --matchmaker-path matchmaker_template/matchmaker.wasm --tx-code-path wasm/txs/tx_from_intent/tx.wasm --ledger-address "127.0.0.1:26657"

# craft intents
cargo run --bin anomac -- craft-intent --address $ALBERT    --token-buy $ETH --amount-buy 10 --token-sell $BTC --amount-sell 20 --file-path intent_A.data
cargo run --bin anomac -- craft-intent --address $BERTHA   --token-buy $BTC --amount-buy 20 --token-sell $XAN --amount-sell 30 --file-path intent_B.data
cargo run --bin anomac -- craft-intent --address $CHRISTEL --token-buy $XAN --amount-buy 30 --token-sell $ETH --amount-sell 10 --file-path intent_C.data

# Subscribe to new network
cargo run --bin anomac -- subscribe-topic --node "http://127.0.0.1:39111" --topic "asset_v1"

# Submit the intents (need a rpc server)
cargo run --bin anomac -- intent --node "http://127.0.0.1:39111" --data-path intent_A.data --topic "asset_v1"
cargo run --bin anomac -- intent --node "http://127.0.0.1:39111" --data-path intent_B.data --topic "asset_v1"
cargo run --bin anomac -- intent --node "http://127.0.0.1:39111" --data-path intent_C.data --topic "asset_v1"

# Format the code
make fmt

# Lint the code
make clippy-check
```

## Logging

To change the log level, set `ANOMA_LOG` environment variable to one of:
- `error`
- `warn`
- `info`
- `debug`
- `trace`

The default is set to `info` for all the modules, expect for Tendermint ABCI, which has a lot of `debug` logging.

For more fine-grained logging levels settings, please refer to the [tracing subscriber docs](https://docs.rs/tracing-subscriber/0.2.18/tracing_subscriber/struct.EnvFilter.html#directives) for more information.

## How to contribute

Hi Anoma team 👋  
I'm new to Anoma and very excited about this project!  
I'd love to contribute to the community and be involved in the testnet.  
Looking forward to learning and growing with this amazing ecosystem. 🌱✨
