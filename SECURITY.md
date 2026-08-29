# Security policy

## Reporting a vulnerability

Report suspected vulnerabilities privately through GitHub's
[private vulnerability reporting](https://github.com/laconc/quorum/security/advisories/new).
**Do not open a public issue or pull request** for a suspected vulnerability.

We aim to acknowledge a report within five business days and will keep you
informed as we confirm the issue, prepare a fix, and agree a disclosure
timeline.

## What we consider a vulnerability

This platform holds statutory records, money, and votes for community
associations, and it is designed on the premise that a board member is a
possible threat rather than only a user. A way to violate any of these is a
security issue:

- **Tenant isolation.** Any path by which a request scoped to one association
  reads or writes another's data. Physical file separation is the control; a way
  around it — `ATTACH`, a constructed path, a handle obtained outside the
  connection factory — is the highest-severity class here.
- **Authorization.** Any route reachable by a role that should not reach it, or
  any member-only content served to an unauthenticated request.
- **Audit integrity.** Any way to alter, delete, or reorder an audit entry
  without breaking the hash chain, or to produce a chain that verifies over
  altered content.
- **Ledger integrity.** Any way to create an unbalanced transaction, mutate a
  posted entry, or make an allocation that does not sum to its source amount.
- **Election integrity.** Any way to count a ballot verified by its own enterer,
  to count more ballots than eligible lots, to alter a certified tally, or to
  re-associate a ballot with the voter who cast it.
- **Authentication and recovery.** Any account takeover path, any bypass of
  step-up authentication or its transaction binding, or any way around the
  72-hour financial suspension that follows a recovery.
- **Hostile input.** Any input causing a panic, a hang, or unsoundness rather
  than a typed error — particularly in the upload pipeline.

## What we claim, and what we do not

Stated plainly, because overclaiming here would be its own kind of defect.

- **The audit log makes tampering detectable, not impossible.** An operator
  controls the database, the code, and the sending of the daily digest. Daily
  Merkle roots are externally timestamped, signed, published to board members,
  and written under separate credentials — which makes a rewrite detectable by
  parties the operator does not control. That is the claim. Detectability, not
  prevention.
- **Two colluding board members can compromise an election.** The two-person
  rule makes collusion evidenced — both identities recorded permanently, ballot
  images retained, per-channel counts published — not impossible.
- **Passkeys are not post-quantum.** WebAuthn has no standardised post-quantum
  algorithm, including for the mandatory platform-administrator passkeys.
- **Virus scanning is a backstop.** Re-encoding images and sanitising documents
  is the real defence against hostile uploads.

See `docs/security.md` for the full posture, including what is and is not
quantum-resistant and under what conditions each answer changes.
