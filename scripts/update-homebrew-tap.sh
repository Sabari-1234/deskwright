#!/usr/bin/env bash
# Publishes a release's Homebrew formula to the Sabari-1234/homebrew-tap repository, using the
# GitHub CLI's login instead of a token stored in CI. Usage: ./scripts/update-homebrew-tap.sh v0.1.0
set -euo pipefail

readonly TAG="${1:?usage: $0 <release tag, e.g. v0.1.0>}"
readonly SOURCE_REPOSITORY="Sabari-1234/deskwright"
readonly TAP_REPOSITORY="Sabari-1234/homebrew-tap"
readonly FORMULA_FILE="deskwright.rb"

work_directory="$(mktemp -d)"
trap 'rm -rf "$work_directory"' EXIT

gh release download "$TAG" --repo "$SOURCE_REPOSITORY" --pattern "$FORMULA_FILE" \
    --dir "$work_directory"
gh repo clone "$TAP_REPOSITORY" "$work_directory/tap" -- --quiet
mkdir -p "$work_directory/tap/Formula"
cp "$work_directory/$FORMULA_FILE" "$work_directory/tap/Formula/$FORMULA_FILE"

cd "$work_directory/tap"
git add "Formula/$FORMULA_FILE"
if git diff --cached --quiet; then
    echo "The tap already has the $TAG formula."
    exit 0
fi
git commit --quiet -m "deskwright ${TAG#v}"
git push --quiet origin HEAD
echo "Published deskwright ${TAG#v} to $TAP_REPOSITORY."
