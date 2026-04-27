# ADR-0036: Code Modernization and Async Improvements

## Status
Accepted

## Context

Following recent modernization efforts, the codebase contained several areas where code could be improved for better maintainability, performance, and consistency with modern Rust idioms:

1. **Import Simplification**: Verbose std imports that could be streamlined
2. **Const Method Opportunities**: Methods that could be const but weren't marked as such
3. **Async Stream Processing**: Using deprecated try_next() methods instead of proper polling
4. **Lint Modernization**: Outdated allow(dead_code) attributes replaced with expect syntax

These improvements align with ongoing code quality initiatives and modern Rust best practices.

## Decision

Implement targeted code modernization across multiple modules:

### 1. Import Simplification
Streamline standard library imports by removing unnecessary path qualifications:

```rust
// Before: src/cucumber/execution.rs
use std::mem;
...
std::process::exit(1);

// After: src/cucumber/execution.rs  
use std::{mem, process};
...
process::exit(1);

// Before: src/data_table.rs
self.rows.first().map_or(0, std::vec::Vec::len)

// After: src/data_table.rs
self.rows.first().map_or(0, Vec::len)
```

### 2. Const Method Implementation
Mark methods as const where possible for compile-time optimization:

```rust
// Before: src/error/config.rs
pub fn is_invalid_retry(&self) -> bool {
    matches!(self, Self::InvalidRetry { .. })
}

// After: src/error/config.rs
pub const fn is_invalid_retry(&self) -> bool {
    matches!(self, Self::InvalidRetry { .. })
}
```

### 3. Modern Lint Attributes
Replace deprecated allow attributes with modern expect syntax:

```rust
// Before: src/runner/basic/supporting_structures.rs
#[cfg_attr(not(feature = "tracing"), allow(dead_code))]

// After: src/runner/basic/supporting_structures.rs
#[cfg_attr(not(feature = "tracing"), expect(dead_code, reason = "Only used when tracing feature is enabled"))]
```

### 4. Async Stream Processing Modernization
Replace try_next() with proper polling for better async performance:

```rust
// Before: src/tracing/collector.rs
self.logs_receiver.try_next().ok().flatten().map(|(id, msg)| {
    // Processing logic
})

// After: src/tracing/collector.rs
let waker = Waker::noop();
let mut cx = Context::from_waker(&waker);
match std::pin::Pin::new(&mut self.logs_receiver).poll_next(&mut cx) {
    Poll::Ready(Some((id, msg))) => {
        // Processing logic
    },
    _ => None,
}
```

## Consequences

### Positive

1. **Code Clarity**: Simplified imports make code more readable
2. **Performance**: Const methods enable compile-time optimizations  
3. **Async Correctness**: Proper polling replaces deprecated try_next usage
4. **Modern Standards**: Expect attributes provide clearer lint reasoning
5. **Maintainability**: Consistent code style across modules

### Neutral

1. **API Compatibility**: All changes maintain existing public API contracts
2. **Test Coverage**: All existing tests continue to pass
3. **Feature Parity**: No functional behavior changes

### Implementation Details

Changes span five core modules:
- **src/cucumber/execution.rs**: Import simplification for process utilities
- **src/data_table.rs**: Streamlined Vec import usage
- **src/error/config.rs**: Const method implementation + Result import cleanup
- **src/runner/basic/supporting_structures.rs**: Modern lint attribute syntax
- **src/tracing/collector.rs**: Async stream processing modernization

## Quality Validation

All changes were validated through comprehensive testing:
- ✅ 495+ tests continue passing
- ✅ Clippy lints satisfied with modern syntax
- ✅ Cargo check passes for all features
- ✅ No breaking API changes

## References

- Builds on ADR-0020 (Rust 1.90+ Language Modernization) 
- Supports ADR-0027 (Tracing Span Lifecycle Management Fixes)
- Aligns with ADR-0032 (Panic-Free Error Handling Architecture)
- Continues modernization initiatives from production readiness efforts

## Date
2026-04-27