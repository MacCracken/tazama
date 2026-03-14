# Tazama Makefile

VERSION := $(shell cat VERSION 2>/dev/null || echo '0.1.0')
CARGO := cargo

BLUE := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
NC := \033[0m

.PHONY: all help build release clean run dev \
        test test-unit test-coverage \
        fmt format-check lint check \
        security-scan docs compile-shaders \
        ci-build ci-test ci-docs \
        version-sync version-bump frontend

all: help

help:
	@echo "$(BLUE)Tazama Build System$(NC) v$(VERSION)"
	@echo ""
	@echo "$(GREEN)Build:$(NC)"
	@echo "  $(YELLOW)build$(NC)          - Build all crates (debug)"
	@echo "  $(YELLOW)release$(NC)        - Build all crates (release)"
	@echo "  $(YELLOW)frontend$(NC)       - Build frontend (Vite)"
	@echo "  $(YELLOW)clean$(NC)          - Remove build artifacts"
	@echo ""
	@echo "$(GREEN)Run:$(NC)"
	@echo "  $(YELLOW)run$(NC)            - Run the Tauri dev server"
	@echo "  $(YELLOW)dev$(NC)            - Run with auto-reload (cargo-watch)"
	@echo ""
	@echo "$(GREEN)Test:$(NC)"
	@echo "  $(YELLOW)test$(NC)           - Run all tests"
	@echo "  $(YELLOW)test-unit$(NC)      - Run unit tests only"
	@echo "  $(YELLOW)test-coverage$(NC)  - Run tests with coverage (65% threshold)"
	@echo ""
	@echo "$(GREEN)Quality:$(NC)"
	@echo "  $(YELLOW)fmt$(NC)            - Format code"
	@echo "  $(YELLOW)format-check$(NC)   - Check formatting"
	@echo "  $(YELLOW)lint$(NC)           - Run clippy (deny warnings)"
	@echo "  $(YELLOW)check$(NC)          - Full quality check (fmt + lint + test)"
	@echo ""
	@echo "$(GREEN)Security:$(NC)"
	@echo "  $(YELLOW)security-scan$(NC)  - Run cargo audit"
	@echo ""
	@echo "$(GREEN)Version:$(NC)"
	@echo "  $(YELLOW)version-sync$(NC)   - Show current version"
	@echo "  $(YELLOW)version-bump$(NC)   - Bump version (usage: make version-bump V=2026.3.15)"

# --- Build ---

build:
	$(CARGO) build --workspace

release:
	$(CARGO) build --release --workspace

frontend:
	npm run build

clean:
	$(CARGO) clean
	rm -rf dist/

# --- Run ---

run:
	$(CARGO) tauri dev

dev:
	cargo watch -x 'build --workspace'

# --- Test ---

test:
	$(CARGO) test --workspace

test-unit:
	$(CARGO) test --workspace --lib

test-coverage:
	$(CARGO) install cargo-tarpaulin --locked 2>/dev/null || true
	$(CARGO) tarpaulin --workspace --exclude tazama --fail-under 65 --timeout 300 --out html --skip-clean
	@echo "Coverage report: tarpaulin-report.html"

# --- Quality ---

fmt:
	$(CARGO) fmt --all

format-check:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check: format-check lint test

# --- Security ---

security-scan:
	$(CARGO) install cargo-audit --locked 2>/dev/null || true
	$(CARGO) audit

# --- Shaders ---

compile-shaders:
	./scripts/compile_shaders.sh

# --- Docs ---

docs:
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --no-deps

# --- CI ---

ci-build: frontend build

ci-test: test

ci-docs: docs

# --- Version ---

version-sync:
	@echo "Current version: $(VERSION)"

version-bump:
ifdef V
	./scripts/bump-version.sh $(V)
else
	./scripts/bump-version.sh today
endif
