#!/bin/bash
set -e

# build Shiika
cargo build

# build corelib
cd lib/skc_rustlib
cargo build
cd ../..
cargo run -- build-corelib

echo "Shiika setup completed successfully!"
