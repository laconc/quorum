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

**Requires Docker**, because images are produced inside pinned containers — see
the determinism rules below. The end-to-end toolchain must be installed first:
`make e2e-install`. The first run compiles the workspace inside a container and
takes a few minutes; later runs reuse the cached build.

Containers run as your own user, so everything they write is yours. If you ever
find root-owned files under `app/target-linux` or in the gallery, that rule has
been broken and `make clean` will fail on Linux.

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

Five things make it hold. When an image changes unexpectedly, check them in this
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

5. **Chromium's rasteriser is pinned.** Skia selects SIMD code paths by
   detecting CPU features at runtime, so an emulated x86_64 container and a
   native runner rasterise the same glyph differently.
   `--disable-skia-runtime-opts` forces the portable path, and subpixel
   antialiasing, subpixel positioning and colour management are disabled for
   the same reason. WebKit needs none of this — its images matched CI before
   any of it was added.

### What this guarantees

Regenerating twice in the same place produces identical bytes — checked by
`make screenshots-verify` — **and** your machine produces the same bytes as CI,
which is checked by continuous integration regenerating the gallery and diffing
it against what you committed.

That second property is what makes the check worth having. It catches a stale
gallery from **any** cause, not only the ones a path list happens to name: a
change to the seed data, to the handler behind a template, or to the screenshot
tests themselves all alter the images without touching anything under
`templates/` or `static/`.

The pixel tolerance is zero, and it stays zero. If images differ, something
about the page or the pipeline changed, and the answer is to find out what —
never to widen a tolerance until the check passes, which converts a check into
a formality.

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
