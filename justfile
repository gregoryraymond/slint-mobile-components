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
MAESTRO_BIN := env_var_or_default("MAESTRO_BIN", env_var("HOME") + "/.maestro/bin")
export PATH := JAVA_HOME + "/bin:" + ANDROID_HOME + "/platform-tools:" + MAESTRO_BIN + ":" + env_var("PATH")
export MAESTRO_CLI_ANALYSIS_NOTIFICATION_DISABLED := "true"

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

# --- Maestro E2E flows ------------------------------------------------------

# Run all Maestro flows against the connected device (you must `just demo-run`
# first to ensure the APK is installed).
maestro:
    maestro test maestro/flows

# Re-capture Maestro baselines (run after an intended visual change).
maestro-refresh:
    maestro test --update-screenshots maestro/flows

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
