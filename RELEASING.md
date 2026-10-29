# Releasing deskwright

Releases are built and published by [`dist`](https://opensource.axo.dev/cargo-dist/) from
`.github/workflows/release.yml`, which `dist generate` writes from the
`[workspace.metadata.dist]` section of `Cargo.toml`. Don't edit the workflow by hand; change
the config and run `dist generate`.

One release publishes:

| Channel | What users run | Published by |
|---|---|---|
| GitHub Releases | Prebuilt `aarch64-apple-darwin` and `x86_64-apple-darwin` archives with checksums | the workflow |
| Shell installer | `curl -LsSf …/deskwright-installer.sh \| sh` | the workflow (attached to the release) |
| Homebrew | `brew install sabari-1234/tap/deskwright` | `scripts/update-homebrew-tap.sh`, run after the workflow |
| npm | `npx -y deskwright` | `scripts/publish-npm.sh`, run after the workflow |

No repository secrets are needed: the workflow uses only the built-in `GITHUB_TOKEN`, the tap
is updated with the maintainer's GitHub CLI login, and npm is published from the
maintainer's `npm login`, approved with their two-factor authentication.

## Cutting a release

1. Update `version` in the root `Cargo.toml` and add the release notes to `CHANGELOG.md`.
2. Run `./scripts/check.sh` and `dist plan` (it lists what will be built).
3. Commit, then tag and push:

   ```bash
   git tag v0.1.0
   git push origin main v0.1.0
   ```

4. When the "Release" workflow has finished, publish the Homebrew formula and the npm
   package (in an interactive terminal: npm asks you to approve with your second factor):

   ```bash
   ./scripts/update-homebrew-tap.sh v0.1.0
   ./scripts/publish-npm.sh v0.1.0
   ```

   A new npm version can take a minute to appear; `npm view deskwright version` shows it.

## Publishing npm from CI instead (optional)

To publish from the workflow rather than by hand, add `"npm"` to `publish-jobs` in
`[workspace.metadata.dist]`, run `dist generate`, and give the job credentials: an
`NPM_TOKEN` repository secret, or, if the generated job supports it, an npm trusted publisher
for `Sabari-1234/deskwright` and `release.yml` (set in the package's settings on npmjs.com).

## Adding a platform

When a new backend exists (for example Windows), add its target triple to `targets` in
`[workspace.metadata.dist]` (and `"powershell"` or `"msi"` to `installers`), then run
`dist generate`.
