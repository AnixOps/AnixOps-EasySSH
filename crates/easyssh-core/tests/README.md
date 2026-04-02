# EasySSH Core Test Framework

This directory contains the unit and integration tests for the EasySSH core library.

## Test Structure

```
tests/
├── common/                    # Shared test utilities
│   └── mod.rs                # Test helpers, fixtures, assertions
├── fixtures/                  # Test data files
│   └── test_data.json        # Sample servers, groups, identities
├── unit/                      # Unit tests
│   ├── crypto_tests.rs       # Cryptography tests
│   ├── database_tests.rs     # Database CRUD tests
│   ├── ssh_tests.rs          # SSH configuration tests
│   ├── server_service_tests.rs # Business logic tests
│   └── search_tests.rs       # Search functionality tests
└── integration/               # Integration tests
    └── workflow_tests.rs     # End-to-end workflow tests
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
cargo test --test database_tests
```

### Run tests with output
```bash
cargo test -- --nocapture
```

### Run tests with coverage
```bash
cargo tarpaulin --out html
```

## Test Coverage Goals

| Module | Target Coverage | Current Status |
|--------|-----------------|----------------|
| Crypto | >= 90% | Tests written |
| Database | >= 85% | Tests written |
| SSH | >= 80% | Tests written |
| Services | >= 85% | Tests written |
| Search | >= 80% | Tests written |

## Test Dependencies

The following dev-dependencies are required for testing:

- `tokio-test` - Async testing utilities
- `tempfile` - Temporary files and directories
- `mockall` - Mock objects for testing
- `criterion` - Benchmark tests
- `serde_json` - JSON test data

## Key Test Scenarios

### Crypto Tests
- Encryption/decryption roundtrip
- Key derivation with Argon2id
- Wrong password handling
- Lock/unlock state management
- Secure memory clearing

### Database Tests
- CRUD operations for all entities
- Transaction support
- Foreign key constraints
- Concurrent access safety
- Pagination

### SSH Tests
- Server configuration validation
- Authentication method handling
- Connection health tracking

### Service Tests
- Server CRUD with business rules
- Duplicate detection
- Import/export functionality
- Search and filtering

### Integration Tests
- End-to-end workflows
- Cross-module interactions
- Error handling chains
- Concurrent operations

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
- All feature combinations
- Code coverage reporting
- Benchmark baselines

See `.github/workflows/test.yml` for CI configuration.
