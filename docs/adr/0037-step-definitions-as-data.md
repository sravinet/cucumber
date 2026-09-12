# ADR-0037: Step Definitions Are Readable as Data

## Status
Accepted

## Context

Every `#[given]`, `#[when]` and `#[then]` function is registered with
`inventory` at link time as a compiled regex, a `step::Location`
(`file:line:column` of the attribute) and the boxed step function.
`World::collection()` drains that registry into a `step::Collection`, whose
three maps are private and which exposes `find`, `merge`, `compose` and the
`*_len` counts — and no iterator. So the only way to learn which definitions a
suite *has* was to run the suite and read `event::Step::Passed { location }`
off the writer stream, one executed step at a time. A definition no scenario
reaches never appears, and a suite that fails early reports the definitions
it got to.

Two consumers want the list without a run:

- a **step-reuse index** — before an agent generates a step definition, it
  should see the patterns that already exist, so it grounds in the suite's
  vocabulary rather than adding a fourth way to say "the user logs in";
- an **impact graph** — which scenarios reach which definitions, and through
  them which screenplay tasks, is a graph whose step-definition nodes are
  exactly the registry entries.

Both are built in the sibling `screenplay-graph` crate (graphlift) from a JSON
file this crate has to write. The shape is a contract between the two
repositories, which share no cargo dependency.

## Decision

`cucumber::step::definitions::all::<W>()` reads the same `inventory` registry
`World::collection()` reads and returns `Vec<StepDefinition>` — keyword,
pattern as the runner compiled it (a cucumber expression is exported in its
translated regex form, so what is exported is what `Collection::find` matches
with), and location — sorted by location so the export is stable across runs
and link orders. `definitions::json::<W>()` renders the list as one JSON array
of `{"keyword","pattern","path","line","column"}` objects.

The JSON is hand-rendered rather than through `serde`, which is an optional
dependency here: an export should not cost a feature flag. The escaper covers
what a regex carries — backslashes, quotes, control characters — and is
tested on those.

`Collection` itself is unchanged. Adding an iterator there would expose the
`(HashableRegex, Option<Location>)` key shape as API; reading the registry
answers the question without widening the type.

Gated on the `macros` feature, like the registry it reads.

## Consequences

### Positive

- A suite's step definitions can be listed, diffed and indexed without
  executing a scenario — including definitions nothing reaches, which the
  event stream can never show.
- The exported pattern is the runner's own compiled form, so a consumer that
  matches step text against it with the same `regex` crate reproduces the
  runner's binding, ambiguity included.
- No new dependency and no new feature flag.

### Negative

- The registry records the attribute's location, not the function's name or
  body; a consumer that wants to know what a definition *does* still has to
  read the source at that location. That is by design — the fork does not
  parse Rust — but it is the limit of what this export can say.
- `file!()` paths are relative to wherever rustc ran, normally the workspace
  root, and the export does not normalise them. A consumer needs the same
  root to resolve them.

## References

- `src/step/definitions.rs`, `tests/step_definitions_export.rs`
- ADR-0022 (modular step builders) — the other way a suite's definitions are
  organised, by composition rather than by enumeration
- ADR-0035 (expression-based step definitions) — why the exported pattern is a
  regex even when the attribute was an expression
- graphlift `typedb/screenplay-graph`, the consumer this export was written for
