# Operations

Running it, and what to do when it breaks.

Most of this arrives in Phase 7 with the production deployment. What is recorded
now is the part that constrains earlier work — because getting it wrong later is
unrecoverable rather than inconvenient.

## The split-brain rule

**Deployment is stop-then-start, never start-then-stop.**

Two processes writing the same SQLite files, or two replication processes
writing the same object-storage prefix, produces divergence that cannot be
resolved afterwards — there is no correct merge of two histories that both
claim to be authoritative. The brief downtime is not a problem for this
application. Split-brain is.

This is the first step of the instance-loss runbook, and instance replacement
follows the same rule with an explicit, scripted verification that the old
instance's application and replication processes are stopped before the new one
starts. A scripted check with a recorded result, not a line in a document
someone reads at 2am.

## Accepted risks

Carried forward from the design document's own record, restated here because an
accepted risk that is not visible is an unmanaged one.

1. **A catastrophic instance loss forfeits the unreplicated window.**
   Replication is asynchronous — typically a sub-second window, but real. It
   also faithfully replicates logical corruption. Accepted by explicit
   instruction. **The weekly restore drill is what converts this from an
   assumption into a verified property**, which is why the drill lands in Phase 1
   rather than at deployment: an unverified backup is a belief, not a control.
2. **The operator can rewrite history**, detectably to anyone who checks the
   anchors, and undetectably to anyone who does not.
3. **Two colluding board members can compromise an election.**
4. **Email compromise is account takeover** for anyone using only email codes.
5. **Virus scanning is weak against targeted malware.**
6. **A single instance is a single point of failure.** Downtime is measured in
   minutes and is acceptable here.

## Local development

```sh
make run           # seeds a database and serves on :8080
make run-frozen    # the same, with the clock frozen as the screenshots see it
make check         # the done-gate
```

Seed databases are written to `.data/` and the end-to-end suite uses
`.e2e-data/`. Both are disposable; delete them and re-run.

## Thresholds that force re-architecture

Instrumented in Phase 7 so they are noticed before they hurt. Acting on them
early would be speculative; being surprised by them would not be acceptable.

| Threshold | Consequence |
|---|---|
| More than ~300 associations | Reconsider file-per-association |
| Sustained concurrency above 50 | Separate the worker |
| Cross-association reporting becomes a product need | Add an aggregation store |
| Any association above 2,000 lots | Revisit assessment runs and notification batching |
| Multiple jurisdictions at scale | Jurisdiction configuration becomes a subsystem |
