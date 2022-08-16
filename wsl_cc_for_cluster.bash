#!usr/bin/bash
RUSTFLAGS='-C target-feature=-static' cargo build --release --target x86_64-unknown-linux-gnu