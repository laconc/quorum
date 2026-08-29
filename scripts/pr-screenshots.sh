#!/usr/bin/env bash
# Emit Markdown embedding the screenshots that changed on this branch, ready to
# paste into a pull request description.
#
# Screenshots are required only when a change touches the frontend; this script
# says so plainly when nothing did, so the pull request can carry the honest
# "N/A — no frontend change" rather than an empty section.
set -euo pipefail

if ! git rev-parse --verify --quiet HEAD > /dev/null 2>&1; then
    echo "## Screenshots"
    echo
    echo "N/A — no commits yet"
    exit 0
fi

base="${1:-origin/main}"
repo_url="$(git config --get remote.origin.url | sed -e 's/\.git$//' -e 's#git@github.com:#https://github.com/#')"
branch="$(git rev-parse --abbrev-ref HEAD)"

if ! git rev-parse --verify --quiet "$base" > /dev/null 2>&1; then
    echo "base ref '$base' not found; pass one as the first argument" >&2
    exit 1
fi

changed="$(git diff --name-only "$base"...HEAD -- docs/screenshots || true)"
changed="$(printf '%s\n' "$changed" | grep -E '\.png$' || true)"

if [ -z "$changed" ]; then
    echo "## Screenshots"
    echo
    echo "N/A — no frontend change"
    exit 0
fi

echo "## Screenshots"
echo
while IFS= read -r file; do
    [ -n "$file" ] || continue
    name="$(basename "$file" .png)"
    echo "### $name"
    echo
    echo "![$name](${repo_url}/blob/${branch}/${file}?raw=true)"
    echo
done <<< "$changed"
