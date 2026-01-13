#!/bin/bash
set -e

# Load cargo
source ~/.cargo/env

# Extract API key from .env
export OPENAI_API_KEY=$(grep OPENAI_API_KEY .env | cut -d= -f2)

# Run the test
cargo test -p docling-core --test llm_verification_tests "$@" -- --exact --ignored --nocapture
