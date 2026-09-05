#!/usr/bin/env bash
set -e

echo "======================================================================"
echo "          Running RestPHP Complete E2E Verification Suite             "
echo "======================================================================"

# Ensure environment PATH has cargo
export PATH="$HOME/.cargo/bin:$PATH"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> 1. Building RestPHP debug binary..."
cargo build

echo ""
echo ">>> 2. Running Rust Integration Test Suite (cargo test --test e2e_test_suite)..."
cargo test --test e2e_test_suite -- --nocapture

echo ""
echo ">>> 3. Running Comprehensive 4-Tier Python E2E Test Suite..."
python3 tests/run_e2e_tests.py

echo ""
echo "======================================================================"
echo "                 E2E Verification Run Completed                       "
echo "======================================================================"
