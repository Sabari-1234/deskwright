#!/usr/bin/env bash
# The single check every change must pass, before and after: formatting, lint with warnings
# as errors, every test, and the coverage gates. Run from anywhere: ./scripts/check.sh
set -euo pipefail

cd "$(dirname "$0")/.."

# rustup from Homebrew is keg-only and not on PATH by default.
HOMEBREW_RUSTUP_BIN="/opt/homebrew/opt/rustup/bin"
if [[ -d "$HOMEBREW_RUSTUP_BIN" ]]; then
    export PATH="$HOMEBREW_RUSTUP_BIN:$PATH"
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "cargo-llvm-cov is missing. Install it with:" >&2
    echo "  brew install cargo-llvm-cov && rustup component add llvm-tools-preview" >&2
    exit 1
fi

# Coverage gates: they may only ever be raised. See docs/TESTING.md.
readonly PROJECT_MINIMUM_LINE_COVERAGE=95
readonly CORE_MINIMUM_LINE_COVERAGE=98

# Thin adapter shells whose every line calls an API that needs a real macOS permission,
# plus the test-only fake. Their logic is extracted into tested modules; the integration
# tests in crates/deskwright-macos/tests cover them when permission is granted.
readonly EXCLUDED_FROM_GATE='(ax_element_actions|ax_hit_tester|ax_menu_bar_locator|ax_permission|ax_window_controller|running_app_terminator|pasteboard_clipboard|enigo_input_synthesizer|xcap_screen_capturer|testing/fake_desktop)\.rs$'
readonly OUTSIDE_CORE='crates/(deskwright-macos|deskwright-portable|deskwright-mcp|deskwright)/|testing/fake_desktop\.rs$'

step() { printf '\n==> %s\n' "$1"; }

step "Formatting"
cargo fmt --all --check

step "Lint (warnings are errors)"
cargo clippy --workspace --all-targets -- -D warnings

step "All tests, with coverage instrumentation"
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --no-report

step "Coverage gate: whole project >= ${PROJECT_MINIMUM_LINE_COVERAGE}% lines"
cargo llvm-cov report --summary-only \
    --ignore-filename-regex "$EXCLUDED_FROM_GATE" \
    --fail-under-lines "$PROJECT_MINIMUM_LINE_COVERAGE"

step "Coverage gate: deskwright-core >= ${CORE_MINIMUM_LINE_COVERAGE}% lines"
cargo llvm-cov report --summary-only \
    --ignore-filename-regex "$OUTSIDE_CORE" \
    --fail-under-lines "$CORE_MINIMUM_LINE_COVERAGE"

step "All checks passed"
