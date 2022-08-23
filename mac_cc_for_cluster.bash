#!usr/bin/bash
TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl
scp /Users/Wyatt/rust/cs_reco/target/release/cs_reco wa41@civmcluster1:~/cs_recon_test