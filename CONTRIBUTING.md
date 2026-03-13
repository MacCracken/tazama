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

## Code Style

- Run `make fmt` before committing
- All clippy warnings are errors (`-D warnings`)
- Edition 2024, resolver 2

## Git Workflow

- Branch from `main`
- Branch names: `feature/*`, `bugfix/*`, `docs/*`, `refactor/*`
- Conventional commits: `feat(core): add scene detection`
- PR into `main`, squash merge

## Architecture Rules

- `tazama-core` must have zero I/O dependencies (no tokio, no filesystem, no network)
- All Tauri commands go in `crates/app/src/commands.rs` — thin wrappers only
- GPU code stays in `tazama-gpu` — core types must not depend on Vulkan
