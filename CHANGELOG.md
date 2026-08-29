# Changelog

## Unreleased

### Phase 0 — repository, gates, and harness

The scaffolding every later phase is built on. No product: one page exists to
prove the pipeline end to end.

- Cargo workspace under `app/` on Rust 1.98, with `make check` as the single
  done-gate.
- `app-db`: the connection factory. `ATTACH` disabled, extension loading not
  compiled in, defensive mode on, double-quoted string literals refused, and
  association identifiers that cannot name a file outside the data directory.
  It is the only crate that depends on `rusqlite`, and a test enforces that.
- `app-crypto`: algorithm-agile hashing (SHA-384), HMAC-SHA-384, 256-bit
  tokens, and a deterministic canonical JSON encoding that rejects floats.
  Property-tested, because a non-deterministic encoding would make the audit
  chain decorative.
- `app-testkit`: injected time, with a lint forbidding every other way to read
  a clock.
- `app-web`: Axum and Askama with a strict Content Security Policy, vendored
  htmx, fingerprinted immutable assets, and `private, no-store` by default.
- `app-seed`: the deterministic seed generator.
- Playwright across three browser projects, axe accessibility checks, and a
  screenshot pipeline that renders inside pinned containers with Chromium's
  rasteriser pinned too — so the gallery is byte-identical across machines, not
  just across consecutive runs, and CI can regenerate it and fail on any diff.
- Planted-defect fixtures for the canonical encoder, the hash chain, and the
  identifier parser.
