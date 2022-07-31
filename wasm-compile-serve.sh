#!/usr/bin/env bash
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' \                                                                                                                                                                                                       wasm ✱ ◼
                    rustup run nightly-2022-04-07 \
                    wasm-pack build --target web
python server.py