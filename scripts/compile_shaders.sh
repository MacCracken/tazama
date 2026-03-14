#!/usr/bin/env bash
# Compile GLSL compute shaders to SPIR-V.
# Requires glslangValidator (from vulkan-tools or glslang package).
set -euo pipefail

SHADER_DIR="$(cd "$(dirname "$0")/../crates/gpu/shaders" && pwd)"

if ! command -v glslangValidator &>/dev/null; then
    echo "ERROR: glslangValidator not found. Install glslang or vulkan-tools."
    exit 1
fi

shaders=(color_grade composite crop dissolve wipe fade)

for name in "${shaders[@]}"; do
    src="${SHADER_DIR}/${name}.comp"
    dst="${SHADER_DIR}/${name}.spv"
    echo "Compiling ${name}.comp → ${name}.spv"
    glslangValidator -V "$src" -o "$dst"
done

echo "All shaders compiled successfully."
