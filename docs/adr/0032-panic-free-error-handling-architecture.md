# ADR-0032: Panic-Free Error Handling Architecture

## Status
Accepted

## Context

The codebase contains 89 instances of `panic!` in production code paths, creating significant reliability and debugging challenges:

1. **Production Crashes**: Panics cause unrecoverable failures instead of graceful degradation
   - Event handling: `panic!("Expected Feature::Rule with Started event")`
   - State management: `panic!("no \`Feature: {}\`", feat.name)`
   - Resource access: `panic!("no \`Rule: {}\`", rule.name)`

2. **Poor Error Experience**: Users receive unhelpful crash dumps instead of actionable error messages
3. **Debugging Complexity**: Stack traces don't provide context about what went wrong
4. **Testing Gaps**: Error recovery paths are not validated
5. **Maintenance Burden**: Panic-driven code is brittle and hard to evolve

The goal is to implement robust error handling that enables graceful degradation, better debugging, and improved user experience.

## Decision

Implement a comprehensive panic-free error handling architecture with typed errors and graceful recovery:

### 1. Hierarchical Error Type System

```rust
// Top-level error classification
#[derive(Debug, thiserror::Error)]
pub enum CucumberError {
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),
    
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),
    
    #[error("Writer error: {0}")]
    Writer(#[from] WriterError),
    
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),
}

// Domain-specific error types
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Scenario state inconsistency: expected {expected}, found {actual}")]
    StateInconsistency { expected: String, actual: String },
    
    #[error("Feature '{feature}' not found in execution context")]
    FeatureNotFound { feature: String },
    
    #[error("Duplicate event for scenario '{scenario}': {event_type}")]
    DuplicateEvent { scenario: String, event_type: String },
}

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("Output format error: {0}")]
    Format(#[from] FormatError),
    
    #[error("IO error during output: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),
}
```

### 2. Graceful Error Recovery Patterns

```rust
// Replace panic-driven state management
impl FeatureExecutor {
    pub fn get_feature_mut(&mut self, name: &str) -> Result<&mut Feature, ExecutionError> {
        self.features.get_mut(name)
            .ok_or_else(|| ExecutionError::FeatureNotFound { 
                feature: name.to_string() 
            })
    }
    
    pub fn handle_scenario_event(&mut self, scenario: &str, event: ScenarioEvent) -> Result<(), ExecutionError> {
        let feature = self.get_feature_mut(&scenario.feature_name)?;
        
        match event {
            ScenarioEvent::Started if feature.scenario_started(scenario) => {
                Err(ExecutionError::DuplicateEvent {
                    scenario: scenario.to_string(),
                    event_type: "Started".to_string(),
                })
            }
            ScenarioEvent::Started => {
                feature.start_scenario(scenario);
                Ok(())
            }
            _ => feature.handle_event(scenario, event),
        }
    }
}
```

### 3. Error Context Propagation

```rust
// Rich error context for debugging
use anyhow::{Context, Result};

impl EventProcessor {
    pub fn process_feature_event(&mut self, event: FeatureEvent) -> Result<()> {
        match event {
            FeatureEvent::Started(feature) => {
                self.start_feature(&feature)
                    .with_context(|| format!("Failed to start feature '{}'", feature.name))
            }
            FeatureEvent::Finished(feature) => {
                self.finish_feature(&feature)
                    .with_context(|| format!("Failed to finish feature '{}'", feature.name))
            }
            FeatureEvent::Rule(rule_event) => {
                self.process_rule_event(rule_event)
                    .with_context(|| "Failed to process rule event")
            }
        }
    }
}
```

### 4. Fallback and Recovery Strategies

```rust
// Graceful degradation for non-critical operations
impl OutputWriter {
    pub fn write_scenario_result(&mut self, result: ScenarioResult) -> Result<(), WriterError> {
        // Attempt primary output format
        if let Err(e) = self.write_primary_format(&result) {
            log::warn!("Primary output failed: {}, attempting fallback", e);
            
            // Attempt fallback format
            if let Err(fallback_error) = self.write_fallback_format(&result) {
                // If both fail, return the original error with context
                return Err(e).with_context(|| format!(
                    "Both primary and fallback output failed. Fallback error: {}", 
                    fallback_error
                ));
            }
            
            log::info!("Successfully wrote result using fallback format");
        }
        Ok(())
    }
}
```

### 5. Validation with Error Types

```rust
// Replace assertion panics with validation
impl EventSequenceValidator {
    pub fn validate_scenario_sequence(&self, events: &[ScenarioEvent]) -> Result<(), ValidationError> {
        let mut state = ScenarioState::Initial;
        
        for event in events {
            state = self.transition_state(state, event)
                .map_err(|invalid_transition| ValidationError::InvalidTransition {
                    from: state,
                    event: event.clone(),
                    expected: invalid_transition.expected_events,
                })?;
        }
        
        if !state.is_terminal() {
            return Err(ValidationError::IncompleteSequence { final_state: state });
        }
        
        Ok(())
    }
}
```

## Consequences

### Positive

1. **Reliability**: No unrecoverable crashes in production
2. **User Experience**: Actionable error messages instead of stack dumps
3. **Debugging**: Rich error context helps identify root causes
4. **Testability**: Error paths can be systematically tested
5. **Maintainability**: Clear error boundaries and responsibilities
6. **Graceful Degradation**: System continues operating when possible

### Implementation Impact

1. **API Changes**: Functions return `Result` types instead of panicking
2. **Error Propagation**: Callers must handle or propagate errors
3. **Test Coverage**: Error scenarios can be comprehensively tested
4. **Documentation**: Error conditions clearly documented in API

### Performance Considerations

1. **Minimal Overhead**: `Result` types have zero-cost abstractions
2. **Error Path Optimization**: Cold error paths don't affect hot paths
3. **Memory Usage**: Error types designed for efficient stack allocation

## Implementation Strategy

### Phase 1: Critical Path Panic Elimination (Sprint 1)
```rust
// Priority order for panic elimination:
1. Event handling panics (highest impact)
2. State management panics  
3. Resource access panics
4. Serialization/format panics
```

### Phase 2: Error Type Hierarchy (Sprint 2)
```rust
// Implement comprehensive error taxonomy:
1. Define domain-specific error types
2. Implement error conversion traits
3. Add error context and debugging info
```

### Phase 3: Graceful Recovery (Sprint 3)
```rust
// Add fallback and recovery mechanisms:
1. Output format fallbacks
2. State recovery strategies
3. Partial failure handling
```

### Phase 4: Testing and Validation (Sprint 4)
```rust
// Comprehensive error scenario testing:
1. Error injection testing
2. Recovery path validation
3. Error message quality assurance
```

## Technical Implementation

### Error Type Design Principles

1. **Composable**: Errors can be combined and nested
2. **Informative**: Include context and suggested actions
3. **Typed**: Specific error types for different failure modes
4. **Recoverable**: Enable programmatic error handling
5. **Debuggable**: Rich information for troubleshooting

### Conversion Strategy

```rust
// Systematic panic-to-Result conversion
// Before:
fn get_feature(&self, name: &str) -> &Feature {
    self.features.get(name)
        .unwrap_or_else(|| panic!("no `Feature: {}`", name))
}

// After:
fn get_feature(&self, name: &str) -> Result<&Feature, ExecutionError> {
    self.features.get(name)
        .ok_or_else(|| ExecutionError::FeatureNotFound { 
            feature: name.to_string() 
        })
}
```

### Testing Strategy

```rust
#[test]
fn test_feature_not_found_error() {
    let executor = FeatureExecutor::new();
    
    let result = executor.get_feature("nonexistent");
    
    assert!(matches!(result, Err(ExecutionError::FeatureNotFound { .. })));
    assert_eq!(
        result.unwrap_err().to_string(),
        "Feature 'nonexistent' not found in execution context"
    );
}

#[test]
fn test_duplicate_event_handling() {
    let mut executor = FeatureExecutor::new();
    executor.add_feature("test_feature");
    
    // Start scenario
    assert!(executor.handle_scenario_event("test_scenario", ScenarioEvent::Started).is_ok());
    
    // Try to start again - should return error, not panic
    let result = executor.handle_scenario_event("test_scenario", ScenarioEvent::Started);
    assert!(matches!(result, Err(ExecutionError::DuplicateEvent { .. })));
}
```

## Quality Metrics

- **Panic Count**: Reduce from 89 to 0 in production code
- **Error Coverage**: 100% of error paths have tests
- **Error Quality**: All errors include actionable context
- **Recovery Success**: Graceful degradation in failure scenarios

## Future Considerations

1. **Error Analytics**: Collect error patterns for improvement
2. **Error Localization**: Multi-language error messages
3. **Error Documentation**: Auto-generated error handling guides
4. **Performance Monitoring**: Track error-related performance impact

## References

- Addresses critical reliability gaps from comprehensive test analysis
- Supports production readiness goals from ADR-0026
- Enables robust testing as outlined in ADR-0030
- Maintains architectural integrity while improving reliability