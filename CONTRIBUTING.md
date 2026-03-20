# Contributing to Tazama

## Dev Setup

```bash
# Clone and install dependencies
git clone https://github.com/anomalyco/tazama
cd tazama
./scripts/setup-dev.sh

# Build and verify
make check
```

### System Dependencies

- Rust 1.89+ (edition 2024)
- Node.js 20+, npm
- GStreamer 1.20+ with plugins-base and plugins-good
- Tarang codec libraries: dav1d, libvpx, openh264, opus, fdk-aac
- Vulkan SDK or lavapipe
- ALSA/PipeWire development libraries

## Code Style

- Run `make fmt` before committing
- All clippy warnings are errors (`-D warnings`)
- Edition 2024, resolver 2
- No emojis in code or comments

## Testing

```bash
make test              # All 143+ tests
make test-unit         # Unit tests only (no integration)
make test-mcp          # MCP integration tests
make test-coverage     # Coverage report via tarpaulin
```

### Writing Tests

- Core crate: test all public API, undo/redo symmetry, edge cases
- GPU crate: test helper functions and shader validation (no Vulkan needed)
- Media crate: test mixing logic, type conversions, error types (no GStreamer needed)
- Storage crate: use temp dirs for filesystem tests
- App crate: requires Tauri runtime — tested via MCP integration tests

### Coverage Targets

- `tazama-core`: 95%+ (pure logic, fully testable)
- `tazama-storage`: 80%+ (filesystem tests with temp dirs)
- `tazama-gpu`: helper functions only (Vulkan tests require hardware)
- `tazama-media`: mixing/type tests only (GStreamer tests require runtime)

## Git Workflow

- Branch from `main`
- Branch names: `feature/*`, `bugfix/*`, `docs/*`, `refactor/*`
- Conventional commits: `feat(core): add scene detection`
- PR into `main`, squash merge

## Architecture Rules

- **`tazama-core`** must have zero I/O dependencies (no tokio, no filesystem, no network)
- **`tazama-gpu`** must not depend on `tazama-media` — use trait objects (`FrameSource`, `AudioOutput`) to bridge
- **`tazama-media`** must not depend on `tazama-gpu` — no circular dependencies
- All Tauri commands go in `crates/app/src/commands.rs` — thin wrappers only
- GPU code stays in `tazama-gpu` — core types must not depend on Vulkan
- New effects require: shader + pipeline + core EffectKind + renderer integration (see [GPU guide](docs/development/gpu-guide.md))

## Project Structure

```
tazama/
├── crates/
│   ├── core/       # Pure data model (no I/O)
│   ├── media/      # GStreamer decode/encode/mix
│   ├── storage/    # SQLite + filesystem persistence
│   ├── gpu/        # Vulkan compute rendering
│   ├── app/        # Tauri v2 desktop shell
│   └── mcp/        # MCP server for AI agents
├── ui/             # React 19 + TypeScript frontend
├── docs/
│   ├── adr/        # Architecture Decision Records
│   └── development/ # Guides and roadmap
├── scripts/        # Build and setup scripts
├── recipes/        # AGNOS marketplace packaging
└── .agnos-agent/   # MCP agent manifest
```
