## What and why

## How it was checked

- [ ] `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --all-targets --locked`
- [ ] Test added or updated for the behaviour change
- [ ] Prompt change: before/after from `benches/prompt-lab`, facts kept
- [ ] User-visible change noted in `docs/en/CHANGELOG.md`
