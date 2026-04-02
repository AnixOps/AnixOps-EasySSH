# EasySSH Core Test Framework

This directory contains the unit and integration tests for the EasySSH core library.

## Test Structure

```
tests/
├── common/                    # Shared test utilities
│   ├── mod.rs                # Test helpers, fixtures, assertions
│   └── data_generator.rs     # Test data generation utilities
├── fixtures/                  # Test data files
│   ├── test_data.json        # Sample servers, groups, identities
│   └── comprehensive_test_data.json # Full test dataset
├── unit/                      # Unit tests
│   ├── crypto_tests.rs       # Cryptography tests
│   ├── database_tests.rs     # Database CRUD tests
│   ├── ssh_tests.rs          # SSH configuration tests
│   ├── server_service_tests.rs # Business logic tests
│   ├── search_tests.rs       # Search functionality tests
│   ├── security_tests.rs     # Security tests (NEW)
│   ├── performance_tests.rs  # Performance tests (NEW)
│   └── fuzz_tests.rs         # Fuzz/property tests (NEW)
└── integration/               # Integration tests
    ├── workflow_tests.rs     # End-to-end workflow tests
    ├── database_integration_tests.rs # Database integration (NEW)
    └── ssh_integration_tests.rs # SSH integration tests (NEW)
```

## Running Tests

### Run all tests
```bash
cd crates/easyssh-core
cargo test
```

### Run specific test file
```bash
cargo test --test crypto_tests
cargo test --test security_tests
cargo test --test performance_tests
cargo test --test fuzz_tests
cargo test --test database_integration_tests
cargo test --test ssh_integration_tests
```

### Run tests by category
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*integration*'

# Security tests
cargo test security

# Performance tests
cargo test performance
```

### Run tests with output
```bash
cargo test -- --nocapture
```

### Run tests with coverage
```bash
cargo tarpaulin --out html
cargo llvm-cov --html
```

### Run benchmarks
```bash
cargo bench
```

## Test Coverage Goals

| Module | Target Coverage | Current Status |
|--------|-----------------|----------------|
| Crypto | >= 90% | Tests written |
| Database | >= 85% | Tests written |
| SSH | >= 80% | Tests written |
| Services | >= 85% | Tests written |
| Search | >= 80% | Tests written |
| Security | >= 95% | Tests written |
| Performance | Critical Paths | Tests written |
| Integration | Core Flows | Tests written |

## Test Utilities

### Common Helpers (`common/mod.rs`)

- `create_test_db()` - Create temporary database
- `create_in_memory_db()` - Create in-memory database
- `load_test_fixtures()` - Load JSON test fixtures
- `test_master_password()` - Get test password
- `test_encryption_data()` - Get test encryption data
- `TestServerFixture` - Pre-configured server fixtures

### Data Generator (`common/data_generator.rs`)

- `TestDataGenerator` - Generate test data programmatically
- `scenarios::production_environment()` - Production-like setup
- `scenarios::team_collaboration_setup()` - Team setup
- `scenarios::edge_cases()` - Edge case data
- `ssh_configs::sample_ssh_config()` - Sample SSH configs

### Assertion Helpers

```rust
use common::assertions::assert_error_contains;
use common::assertions::assert_bytes_eq;

assert_error_contains(result, "expected error message");
assert_bytes_eq(&data1, &data2);
```

## Key Test Scenarios

### Crypto Tests
- Encryption/decryption roundtrip
- Key derivation with Argon2id
- Wrong password handling
- Lock/unlock state management
- Secure memory clearing
- Timing attack resistance
- Non-deterministic encryption (nonce uniqueness)

### Database Tests
- CRUD operations for all entities
- Transaction support
- Foreign key constraints
- Concurrent access safety
- Pagination
- Bulk operations
- Import/export

### SSH Tests
- Server configuration validation
- Authentication method handling
- Connection health tracking
- Known hosts management
- Port forwarding configuration
- Proxy/Jump host configuration

### Service Tests
- Server CRUD with business rules
- Duplicate detection
- Import/export functionality
- Search and filtering
- Bulk operations

### Security Tests
- SQL injection prevention
- Command injection prevention
- Path traversal prevention
- Input validation
- Rate limiting
- Memory zeroization
- Side-channel resistance

### Performance Tests
- Encryption/decryption throughput
- Database query performance
- Search performance
- Memory usage patterns
- Concurrent access performance
- Startup time

### Fuzz Tests
- Random input handling
- Configuration parsing edge cases
- Protocol compliance
- Extreme values
- Pathological patterns
- Concurrent stress testing

### Integration Tests
- End-to-end workflows
- Cross-module interactions
- Error handling chains
- Concurrent operations
- SSH connection lifecycle
- Database transaction integrity

## Writing New Tests

1. Add test utilities to `tests/common/mod.rs` if needed
2. Create test file in `tests/unit/` or `tests/integration/`
3. Include `mod common;` at the top of test files
4. Use `#[test]` for sync tests, `#[tokio::test]` for async
5. Follow naming convention: `test_<functionality>_<scenario>`

### Example Test

```rust
#[test]
fn test_encryption_roundtrip() {
    let mut state = CryptoState::new();
    state.initialize("password").unwrap();

    let plaintext = b"test data";
    let encrypted = state.encrypt(plaintext).unwrap();
    let decrypted = state.decrypt(&encrypted).unwrap();

    assert_eq!(decrypted, plaintext);
}
```

## CI Integration

Tests are configured to run in CI with:
- Debug and release builds
- All feature combinations (Lite, Standard, Pro)
- Code coverage reporting with cargo-llvm-cov
- Security audit with cargo-audit
- Benchmark baselines
- Multi-platform testing (Ubuntu, Windows, macOS)

See `.github/workflows/test.yml` for CI configuration.

### CI Jobs Overview

| Job | Purpose | Trigger |
|-----|---------|---------|
| Check & Format | Linting and formatting | Every push/PR |
| Unit Tests | Fast feedback on core logic | Every push/PR |
| Integration Tests | Full system testing | Every push/PR |
| Security Tests | Vulnerability scanning | Every push/PR |
| Coverage | Track test coverage | Every push/PR |
| Benchmarks | Performance regression | Every push/PR |
| Build Verification | Cross-platform builds | Every push/PR |

## Test Dependencies

The following dev-dependencies are required for testing:

- `tokio-test` - Async testing utilities
- `tempfile` - Temporary files and directories
- `mockall` - Mock objects for testing
- `criterion` - Benchmark tests
- `serde_json` - JSON test data

### Additional Tools (install separately)

```bash
# Code coverage
cargo install cargo-llvm-cov

# Security audit
cargo install cargo-audit

# Mutation testing
cargo install cargo-mutants

# Next-gen test runner
cargo install cargo-nextest
```

---

*Last updated: 2026-04-02*
