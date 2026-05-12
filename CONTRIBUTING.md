# Contributing to x8

Thanks for your interest in x8, a minimal JavaScript runtime in Rust (Boa + oxc).
This is a small project with a single maintainer, so please keep changes focused
and discuss anything substantial before writing a lot of code.

## Setup

You'll need a recent stable Rust toolchain.

```sh
git clone <repo-url> x8
cd x8
cargo build
```

Optionally install the binary to your `PATH`:

```sh
cargo install --path .
```

## Running tests

The test runner is x8 itself. Build first, then point it at a `.test.ts` file:

```sh
cargo build
./target/debug/x8 --allow-read test examples/sample.test.ts
```

Add new tests under `examples/` (or wherever fits) and run them the same way.
If you touch the test runner internals, run the existing suite to confirm
nothing regressed.

## Code style

Before opening a PR:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Both must pass cleanly. The codebase lives in a single `src/lib.rs` (~1600
lines) plus a thin `src/main.rs`; keep new code there unless there's a clear
reason to split a module out.

## Pull requests

- **Small fixes** (typos, obvious bugs, doc tweaks, small refactors): open a
  PR directly.
- **Anything larger** (new features, behavior changes, new dependencies):
  please open an issue first so we can agree on the shape before you spend
  time on it.
- Keep PRs scoped to one thing. Split unrelated changes into separate PRs.
- Include a short description of *why*, not just *what*.

## Commit messages

- Short imperative subject line (e.g. `add JSX fragment support`, not
  `added JSX fragments`).
- Wrap the body at ~72 chars if you need one.
- The project uses `Co-Authored-By:` trailers when work is collaborative
  (including with AI assistants) — feel free to add them.

## Areas where contributions are welcome

These are good places to jump in:

- Better error messages (source locations, friendlier stack traces).
- More built-in tests covering edge cases of the runtime.
- JSX automatic runtime support.
- Source map support.
- npm compatibility improvements.
- Path-scoped permissions (e.g. `--allow-read=./data`).
- A V8 backend as an alternative to Boa.

If you want to tackle one of these, an issue confirming the approach is
appreciated but not required for a first cut.

## Areas that need discussion first

Please open an issue before working on any of these — they're not off-limits,
but they need design agreement up front:

- **Major dependency additions.** The dep tree is intentionally small.
- **Breaking API changes** to the public CLI flags or the runtime's
  exposed JS APIs.
- **Removing or weakening the `Permissions` model.** Permissions are a
  core design constraint, not an implementation detail.

## License

By contributing, you agree that your contributions are licensed under the
MIT license, same as the rest of the project (see `LICENSE`).
