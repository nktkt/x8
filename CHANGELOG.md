# Changelog

All notable changes to **x8**, a minimal JavaScript runtime written in pure Rust, are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v2.0.0] — 2026-05-12

### Changed (breaking)
- Default permission policy flipped from allow-all to **deny-all**; scripts using `readFile`, `writeFile`, `fetch`, or `Worker` must now opt in via explicit `--allow-*` flags.

### Added
- `x8 test [paths...]` subcommand that discovers `*.test.{js,ts}` and `*.spec.{js,ts}` files, runs them with `test()` / `assert()` / `assertEq()` globals, and reports pass/fail counts.
- `x8 fmt [--write] [paths...]` subcommand that re-emits source code through the oxc Codegen.

### Deferred
- V8 backend, startup snapshots, `x8 compile`, and pre-built binaries pushed to a future v2.x release.

## [v1.6.0] — 2026-05-12

### Changed
- Split the crate into a library (`src/lib.rs`) and a thin binary wrapper (`src/main.rs`) so x8 can be embedded in other Rust applications.

### Added
- Public `run_cli(args: Vec<String>) -> ExitCode` entry point that exposes full CLI behavior to embedders.
- `Permissions` struct is now public, with `all_allowed` / `all_denied` constructors.
- Crate-level rustdoc including an embedding usage example.

## [v1.5.0] — 2026-05-12

### Added
- Granular permission system with `--allow-read`, `--allow-write`, `--allow-net`, `--allow-env`, `--allow-run`, and matching `--deny-*` counterparts.
- Bulk `--allow-all` / `--deny-all` switches.
- Enforcement integrated into `readFile`, `writeFile`, `fetch`, HTTP module imports, and `Worker` spawn paths.
- Workers inherit the permission set of the spawning context.

### Notes
- v1.x remains default-allow for backwards compatibility; the default flips in v2.0.

## [v1.4.0] — 2026-05-12

### Added
- Basic Web Workers: `new Worker(scriptPath)` spawns a script on a dedicated OS thread, each with its own Boa context, tokio runtime, and module loader.
- Bidirectional string-based messaging through mpsc channels.
- `worker.postMessage`, `worker.onmessage`, `worker.onerror`, `worker.terminate` on the parent side.
- `self.postMessage` and `self.onmessage` inside worker scripts.
- Main event loop drains worker events and blocks while only workers remain alive.

### Notes
- Messages are strings; pass structured data via `JSON.stringify` / `JSON.parse`.

## [v1.3.0] — 2026-05-12

### Added
- Custom `ModuleLoader` supporting both `file://` and `https://` specifiers.
- Static `import` / `export`, dynamic `import()`, and top-level `await`.
- Automatic TypeScript transpilation for imported modules via oxc.
- HTTP(S) imports cached at `~/.cache/x8/deps/` (overridable with the `X8_CACHE` environment variable).
- Module-mode auto-detection for `.mjs`, `.mts`, `.ts`, `.tsx`, `.jsx`, and `.cts` files.

## [v1.2.0] — 2026-05-12

### Added
- Automatic transpilation of `.ts`, `.tsx`, `.jsx`, `.mts`, and `.cts` files prior to execution.
- TypeScript type stripping covering interfaces, type aliases, generics, enums, and class type annotations.
- JSX classic runtime with a user-defined `h` / `Fragment` pragma.

### Notes
- Built on oxc 0.130, chosen over SWC for a smaller dependency tree.

## [v1.1.0] — 2026-05-12

### Added
- `AsyncJobQueue` that bridges tokio futures into Boa's job queue, enabling async/await throughout the runtime.
- Timer primitives: `setTimeout`, `clearTimeout`, `setInterval`, `clearInterval`, `queueMicrotask`.
- `fetch(url, opts)` returning `Promise<Response>` with `text()` and `json()` body helpers.
- Response objects expose `ok`, `status`, `statusText`, `url`, and `headers`.
- `readFileSync` / `writeFileSync` aliases for Node.js compatibility.

## [v1.0.0] — 2026-05-12

### Added
- Initial release of x8: a minimal JavaScript runtime built on the Boa engine, shipped as a single static binary with no native dependencies.
- Run scripts from a file, evaluate inline with `-e`, or drop into an interactive REPL.
- Console API: `console.log`, `console.error`, `console.warn`, `console.info`, `console.debug`.
- Synchronous filesystem primitives: `readFile`, `writeFile`.
- Process globals: `args`, `exit(code)`, `x8.version`, `x8.name`.

[v2.0.0]: https://github.com/nktkt/x8/releases/tag/v2.0.0
[v1.6.0]: https://github.com/nktkt/x8/releases/tag/v1.6.0
[v1.5.0]: https://github.com/nktkt/x8/releases/tag/v1.5.0
[v1.4.0]: https://github.com/nktkt/x8/releases/tag/v1.4.0
[v1.3.0]: https://github.com/nktkt/x8/releases/tag/v1.3.0
[v1.2.0]: https://github.com/nktkt/x8/releases/tag/v1.2.0
[v1.1.0]: https://github.com/nktkt/x8/releases/tag/v1.1.0
[v1.0.0]: https://github.com/nktkt/x8/releases/tag/v1.0.0
