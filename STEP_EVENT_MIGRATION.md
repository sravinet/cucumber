# Step Event Output Format Migration Guide

## Overview

Starting with version 0.22.0, cucumber-rs has standardized on **struct variant format** for all step event outputs. This change affects debug output parsing and external tools that consume step event output.

## What Changed

### Before (v0.21.x and earlier)
```rust
// Old tuple variant format
Passed(CaptureLocations(...), Some(Location(...)))
Failed(Some(CaptureLocations(...)), Some(Location(...)), None, Panic(...))
```

### After (v0.22.0+)
```rust
// New struct variant format (CANONICAL)
Passed { captures: CaptureLocations(...), location: Some(Location(...)) }
Failed { captures: Some(...), location: Some(...), world: None, error: Panic(...) }
```

## Who Is Affected

- External tools parsing debug output from cucumber-rs
- Test runners that consume step event streams  
- Monitoring/observability systems reading step results
- Custom writers that process step events

## Migration Steps

### 1. Update Debug Output Parsing

If you parse debug output, update your parsing logic to handle named fields:

```rust
// Before: Look for tuple patterns
if line.contains("Passed(") {
    // Extract tuple values...
}

// After: Look for struct patterns  
if line.contains("Passed {") {
    // Extract named field values...
    // captures: ...
    // location: ...
}
```

### 2. Update External Tool Integration

For tools consuming step events programmatically:

```rust
// Before: Destructure tuples
match step_result {
    StepResult::Passed(captures, location) => {
        // Handle passed step
    }
}

// After: Destructure structs
match step_result {
    StepResult::Passed { captures, location } => {
        // Handle passed step  
    }
}
```

### 3. Update Test Output Expectations

If you have tests that verify output format:

- Replace tuple variant patterns with struct variant patterns
- Update expected output files to use canonical format
- Look for named fields: `captures:`, `location:`, `world:`, `error:`

## Benefits of the New Format

1. **Better Readability**: Named fields are easier to understand than positional tuples
2. **Self-Documenting**: Field names provide context about what each value represents
3. **Consistency**: All output modes (debug, basic, colored) now use the same canonical format
4. **Debugging**: Easier to identify specific fields when troubleshooting

## Canonical Format Specification

### Step Result Variants

```rust
// Successful step execution
Passed { 
    captures: CaptureLocations(...), 
    location: Some(Location(...)) 
}

// Failed step execution  
Failed { 
    captures: Some(CaptureLocations(...)), 
    location: Some(Location(...)), 
    world: None, 
    error: Panic(Any { .. }) 
}

// Skipped step (no additional fields)
Skipped
```

### Data Table Preservation

Steps with data tables preserve table information:

```rust
Step { 
    keyword: "Given ", 
    ty: Given, 
    value: "foo is 0", 
    docstring: None, 
    table: Some(Table { 
        rows: [["key", "value"], ["1", "0"], ["2", "1"]], 
        position: LineCol { line: 5 } 
    }), 
    position: LineCol { line: 4 } 
}
```

## Compatibility

- **Breaking Change**: This is a breaking change for external tools
- **Version**: Introduced in v0.22.0
- **Rollback**: Not recommended - the new format is canonical and production-ready

## Need Help?

- Review [ADR-0023](docs/adr/0023-step-event-output-standardization.md) for technical details
- Check the [changelog](CHANGELOG.md) for complete list of changes
- File an issue if you need assistance migrating your tools

## Related Documentation

- [ADR-0023: Step Event Output Format Standardization](docs/adr/0023-step-event-output-standardization.md)
- [ADR-0024: Event Metadata Integration Enhancement](docs/adr/0024-event-metadata-integration.md)
- [ADR-0003: Step Enum Using Struct Variants](docs/adr/0003-step-enum-struct-format.md)