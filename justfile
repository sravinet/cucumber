# Rust Cucumber Testing Framework - Development Commands

# Validate the entire monorepo (skip format-check due to strict max_width=80 config)
validate: lint test

# Format check with rustfmt (may fail due to strict 80-char limit)
format-check:
    cargo +nightly fmt --all -- --check

# Format code with rustfmt (best effort, may have warnings)
format:
    cargo +nightly fmt --all

# Validate with formatting (attempts format first, then lint and test)
validate-with-format: format lint test

# Lint with Clippy - focus on critical errors only
lint:
    cargo clippy --workspace -- -D clippy::correctness -D clippy::suspicious -D clippy::perf -W clippy::pedantic -A clippy::module-name-repetitions -A clippy::missing-errors-doc -A clippy::missing-panics-doc -A clippy::must-use-candidate -A clippy::missing-const-for-fn
    cargo clippy --workspace --all-features -- -D clippy::correctness -D clippy::suspicious -D clippy::perf -W clippy::pedantic -A clippy::module-name-repetitions -A clippy::missing-errors-doc -A clippy::missing-panics-doc -A clippy::must-use-candidate -A clippy::missing-const-for-fn

# Lint with all warnings (stricter, for CI)
lint-strict:
    cargo clippy --workspace -- -D warnings
    cargo clippy --workspace --all-features -- -D warnings

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with careful (requires nightly)
test-careful:
    #!/usr/bin/env bash
    if ! cargo install --list | grep -q cargo-careful; then
        cargo install cargo-careful
    fi
    if ! rustup component list --toolchain=nightly | grep -q 'rust-src (installed)'; then
        rustup component add --toolchain=nightly rust-src
    fi
    cargo +nightly careful test --workspace --all-features

# Generate documentation
docs:
    cargo doc --workspace --all-features --document-private-items

# Generate documentation with docs.rs config
docs-rs:
    RUSTDOCFLAGS='--cfg docsrs' cargo +nightly doc --workspace --all-features --document-private-items

# Clean build artifacts
clean:
    cargo clean

# Build the book
book:
    mdbook build book/

# Serve the book locally
book-serve port="3000":
    mdbook serve book/ -p={{port}}

# Test book examples
book-test:
    #!/usr/bin/env bash
    target=$(cargo -vV | sed -n 's/host: //p')
    cargo build --all-features --tests
    OUT_DIR="$(realpath .)/target" mdbook test book -L target/debug/deps

# Run specific test by name
test-name name:
    cargo test --workspace --all-features {{name}}

# Run tests for specific crate
test-crate crate:
    cargo test -p {{crate}} --all-features

# Check for security vulnerabilities
audit:
    cargo audit

# Update dependencies
update:
    cargo update

# Show workspace information
info:
    cargo metadata --no-deps --format-version 1 | jq '.workspace_members'

# Quick development cycle (format, lint, test)
dev: format lint test

# CI-like validation (what should pass in CI)
ci: format-check lint test book-test