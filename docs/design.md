# Design

Why the system is shaped this way. What to build is in the design document; this
records the shape the implementation took and the reasoning that is easy to lose.

## The audience is the constraint

Owners and residents skew heavily past sixty-five and log in two to six times a
year. Board members are unpaid volunteers with variable technical skill, no IT
support, and a one-to-three-year turnover. Neither group can be trained.

This is not a preference about design; it is the primary engineering constraint,
and it decides things that look unrelated to it. Sessions are long because
forcing re-authentication on every visit is a real barrier. Text is 17–18px
because presbyopia is the default assumption. Nothing hides behind a menu.
Deadlines that carry legal weight look different from courtesy ones. Paper
notices carry a code to scan, because the physical world is where this audience
actually lives.

## One binary, one box, one file per association

A single Rust binary runs the HTTP handlers, the job worker, and the scheduler
as separate tasks in one process. There is no separate worker deployment.

This is correct at the target scale and has one consequence worth stating
plainly: **a deploy interrupts in-flight jobs.** Jobs are therefore idempotent
and re-runnable with visibility timeouts, never at-most-once.

Each association's data lives in its own SQLite file, with one platform database
for identity, sessions, the association registry, and cross-association
membership. A person exists once; their standing exists per association, and
sessions must be resolvable before any association database is opened.

## Tenant isolation is structural

Physical file separation is the strongest practical isolation at this scale: a
query cannot reach data in a file that was never opened.

But SQLite enforces no tenant boundary of its own, so the guarantee is only
worth what the surrounding arrangement is worth. Three things make it real:

1. **`app-db` is the only crate that depends on `rusqlite`**, enforced by a
   test. The connection type never leaves the crate, so a handler cannot open a
   database — it has nothing to open one with.
2. **`ATTACH` is disabled** on every connection by setting the attached-database
   limit to zero. Without this, one statement reduces file separation to a
   naming convention.
3. **Association identifiers are parsed, not passed through.** The character set
   is narrow enough that traversal, absolute paths, null bytes, and Unicode
   normalisation tricks are excluded by construction rather than by a filter
   that has to anticipate them.

Each association has one origin serving both its public site and its portal, so
a session cookie's `__Host-` prefix pins it to a single association: the browser
will not send it to another's origin. That is a second, structural layer under
the same guarantee.

**The host header is never an authorization input.** It selects the origin; the
association identifier still comes from the session record and nowhere else; and
the two MUST agree, with a mismatch refused as a hard error. Preferring the
session would let a misrouted request succeed, and preferring the host would
make the host authoritative — so neither is allowed to win.

## Time is injected

Nothing reads a clock at the point of use; a lint enforces it, with one
documented exception. Cure periods, term expiries, vote closings, delay windows,
and suspension windows are all defined by elapsed time, and none of them are
testable if the current instant is read where it is used. A test advances a
clock instead of sleeping.

The same mechanism keeps the screenshot pipeline deterministic: a frozen clock
means "due in 14 days" does not drift between runs.

## Cryptography records its own algorithm

Every digest is `{algorithm, bytes}`, never a bare byte array, and verification
dispatches on the stored algorithm. Hashing outside `app-crypto` is refused by a
lint.

The reason is the retention period. Records written today are kept for years —
longer than the expected service life of any particular algorithm. A hash chain
whose links do not say which hash produced them is a chain with an expiry date,
and the migration is only cheap if it was designed for before the first record
was written.

The audit chain uses SHA-384 rather than SHA-256: Grover's algorithm halves
effective preimage strength, and rehashing a statutory chain after the fact
means re-anchoring everything ever published.

## Canonical encoding rejects floats

Anything hashed is encoded as a strict subset of RFC 8785 — keys sorted by
UTF-16 code unit, no insignificant whitespace, minimal escapes — with one
deliberate difference: floating-point numbers are rejected rather than
serialised.

RFC 8785's hardest requirement is reproducing ECMAScript's double formatting,
which is where independent implementations disagree. Nothing hashed here has a
legitimate float: money is `i64` minor units, fractional shares are integer
numerator/denominator pairs, timestamps are integers. Refusing floats removes
the only genuinely subtle part of the specification and converts a class of
"the chain does not verify on the other machine" into a typed error at the point
of encoding.

## Server-rendered, with htmx

Templates render server-side and htmx handles partial updates. There is no
client-side framework and no JSON interface consumed by a browser.

No client-side session token means a scripting flaw cannot exfiltrate one. The
application is forms and lists; a bundle download and a token-handling problem
would not improve it. It works on weak connections.

The cost is accepted deliberately: htmx swaps content silently, so **every swap
that changes meaning targets a live region, and focus moves deliberately after
swaps that change context.** An update nobody is told about is an update the
user never learns of. The Content Security Policy has no `unsafe-inline`, which
forbids htmx's inline event attributes and its injected indicator stylesheet —
the latter is disabled by configuration and the styles carried in ours.

## Invariants are expressed twice

Every invariant exists as a test and as a database constraint: CHECK
constraints, unique partial indexes, foreign keys, and triggers refusing
`UPDATE` and `DELETE` on ledger and audit tables.

A test tells you it broke in continuous integration. A constraint stops it in
production — including against a manual query someone runs at 11pm.

## Harnesses are inherited, not remembered

From Phase 1, a route registry is the single source of truth for the router, the
authorization matrix, the unauthenticated cache-leak sweep, and the
tenant-isolation sweep. A new route is covered by all three by construction.

The alternative — remembering to add a test — fails silently and looks like it
works, which is the worst available failure mode for exactly the checks whose
failure would end the business.
