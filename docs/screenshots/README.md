# Screenshots

The visual record of the product.

These files do three jobs at once, which is why they are generated rather than
captured by hand:

1. **The visual record.** What the product looked like at each point.
2. **Pull request descriptions.** `make pr-screenshots` emits Markdown embedding
   the ones that changed on a branch.
3. **The visual-regression baseline.** An unintended change to a page shows up
   as a changed image.

**These files are generated. Never edit one.** If a screenshot looks wrong, the
page is wrong.

## Regenerating

```sh
make screenshots          # regenerate the gallery
make screenshots-verify   # prove the pipeline is deterministic (two runs, byte-identical)
```

The end-to-end toolchain must be installed first: `make e2e-install`.

## Naming

```
<surface>-<screen>-<viewport>.png
```

Surfaces are `public`, `resident`, `board`, `admin`, and `harness` (the Phase 0
proof-of-pipeline page). Viewport is the CSS width in pixels.

## Viewports, and one engine per viewport

| Width | Engine | Why |
|---|---|---|
| 1280 | Chromium | The board and administration surfaces are desktop-primary. |
| 375 | WebKit | The width the design is decided at, on the engine this audience actually holds — iOS Safari. |

Three browser projects run the functional and accessibility checks, but only
these two produce images. Two projects share a 375px width, and both writing the
same filename would mean whichever finished last decided the gallery.

## Determinism

Images must be byte-identical between runs, and `make screenshots-verify`
enforces it. A gallery that churns on every run is a gallery everyone learns to
ignore, and a visual-regression baseline that moves on its own is not a
baseline.

Four things make it hold. When an image changes unexpectedly, check them in this
order — it is nearly always the first two:

1. **The frozen clock.** The application reads `APP_CLOCK` and installs a fixed
   clock, so anything relative — "due in 14 days", "reported this morning" —
   does not drift. The instant is `2026-03-01T12:00:00Z`.
2. **Fixed identifiers.** The seed generator is deterministic: same input, same
   row identifiers, same case numbers, same ordering. It draws no randomness and
   reads no clock.
3. **Rendering environment.** `make screenshots` runs inside pinned containers —
   a Rust image to build Linux binaries and the matching Playwright image to
   drive the browsers — at a pinned architecture. Text rasterisation differs
   between macOS and Linux, so generating on a laptop without this would churn
   every image.
4. **Animation.** Disabled during capture, and the page honours
   `prefers-reduced-motion`.

### What this does and does not guarantee

**It does** guarantee that regenerating twice in the same place produces
identical bytes. That is what `make screenshots-verify` checks, and it is the
property that makes the gallery a usable record.

**It does not** guarantee that your machine produces the same bytes as CI. On an
Apple Silicon machine the container emulates x86_64, and Chromium's rasteriser
takes a different path under emulation than on a native runner — WebKit's images
match CI exactly, Chromium's do not. Chasing that last gap would mean either
giving every contributor an x86_64 machine or tuning a pixel tolerance until it
passes, and a tolerance tuned to make a check go green is not a check.

So the gallery is **the visual record and the input to pull request
descriptions**. It is not a cross-machine byte oracle. What enforces that it
stays current is the frontend gate: a change touching the interface must change
the gallery, or continuous integration fails.

The pixel tolerance is **zero**. A budget hides exactly the small regressions
this exists to catch.

## When screenshots are required

Only when a change touches the frontend — anything under
`app/crates/web/templates/`, `app/crates/web/static/`, or
`app/crates/web/src/view/`. Continuous integration decides this from the diff,
not from the author's judgement. A backend-only change carries
`N/A — no frontend change` in its pull request instead.

## The gallery

| File | Route | Persona | Seed scenario | Shows |
|---|---|---|---|---|
| `harness-check-1280.png` | `/` | none | none | Phase 0's proof that the pipeline works end to end: compile-time-checked templates, fingerprinted assets under a strict Content Security Policy, the injected clock, and a live region for the partial update. |
| `harness-check-375.png` | `/` | none | none | The same page at the width the design is decided at. |

Phase 2 replaces this table with the real product: every key screen at both
viewports, and — the most useful pair in the gallery — the zero state and the
populated state of the same screen side by side.
