# Justfile for slint-mobile-components.
#
# Install just locally with: `cargo install just --locked`
# (or `brew install just`, `apt install just`, etc.)
# Run `just` with no arguments to see the recipe list. CI under
# .github/workflows/ci.yml invokes the same recipes you run locally.

set shell := ["bash", "-cu"]

# Show available recipes
default:
    @just --list

# Format all Rust code in place
fmt:
    cargo fmt --all

# Lint with clippy, treating warnings as errors
clippy:
    cargo clippy --all-targets -- -D warnings

# Type-check the crate (compiles .slint files via build.rs)
check:
    cargo check --all-targets

# Run host-side tests
test:
    cargo test

# Full local CI pipeline (mirrors what runs on PRs)
ci: fmt-check clippy check test

# --- private helpers (callable, but hidden from `just --list`) -------------

# CI-only: verify formatting without modifying files
[private]
fmt-check:
    cargo fmt --all -- --check

# CI-only: install Linux apt packages Slint's renderer needs to build.
# (Not strictly required for this library crate since it doesn't enable a
# renderer feature, but kept for parity with consuming apps.)
[private]
install-host-deps:
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends \
        pkg-config \
        libfontconfig1-dev \
        libfreetype6-dev \
        clang \
        cmake \
        ninja-build
