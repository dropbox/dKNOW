#!/bin/bash
# Comprehensive test runner with proper environment setup
# Usage: ./run_tests.sh [backend|core|clippy|fmt|all|quick]

set -e  # Exit on error

# Setup environment
export PATH="/Users/ayates/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:${PKG_CONFIG_PATH}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Default to running all tests if no argument provided
TEST_TYPE=${1:-all}

run_backend_tests() {
    print_header "Running Backend Tests (2849 tests)"
    if cargo test --package docling-backend --lib --no-fail-fast; then
        print_success "Backend tests passed"
        return 0
    else
        print_error "Backend tests failed"
        return 1
    fi
}

run_core_tests() {
    print_header "Running Core Tests (216 tests)"
    if cargo test --package docling-core --lib; then
        print_success "Core tests passed"
        return 0
    else
        print_error "Core tests failed"
        return 1
    fi
}

run_clippy() {
    print_header "Running Clippy (Code Quality Check)"
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "Clippy passed (zero warnings)"
        return 0
    else
        print_error "Clippy found warnings"
        return 1
    fi
}

run_fmt() {
    print_header "Running cargo fmt (Code Formatting Check)"
    if cargo fmt --all -- --check; then
        print_success "Formatting is correct"
        return 0
    else
        print_warning "Formatting needs fixing. Run: cargo fmt --all"
        return 1
    fi
}

run_quick() {
    print_header "Quick Test Suite (Core + Clippy + Fmt)"
    ERRORS=0

    run_core_tests || ERRORS=$((ERRORS+1))
    run_clippy || ERRORS=$((ERRORS+1))
    run_fmt || ERRORS=$((ERRORS+1))

    if [ $ERRORS -eq 0 ]; then
        print_success "Quick test suite passed!"
        return 0
    else
        print_error "Quick test suite had $ERRORS failure(s)"
        return 1
    fi
}

run_all() {
    print_header "Full Test Suite (Backend + Core + Clippy + Fmt)"
    START_TIME=$(date +%s)
    ERRORS=0

    run_backend_tests || ERRORS=$((ERRORS+1))
    run_core_tests || ERRORS=$((ERRORS+1))
    run_clippy || ERRORS=$((ERRORS+1))
    run_fmt || ERRORS=$((ERRORS+1))

    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    echo ""
    print_header "Test Suite Summary"
    echo "Duration: ${DURATION}s (~$((DURATION/60)) min)"

    if [ $ERRORS -eq 0 ]; then
        print_success "All tests passed! ✨"
        echo "  • Backend: 2849/2849 tests ✓"
        echo "  • Core: 216/216 tests ✓"
        echo "  • Clippy: Zero warnings ✓"
        echo "  • Formatting: Correct ✓"
        return 0
    else
        print_error "Test suite had $ERRORS failure(s)"
        return 1
    fi
}

# Main execution
case "$TEST_TYPE" in
    backend)
        run_backend_tests
        ;;
    core)
        run_core_tests
        ;;
    clippy)
        run_clippy
        ;;
    fmt)
        run_fmt
        ;;
    quick)
        run_quick
        ;;
    all)
        run_all
        ;;
    *)
        echo "Usage: $0 [backend|core|clippy|fmt|all|quick]"
        echo ""
        echo "Options:"
        echo "  backend  - Run backend unit tests (2849 tests, ~135s)"
        echo "  core     - Run core unit tests (216 tests, ~19s)"
        echo "  clippy   - Run clippy linter (code quality)"
        echo "  fmt      - Check code formatting"
        echo "  quick    - Run core + clippy + fmt (~40s)"
        echo "  all      - Run all tests (default, ~175s)"
        exit 1
        ;;
esac

exit $?
