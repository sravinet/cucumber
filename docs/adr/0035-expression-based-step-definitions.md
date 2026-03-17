# ADR-0035: Expression-based Step Definitions and Test Infrastructure Improvements

## Status

Accepted

## Context

The codebase had several interconnected issues that needed addressing:

1. **Duplicate Step Definitions**: Both regex and expression-based patterns across test files
2. **Failing Doctests**: Documentation examples using `run_and_exit()` causing process termination
3. **Codegen Test Issues**: Similar process termination problems in test context
4. **Code Quality**: Various clippy warnings and unused imports

Example of the problematic patterns:
```rust
// Regex vs Expression duplication
#[given(regex = r"(\d+) secs?")]
#[given(expr = "{int} sec")]

// Doctest process termination
.run_and_exit("tests/features/readme") // Terminates process in test context

// Missing step definitions
MyWorld::cucumber()
    .run("tests/features/readme") // But no step functions provided
```

## Decision

**1. Eliminate regex-based step definitions** in favor of expression-based across all test files
**2. Fix doctest examples** to use `.run()` with proper step definitions
**3. Fix codegen tests** to avoid process termination in test context
**4. Improve code quality** with clippy fixes and cleanup

## Implementation

### Expression-Based Step Definitions

**Before:**
```rust
#[given(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    // Manual parameter extraction required
}
```

**After:**
```rust
#[given(expr = "{int} sec")]
#[given(expr = "{int} secs")]
fn step(world: &mut World, secs: usize) {
    // Automatic parameter parsing
}
```

**Files Changed:**
- `tests/wait.rs`: Removed CustomU64 parameter type, unified to expression-based
- `tests/libtest.rs`: Converted regex patterns to expression-based
- `tests/junit.rs`: Converted regex patterns to expression-based  
- `tests/json.rs`: Converted regex patterns to expression-based

### Doctest Infrastructure Fixes

**Before:**
```rust
/// ```rust
/// MyWorld::cucumber()
///     .run_and_exit("tests/features/readme") // Process termination!
///     .await;
/// ```
```

**After:**
```rust
/// ```rust
/// # #[given("Alice is hungry")]
/// # fn alice_is_hungry(_world: &mut MyWorld) {}
/// # #[when("she eats 3 cucumbers")]  
/// # fn she_eats_cucumbers(_world: &mut MyWorld) {}
/// # #[then("she is full")]
/// # fn she_is_full(_world: &mut MyWorld) {}
/// 
/// let writer = MyWorld::cucumber()
///     .run("tests/features/readme")
///     .await;
/// # assert!(!writer.execution_has_failed());
/// ```
```

**Files Fixed:**
- `src/writer/types.rs`: Added step definitions and result validation
- `src/writer/summarize/core.rs`: Added step definitions and result validation

### Codegen Test Fixes

**Before:**
```rust
let res = MyWorld::cucumber()
    .run_and_exit("./tests/features"); // Process termination!

let err = AssertUnwindSafe(res).catch_unwind().await.expect_err("should err");
```

**After:**
```rust
let writer = MyWorld::cucumber()
    .run("./tests/features")
    .await;

assert!(writer.execution_has_failed(), "Execution should have failed");
assert_eq!(writer.failed_steps(), 1, "Expected 1 failed step");
```

**Files Fixed:**
- `codegen/tests/example.rs`: Proper result validation without process termination

### Code Quality Improvements

- **Clippy fixes**: Added `#[must_use]` attributes, Default implementations
- **Import cleanup**: Removed unused imports and conditional compilation
- **Documentation**: Improved inline documentation and examples

## Consequences

### Positive
- **Unified Testing Approach**: Single expression-based pattern across all tests
- **Working Documentation**: All doctests now compile and run successfully  
- **Reliable Test Suite**: No more process termination in test contexts
- **Improved Code Quality**: Better linting compliance and cleaner imports
- **Better Developer Experience**: Examples that actually work out-of-the-box

### Negative
- **Migration Effort**: Required systematic updates across multiple files
- **Increased Boilerplate**: Doctests now require step definition setup

### Neutral
- **Test Behavior**: All existing test functionality preserved
- **Performance**: No significant performance impact

## Notes

This comprehensive improvement addresses architectural debt, infrastructure reliability, and code quality issues. The changes ensure that:

1. **Tests are maintainable**: Single step definition approach
2. **Documentation works**: Runnable examples that don't terminate processes  
3. **CI/CD reliability**: No flaky tests due to process termination
4. **Developer onboarding**: Working examples reduce confusion

The unified approach follows modern Cucumber best practices and eliminates multiple sources of technical debt simultaneously.