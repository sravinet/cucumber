# ADR-0022: Modular Step Builder Architecture for Enterprise-Scale BDD

## Status
Accepted

## Context

As BDD test suites grow in enterprise environments, managing hundreds of step definitions becomes challenging. Different teams need to own domain-specific step definitions while maintaining clean separation and avoiding conflicts. The existing monolithic approach to step registration creates several problems:

1. **Team Ownership**: No clear way for different teams (auth, crypto, infrastructure, compliance) to own their step definitions independently
2. **Scalability**: Managing 200+ steps in a single collection leads to merge conflicts and maintenance overhead
3. **Reusability**: Step definitions are tightly coupled to specific test configurations rather than being modular and reusable
4. **Testability**: Individual step groups cannot be unit tested in isolation
5. **Maintainability**: Large step collections become difficult to navigate and maintain

## Decision

We introduce a modular step builder architecture with three key components:

### 1. StepBuilder Trait

```rust
pub trait StepBuilder<World> {
    fn register_steps(collection: Collection<World>) -> Collection<World>;
    fn domain_name() -> &'static str;
}
```

This trait allows domain-specific step builders to be owned by different teams while maintaining consistent patterns.

### 2. Collection Composition Methods

```rust
impl<World> Collection<World> {
    pub fn merge(mut self, other: Self) -> Self;
    pub fn compose(collections: Vec<Self>) -> Self;
    
    // Inspection methods for testing
    pub fn given_len(&self) -> usize;
    pub fn when_len(&self) -> usize; 
    pub fn then_len(&self) -> usize;
    pub fn total_len(&self) -> usize;
}
```

These methods enable combining step collections from multiple domains into unified test suites.

### 3. Functional Composition Support

```rust
pub fn compose_step_builders<World>(
    builders: Vec<Box<dyn Fn(Collection<World>) -> Collection<World>>>
) -> Collection<World>;
```

This provides functional composition patterns for advanced use cases.

### 4. Step Builder Macro

```rust
step_builder!(
    CryptoSteps,
    "Cryptographic Operations", 
    TestWorld,
    |collection| {
        collection
            .when(None, Regex::new(r"creating a key").unwrap(), test_step)
            .then(None, Regex::new(r"key should be created").unwrap(), test_step)
    }
);
```

A convenience macro for consistent step builder implementation patterns.

## Consequences

### Positive

1. **Team Ownership**: Each domain team can own their step definitions in separate modules/crates
2. **Scalability**: Supports 200+ steps without conflicts through modular composition 
3. **Reusability**: Step builders can be mixed and matched across different test suites
4. **Testability**: Individual step groups can be unit tested independently
5. **Maintainability**: Clean separation prevents merge conflicts and improves navigation
6. **Enterprise Readiness**: Supports enterprise-scale BDD architectures with clear domain boundaries
7. **Backward Compatibility**: Existing step registration patterns continue to work unchanged
8. **Documentation**: Built-in domain naming for better documentation and debugging

### Negative

1. **Initial Complexity**: Introduces additional abstraction layer for simple use cases
2. **Learning Curve**: Teams need to understand the modular patterns
3. **Potential Over-Engineering**: May be overkill for smaller projects with few step definitions

## Implementation Details

The implementation includes:

- New `src/step/builder.rs` module with the core traits and functions
- Enhanced `src/step/collection.rs` with composition methods and length inspection
- Complete enterprise example in `examples/modular_enterprise_bdd.rs`
- Comprehensive unit tests demonstrating all patterns
- Re-exports in `src/step/mod.rs` for easy access

## References

- Related to ADR-0001 (Modular Architecture) - extends modularity to step definitions
- Influenced by enterprise BDD patterns from cucumber-js and other mature BDD frameworks
- Supports the 300 LOC modularity principle established in ADR-0001