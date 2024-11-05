#!/bin/bash

IS_CI=${IS_CI:-false}

# clean up Rust builds to free space
cargo install cargo-clean-all
cargo clean-all -y
if [ "$IS_CI" = true ]; then
  # remove all docker images
  docker system prune -af
fi