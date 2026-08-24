# Contributing

## Getting set up

```sh
cargo test
cargo clippy -- -D warnings
```

Rust 2021 edition. No dependencies — the parser and the rollup are both hand-rolled, and adding a crate should be a deliberate decision.

## Before opening a pull request

- The test command above passes
- New behaviour has a test alongside it
- Public functions carry a comment saying *why*, not restating the signature

## Commit messages

Explain why the change is needed. The diff already says what it does.
