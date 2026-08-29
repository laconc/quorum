## Summary

<!-- What changed, and why. One paragraph. -->

## Design-document traceability

<!-- Which sections of the design document this implements, and any amendment
     it required. If it changes behaviour the document does not describe, the
     document is amended first — link that change here. -->

- Sections:
- Amendments:

## Testing

<!-- What proves this works. Name the tests, not the intention. If a harness
     was extended, say which. -->

- [ ] `make check` green
- [ ] New behaviour has a test that would fail without it
- [ ] Any deferred work has an `#[ignore = "reason"]` test carrying its contract

## Screenshots

<!-- Required when this change touches the frontend: anything under
     app/crates/web/templates/, app/crates/web/static/, or
     app/crates/web/src/view/. CI checks this from the diff.

     Run `make pr-screenshots` to generate this section.

     If nothing frontend changed, replace this comment with exactly:
     N/A — no frontend change -->

## Risk

<!-- What could this break, and what would catch it? For anything touching
     tenant isolation, authorization, money, votes, or the audit chain, say so
     explicitly — those carry a different review bar. -->
