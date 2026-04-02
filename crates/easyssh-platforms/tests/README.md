# Native Platform Testing Infrastructure

This directory contains test suites for all EasySSH native platforms.

## Platform Test Structure

```
platforms/
├── windows/easyssh-winui/tests/
│   ├── ui_tests.rs           # egui UI component tests
│   ├── test_utils.rs         # Test helpers and mocks
│   └── README.md             # Windows test documentation
│
├── linux/easyssh-gtk4/tests/
│   ├── integration_tests.rs  # GTK4 integration tests
│   ├── test_utils.rs         # GTK test utilities
│   └── README.md             # Linux test documentation
│
└── macos/EasySSH/Tests/EasySSHTests/
    ├── EasySSHTests.swift        # SwiftUI XCTest suite
    ├── EasySSHCoreBridgeTests.m  # Objective-C bridge tests
    ├── TestUtils.swift           # Swift test utilities
    └── README.md                 # macOS test documentation
```

## Running Tests

### Windows (egui)

```bash
cd platforms/windows/easyssh-winui

# Run all tests
cargo test

# Run with UI test features
cargo test --all-features

# Run specific test
cargo test test_server_view_model
```

### Linux (GTK4)

```bash
cd platforms/linux/easyssh-gtk4

# Run unit tests (headless)
cargo test --lib

# Run integration tests (requires display)
cargo test --test integration_tests

# Run all tests with features
cargo test --all-features
```

### macOS (SwiftUI)

```bash
cd platforms/macos/EasySSH

# Build core first
cd ../../core && cargo build --release

# Run Swift tests
cd platforms/macos/EasySSH
swift test

# Run with coverage
swift test --enable-code-coverage
```

## CI/CD Integration

GitHub Actions workflows automatically run these tests:

- `.github/workflows/native-ci.yml` - Native platform builds and tests
- `.github/workflows/cross-platform-tests.yml` - Cross-platform test matrix

## Test Categories

### Unit Tests
- Model validation
- Form input validation
- Data serialization
- Helper functions

### UI Tests
- Component rendering
- User interaction simulation
- State management

### Integration Tests
- Database operations
- SSH connection handling
- File operations (SFTP)
- Core bridge functionality

## Writing New Tests

### Rust Tests (Windows/Linux)

```rust
#[test]
fn test_new_feature() {
    // Arrange
    let input = create_test_data();

    // Act
    let result = function_under_test(input);

    // Assert
    assert_eq!(result.expected_value, actual_value);
}
```

### Swift Tests (macOS)

```swift
func testNewFeature() {
    // Arrange
    let input = TestDataBuilder.makeServer()

    // Act
    let result = functionUnderTest(input)

    // Assert
    XCTAssertEqual(result, expectedValue)
}
```

## Test Utilities

Each platform provides test utilities:

- **Windows**: `test_utils.rs` - Mock view models, WebSocket helpers
- **Linux**: `test_utils.rs` - GTK initialization, mock models
- **macOS**: `TestUtils.swift` - Test data builders, async helpers

## Coverage

Test coverage reports are generated automatically in CI. View them in:
- GitHub Actions artifacts
- codecov.io (when configured)
