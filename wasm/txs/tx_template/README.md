# Transaction script wasm template

This is just a template of a wasm module for transaction script. This template
receives arbitrary bytes as its input.

## Quick start

```shell
# To be able to build this, make sure to have
make deps

# Build - this will create `tx.wasm` file
make build-release
```

The crate is configured to build into wasm in [cargo config](.cargo/config).
