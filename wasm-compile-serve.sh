#!/usr/bin/env bash
# Currently working toolchain according to https://github.com/GoogleChromeLabs/wasm-bindgen-rayon
#rustup run nightly-2022-04-07 \
#wasm-pack build --target web
yarn build
# Serve files
python server.py
