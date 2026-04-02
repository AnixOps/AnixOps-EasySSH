#!/bin/bash
#
# CI Error Parser Script for EasySSH
# Parses compilation errors, categorizes them, and generates reports
#

set -e

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
OUTPUT_DIR="${OUTPUT_DIR:-./ci-reports}"
CRATES=("easyssh-core" "easyssh-tui" "easyssh-winui" "easyssh-gtk4" "easyssh-pro-server" "api-core")
VERSIONS=("lite" "standard" "pro")

# Error tracking
TOTAL_ERRORS=0
TOTAL_WARNINGS=0
ERROR_TYPES=()
ERROR_COUNTS=()
ERROR_FILES=()

# Initialize output directory
init_output() {
    mkdir -p "$OUTPUT_DIR"
    echo "{\"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\", \"errors\": [], \"summary\": {}}" > "$OUTPUT_DIR/errors.json"
}

# Log functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }

# Parse a single error line and categorize it
parse_error() {
    local line="$1"
    local crate="$2"
    local version="$3"

    # Error patterns
    if [[ "$line" =~ error\[E([0-9]+)\] ]]; then
        local code="E${BASH_REMATCH[1]}"
        local type="compile_error"
        local message=$(echo "$line" | sed 's/.*error\[E[0-9]*\]: //')

        # Categorize error type
        case "$code" in
            E0001|E0002|E0003|E0004|E0005) category="pattern_matching" ;;
            E0106|E0107|E0109|E0110) category="lifetime" ;;
            E0308|E0309|E0310|E0311) category="type_mismatch" ;;
            E0381|E0382|E0383|E0384) category="ownership" ;;
            E0422|E0423|E0424|E0425) category="unresolved" ;;
            E0432|E0433|E0434|E0435) category="import" ;;
            E0501|E0502|E0503|E0505|E0506|E0507) category="borrow_check" ;;
            E0603|E0609) category="visibility" ;;
            *) category="other" ;;
        esac

        echo "{\"type\": \"$type\", \"category\": \"$category\", \"code\": \"$code\", \"message\": \"$message\", \"crate\": \"$crate\", \"version\": \"$version\"}"
        return 0
    fi

    # Warning patterns
    if [[ "$line" =~ ^warning: ]]; then
        local message=$(echo "$line" | sed 's/^warning: //')
        echo "{\"type\": \"warning\", \"category\": \"warning\", \"code\": \"W001\", \"message\": \"$message\", \"crate\": \"$crate\", \"version\": \"$version\"}"
        return 0
    fi

    # Clippy warning patterns
    if [[ "$line" =~ warning:\ (clippy::[a-z_]+) ]]; then
        local lint="${BASH_REMATCH[1]}"
        local category="clippy"
        echo "{\"type\": \"clippy\", \"category\": \"$category\", \"code\": \"$lint\", \"message\": \"$line\", \"crate\": \"$crate\", \"version\": \"$version\"}"
        return 0
    fi

    # Test failure patterns
    if [[ "$line" =~ ^test.*\.\.\.\ FAILED ]]; then
        local test_name=$(echo "$line" | sed 's/^test //; s/ \.\.\. FAILED//')
        echo "{\"type\": \"test_failure\", \"category\": \"test\", \"code\": \"TF001\", \"message\": \"Test failed: $test_name\", \"crate\": \"$crate\", \"version\": \"$version\"}"
        return 0
    fi

    return 1
}

# Run cargo command and capture errors
run_and_capture() {
    local crate="$1"
    local version="$2"
    local cmd="$3"
    local log_file="$OUTPUT_DIR/${crate}-${version}-build.log"

    log_info "Running: $cmd (crate: $crate, version: $version)"

    # Run command and capture output
    if $cmd 2>&1 | tee "$log_file"; then
        log_success "Build succeeded for $crate ($version)"
        return 0
    else
        local exit_code=$?
        log_error "Build failed for $crate ($version) with exit code $exit_code"

        # Parse errors from log
        local error_file="$OUTPUT_DIR/${crate}-${version}-errors.json"
        echo "[" > "$error_file"
        local first=true

        while IFS= read -r line; do
            local parsed=$(parse_error "$line" "$crate" "$version")
            if [ -n "$parsed" ]; then
                if [ "$first" = true ]; then
                    first=false
                else
                    echo "," >> "$error_file"
                fi
                echo "$parsed" >> "$error_file"
            fi
        done < "$log_file"

        echo "]" >> "$error_file"

        return $exit_code
    fi
}

# Build all crates and collect errors
build_all_crates() {
    log_info "Starting comprehensive build check for all crates..."

    local all_success=true

    for crate in "${CRATES[@]}"; do
        case "$crate" in
            "easyssh-core")
                for version in "${VERSIONS[@]}"; do
                    if ! run_and_capture "$crate" "$version" "cargo build -p $crate --features $version"; then
                        all_success=false
                    fi
                done
                ;;
            "easyssh-winui")
                # Windows UI only builds on Windows
                if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
                    for version in "${VERSIONS[@]}"; do
                        if ! run_and_capture "$crate" "$version" "cargo build -p easyssh-winui --features $version --no-default-features"; then
                            all_success=false
                        fi
                    done
                else
                    log_warning "Skipping $crate (Windows only)"
                fi
                ;;
            "easyssh-gtk4")
                # GTK4 only builds on Linux
                if [[ "$OSTYPE" == "linux-gnu" ]]; then
                    for version in "${VERSIONS[@]}"; do
                        if ! run_and_capture "$crate" "$version" "cargo build -p easyssh-gtk4 --features easyssh-core/$version --no-default-features"; then
                            all_success=false
                        fi
                    done
                else
                    log_warning "Skipping $crate (Linux only)"
                fi
                ;;
            "easyssh-tui")
                for version in "${VERSIONS[@]}"; do
                    if ! run_and_capture "$crate" "$version" "cargo build -p easyssh-tui --features $version --no-default-features"; then
                        all_success=false
                    fi
                done
                ;;
            "easyssh-pro-server")
                if ! run_and_capture "$crate" "default" "cargo build -p easyssh-pro-server"; then
                    all_success=false
                fi
                ;;
            "api-core")
                if ! run_and_capture "$crate" "default" "cargo build -p api-core"; then
                    all_success=false
                fi
                ;;
        esac
    done

    if $all_success; then
        return 0
    else
        return 1
    fi
}

# Aggregate all errors into a single report
aggregate_errors() {
    log_info "Aggregating errors from all builds..."

    local report_file="$OUTPUT_DIR/error-report.json"
    local markdown_file="$OUTPUT_DIR/error-report.md"

    # Combine all error files
    echo "{\"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\", \"errors\": [" > "$report_file"

    local first=true
    local total_errors=0
    local total_warnings=0
    declare -A category_counts
    declare -A crate_errors

    for error_file in "$OUTPUT_DIR"/*-errors.json; do
        if [ -f "$error_file" ]; then
            # Parse JSON array (simple parsing for CI environment)
            while IFS= read -r line; do
                if [[ "$line" =~ ^\{ ]]; then
                    if [ "$first" = true ]; then
                        first=false
                    else
                        echo "," >> "$report_file"
                    fi
                    echo "$line" >> "$report_file"

                    # Count statistics
                    if [[ "$line" =~ \"type\":\ \"([^\"]+)\" ]]; then
                        local type="${BASH_REMATCH[1]}"
                        if [[ "$type" == "compile_error" ]] || [[ "$type" == "test_failure" ]]; then
                            ((total_errors++))
                        else
                            ((total_warnings++))
                        fi
                    fi

                    # Count by category
                    if [[ "$line" =~ \"category\":\ \"([^\"]+)\" ]]; then
                        local cat="${BASH_REMATCH[1]}"
                        ((category_counts[$cat]++))
                    fi

                    # Count by crate
                    if [[ "$line" =~ \"crate\":\ \"([^\"]+)\" ]]; then
                        local cr="${BASH_REMATCH[1]}"
                        ((crate_errors[$cr]++))
                    fi
                fi
            done < "$error_file"
        fi
    done

    echo "], \"summary\": {" >> "$report_file"
    echo "  \"total_errors\": $total_errors," >> "$report_file"
    echo "  \"total_warnings\": $total_warnings," >> "$report_file"
    echo "  \"total_issues\": $((total_errors + total_warnings))," >> "$report_file"
    echo "  \"categories\": {" >> "$report_file"

    first=true
    for cat in "${!category_counts[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "$report_file"
        fi
        echo "    \"$cat\": ${category_counts[$cat]}" >> "$report_file"
    done

    echo "  }," >> "$report_file"
    echo "  \"crates\": {" >> "$report_file"

    first=true
    for cr in "${!crate_errors[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "$report_file"
        fi
        echo "    \"$cr\": ${crate_errors[$cr]}" >> "$report_file"
    done

    echo "  }" >> "$report_file"
    echo "}}" >> "$report_file"

    # Generate markdown report
    generate_markdown_report "$markdown_file" "$total_errors" "$total_warnings" category_counts crate_errors

    log_info "Report generated: $report_file"
    log_info "Markdown report: $markdown_file"

    # Set outputs for GitHub Actions
    if [ -n "$GITHUB_OUTPUT" ]; then
        echo "total_errors=$total_errors" >> "$GITHUB_OUTPUT"
        echo "total_warnings=$total_warnings" >> "$GITHUB_OUTPUT"
        echo "has_errors=$([ $total_errors -gt 0 ] && echo 'true' || echo 'false')" >> "$GITHUB_OUTPUT"
        echo "report_path=$report_file" >> "$GITHUB_OUTPUT"
        echo "markdown_path=$markdown_file" >> "$GITHUB_OUTPUT"
    fi

    return $total_errors
}

# Generate markdown report
generate_markdown_report() {
    local file="$1"
    local errors="$2"
    local warnings="$3"
    local -n cats=$4
    local -n crts=$5

    cat > "$file" << EOF
# CI Error Detection Report

**Generated:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Commit:** ${GITHUB_SHA:-unknown}
**Ref:** ${GITHUB_REF:-unknown}

## Summary

| Metric | Count |
|--------|-------|
| Errors | $errors |
| Warnings | $warnings |
| **Total Issues** | **$((errors + warnings))** |

## Error Categories

| Category | Count |
|----------|-------|
EOF

    for cat in "${!cats[@]}"; do
        local count=${cats[$cat]}
        local icon="⚠️"
        case "$cat" in
            "compile_error") icon="🛑" ;;
            "type_mismatch") icon="🔀" ;;
            "lifetime") icon="⏱️" ;;
            "borrow_check") icon="📎" ;;
            "ownership") icon="🔒" ;;
            "unresolved") icon="❓" ;;
            "import") icon="📦" ;;
            "test") icon="🧪" ;;
            "clippy") icon="🔧" ;;
            *) icon="⚠️" ;;
        esac
        echo "| $icon $cat | $count |" >> "$file"
    done

    cat >> "$file" << EOF

## Crate Breakdown

| Crate | Issues |
|-------|--------|
EOF

    for cr in "${!crts[@]}"; do
        echo "| $cr | ${crts[$cr]} |" >> "$file"
    done

    cat >> "$file" << EOF

## Files

- JSON Report: \`error-report.json\`
- Markdown Report: \`error-report.md\`
- Build Logs: \`*-build.log\`

## Actions Required

EOF

    if [ $errors -eq 0 ]; then
        echo "✅ No errors detected. Build is clean!" >> "$file"
    else
        echo "❌ Errors detected. Please review the detailed logs above." >> "$file"
        echo "" >> "$file"
        echo "### Recommended Actions:" >> "$file"
        echo "1. Review error categories above" >> "$file"
        echo "2. Check individual build logs" >> "$file"
        echo "3. Run local build to reproduce" >> "$file"
    fi

    echo "" >> "$file"
    echo "---" >> "$file"
    echo "*Report generated by CI Error Detection*" >> "$file"
}

# Generate GitHub Actions step summary
generate_step_summary() {
    if [ -f "$OUTPUT_DIR/error-report.md" ]; then
        cat "$OUTPUT_DIR/error-report.md" >> "$GITHUB_STEP_SUMMARY"
    fi
}

# Main execution
main() {
    local command="${1:-full}"

    case "$command" in
        init)
            init_output
            ;;
        build)
            init_output
            build_all_crates
            ;;
        aggregate)
            aggregate_errors
            ;;
        summary)
            generate_step_summary
            ;;
        full)
            init_output
            if build_all_crates; then
                log_success "All builds passed!"
                aggregate_errors
                exit 0
            else
                log_error "Some builds failed!"
                aggregate_errors
                generate_step_summary
                exit 1
            fi
            ;;
        *)
            echo "Usage: $0 {init|build|aggregate|summary|full}"
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
