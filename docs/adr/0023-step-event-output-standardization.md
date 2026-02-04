# ADR-0023: Step Event Output Format Standardization

## Status
Accepted

## Context

The test output system was using inconsistent enum variant formats between the actual implementation and expected test outputs. This created maintenance overhead and confusion during development:

1. **Format Inconsistency**: Implementation was producing canonical struct variants (`Passed { captures, location }`) while test expectations used old tuple variants (`Passed(captures, location)`)
2. **Test Maintenance**: Failing tests made it unclear whether implementation or expectations were correct
3. **Development Friction**: Developers couldn't easily verify if step event output changes were intentional
4. **Documentation Gap**: No clear specification of the canonical output format

The step events are critical for debugging, tracing, and integration with external tools, so having a consistent and well-documented format is essential.

## Decision

We standardize on **struct variant format** for all step event outputs across debug, basic, and colored output modes:

### Canonical Format
```rust
// Step Results - Struct Variants (CANONICAL)
Passed { captures: CaptureLocations(...), location: Some(Location(...)) }
Failed { captures: Some(...), location: Some(...), world: None, error: Panic(...) }
Skipped

// NOT tuple variants (DEPRECATED)
Passed(CaptureLocations(...), Some(Location(...)))
Failed(Some(...), Some(...), None, Panic(...))
```

### Implementation Rules
1. **All step events use struct variant debug format** - provides named field access and better readability
2. **Data tables preserved in step values** - steps with tables show `table: Some(Table { rows: [...] })`
3. **Consistent error reporting** - failed steps include full context with named fields
4. **Skipped step handling** - clear indication when steps are skipped due to previous failures

### Output Modes
- **Debug Output**: Full struct representation with all fields
- **Basic Output**: Human-readable format with table data displayed
- **Colored Output**: ANSI-colored version of basic format

## Consequences

### Positive

1. **Consistency**: All output modes now use the same canonical struct variant format
2. **Debuggability**: Named fields make step event debugging much clearer
3. **Maintainability**: Expected test outputs match implementation exactly
4. **Documentation**: Clear specification of canonical output format
5. **Tool Integration**: External tools can rely on consistent step event format
6. **Developer Experience**: No more confusion about which format is correct

### Negative

1. **Breaking Change**: External tools parsing old tuple variant format need updates
2. **Test Updates**: All expected output files needed regeneration
3. **Learning Curve**: Developers need to understand new canonical format

## Implementation Details

Updated the following components:

### Test Output Files
- `tests/features/output/*.debug.out` - Updated to struct variant format
- `tests/features/output/*.basic.out` - Updated to show proper table handling
- `tests/features/output/*.colored.out` - Updated with ANSI escape sequences

### Key Changes
- Step events now consistently use `Passed { captures, location }` format
- Data tables properly preserved in step values with `table: Some(Table { ... })`
- Error events include full context with named fields
- Skipped steps clearly indicated in output

### Migration Path
1. External tools should parse struct variants instead of tuple variants
2. Look for named fields: `captures:`, `location:`, `world:`, `error:`
3. Table data is available in `step.table` field when present

## References

- Related to ADR-0003 (Step Enum Struct Format) - completes the standardization
- Builds on ADR-0004 (Event Driven Architecture) - ensures consistent event output
- Supports debugging and integration requirements from enterprise usage patterns