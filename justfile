# Justfile for slint-mobile-components.
#
# Install just locally with: `cargo install just --locked`
# (or `brew install just`, `apt install just`, etc.)
# Run `just` with no arguments to see the recipe list. CI under
# .github/workflows/ci.yml invokes the same recipes you run locally.

set shell := ["bash", "-cu"]

# Default Android SDK + JDK locations on this machine. Override on the
# command line (`just ANDROID_HOME=… demo-run`) or in your environment
# if your install lives elsewhere.
export ANDROID_HOME := env_var_or_default("ANDROID_HOME", "/home/user/android-build/sdk")
export ANDROID_NDK_ROOT := env_var_or_default("ANDROID_NDK_ROOT", "/home/user/android-build/sdk/ndk/27.0.12077973")
export JAVA_HOME := env_var_or_default("JAVA_HOME", "/home/user/android-build/jdk-17.0.13+11")
export PATH := JAVA_HOME + "/bin:" + ANDROID_HOME + "/platform-tools:" + env_var("PATH")

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

# Full local CI pipeline (mirrors what runs on PRs).
# No standalone `check`: `clippy` already runs the same type-checking
# pass plus lints, so a separate `cargo check` is redundant work
# (~30s warm / 3 min cold saved). `check` is still available as its
# own recipe when you want the lighter version locally.
# `RUSTC_WORKSPACE_WRAPPER=clippy-driver` makes both `cargo test` and
# `cargo clippy` compile every workspace crate through clippy-driver,
# which means the second invocation reuses the artefacts the first
# one built (the workspace-wrapper hash is part of cargo's
# fingerprint, so without aligning them each pass rebuilds the whole
# tree). Net: one compile pass instead of two. The explicit
# `cargo clippy ... -- -D warnings` after is what actually fails the
# build on a lint — `cargo test` emits the warnings during the
# shared compile but doesn't deny them on its own.
ci: fmt-check
    RUSTC_WORKSPACE_WRAPPER=clippy-driver cargo test --all-targets
    RUSTC_WORKSPACE_WRAPPER=clippy-driver cargo clippy --all-targets -- -D warnings

# --- private helpers (callable, but hidden from `just --list`) -------------

# CI-only: verify formatting without modifying files
[private]
fmt-check:
    cargo fmt --all -- --check

# CI-only: install Linux apt packages Slint's renderer needs to build.
# (Not strictly required for this library crate since it doesn't enable a
# renderer feature, but kept for parity with consuming apps.)

# --- wasm-viewer (browser catalogue) --------------------------------------

# Build + serve the wasm-viewer via trunk on http://127.0.0.1:8081.
# Trunk does the wasm-pack build, copies index.html, and watches the
# source tree for changes (hot-reload). Re-run is automatic — leave
# this running while editing .slint or .rs files.
serve:
    @echo "Catalogue: http://127.0.0.1:8081/"
    cd crates/wasm-viewer/web && trunk serve --release

# One-shot wasm-viewer build into dist/. Mirrors what
# .github/workflows/pages.yml does in CI so the local build matches
# the deployed artefact.
build-wasm:
    cd crates/wasm-viewer/web && trunk build --release --dist ../../../dist

# --- Android demo APK ------------------------------------------------------

# Build the demo APK (debug, multi-arch).
demo-build:
    cd android-demo && cargo apk build

# Build a release APK.
demo-release:
    cd android-demo && cargo apk build --release

# Build, install, and launch the demo on the connected emulator/device.
demo-run:
    cd android-demo && cargo apk run

# --- private helpers (callable, but hidden from `just --list`) -------------

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
