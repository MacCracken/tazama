#!/usr/bin/env bash
# Generate test media fixtures for benchmarks and integration tests.
# Requires: ffmpeg
# Output: tests/fixtures/

set -euo pipefail
cd "$(dirname "$0")/.."

FIXTURES_DIR="tests/fixtures"
mkdir -p "$FIXTURES_DIR"

echo "Generating test fixtures in $FIXTURES_DIR..."

# 2-second 320x240 H.264 + AAC in MP4
ffmpeg -y -f lavfi -i "testsrc=s=320x240:d=2,format=yuv420p" \
       -f lavfi -i "sine=f=440:d=2" \
       -c:v libx264 -preset ultrafast -c:a aac -b:a 128k \
       "$FIXTURES_DIR/test_h264.mp4" 2>/dev/null

# 2-second 320x240 VP9 + Opus in WebM
ffmpeg -y -f lavfi -i "testsrc=s=320x240:d=2,format=yuv420p" \
       -f lavfi -i "sine=f=440:d=2" \
       -c:v libvpx-vp9 -b:v 500k -c:a libopus \
       "$FIXTURES_DIR/test_vp9.webm" 2>/dev/null

# 2-second 320x240 H.264 + AAC in MKV
ffmpeg -y -f lavfi -i "testsrc=s=320x240:d=2,format=yuv420p" \
       -f lavfi -i "sine=f=440:d=2" \
       -c:v libx264 -preset ultrafast -c:a aac -b:a 128k \
       "$FIXTURES_DIR/test_h264.mkv" 2>/dev/null

echo "Done:"
ls -lh "$FIXTURES_DIR"
