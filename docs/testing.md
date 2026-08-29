# Testing taxonomy

What each layer proves and where it lives. The done-gate is `make check`; the
heavier suites run in continuous integration on their own cadence.

## Layers

| Layer | Proves | Location | Since |
|-------|--------|----------|-------|
| Unit | one rule per test, named for the rule | `#[cfg(test)]` beside the code | Phase 0 |
| Property | an invariant holds over generated input | `crates/*/tests/` with `proptest` | Phase 0 |
| Integration | the real thing against real SQLite — **no mocked data layer** | `crates/*/tests/` | Phase 0 |
| Planted defect | the harness catches what it claims to catch | `crates/*/tests/planted_defects.rs` | Phase 0 |
| End-to-end | the real binary, a real browser, a seeded database | `e2e/` | Phase 0 |
| Accessibility | axe over every route, plus assistive-technology testing by hand | `e2e/`, `make a11y` | Phase 0 |
| Screenshot | the visual record, and that it is reproducible | `make screenshots-verify` | Phase 0 |
| Handle containment | no crate but `app-db` can open a database | `crates/db/tests/handle_containment.rs` | Phase 0 |
| Structural analysis | reachability, deadlocks, dead transitions in every state graph | `ironstate::analyze!` | Phase 3 |
| Determinism | identical inputs produce an identical digest, across architectures | `determinism_test!` | Phase 4 |
| Authorization matrix | every route × role has a declared expected outcome | generated from the route registry | Phase 1 |
| Isolation sweep | every read path returns one association's rows | generated, against the two-association fixture | Phase 1 |
| Cache leak | no member-only route answers an unauthenticated request | generated | Phase 1 |
| Invariant harness | the §15.2 invariants, in CI **and** nightly against live data | `make invariants` | Phase 1 |
| Mutation | the suite actually catches injected bugs | `cargo-mutants --in-diff` | Phase 4 |
| Fuzz | garbage is rejected with a typed error, never a panic | `cargo-fuzz` on the upload pipeline | Phase 5 |
| Restore drill | the backups are real | scheduled, weekly | Phase 1 |

## Rules

- **Red tests are executable specifications.** Unbuilt features live as
  `#[ignore = "reason"]` tests, and the ignore reason is the contract. They get
  implemented, never deleted.
- **A bug fix ships with the test that would have caught it.**
- **No flaky tests.** Anything nondeterministic takes a seed.
- **Golden vectors are write-once.** Never regenerated to turn a red test green.
- **Never weaken a test to make it pass.** Not by loosening an assertion, not by
  adding `#[ignore]` to something that used to run, not by widening a pixel
  tolerance. If a test is wrong, fix it as a reviewed contract change with the
  reasoning recorded.
- **Every harness has a planted defect it must catch.** A build where a planted
  defect goes uncaught is red.

## Test the testers

A harness that has never been shown to catch anything is not a harness. Each
fixture plants a deliberate defect and asserts the real check catches it.

| Planted defect | Caught by |
|---|---|
| A canonical encoder sorting keys by UTF-8 bytes instead of UTF-16 code units | the ordering comparison in `crypto/tests/planted_defects.rs` |
| A canonical encoder that does not sort at all | the order-independence property |
| A chain link that ignores its predecessor | the chain-dependence assertion |
| A validator that only rejects the literal `".."` | the hostile-identifier table in `db/tests/planted_defects.rs` |

The UTF-8-sorting encoder is the one worth understanding: it agrees with the
correct encoder on every ASCII input, so an ordinary suite passes. It diverges
only above the basic multilingual plane — which is to say, on someone's name.

## Screenshot determinism

Screenshots serve three purposes at once — the visual record, the input to pull
request descriptions, and the visual-regression baseline — so they must be
reproducible. `make screenshots-verify` runs the pipeline twice and fails if the
output differs.

Five things make it hold, and when an image churns unexpectedly the cause is
almost always one of them, in this order. `docs/screenshots/README.md` is the
canonical list; this is a summary of it:

1. **The frozen clock.** `APP_CLOCK` freezes time, so anything relative does not
   drift.
2. **Fixed identifiers.** The seed generator is deterministic — same input, same
   identifiers, same ordering. It draws no randomness and reads no clock.
3. **Pinned containers.** `make screenshots` builds Linux binaries in a Rust
   image tied to the declared MSRV and drives the browsers in the matching
   Playwright image, at a pinned architecture, as the invoking user. Text
   rasterises differently on macOS and Linux, so generating on a host would
   churn every image.
4. **Chromium's rasteriser flags.** Skia picks SIMD paths from runtime CPU
   detection, so an emulated x86_64 container and a native runner rasterise the
   same glyph differently. `--disable-skia-runtime-opts` pins the portable path.
   WebKit needs none of this.
5. **Animation.** Disabled in the capture, and `prefers-reduced-motion` honoured
   by the page.

Together those make the gallery byte-identical **across machines**, not merely
within one — which is what lets CI regenerate it and fail on any diff. That
check catches staleness from any cause, including ones no path list would
predict: a change to seed data, to a template's backing handler, or to the
screenshot tests themselves.

The pixel tolerance is **zero**. A budget hides exactly the small regressions
the suite exists to catch, and invites tuning the number instead of fixing the
cause.

**Only `make screenshots` writes the gallery.** `make e2e` deliberately excludes
the screenshot tests; if both wrote it, whichever ran last would decide what got
committed.

Screenshots come from **one engine per viewport** — Chromium at 1280, WebKit at
375 — because three browser projects run the checks and two share a width, and
both writing the same filename would mean whichever finished last decided the
gallery.

## Accessibility

Automated tooling catches roughly a third of real accessibility problems. **A
clean axe run is a floor, not a verdict.** From Phase 2, every phase touching the
interface includes a session with assistive technology on a physical device, and
its findings — including the ones not fixed — are recorded in the phase review.
