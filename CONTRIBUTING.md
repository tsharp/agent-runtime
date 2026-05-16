# Contributing to agent-runtime

Thanks for taking the time to contribute! This project is maintained by a single person, so community help is genuinely appreciated.

## Getting Started

1. Fork the repository and clone your fork.
2. Make sure you have the Rust toolchain installed (see [rust-toolchain.toml](rust-toolchain.toml) for the required version).
3. Build and run the tests to confirm everything works:

```powershell
cargo build
cargo test
```

## How to Contribute

### Bug Reports

Open a [GitHub issue](../../issues/new) with:
- A clear description of the problem
- Steps to reproduce
- Expected vs. actual behavior
- Rust version and OS

### Feature Requests

Open an issue first to discuss the idea before writing code. This avoids wasted effort if the feature doesn't align with the project direction.

### Pull Requests

1. Open an issue or comment on an existing one so we can discuss the approach.
2. Create a branch from `main` with a descriptive name (e.g. `fix/tool-loop-timeout`, `feat/openai-provider`).
3. Keep changes focused, one concern per PR.
4. Run the full test suite and lints before submitting:

```powershell
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

5. Write or update tests for any changed behavior.
6. Open the PR against `main` with a clear description of what changed and why.

## Commit Message Format

This project follows the Conventional Commits Specification v1.0.0.

Reference:
https://www.conventionalcommits.org/en/v1.0.0/#specification

Use this format for commit messages:

```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Common types used in this repository:
- `feat` for new features
- `fix` for bug fixes
- `docs` for documentation changes
- `chore` for maintenance tasks
- `refactor` for code changes that do not add features or fix bugs
- `test` for adding or updating tests
- `perf` for performance improvements
- `ci` for CI pipeline or automation changes

Examples:
- `docs: add security and contributing guides`
- `fix(config): handle missing yaml file path`
- `feat(runtime): add workflow retry strategy options`

## Security Vulnerabilities

Please do **not** open public issues for security bugs. See [SECURITY.md](SECURITY.md) for how to report them privately.

## Code Style

- Follow standard Rust idioms and the project's existing patterns.
- Run `cargo fmt` before committing.
- Clippy warnings are treated as errors, fix them rather than suppressing them unless there is a good reason.

## License

By contributing, you agree that your contributions will be licensed under the same terms as this project: [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at the user's option.
