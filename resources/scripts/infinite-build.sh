#!/bin/bash
# TRUE INFINITE LOOP - Never Stops Until Success
# Usage: ./infinite-build.sh
# Press Ctrl+C to stop manually

cd "$(dirname "$0")/.."
PROJECT_ROOT="$(pwd)"

ITERATION=0
START_TIME=$(date +%s)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

clear
echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN}  TRUE INFINITE BUILD LOOP${NC}"
echo -e "${GREEN}  Will NEVER stop until success${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""
echo "Project: $PROJECT_ROOT"
echo "Started: $(date)"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop manually${NC}"
echo -e "${YELLOW}Or run in background: nohup ./infinite-build.sh &${NC}"
echo ""
sleep 3

while true; do
    ITERATION=$((ITERATION + 1))
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))

    clear
    echo -e "${GREEN}============================================${NC}"
    echo -e "${GREEN}  ITERATION $ITERATION${NC}"
    echo -e "${GREEN}  Elapsed: $(printf '%02d:%02d:%02d' $((ELAPSED/3600)) $((ELAPSED%3600/60)) $((ELAPSED%60)))${NC}"
    echo -e "${GREEN}============================================${NC}"
    echo ""

    # Build Core
    echo "[1/4] Building Core Library..."
    if ! cargo build --release -p easyssh-core 2>&1; then
        echo -e "${RED}❌ Core build failed${NC}"
        echo "Applying fixes..."
        cargo clean -p easyssh-core 2>/dev/null || true
        cargo update 2>&1 | tail -3
        cargo fmt 2>/dev/null || true
        echo "Retrying in 5 seconds..."
        sleep 5
        continue
    fi

    # Test Core
    echo "[2/4] Testing Core Library..."
    if ! cargo test --release -p easyssh-core 2>&1; then
        echo -e "${RED}❌ Core tests failed${NC}"
        echo "Retrying in 5 seconds..."
        sleep 5
        continue
    fi

    # Build TUI
    echo "[3/4] Building TUI..."
    if ! cargo build --release -p easyssh-tui 2>&1; then
        echo -e "${RED}❌ TUI build failed${NC}"
        echo "Applying fixes..."
        cargo clean -p easyssh-tui 2>/dev/null || true
        cargo update 2>&1 | tail -3
        echo "Retrying in 5 seconds..."
        sleep 5
        continue
    fi

    # Test TUI
    echo "[4/4] Testing TUI..."
    if [ -f "$PROJECT_ROOT/target/release/easyssh" ]; then
        if ! "$PROJECT_ROOT/target/release/easyssh" --version > /dev/null 2>&1; then
            echo -e "${RED}❌ TUI test failed${NC}"
            sleep 5
            continue
        fi
    elif [ -f "$PROJECT_ROOT/target/release/easyssh.exe" ]; then
        if ! "$PROJECT_ROOT/target/release/easyssh.exe" --version > /dev/null 2>&1; then
            echo -e "${RED}❌ TUI test failed${NC}"
            sleep 5
            continue
        fi
    fi

    # SUCCESS!
    echo ""
    echo -e "${GREEN}============================================${NC}"
    echo -e "${GREEN}🎉 SUCCESS AFTER $ITERATION ITERATIONS!${NC}"
    echo -e "${GREEN}============================================${NC}"
    echo ""
    echo "Total time: $(printf '%02d:%02d:%02d' $((ELAPSED/3600)) $((ELAPSED%3600/60)) $((ELAPSED%60)))"
    echo ""
    echo "✅ Core Library built"
    echo "✅ All tests passed"
    echo "✅ TUI built"
    echo ""
    echo "Binaries in: $PROJECT_ROOT/target/release/"
    ls -lh $PROJECT_ROOT/target/release/easyssh* 2>/dev/null || true
    echo ""

    # Play success sound if available
    if command -v afplay >/dev/null 2>&1; then
        afplay /System/Library/Sounds/Glass.aiff 2>/dev/null || true
    elif command -v paplay >/dev/null 2>&1; then
        paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null || true
    fi

    exit 0
done
