#!/usr/bin/env bash
# setup-dev.sh — Install development dependencies for Tazama.
set -euo pipefail

echo "=== Tazama dev setup ==="

# Rust tools
echo "Installing Rust dev tools..."
cargo install cargo-watch cargo-tarpaulin cargo-audit --locked 2>/dev/null || true

# System dependencies (Arch/AGNOS)
echo ""
echo "Required system packages (install via your package manager):"
echo "  - gstreamer, gst-plugins-base, gst-plugins-good, gst-plugins-bad, gst-plugins-ugly"
echo "  - vulkan-icd-loader, vulkan-headers, vulkan-validation-layers"
echo "  - pipewire, pipewire-pulse"
echo "  - webkit2gtk (for Tauri)"
echo "  - pkg-config, cmake"

echo ""
echo "Done. Run 'make build' to verify."
