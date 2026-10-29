# Releasing deskwright

Releases are built and published by [`dist`](https://opensource.axo.dev/cargo-dist/) from
`.github/workflows/release.yml`, which `dist generate` writes from the
`[workspace.metadata.dist]` section of `Cargo.toml`. Don't edit the workflow by hand; change
the config and run `dist generate`.

One release publishes:

| Channel | What users run |
|---|---|
| GitHub Releases | Prebuilt `aarch64-apple-darwin` and `x86_64-apple-darwin` archives with checksums |
| npm | `npx -y deskwright` |
| Homebrew | `brew install sabari-1234/tap/deskwright` (formula pushed to `Sabari-1234/homebrew-tap`) |
| Shell installer | `curl -LsSf …/deskwright-installer.sh \| sh` |

## One-time setup (repository secrets)

In **Settings → Secrets and variables → Actions → New repository secret** of
`Sabari-1234/deskwright`, add:

- `NPM_TOKEN`: an npm **granular access token** with read-and-write permission for the
  `deskwright` package (or all packages, for the very first publish), created at
  npmjs.com → Access Tokens.
- `HOMEBREW_TAP_TOKEN`: a GitHub **fine-grained personal access token** limited to the
  `Sabari-1234/homebrew-tap` repository with *Contents: read and write*, created at
  github.com → Settings → Developer settings → Personal access tokens.

## Cutting a release

1. Update `version` in the root `Cargo.toml` and add the release notes to `CHANGELOG.md`.
2. Run `./scripts/check.sh` and `dist plan` (it lists what will be published).
3. Commit, then tag and push:

   ```bash
   git tag v0.1.0
   git push origin main v0.1.0
   ```

The workflow builds both macOS targets, creates the GitHub Release with the archives and
installers, then publishes the npm package and updates the Homebrew formula.

## Adding a platform

When a new backend exists (for example Windows), add its target triple to `targets` in
`[workspace.metadata.dist]` (and `"powershell"` or `"msi"` to `installers`), then run
`dist generate`.
