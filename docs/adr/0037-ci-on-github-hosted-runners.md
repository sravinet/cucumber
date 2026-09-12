# ADR-0037: CI on GitHub-Hosted Runners

## Status
Implemented

## Context

The repository inherited its workflows from upstream `cucumber-rs/cucumber`,
but the code base has diverged substantially since the fork. Several of the
inherited gates could not pass on the current tree, which left the project
without a usable signal:

1. **Lint gate unachievable**: `make cargo.lint` denied every warning
   (`-D warnings`). Combined with the crate-level `#![warn(clippy::pedantic)]`
   attributes, that produced 527 errors, while `justfile` already documented a
   narrower policy the project actually follows.
2. **Formatting gate unachievable**: `.rustfmt.toml` enabled
   `error_on_line_overflow`/`error_on_unformatted`, and 422 comment and string
   lines exceed `max_width = 80`. Those lines cannot be re-wrapped by rustfmt,
   so `cargo fmt` could never make `cargo fmt --check` pass.
3. **Documentation gate broken**: `#![cfg_attr(docsrs, feature(doc_auto_cfg))]`
   fails on nightly ≥ 1.92, where `doc_auto_cfg` was merged into `doc_cfg`.
4. **MSRV gate broken**: the declared MSRV of 1.87 is not reachable — the
   minimum supported `gherkin` 0.15.0 requires 1.88 — and under
   `-Z minimal-versions` the JSON writer tests failed because
   `serde_json` < 1.0.93 cannot serialize 128-bit integers.
5. **Weak aggregate check**: the `pr` gate job used a job-level `if` listing
   every dependency, so a failing dependency made the gate *skipped* rather
   than *failed* — and GitHub treats a skipped required check as satisfied.
6. **Unmaintained action**: the security audit used the archived
   `actions-rs/audit-check@v1`.

## Decision

Run the full check suite on GitHub-hosted runners only (`ubuntu-latest`,
`macos-latest`, `windows-latest`), and make every gate one the tree can
actually satisfy:

### 1. Checks

| Job | Runner(s) | What it guards |
|-----|-----------|----------------|
| `clippy` | ubuntu | Project lint policy, default and all features |
| `rustfmt` | ubuntu | Formatting (`cargo +nightly fmt --check`) |
| `feature` | ubuntu | Each feature and selected combinations, resolved with `-Z minimal-versions`, `-D warnings` |
| `msrv` | ubuntu, macOS, Windows | Both crates build and test on the declared MSRV |
| `test` | ubuntu, macOS, Windows | Both crates on stable, beta and nightly (`cargo careful` on nightly) |
| `test-book` | ubuntu, macOS, Windows | Book examples compile and run |
| `rustdoc` | ubuntu | Documentation builds warning-free with `--cfg docsrs` |

### 2. Lint policy

`make cargo.lint` denies `clippy::correctness`, `clippy::suspicious` and
`clippy::perf`, keeps `clippy::pedantic` advisory, and allows the five
pedantic lints that the project has opted out of — the same policy as the
`lint` recipe in `justfile`. `make cargo.lint strict=yes` keeps the upstream
"deny everything" gate available for the eventual clean-up.

### 3. Formatting policy

`error_on_line_overflow` and `error_on_unformatted` are disabled, because they
report lines rustfmt itself cannot fix. Code layout is still fully enforced:
the gate fails on anything `cargo fmt` would rewrite.

### 4. MSRV

The MSRV is 1.88 (both manifests, the README badge and the CI matrix), and the
`msrv` job asserts the matrix value matches `rust-version` so the three cannot
drift apart. `serde_json`'s declared minimum is 1.0.93, the first version that
serializes 128-bit integers, so `-Z minimal-versions` resolves to a set that
actually works.

### 5. Aggregate gate

`ci-success` runs with `if: always()`, needs every check job, and fails
explicitly when any of them reports `failure`, `cancelled` or `skipped`. It is
the single status check to require in the branch protection rules of `main`.

### 6. Hygiene

`permissions: contents: read` by default (write is granted only to the release
and Book deployment jobs), per-job `timeout-minutes`, `Swatinem/rust-cache` for
dependency and build caching, `workflow_dispatch` for manual runs,
`pull_request` without a base-branch filter so work targeting any branch is
checked, and `rustsec/audit-check@v2` in place of the archived
`actions-rs/audit-check@v1`.

## Consequences

### Positive

- Every gate is satisfiable by the current tree, so red means "regression"
  rather than "inherited backlog".
- One required status check that cannot be bypassed by a skipped job.
- Feature coverage includes `observability` and cross-category combinations
  (see [ADR-0033](0033-feature-flag-test-matrix-strategy.md)).
- No self-hosted infrastructure to maintain; caching keeps the matrix
  affordable.

### Negative

- `clippy::pedantic` findings stay advisory, so they accumulate until the
  `strict=yes` clean-up happens.
- Over-long comments are no longer reported by the formatter.
- Raising the MSRV to 1.88 and `serde_json` to 1.0.93 narrows the supported
  range for downstream users.

## References

- [ADR-0033](0033-feature-flag-test-matrix-strategy.md) — feature matrix strategy
- [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)
- [`.github/workflows/audit.yml`](../../.github/workflows/audit.yml)
- [rust-lang/rust#138907](https://github.com/rust-lang/rust/pull/138907) — `doc_auto_cfg` merged into `doc_cfg`
