#!/bin/bash
# Continuous Fix Loop - Runs until success or manually stopped

set -e

ITERATION=0
MAX_ITERATIONS=${MAX_ITERATIONS:-100}
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=============================================="
echo "EASYSSH CONTINUOUS FIX LOOP"
echo "=============================================="
echo "Project: $PROJECT_ROOT"
echo "Max Iterations: $MAX_ITERATIONS (0 = unlimited)"
echo ""
echo "This script will:"
echo "  1. Build the project"
echo "  2. Run tests"
echo "  3. If failed, analyze errors and apply fixes"
echo "  4. Repeat until success"
echo ""
echo "Press Ctrl+C to stop"
echo "=============================================="
sleep 3

fix_build_error() {
    local error_log="$1"
    local fix_applied=false

    echo ""
    echo "=== Analyzing errors ==="

    # Check for specific error patterns and apply fixes

    # Fix 1: Missing dependencies
    if grep -q "could not find.*in crates.io" "$error_log" 2>/dev/null; then
        echo "Detected: Missing dependency"
        echo "Running cargo update..."
        cd "$PROJECT_ROOT"
        cargo update 2>&1 | head -20 || true
        fix_applied=true
    fi

    # Fix 2: Type mismatches (Clippy auto-fix)
    if grep -q "error:.*mismatched types" "$error_log" 2>/dev/null; then
        echo "Detected: Type mismatch"
        echo "Attempting cargo fix..."
        cd "$PROJECT_ROOT"
        rustup component add rustfmt clippy 2>/dev/null || true
        cargo fix --allow-dirty --allow-staged 2>&1 | head -30 || true
        fix_applied=true
    fi

    # Fix 3: Formatting issues
    if grep -q "differs from earlier declaration" "$error_log" 2>/dev/null; then
        echo "Detected: Naming inconsistency"
        echo "Running cargo fmt..."
        cd "$PROJECT_ROOT"
        cargo fmt 2>&1 || true
        fix_applied=true
    fi

    # Fix 4: Clippy warnings treated as errors
    if grep -q "error:.*clippy::" "$error_log" 2>/dev/null; then
        echo "Detected: Clippy error"
        echo "Attempting auto-fix..."
        cd "$PROJECT_ROOT"
        cargo clippy --fix --allow-dirty --allow-staged 2>&1 | head -50 || true
        fix_applied=true
    fi

    # Fix 5: Module not found - check if file exists
    if grep -q "unresolved import.*bridge" "$error_log" 2>/dev/null; then
        echo "Detected: Missing bridge module"
        # Create minimal bridge if missing
        for platform in linux macos windows; do
            bridge_file="$PROJECT_ROOT/platforms/$platform/easyssh-*/src/bridge.rs"
            if [ ! -f $bridge_file ]; then
                echo "Note: bridge.rs may need to be created for $platform"
            fi
        done
    fi

    if [ "$fix_applied" = true ]; then
        echo "✅ Fix applied"
        return 0
    else
        echo "⚠️ No automatic fix available for this error"
        echo "Error excerpt:"
        tail -50 "$error_log"
        return 1
    fi
}

run_build() {
    local iter=$1
    local log_file="/tmp/easyssh_build_${iter}.log"

    echo ""
    echo "=============================================="
    echo "ITERATION $iter"
    echo "=============================================="

    cd "$PROJECT_ROOT"

    # Step 1: Build Core
    echo "Building Core Library..."
    if ! cargo build --release -p easyssh-core 2>&1 | tee "$log_file"; then
        echo "❌ Core build failed"
        fix_build_error "$log_file"
        return 1
    fi

    # Step 2: Test Core
    echo "Testing Core Library..."
    if ! cargo test --release -p easyssh-core 2>&1 | tee -a "$log_file"; then
        echo "❌ Core tests failed"
        fix_build_error "$log_file"
        return 1
    fi

    # Step 3: Clippy
    echo "Running Clippy..."
    if ! cargo clippy -p easyssh-core -- -D warnings 2>&1 | tee -a "$log_file"; then
        echo "❌ Clippy failed"
        fix_build_error "$log_file"
        return 1
    fi

    # Step 4: Build TUI
    echo "Building TUI..."
    if ! cargo build --release -p easyssh-tui 2>&1 | tee -a "$log_file"; then
        echo "❌ TUI build failed"
        fix_build_error "$log_file"
        return 1
    fi

    # Step 5: Test TUI
    echo "Testing TUI..."
    if [ -f "$PROJECT_ROOT/target/release/easyssh" ]; then
        if ! "$PROJECT_ROOT/target/release/easyssh" --version; then
            echo "❌ TUI test failed"
            return 1
        fi
    elif [ -f "$PROJECT_ROOT/target/release/easyssh.exe" ]; then
        if ! "$PROJECT_ROOT/target/release/easyssh.exe" --version; then
            echo "❌ TUI test failed"
            return 1
        fi
    fi

    echo "✅ All builds successful!"
    return 0
}

# Main loop
while true; do
    ITERATION=$((ITERATION + 1))

    # Check max iterations
    if [ "$MAX_ITERATIONS" -gt 0 ] && [ "$ITERATION" -gt "$MAX_ITERATIONS" ]; then
        echo ""
        echo "=============================================="
        echo "Reached maximum iterations ($MAX_ITERATIONS)"
        echo "Stopping."
        echo "=============================================="
        exit 1
    fi

    # Run build
    if run_build "$ITERATION"; then
        echo ""
        echo "=============================================="
        echo "🎉 SUCCESS AFTER $ITERATION ITERATION(S)"
        echo "=============================================="
        echo ""
        echo "All builds passing:"
        echo "  ✅ Core Library"
        echo "  ✅ TUI CLI"
        echo ""
        echo "Binaries available in:"
        echo "  $PROJECT_ROOT/target/release/"
        echo ""
        exit 0
    fi

    echo ""
    echo "Build failed, retrying in 3 seconds..."
    echo "(Press Ctrl+C to stop)"
    sleep 3
done
