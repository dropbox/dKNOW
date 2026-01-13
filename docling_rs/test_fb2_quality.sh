#!/bin/bash
# Test FB2 LLM Quality

# Load API key from .env
export $(grep -v '^#' .env | xargs)

# Run test
/Users/ayates/.cargo/bin/cargo test -p docling-core --test llm_verification_tests test_llm_mode3_fb2 -- --exact --ignored --nocapture
