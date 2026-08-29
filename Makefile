# Universal verb interface for the Quorum workspace.
#
# The Rust workspace lives under app/; every target drives cargo there so the
# repo root stays a thin control surface. `make help` lists everything; `make
# check` is the done-gate a human or an agent runs before calling work finished.
# There is no looser "agent mode": the gate is the same for everyone.

APP := app
E2E := e2e
CARGO := cargo
# Mirrors workspace.package.rust-version; the `msrv` target verifies it still
# builds on exactly that version, so the declared floor stays honest.
MSRV := 1.98.0
# Frozen instant for anything that must render identically between runs. A
# moving clock makes "due in 14 days" drift and churns every screenshot.
APP_CLOCK ?= 2026-03-01T12:00:00Z
# Where the seed databases are written.
DATA_DIR ?= .data

# Screenshots are produced inside containers, and this is not incidental.
# Text rasterisation differs between macOS and Linux, so a gallery regenerated
# on a laptop will never byte-match one regenerated in CI — the images churn
# depending on who ran the command last, and a baseline that moves on its own
# is not a baseline. Pinning the image *and* the architecture makes the two
# identical by construction rather than by luck: CI runs x86_64, so an Apple
# Silicon machine emulates it rather than producing something merely similar.
PLAYWRIGHT_IMAGE := mcr.microsoft.com/playwright:v1.62.1-noble
RUST_IMAGE := rust:1.98
CONTAINER_PLATFORM := linux/amd64
# A separate target directory: the host's macOS build and the container's Linux
# build must not evict each other.
LINUX_TARGET := target-linux
IN_CONTAINER := docker run --rm --platform $(CONTAINER_PLATFORM) -v "$(CURDIR)":/repo

CARGO_DIR := cd $(APP) &&

.DEFAULT_GOAL := help

.PHONY: help
help: ## List available targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# --- The done-gate -----------------------------------------------------------

.PHONY: check
check: fmt-check clippy test ## The done-gate: formatting, lints, and tests

# --- Build and test ----------------------------------------------------------

.PHONY: build
build: ## Build the whole workspace
	$(CARGO_DIR) $(CARGO) build --workspace

.PHONY: test
test: ## Run the workspace test suite with all features
	$(CARGO_DIR) $(CARGO) test --workspace --all-features

.PHONY: msrv
msrv: ## Verify the workspace builds on the declared minimum Rust version
	$(CARGO_DIR) $(CARGO) +$(MSRV) check --workspace --all-features --locked

.PHONY: fmt
fmt: ## Format the code
	$(CARGO_DIR) $(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Check formatting without writing changes
	$(CARGO_DIR) $(CARGO) fmt --all --check

.PHONY: clippy
clippy: ## Lint with clippy, warnings denied
	$(CARGO_DIR) $(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: doc
doc: ## Build docs, warnings denied (every public item must be documented)
	$(CARGO_DIR) RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --all-features --no-deps

.PHONY: deny
deny: ## Supply-chain gate: licences, advisories, duplicate majors
	$(CARGO_DIR) $(CARGO) deny check

# --- Running -----------------------------------------------------------------

.PHONY: seed
seed: ## Regenerate the seed databases (into .data by default)
	$(CARGO_DIR) $(CARGO) run --quiet -p app-seed -- ../$(DATA_DIR)

.PHONY: run
run: seed ## Run the application locally against a seeded database
	$(CARGO_DIR) $(CARGO) run -p app-web

.PHONY: run-frozen
run-frozen: seed ## Run with the clock frozen, as the screenshot pipeline does
	$(CARGO_DIR) APP_CLOCK=$(APP_CLOCK) $(CARGO) run -p app-web

# --- End-to-end and screenshots ---------------------------------------------

.PHONY: e2e-install
e2e-install: ## Install the end-to-end toolchain and its browsers
	cd $(E2E) && npm ci && npx playwright install --with-deps

.PHONY: e2e
e2e: ## Run the end-to-end suite against a release build
	$(CARGO_DIR) $(CARGO) build --release -p app-web -p app-seed
	# Screenshots are excluded deliberately. `make screenshots` owns the gallery
	# and writes it from inside a container; if this ran them too it would
	# overwrite those images with host-rendered ones, and whichever command ran
	# last would decide what got committed.
	cd $(E2E) && npx playwright test --grep-invert @screenshot

.PHONY: a11y
a11y: ## Run the accessibility checks over every route
	$(CARGO_DIR) $(CARGO) build --release -p app-web -p app-seed
	cd $(E2E) && npx playwright test --grep @a11y

.PHONY: screenshots
screenshots: ## Regenerate docs/screenshots/ (in a container, so CI and a laptop agree)
	$(IN_CONTAINER) -w /repo/app -v quorum-cargo-registry:/usr/local/cargo/registry \
		$(RUST_IMAGE) cargo build --release --target-dir $(LINUX_TARGET) -p app-web -p app-seed
	# Cleared first so an image whose test was removed shows up as a deletion.
	# Left in place it would sit in the gallery forever: `git diff` cannot see a
	# file that never changed, so nothing downstream would ever notice.
	rm -f docs/screenshots/*.png
	$(IN_CONTAINER) -w /repo/$(E2E) -e APP_BIN_DIR=../app/$(LINUX_TARGET)/release \
		$(PLAYWRIGHT_IMAGE) npx playwright test --grep @screenshot

.PHONY: screenshots-verify
screenshots-verify: ## Prove the screenshot pipeline is deterministic (two runs, byte-identical)
	@$(MAKE) --no-print-directory screenshots
	@rm -rf .screenshot-verify && cp -R docs/screenshots .screenshot-verify
	@$(MAKE) --no-print-directory screenshots
	@if diff -r --exclude=README.md .screenshot-verify docs/screenshots > /dev/null 2>&1; then \
		rm -rf .screenshot-verify; \
		echo "screenshots are byte-identical across two runs"; \
	else \
		echo "ERROR: screenshots differ between runs. Check, in order: the frozen"; \
		echo "clock (APP_CLOCK), asset fingerprints, fonts, and animation."; \
		diff -r --exclude=README.md .screenshot-verify docs/screenshots || true; \
		rm -rf .screenshot-verify; \
		exit 1; \
	fi

.PHONY: pr-screenshots
pr-screenshots: ## Emit Markdown embedding the screenshots changed on this branch
	@./scripts/pr-screenshots.sh

# --- Housekeeping ------------------------------------------------------------

.PHONY: clean
clean: ## Remove build artifacts and generated data
	$(CARGO_DIR) $(CARGO) clean
	rm -rf $(DATA_DIR) .screenshot-verify
