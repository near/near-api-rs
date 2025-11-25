#!/usr/bin/env bash
set -e
# Run all examples
for example in $(find . -name "*.rs" -type f); do
    example_name=$(basename $example .rs)

    echo "--------------------------------"
    echo "Running $example_name"
    echo "--------------------------------"
    CI=true cargo run --release --all-features --example $example_name
    echo "--------------------------------"
done
