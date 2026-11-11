# Contributing

Bug reports, false positives from `nab lint`, and rewrites that dropped a fact are the most useful things you can send. Open an issue with the input text (or a synthetic stand-in), the command you ran, and what came back.

## Pull requests

1. Fork, branch from `main`, keep the change to one topic.
2. Run the same checks CI runs:

   ```sh
   cargo fmt --all --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo test --all-targets --locked
   ```

3. Add or update a test for any behaviour change. Prompt changes need a before/after from `benches/prompt-lab` (see its README); a lower detector score that loses facts is a regression.
4. Note user-visible changes under a new heading in `docs/en/CHANGELOG.md`.

The maintainer reviews and merges. CI on pull requests from new contributors starts after approval.

## Scope

- Facts first: no change may let a name, number, date, amount, URL or code span go missing silently.
- The product is the Rust binary. Python lives only in `benches/` as development tooling.
- No claims that output is "undetectable". Report measured numbers with the method.

## Licence

Contributions are dual-licensed under MIT and Apache-2.0, the same as the project.
