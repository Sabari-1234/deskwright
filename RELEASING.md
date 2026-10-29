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
| npm | `npx -y deskwright` | not yet published (see below) |

No repository secrets are needed: the workflow uses only the built-in `GITHUB_TOKEN`, and the
tap is updated with the maintainer's GitHub CLI login.

## Cutting a release

1. Update `version` in the root `Cargo.toml` and add the release notes to `CHANGELOG.md`.
2. Run `./scripts/check.sh` and `dist plan` (it lists what will be built).
3. Commit, then tag and push:

   ```bash
   git tag v0.1.0
   git push origin main v0.1.0
   ```

4. When the "Release" workflow has finished, publish the Homebrew formula:

   ```bash
   ./scripts/update-homebrew-tap.sh v0.1.0
   ```

## npm (not yet enabled)

Each release already contains `deskwright-npm-package.tar.gz`. To publish it:

1. Sign in once with `npm login`, then publish the first version from the release asset:
   `npm publish deskwright-npm-package.tar.gz --access public`.
2. To publish later releases from the workflow, add `"npm"` to `publish-jobs` in
   `[workspace.metadata.dist]`, run `dist generate`, and give the job credentials: either an
   `NPM_TOKEN` repository secret, or, if the generated job supports it, an npm **trusted
   publisher** for `Sabari-1234/deskwright` and `release.yml` (set in the package's settings
   on npmjs.com), which needs no token.

## Adding a platform

When a new backend exists (for example Windows), add its target triple to `targets` in
`[workspace.metadata.dist]` (and `"powershell"` or `"msi"` to `installers`), then run
`dist generate`.
