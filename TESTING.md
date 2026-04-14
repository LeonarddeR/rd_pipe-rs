# Testing Documentation

## Overview

This document describes the test coverage for the RD Pipe project and provides guidance for running and extending tests.

## Test Coverage Summary

The project now includes comprehensive unit tests for the following modules:

### 1. Registry Module (`src/registry.rs`)
- **CLSID validation tests**: Verify the GUID is valid and non-zero
- **Constant validation tests**: Ensure registry paths and constants are properly formatted
- **Path format tests**: Validate registry key path generation logic
- **Plugin name consistency tests**: Verify naming consistency across modules

### 2. Security Descriptor Module (`src/security_descriptor.rs`)
- **SDDL conversion tests**: Test conversion of valid and invalid SDDL strings to security descriptors
- **Structure validation tests**: Ensure SECURITY_ATTRIBUTES structures are properly initialized
- **SID format tests**: Validate SID string format when retrieved
- **Memory management tests**: Verify proper cleanup of allocated security descriptors

### 3. RD Pipe Plugin Module (`src/rd_pipe_plugin.rs`)
- **Pipe name tests**: Validate named pipe path format and generation
- **Message constant tests**: Verify XON/XOFF protocol constants
- **Registry path tests**: Ensure proper registry path formatting
- **Plugin construction tests**: Test plugin and callback object creation
- **Listener callback tests**: Validate listener callback functionality

### 4. Library Entry Points (`src/lib.rs`)
- **Command constant tests**: Verify DllInstall command characters are distinct
- **CLSID consistency tests**: Ensure CLSID matches across modules
- **Runtime initialization tests**: Test async runtime lazy initialization
- **Command parsing tests**: Validate command string parsing logic

### 5. Class Factory Module (`src/class_factory.rs`)
- **Construction tests**: Verify ClassFactory can be created and converted to IClassFactory
- **LockServer tests**: Ensure LockServer always succeeds regardless of lock state
- **Interface IID tests**: Validate supported interface GUIDs (IUnknown, IWTSPlugin)
- **Debug implementation tests**: Verify Debug trait is properly implemented

## Running Tests

### Prerequisites
- Rust toolchain (stable)
- Windows environment (tests are Windows-specific)
- Visual Studio Build Tools or MSVC compiler

### Run All Tests
```bash
cargo test
```

### Run Tests for Specific Module
```bash
cargo test --lib registry
cargo test --lib security_descriptor
cargo test --lib rd_pipe_plugin
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Tests with Specific Target
```bash
cargo test --target x86_64-pc-windows-msvc
```

## Test Limitations and Considerations

### Windows-Specific Tests
All tests are designed for Windows environments only. They will not compile or run on Linux or macOS.

### Security Context Requirements
Some tests in `security_descriptor.rs` may behave differently depending on the security context:
- `test_get_logon_sid_returns_string` may fail in restricted environments or CI without proper logon sessions
- Tests are designed to handle such failures gracefully

### Registry Access
Tests that validate registry operations do NOT actually modify the registry. They test:
- Path formatting and generation
- Constant values
- Logic flows

### COM Interface Testing
Full COM interface testing requires:
- Proper COM runtime initialization
- Windows Remote Desktop Services environment
- Active RDS session

These are not easily testable in unit tests and require integration testing in a live environment.

## Areas Needing Additional Coverage

### High Priority
1. **Integration Tests**: End-to-end tests with actual RDS connections
2. **Named Pipe Tests**: Tests for pipe server creation and client communication
3. **Channel Callback Tests**: Tests for virtual channel data flow

### Medium Priority
1. **Error Handling**: More comprehensive error path testing
2. **Concurrent Access**: Multi-threaded access patterns
3. **Memory Leak Tests**: Verify proper cleanup in all code paths

### Low Priority (Requires Live Environment)
1. **DllMain Tests**: Full DLL lifecycle testing
2. **DllGetClassObject Tests**: COM object creation and querying
3. **DllInstall Tests**: Registry modification testing
4. **IWTSPlugin Interface Tests**: Full plugin lifecycle with RDS

## Adding New Tests

### Test Structure
Follow this pattern for new tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange: Set up test data
        let test_value = "test";

        // Act: Execute the code being tested
        let result = function_under_test(test_value);

        // Assert: Verify the results
        assert_eq!(result, expected_value);
    }
}
```

### Test Guidelines
1. **Be descriptive**: Test names should clearly indicate what is being tested
2. **Test one thing**: Each test should focus on a single behavior
3. **Clean up resources**: Always free allocated memory (LocalFree, etc.)
4. **Handle platform differences**: Use conditional compilation where needed
5. **Document limitations**: Add comments explaining test constraints

## CI/CD Integration

The CI pipeline now includes a dedicated test job that:
1. Runs on Windows Server 2025
2. Uses x86_64-pc-windows-msvc target
3. Executes all unit tests
4. Fails the build if any tests fail

Tests run automatically on:
- Pull requests to master branch
- Pushes to master branch
- Manual workflow triggers

## Test Metrics

Current test coverage by module:
- `registry.rs`: 6 unit tests (constants, GUID, path formatting)
- `security_descriptor.rs`: 6 unit tests (SDDL, SID, memory management)
- `rd_pipe_plugin.rs`: 8 unit tests (constants, construction, naming)
- `lib.rs`: 6 unit tests (constants, initialization, parsing)
- `class_factory.rs`: 6 unit tests (construction, interfaces, Debug impl)

**Total**: 32 unit tests covering core functionality

## Future Improvements

1. **Code Coverage Reporting**: Integrate tools like tarpaulin or grcov
2. **Benchmark Tests**: Performance testing for critical paths
3. **Mock Objects**: Create mock COM objects for isolated testing
4. **Property-Based Tests**: Use proptest for fuzzing inputs
5. **Integration Test Suite**: Separate integration tests requiring full RDS environment

## Contributing

When contributing new features:
1. Add unit tests for all new functions
2. Update this document with new test descriptions
3. Ensure tests pass in CI before submitting PR
4. Consider edge cases and error conditions
5. Document any test limitations or requirements
