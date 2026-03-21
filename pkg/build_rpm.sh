#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
TOPDIR="$(mktemp -d)/rpmbuild"

cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"

mkdir -p "$TOPDIR"/{RPMS,SPECS}
cp "$REPO_ROOT/pkg/tascli.spec" "$TOPDIR/SPECS/"

rpmbuild -bb "$TOPDIR/SPECS/tascli.spec" \
    --define "_topdir $TOPDIR" \
    --define "version $VERSION" \
    --define "source_bindir $REPO_ROOT/target/release"

echo ""
echo "RPMs built:"
find "$TOPDIR/RPMS" -name '*.rpm'
