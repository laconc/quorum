# Contributing to Quorum (humans and agents)

**What this is.** Quorum is a governance and records platform for community
associations: a public site, a resident portal, and the administrative tooling a
volunteer board needs. Rust, Axum, server-rendered templates with htmx, one
SQLite file per association.

**Documents are law** — design intent is written down; code implements it; tests
cite it. When code and a document disagree, the code is wrong until the document
is amended.

## Reading list

- **The Quorum System Design Document** — the specification. It defines what to
  build and why every choice was made, including a decision log (§3) and a
  red-team record (Appendix A). **It is the decision log**, amended in place;
  there is no separate decision-record directory.
- **The build plan** — how the specification is sequenced, what has been
  amended, and the rules that bind every phase.
- **The phase documents** — one per phase, each written to be the complete
  context needed to build that phase.
- `docs/README.md` — the documentation index and reading order.
- "Counter-intuitive things" and "What deliberately doesn't exist", below — why
  the code is shaped this way, and what was deferred or dropped on purpose. A
  fresh context should be able to reconstruct every "why" from the repository,
  not from chat.

The first three live outside this repository and are provided when they are
needed. If you are starting work on a phase and do not have them, **ask for them
rather than inferring what they say** — they carry the requirements, the
red-team findings, and the out-of-scope boundaries that make a phase reviewable.

## Engineering standard

Documents are law; one source of truth per domain; machine-checkable beats prose
(every invariant has a test **and** a database constraint); honest state (debt
and deferrals are recorded under "What deliberately doesn't exist" with their
trigger, never silent); one done-gate for everyone (`make check`). Generated
artifacts — screenshots, migrations, the authorization matrix — are never
hand-edited. Rationale lives in files in the repository, not chat history.

## Non-negotiable invariants (each with the mechanism that enforces it)

A rule with no mechanism is a wish. Each of these is enforced by something that
fails a build.

| Invariant | Enforced by |
|---|---|
| Time is never read at the point of use | `clippy.toml` `disallowed-methods`; one documented exception in `app_testkit::SystemClock` |
| No database handle outside `app-db` | `rusqlite` is a dependency of `app-db` alone, proven by `crates/db/tests/handle_containment.rs`; the connection type never leaves the crate |
| `ATTACH` is impossible | attached-database limit set to zero on every connection, proven against real SQLite |
| An association identifier cannot name a file outside the data directory | `AssociationId::parse`, with the hostile-input cases as tests |
| No bare hash or code outside `app-crypto` | `clippy.toml` `disallowed-types`; every digest carries the algorithm that produced it |
| Canonical encoding is deterministic | property tests over ordering, round-trips, and injectivity — a non-deterministic encoding makes the audit chain decorative |
| Every response carries the security headers | applied as a layer, asserted over the real router in `crates/web/tests/security_headers.rs` |
| Authenticated HTML is never stored by a cache | `private, no-store` is the default; caching is opt-in per route |
| No floating point in the money path | `clippy.toml` `disallowed-types` |
| Screenshots are byte-identical between runs, **and across machines** | `make screenshots-verify`, plus CI regenerating and diffing the committed gallery |
| The gallery changes whenever the frontend does | the `frontend-gate` job, from the diff rather than the author's judgement |

Later phases add: every route has an authorization-matrix entry, every read path
returns one association's rows, no member-only route answers an unauthenticated
request, and no `UPDATE` or `DELETE` succeeds on ledger or audit tables.

## Done-gate

```
make check        # fmt-check + clippy (-D warnings) + test --all-features
```

Agents and humans use the same gate. There is no looser "agent mode." Never
report work as complete without it green; if it fails, say so and show the
output.

Beyond it: `make deny` (licences and advisories), `make doc` (rustdoc under
`-D warnings`), `make msrv`, `make e2e`, `make a11y`, and
`make screenshots-verify`. CI runs all of them.

## Workflow order

Document change → code → tests → gates. If you are adding behaviour the
specification does not describe: stop and amend the design document first.

## Phase discipline

Work proceeds in phases, each with its own document. Three rules:

1. **The phase document's "out of scope" is binding.** Do not build ahead
   because it seems convenient. If a phase genuinely cannot complete without
   something from a later one, stop and say so.
2. **Every phase ends with a review** — the exit criteria checked against their
   evidence, a code review, and a security review. A phase is not complete when
   the code works; it is complete when it has been reviewed and the findings
   dealt with. The review is a step, not a document: findings get fixed, and
   anything worth remembering afterwards goes where someone will actually read
   it — "Counter-intuitive things" or "What deliberately doesn't exist" below,
   a code comment at the place it matters, or the design document if it changes
   the specification. A file recording that a review happened is a file nobody
   opens again.
3. **Anything deferred gets an `#[ignore = "reason"]` test**, where the ignore
   reason is the contract. Unbuilt features are executable specifications. They
   get implemented, never deleted.

## Testing

| Layer | Proves | Where |
|---|---|---|
| Unit | one rule per test, named for the rule | beside the code |
| Property | invariants over generated input | `proptest`, in `tests/` |
| Integration | the real thing against real SQLite — **no mocked data layer** | `crates/*/tests/` |
| Planted defect | the harness catches what it claims to | `crates/*/tests/planted_defects.rs` |
| End-to-end | the real binary, a real browser | `e2e/` |
| Accessibility | axe over every route, plus manual assistive-technology testing | `e2e/`, `make a11y` |
| Screenshot | the visual record, and determinism | `make screenshots` |

**Rules.** Red tests are executable specifications. A bug fix ships with the
test that would have caught it. No flaky tests — anything nondeterministic takes
a seed. Golden vectors are write-once and are never regenerated to turn a red
test green. Every harness has a planted-defect fixture it must catch; a build
where a planted defect goes uncaught is red.

**Never weaken a test to make it pass.** Not by loosening an assertion, not by
adding `#[ignore]` to something that used to run, not by widening a pixel
tolerance. If a test is wrong, fix it as a reviewed contract change with the
reasoning recorded.

## Pull requests

Required sections: Summary, Design-document traceability, Testing, Screenshots,
Risk.

**Screenshots are required only when the change touches the frontend** — that
is, anything under `app/crates/web/templates/`, `app/crates/web/static/`, or
`app/crates/web/src/view/`. CI decides this from the diff, not from the author's
judgement. When nothing frontend changed, the section reads
`N/A — no frontend change`. `make pr-screenshots` emits the Markdown.

**Anything an agent writes on a pull request says so.** Descriptions, review
replies, and comments all end with an attribution line naming Claude and the
account it acted on behalf of. A human reading a review thread is entitled to
know which side of it was written by a person — the reply may be right either
way, but who wrote it changes how much independent checking it deserves.

**Answer a review comment, then resolve its thread.** An answered thread left
open is indistinguishable from an ignored one, and the reviewer has to re-read
it to find out which. Do not resolve a thread whose comment has not actually
been addressed in the code.

## Layout

- `app/` — the Cargo workspace. Rust lives under `app/`; the `Makefile` at the
  root is a thin control surface. (This mirrors ironstate, and exists so the
  root stays readable.)
  - `app/crates/db` — **the only source of a database handle.**
  - `app/crates/crypto` — the only hashing, code, and token surface.
  - `app/crates/testkit` — injected time, and the shared harnesses.
  - `app/crates/web` — Axum, Askama, htmx, security headers.
  - `app/crates/seed` — the deterministic seed generator.
- `e2e/` — Playwright. Deliberately outside the Cargo workspace: it is a
  different toolchain and mixing them complicates both.
- `docs/` — design, testing, security, operations, screenshots.
- `scripts/` — small tools the Makefile calls.
- `ops/` — deployment and replication configuration (Phase 7).

## Vocabulary (binding)

- **"Member" is reserved** for its statutory meaning: a right-holder with voting
  standing. It must never appear in code or interface copy meaning "person with
  an account." Use *resident*, *owner*, or *affiliate*.
- **The product name is not a namespace.** No `quorum::` root module, no
  `QuorumService`, no `Quorum` type. Crates are named for their domain.
- **In the elections module, `quorum` always means the participation
  threshold** for a valid vote. It is the correct domain term there and must not
  be renamed to avoid colliding with the product name.

## Counter-intuitive things (don't "helpfully" undo)

- **The workspace is under `app/`, not the repository root.** Deliberate, and
  mirrors ironstate.
- **The toolchain tracks latest stable**, not a pinned version.
  `rust-version = "1.98"` is a floor the `msrv` target verifies, not a pin.
- **`rusqlite`, not `sqlx`.** `set_limit(SQLITE_LIMIT_ATTACHED, 0)` is reachable
  only through `rusqlite`, and disabling `ATTACH` is mandatory — SQLite enforces
  no tenant boundary of its own, so one `ATTACH` would reduce physical file
  separation to a naming convention. The cost is losing compile-time-checked
  queries; the mitigation is that every query lives behind a typed function with
  an integration test against real SQLite.
- **The `load_extension` feature is deliberately absent** from `app-db`'s
  manifest. Not compiling the capability in is a stronger guarantee than
  remembering to disable it at runtime. If anyone adds it, ask why.
- **Reserved device names like `con` are accepted as identifiers.** Every
  filename is `assoc_<id>.db`, and `assoc_con` is not reserved. The prefix is
  what makes this safe, and a test pins the prefix rather than adding a
  rejection rule for a constraint we do not have.
- **The canonical JSON encoder rejects floats rather than serialising them.**
  RFC 8785's hardest requirement is reproducing ECMAScript double formatting,
  which is where implementations disagree. Nothing hashed here has a legitimate
  float — money is `i64` cents, shares are integer pairs, timestamps are
  integers — so refusing them removes the only genuinely subtle part of the
  specification.
- **The audit chain uses SHA-384, not SHA-256.** Grover halves preimage
  strength, and a seven-year statutory chain cannot be cheaply rehashed later.
- **htmx is vendored, not loaded from a content delivery network**, and its
  inline indicator styles are disabled by meta tag with the styles carried in
  our own stylesheet. A third-party script origin would need admitting to the
  Content Security Policy, and htmx's injected `<style>` is refused by a policy
  with no `unsafe-inline` — which is the policy working.
- **`ironstate` is consumed from crates.io**, never as a path dependency, even
  though it is developed locally and moves fast. A path dependency would make
  this repository's builds depend on someone's uncommitted work, and would mean
  a local experiment upstream could break this build with nothing recorded
  about why. Check what is actually published before planning around a feature.
- **Screenshots come from one engine per viewport.** Three browser projects run
  the checks, but two share a 375px width and would race for one filename.
- **The screenshot pixel tolerance is zero.** A budget hides exactly the small
  regressions the suite exists to catch, and invites tuning the number instead
  of fixing the cause.
- **`make e2e` deliberately skips the screenshot tests.** `make screenshots`
  owns the gallery and writes it from inside a container. If the end-to-end run
  wrote it too, host-rendered images would overwrite container-rendered ones and
  whichever command ran last would decide what got committed.
- **`make screenshots` runs in containers, and is therefore slow.** Text
  rasterises differently on macOS and Linux, so a gallery regenerated on a
  laptop would never match one regenerated in CI. The Rust and Playwright
  images and the architecture are all pinned. Do not "speed this up" by running
  the browsers natively.
- **Chromium is launched with `--disable-skia-runtime-opts` and friends.** Skia
  picks SIMD paths by detecting CPU features at runtime, so an emulated x86_64
  container and a native runner rasterise the same glyph differently. Pinning
  the portable path is what lets an Apple Silicon machine produce byte-identical
  images to CI — without it the gallery check fails for a reason no author can
  act on. WebKit needs none of this; its images already matched.

## What deliberately doesn't exist

Recorded so it is not re-derived later, each with what would change the answer.

- **No `docs/decisions/` directory.** The design document's §3 is the decision
  log and is amended in place; phase reviews carry everything else.
- **No `ironstate-journal` yet — reopened for Phase 3, not settled.** It was
  declined because `append` owned its own transaction boundary and could not
  enlist in the caller's, while a state change and the work it causes must
  commit together — a cure period that fires with no notice sent is a legal
  defect, not a bug — and because one journal was one stream, while this system
  has thousands of live instances. Feedback went upstream and **both gaps are
  fixed** in `ironstate-journal` 0.2.0: `Journal` has an associated `Tx<'a>`
  with `append_in`, takes a `StreamId` on every operation, and
  `RetainableJournal` adds the retention a seven-year records policy needs.

  Three things to know before anyone writes the adapter, all learned from
  upstream rather than from reading the trait:

  1. **`execute_in` does not evolve the aggregate.** It returns a `#[must_use]
     Pending` that you commit *after* your transaction resolves. Evolving on
     append — which is what the original proposal implied — would leave the
     in-memory aggregate ahead of the durable log whenever the enclosing
     transaction rolls back, which is the exact failure the seam exists to
     prevent, reintroduced one layer up.
  2. **One `execute_in` per transaction.** `head` and `entropy_pos` take no
     transaction and see only committed state, so a second call against the
     same open transaction computes a stale rewind anchor. One transition plus
     our own writes fits; more does not.
  3. **`journal_contract_test!` cannot yet drive an adapter whose `Tx` is a real
     transaction** — every entry point is bound `Tx<'a> = ()`. So the adapter
     shape the seam was added for is currently outside the gate meant to hold
     it, and such adapters are held instead by a twin over the same storage
     whose `Tx` is `()`.

  What remains is not a capability question, and upstream says so plainly:
  every invariant must exist twice, as a test *and* as a database constraint,
  and a CHECK constraint cannot see inside an event blob. Double-entry balance,
  the singular-role partial index, and the triggers refusing `UPDATE` on ledger
  and audit rows all need real columns — so the relational tables are the system
  of record either way, and Phase 3 decides whether a journal beside them earns
  its keep or is a second copy of the truth. Concluding it does not is a sound
  outcome.

  The transition log is a plain table shaped like an adapter for exactly this
  reason, so adoption stays a swap rather than a rewrite.
- **The audit hash chain is ours to build, permanently.** Proposal C — opt-in
  hash-chained append — was declined upstream for a reason worth keeping: a
  chain is evidence only once its root is externally anchored, and anchoring is
  application infrastructure, so shipping the chain alone would deliver the
  half that does not provide the property. If it is ever upstreamed it will be
  pure functions over bytes the adapter already persists, hashed **per batch
  rather than per event**, carrying a per-link algorithm tag. Build ours that
  way, so it could be contributed later without reshaping.
- **No empty crates for later phases.** A crate arrives when it has something in
  it.
- **No dark mode.** The design document does not call for one, and the contrast
  targets are tuned for a single palette.
- **No bundled webfont yet.** A system stack resolves identically inside the
  end-to-end container, which is where images are produced. *Trigger:* Phase 2,
  when real typography arrives.
- **No `routes!` registry yet.** There are no authorization, isolation, or
  cache-leak harnesses for it to generate in Phase 0. *Trigger:* Phase 1 — and
  from then on, a route registered outside it silently opts out of all three
  checks.
- **No post-quantum signatures yet.** ML-DSA-87 over daily audit roots, export
  bundles, and certified tallies is Phase 1, gated on selecting an
  implementation against recorded criteria. The ignored tests in
  `crates/crypto/src/digest.rs` carry the contract.
- **Passkeys are not post-quantum, and cannot be.** WebAuthn has no standardised
  post-quantum algorithm, including for the mandatory platform-administrator
  passkeys. The residual risk is forgery requiring a quantum adversary at
  authentication time, not a later break of a recorded assertion. *Trigger:* a
  standardised algorithm with authenticator support.
