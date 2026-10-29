#!/usr/bin/env bash
# Publishes a release's npm package (built by the Release workflow) to the npm registry, using
# the maintainer's `npm login`. Run it in an interactive terminal: npm asks you to approve the
# publish with your two-factor authentication. Usage: ./scripts/publish-npm.sh v0.1.0
set -euo pipefail

readonly TAG="${1:?usage: $0 <release tag, e.g. v0.1.0>}"
readonly SOURCE_REPOSITORY="Sabari-1234/deskwright"
readonly PACKAGE_ARCHIVE="deskwright-npm-package.tar.gz"

npm whoami >/dev/null 2>&1 || { echo "Run 'npm login' first." >&2; exit 1; }

work_directory="$(mktemp -d)"
trap 'rm -rf "$work_directory"' EXIT

gh release download "$TAG" --repo "$SOURCE_REPOSITORY" --pattern "$PACKAGE_ARCHIVE" \
    --dir "$work_directory"
tar -xzf "$work_directory/$PACKAGE_ARCHIVE" -C "$work_directory"
npm publish "$work_directory/package" --access public
