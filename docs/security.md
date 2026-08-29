# Security posture

What is controlled, what is claimed, and — the part that matters most — what is
not.

## Threat model

This platform holds statutory records, money, and votes. Three adversaries
shape the design, and the second is the one that surprises people:

1. **An external attacker.** Ordinary web-application threats, plus phishing
   aimed at an audience trained by decades of scams and not by us.
2. **A board member.** Board and treasurer fraud is a documented, recurring
   pattern in this sector with a median detection time of roughly a year. The
   board is a fraud risk, not just a user — designed against, not as an insult
   to any particular board.
3. **The operator.** We control the database, the code, and the sending of the
   daily digest. Honesty about what that means is a design requirement.

## Controls in place (Phase 0)

| Control | Mechanism |
|---|---|
| Tenant isolation | One file per association; `app-db` is the only crate with a database handle, proven by test |
| `ATTACH` | Disabled by setting the attached-database limit to zero, on every connection |
| Extension loading | Not compiled in — the `load_extension` feature is deliberately absent |
| Defensive mode | On, refusing direct writes to shadow tables and the schema |
| Path traversal | Association identifiers are parsed against a narrow character set; traversal, absolute paths, null bytes, and Unicode tricks are excluded by construction |
| Content Security Policy | `default-src 'none'` with explicit allowances; no `unsafe-inline`, no `unsafe-eval` |
| Transport | `Strict-Transport-Security` for a year, subdomains included, asking for preload |
| Caching | `private, no-store` by default on every response; caching is opt-in per route |
| Static assets | Fingerprinted URLs, immutable caching, vendored — no third-party script origin |
| Hashing | SHA-384 throughout, with every digest carrying its algorithm |
| Message authentication | HMAC-SHA-384, compared in constant time |
| Tokens | 256 bits from the operating system's entropy source; redacted in `Debug` |
| Canonical encoding | Deterministic and property-tested; floats refused |
| Supply chain | `cargo deny` on licences and advisories; every CI action pinned by hash |

Later phases add authorization matrices, session and step-up authentication,
the audit chain with external anchoring, the upload pipeline, and the recovery
authority model.

## Post-quantum posture

Two clocks, and conflating them produces bad decisions.

**Confidentiality is urgent.** An adversary recording traffic today can decrypt
it when a cryptographically relevant quantum computer exists. For a platform
holding seven-year records, that is a live threat now, and key agreement is the
surface it applies to.

**Integrity is urgent only for what must be verified later.** A sign-in
assertion matters for minutes; a signature on a certified election result or an
audit anchor matters for years.

| Surface | Choice |
|---|---|
| Audit chain | SHA-384 — Grover halves preimage strength, and a statutory chain cannot be cheaply rehashed |
| Signed URLs, email thread tokens | HMAC-SHA-384 — symmetric, already quantum-resistant |
| Session tokens, recovery codes | 256-bit random |
| Audit anchors, export bundles, certified tallies | ML-DSA-87 (FIPS 204), *Phase 1* |
| Encryption at rest | AES-256-GCM, *Phase 7* |
| Transport | Hybrid post-quantum key agreement where each hop offers it, *Phase 7* |

**The structural requirement matters more than any algorithm above.** All
hashing and signing goes through one crate, and every artifact stores the
algorithm that produced it. These choices will change within the retention
period of records being written today; what survives being wrong about all of
them is that a future migration can tell what produced each record.

## What is not post-quantum

Stated plainly, with what would change the answer.

- **WebAuthn and passkeys.** No standardised post-quantum algorithm exists,
  including for the passkeys that will be mandatory for platform
  administrators — the most privileged surface in the system. The residual risk
  is credential forgery requiring a quantum adversary *at authentication time*,
  not a later break of a recorded assertion. *Changes when:* a standardised
  post-quantum COSE algorithm ships with authenticator support.
- **RFC 3161 timestamps.** The third-party timestamping authority signs
  classically and we do not control it. Mitigated by signing the same daily root
  ourselves with ML-DSA-87: their token proves *time* to someone who does not
  trust us, ours proves *integrity* against a future adversary. Neither alone is
  sufficient.
- **Third-party interfaces.** The payment processor, the mail provider, and
  object storage negotiate their own transport. We take the best available and
  record what was agreed.

## What we do not claim

Overclaiming here would be its own kind of defect.

- **The audit log makes tampering detectable, not impossible.** Daily Merkle
  roots are externally timestamped, signed, published to board members, and
  written under separate credentials — which makes a rewrite detectable by
  parties the operator does not control. Detectability, not prevention. This is
  what associations are told, in these words.
- **Two colluding board members can compromise an election.** The two-person
  rule makes collusion *evidenced* — both identities recorded permanently,
  ballot images retained, per-channel counts published — not impossible. It is
  structural to volunteer governance.
- **Virus scanning is a backstop.** Re-encoding images and sanitising documents
  is the real defence; the scanner is weak against targeted malware.
- **Email compromise is account takeover for anyone using only email codes.**
  Mitigated by the 72-hour financial suspension and by step-up authentication,
  not removed. Passkeys are the real fix and are optional by design, because
  forcing them would strand people who cannot manage enrolment.
