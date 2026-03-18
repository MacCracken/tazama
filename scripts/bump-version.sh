#!/usr/bin/env bash
# bump-version.sh — Update all version references from the VERSION file.
#
# Usage:
#   ./scripts/bump-version.sh              # sync all files to current VERSION
#   ./scripts/bump-version.sh 2026.3.18    # set specific version and sync
#   ./scripts/bump-version.sh patch        # bump YYYY.M.D-N (increment N)
#   ./scripts/bump-version.sh today        # set to today's date YYYY.M.D
#
# The VERSION file is the single source of truth.
# Version format: YYYY.M.D or YYYY.M.D-N for patches.

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION_FILE="VERSION"

if [[ $# -ge 1 ]]; then
    if [[ "$1" == "patch" ]]; then
        current=$(cat "$VERSION_FILE" | tr -d '[:space:]')
        if [[ "$current" =~ ^([0-9]+\.[0-9]+\.[0-9]+)-([0-9]+)$ ]]; then
            base="${BASH_REMATCH[1]}"
            n="${BASH_REMATCH[2]}"
            new_version="${base}-$((n + 1))"
        else
            new_version="${current}-1"
        fi
        echo -n "$new_version" > "$VERSION_FILE"
    elif [[ "$1" == "today" ]]; then
        year=$(date +%Y)
        month=$(date +%-m)
        day=$(date +%-d)
        echo -n "${year}.${month}.${day}" > "$VERSION_FILE"
    else
        echo -n "$1" > "$VERSION_FILE"
    fi
fi

VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
echo "Syncing version: $VERSION"
echo ""

# 1. Cargo.toml workspace version
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
echo "  Cargo.toml              -> $VERSION"

# 2. tauri.conf.json
if [[ -f crates/app/tauri.conf.json ]]; then
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" crates/app/tauri.conf.json
    echo "  crates/app/tauri.conf.json -> $VERSION"
fi

# 3. ui/package.json
if [[ -f ui/package.json ]]; then
    sed -i "0,/\"version\": \".*\"/{s/\"version\": \".*\"/\"version\": \"$VERSION\"/}" ui/package.json
    echo "  ui/package.json         -> $VERSION"
fi

# 4. ui/package-lock.json
if [[ -f ui/package-lock.json ]]; then
    (cd ui && npm install --package-lock-only --silent 2>/dev/null) || true
    echo "  ui/package-lock.json    -> $VERSION"
fi

# 5. Cargo.lock
if command -v cargo &> /dev/null; then
    cargo update --workspace --quiet 2>/dev/null || true
    echo "  Cargo.lock              -> updated"
fi

echo ""
echo "Done. Verify:"
echo "  VERSION:            $(cat "$VERSION_FILE")"
grep '^version' Cargo.toml | head -1 | sed 's/^/  Cargo.toml:         /'
grep '"version"' crates/app/tauri.conf.json 2>/dev/null | head -1 | xargs | sed 's/^/  tauri.conf.json:    /'
grep '"version"' ui/package.json 2>/dev/null | head -1 | xargs | sed 's/^/  package.json:       /'
