# Quorum

A governance and records platform for community associations: a public site, a
resident portal, and the administrative tooling a volunteer board needs to run
an association — dues, architectural review, rule enforcement, document custody,
communications, amenity booking, and election administration.

- **Product name:** Quorum
- **Platform domain:** `common-interest.community`
- **Platform administration:** `admin.common-interest.community`

**Each association has one origin, and it serves everything** — public site and
resident portal alike. A resident never leaves the domain they know, and there
is no sign-in redirect.

That origin is either a domain the association controls (`oakwoodhoa.org`) or a
platform subdomain we provision (`oakwood.common-interest.community`) for boards
that cannot manage DNS. Both are first-class; the application treats them
identically, and the platform subdomain stays available either way so a lapsed
domain cannot take dues payment down with it.

One consequence is worth knowing early: a session cookie carries the `__Host-`
prefix, so it is pinned to one association's origin and **cannot be sent to
another's at all**. Tenant isolation gets a structural second layer for free.

## Status

**Phase 0 — repository, gates, and harness.** There is no product yet. One page
exists to prove the pipeline end to end.

The specification, the build plan, and the per-phase documents live outside this
repository and are provided when needed.

## Getting started

Requires [rustup](https://rustup.rs) (the toolchain installs itself from
`app/rust-toolchain.toml`). Node.js is needed for the end-to-end suite, and
Docker for regenerating screenshots — the gallery is produced inside pinned
containers so that a laptop and CI render identically. Neither is needed for
`make check`.

```sh
make help          # every target
make check         # the done-gate: fmt, clippy, tests
make run           # serve locally against a seeded database
```

For the end-to-end suite and the screenshot gallery:

```sh
make e2e-install   # once: toolchain and browsers
make e2e           # functional and accessibility checks
make screenshots   # regenerate docs/screenshots/ (needs Docker)
```

The first `make screenshots` compiles the workspace inside a container, so it
takes a few minutes; later runs reuse the cached build.

## How it is put together

One Rust binary serves HTTP, runs the job worker, and runs the scheduler. Pages
are rendered server-side with htmx for partial updates — there is no client-side
framework, so there is no session token in JavaScript's reach for a scripting
flaw to steal.

Each association's data lives in **its own SQLite file**, with one platform
database for identity and the association registry. Physical file separation is
the isolation mechanism: a query cannot reach data in a file that was never
opened. That guarantee rests on there being exactly one way to open a file,
which is why `app-db` is the only crate that can, and why `ATTACH` is disabled
on every connection.

| Crate | What it does |
|---|---|
| `app-db` | The connection factory. The only source of a database handle. |
| `app-crypto` | Hashing, message authentication, tokens, canonical encoding. Every artifact records the algorithm that produced it. |
| `app-testkit` | Injected time. Nothing else may read a clock. |
| `app-web` | Axum, Askama, htmx, and the security headers every response carries. |
| `app-seed` | Deterministic seed data, shared by tests, the isolation harness, and the screenshots. |

## Documentation

| Read | When |
|---|---|
| [AGENTS.md](AGENTS.md) | **First**, before changing anything. The contributor guide, for humans and agents alike. |
| [docs/design.md](docs/design.md) | To understand why the system is shaped this way. |
| [docs/testing.md](docs/testing.md) | What each test layer proves and where it lives. |
| [docs/security.md](docs/security.md) | The controls, the threat model, and what we do *not* claim. |
| [docs/operations.md](docs/operations.md) | Running it, and what to do when it breaks. |
| [docs/screenshots/](docs/screenshots) | The visual record of the product. |

## Licence

Proprietary. All rights reserved.
