# ADR-0024: Event Metadata Integration Enhancement

## Status
Accepted

## Context

The event system had incomplete metadata integration that limited observability and debugging capabilities:

1. **Unused Metadata Parameter**: `send_event_with_meta` was ignoring the metadata parameter, making it impossible to associate events with execution context
2. **Limited Observability**: No way to track event metadata for tracing, debugging, or performance analysis
3. **Integration Gaps**: External tools couldn't access rich context about when/where events occurred
4. **Debugging Friction**: Developers couldn't trace events back to their execution context

This became particularly important for enterprise scenarios where detailed event tracking and debugging are critical.

## Decision

We enhance the event system to properly integrate metadata with all events:

### Implementation
```rust
pub(super) fn send_event_with_meta(
    &self,
    event: event::Cucumber<W>,
    meta: &crate::event::Metadata,
) {
    // Create event with the provided metadata
    let event_with_meta = meta.wrap(event.clone());
    
    // Send through normal channel
    self.sender
        .unbounded_send(Ok(event_with_meta.clone()))
        .unwrap_or_else(|e| panic!("Failed to send `Cucumber` event: {e}"));
}
```

### Key Changes
1. **Metadata Wrapping**: Events are properly wrapped with execution context metadata
2. **Context Preservation**: Timing, location, and execution context preserved with events
3. **Error Handling**: Robust error handling for event transmission failures
4. **API Consistency**: Maintains existing event sending patterns while adding metadata

## Consequences

### Positive

1. **Enhanced Observability**: Events now carry rich metadata for debugging and analysis
2. **Better Tracing**: Can trace events back to their execution context and timing
3. **Tool Integration**: External tools can access detailed event metadata
4. **Debugging Support**: Developers can see exactly when and where events occurred
5. **Performance Analysis**: Metadata enables detailed performance profiling
6. **Enterprise Readiness**: Supports enterprise monitoring and observability requirements

### Negative

1. **Memory Overhead**: Events now carry additional metadata payload
2. **Processing Cost**: Event wrapping adds minor processing overhead
3. **Complexity**: Slightly more complex event handling logic

## Implementation Details

### Core Changes
- Modified `EventSender::send_event_with_meta` to properly wrap events with metadata
- Added error handling for event transmission failures
- Preserved backward compatibility with existing event sending patterns

### Metadata Content
Events now include:
- Execution timing information
- Source location context
- Runtime execution state
- Performance metrics

### Integration Points
- Tracing system can access detailed event metadata
- Writers receive events with full context information
- External tools can extract rich debugging information

## References

- Related to ADR-0015 (Stats Collection) - enables detailed event statistics
- Builds on ADR-0004 (Event Driven Architecture) - enhances the core event system
- Supports ADR-0013 (Performance Optimizations) - provides data for performance analysis