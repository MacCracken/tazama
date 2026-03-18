#!/bin/bash
# Creates minimal stub crates for tarang dependencies when
# the tarang repo is not available (e.g., CI environments).
# These stubs satisfy Cargo manifest resolution for optional deps.

set -euo pipefail

TARANG_ROOT="../tarang"

# If tarang already exists, nothing to do
if [ -d "$TARANG_ROOT/crates/tarang-core" ]; then
    echo "tarang repo found, skipping stub creation"
    exit 0
fi

echo "Creating tarang stub crates for CI..."

CRATES=("tarang-core" "tarang-audio" "tarang-demux" "tarang-video" "tarang-ai")

for crate in "${CRATES[@]}"; do
    dir="$TARANG_ROOT/crates/$crate/src"
    mkdir -p "$dir"

    # Create minimal Cargo.toml
    cat > "$TARANG_ROOT/crates/$crate/Cargo.toml" <<TOML
[package]
name = "$crate"
version = "0.1.0"
edition = "2021"
TOML

    # Add feature stubs for tarang-video
    if [ "$crate" = "tarang-video" ]; then
        cat >> "$TARANG_ROOT/crates/$crate/Cargo.toml" <<TOML

[features]
default = []
openh264 = []
vpx = []
dav1d = []
TOML
    fi

    # Add TarangError for tarang-core
    if [ "$crate" = "tarang-core" ]; then
        cat > "$dir/lib.rs" <<RUST
#[derive(Debug)]
pub struct TarangError(pub String);

impl std::fmt::Display for TarangError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TarangError {}
RUST
    else
        echo "// Stub crate for CI" > "$dir/lib.rs"
    fi
done

echo "Tarang stubs created successfully"
