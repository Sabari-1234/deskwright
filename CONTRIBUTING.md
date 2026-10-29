# Contributing to deskwright

Thanks for helping. Bug reports, fixes, new tools and documentation are all welcome.

## Reporting a bug

Open an issue with the macOS version, the app you were driving, the tool call (or flow) you
ran, and what happened. `RUST_LOG=debug` output and a `--trace` report help a lot. Leave out
anything private that appears in the output (names, messages, file paths).

## Making a change

1. Fork the repository and create a branch.
2. Run the check once before changing anything, so you know the baseline passes:

   ```bash
   brew install cargo-llvm-cov && rustup component add llvm-tools-preview   # once
   ./scripts/check.sh
   ```

3. Make your change with tests. Every behavior needs a test for its main path and its failure
   paths, using the in-memory `FakeDesktop` (`deskwright_core::testing`), so tests need no
   macOS permissions.
4. Document it: each feature has a page in `docs/features/`; update it, or add one from the
   existing pages' layout, and keep `docs/ARCHITECTURE.md` current.
5. Run `./scripts/check.sh` again. It must pass: formatting, clippy with warnings as errors,
   every test, and the coverage gates (95% of lines overall, 98% for `deskwright-core`).
6. Open a pull request that explains what changed and why.

Never make the check pass by deleting, ignoring or weakening tests, or by lowering the gates.
If a test's expectation changes because the behavior deliberately changed, say so in the pull
request.

## Code layout and rules

- `crates/deskwright-core`: domain models, port traits and services. No OS, protocol or async
  code.
- `crates/deskwright-macos`, `crates/deskwright-portable`: adapters that implement the ports.
  `unsafe` is only allowed in `deskwright-macos`'s system-call modules, with a `// SAFETY:`
  comment on every block.
- `crates/deskwright-mcp`: one file per MCP tool; `crates/deskwright-runner`: flows and reports.
- `crates/deskwright`: the only place that names concrete adapters and wires services.

Prefer small types with one job, names that say what things are, and a new strategy or tool
type over a growing `match`. See `docs/ARCHITECTURE.md` and `docs/TESTING.md`.

## Adding a tool

Implement `TypedTool` in a new file under `crates/deskwright-mcp/src/tools/`, register it in
`crates/deskwright/src/composition.rs` and in the test fixture
(`crates/deskwright-mcp/tests/support/mod.rs`), and add tests and a feature page. See
`docs/features/mcp-server.md`.

## Releasing

Maintainers: see [RELEASING.md](RELEASING.md).
