#!/bin/bash
# Auto-build script for EasySSH versions
# Usage: ./auto_build.sh [lite|standard|pro|all]

set -e

cd "C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH"

LOG_DIR="./build_logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)

build_lite() {
    echo "🔨 Building EasySSH Lite..."
    cargo build --release --bin EasySSH-Lite 2>&1 | tee "$LOG_DIR/lite_$TIMESTAMP.log"
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo "✅ EasySSH Lite built successfully"
        ls -lh target/release/EasySSH-Lite.exe 2>/dev/null || true
    else
        echo "❌ EasySSH Lite build failed - see $LOG_DIR/lite_$TIMESTAMP.log"
        return 1
    fi
}

build_standard() {
    echo "🔨 Building EasySSH Standard..."
    cargo build --release --bin EasySSH-Standard --features=embedded-terminal,split-screen,sftp,monitoring 2>&1 | tee "$LOG_DIR/standard_$TIMESTAMP.log"
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo "✅ EasySSH Standard built successfully"
        ls -lh target/release/EasySSH-Standard.exe 2>/dev/null || true
    else
        echo "❌ EasySSH Standard build failed - see $LOG_DIR/standard_$TIMESTAMP.log"
        return 1
    fi
}

build_pro() {
    echo "🔨 Building EasySSH Pro..."
    cargo build --release --bin EasySSH-Pro --features=pro 2>&1 | tee "$LOG_DIR/pro_$TIMESTAMP.log"
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo "✅ EasySSH Pro built successfully"
        ls -lh target/release/EasySSH-Pro.exe 2>/dev/null || true
    else
        echo "❌ EasySSH Pro build failed - see $LOG_DIR/pro_$TIMESTAMP.log"
        return 1
    fi
}

case "${1:-all}" in
    lite)
        build_lite
        ;;
    standard)
        build_standard
        ;;
    pro)
        build_pro
        ;;
    all)
        build_lite && build_standard && build_pro
        ;;
    *)
        echo "Usage: $0 [lite|standard|pro|all]"
        exit 1
        ;;
esac
