# ADR-0001: Consolidate Observability Features

## Status
Accepted

## Context
The crate currently uses separate feature flags for observability concerns:
- `timestamps` - for timing data collection
- `tracing` - for detailed execution tracing
- `observability` - for test observers

This granular approach creates:
1. Complex feature matrix combinations (2^3 = 8 possible states)
2. Increased maintenance burden with conditional compilation
3. Operational complexity for users choosing observability levels
4. Documentation fragmentation across multiple features

## Decision
Consolidate all observability functionality under a single `observability` feature flag with runtime configuration.

### New Architecture
```rust
#[cfg(feature = "observability")]
pub struct ObservabilityConfig {
    pub timestamps: bool,
    pub tracing: bool,
    pub observers: bool,
}
```

### Benefits
1. **Operational Simplicity**: Single feature flag for build systems
2. **Runtime Flexibility**: Configure observability levels without rebuilding
3. **Industry Alignment**: Follows patterns used by Kubernetes, AWS SDKs, OpenTelemetry
4. **Future-Proof**: Easy to add new observability signals without feature proliferation
5. **Testing Simplicity**: Reduces feature matrix from 8 to 2 states

### Implementation Strategy
1. Maintain backward compatibility during transition
2. Deprecate individual features in favor of unified approach
3. Provide runtime configuration with sensible defaults
4. Ensure zero-cost abstraction when feature is disabled

## Consequences
- **Positive**: Simplified API, better operational experience, aligned with industry standards
- **Negative**: Slight increase in binary size when observability is enabled (but unused components can be optimized away at runtime)
- **Migration**: Existing users will need to update feature flags in Cargo.toml

## Alternatives Considered
1. **Status Quo**: Keep granular features (rejected due to complexity)
2. **Always-On Observability**: Runtime-only configuration (rejected due to dependency bloat)
3. **Build-Time Only**: Multiple features with no runtime config (rejected due to inflexibility)

## References
- OpenTelemetry Architecture: Single SDK with per-signal configuration
- Kubernetes Observability: Unified telemetry with runtime controls
- AWS SDK Patterns: Single telemetry feature with granular runtime options