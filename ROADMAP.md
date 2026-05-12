# x8 Roadmap

This document tracks the planned evolution of x8. Items in earlier
milestones are firmer than later ones; everything is subject to change
as we learn what the runtime is actually used for.

**Guiding principles**

1. **Stay small.** Every feature must justify the binary-size and
   build-time cost it adds. If a feature requires pulling in a heavy
   crate, it goes behind a Cargo feature flag.
2. **Pure Rust by default.** No C/C++ deps in the default build. V8
   and other native engines are opt-in only.
3. **Familiar over novel.** Where a Web/Node/Deno API exists, follow
   it. Invent only where there is no prior art.
4. **Embeddable first.** x8 is a library that ships a CLI, not a CLI
   that happens to expose a library. The public Rust API matters.

---

## v1.0.0 — Initial release ✅

**Status:** Released.

The minimum viable JavaScript runtime.

- [x] Script file execution (`x8 file.js`)
- [x] Inline evaluation (`x8 -e <code>`)
- [x] Interactive REPL with line editing and history
- [x] `console.log` / `error` / `warn` / `info` / `debug`
- [x] Synchronous `readFile(path)` / `writeFile(path, content)`
- [x] `args` array, `exit(code)`
- [x] `x8.version`, `x8.name` namespace
- [x] MIT license, English README

---

## v1.1.0 — Async I/O and `fetch` ✅

**Status:** Released.

### Delivered

- [x] Embedded `tokio` runtime, driven from `main`
- [x] Microtask queue bridged to Boa's job queue
- [x] `Promise`-returning native functions (`fetch`)
- [x] **Timers**: `setTimeout`, `setInterval`, `clearTimeout`,
      `clearInterval`, `queueMicrotask`
- [x] **`fetch`** (subset of the WHATWG Fetch spec)
  - `fetch(url)` and `fetch(url, { method, headers, body })`
  - `Response` with `.text()`, `.json()`, `.ok`, `.status`,
    `.statusText`, `.url`, `.headers`
  - Full body buffering (no streaming bodies yet)
- [x] `readFileSync` / `writeFileSync` aliases for Node compat

### Deferred to later versions

- `AbortController` / `AbortSignal` — v1.5 (ties into permissions)
- Streaming bodies — v1.4 (after Workers)
- Top-level `await` — depends on v1.3 module support
- Parallel future execution — currently sequential `block_on`

### Open questions

- Do we use `reqwest` (heavier, batteries-included) or build directly
  on `hyper` + `rustls` for smaller binary size?
- How much of the Fetch spec is worth implementing in v1.1 vs
  deferring to v1.3?

### Risks

- `tokio` plus TLS roughly doubles the binary size. We may need a
  `--no-default-features` build for users who don't need networking.

---

## v1.2.0 — TypeScript and JSX ✅

**Status:** Released.

### Delivered

- [x] **TypeScript** transpilation via [oxc](https://oxc.rs) (chosen
      over SWC for smaller dep tree and faster builds)
- [x] **JSX/TSX** transformation in classic mode with configurable
      pragma (`h` / `Fragment` by default)
- [x] Auto-detect based on file extension: `.ts`, `.tsx`, `.jsx`,
      `.mts`, `.cts`
- [x] **Type stripping** — interfaces, type aliases, generics, return
      types, parameter annotations
- [x] **Enums and classes** with private fields work

### Deferred to later versions

- Source maps for error stack traces — v2.0
- `--check` flag for type-checking — explicit non-goal
- Automatic JSX runtime (requires module support) — v1.3

---

## v1.3.0 — ES modules and HTTP imports ✅

**Status:** Released.

### Delivered

- [x] **Static `import` / `export`** via a custom `ModuleLoader`
- [x] **Dynamic `import()`** returning a Promise
- [x] **Top-level `await`** in module-mode files
- [x] **Relative imports**: `./foo.js`, `../bar/baz.ts`
- [x] **HTTP(S) imports** with on-disk cache at `~/.cache/x8/deps/`
      (override with `X8_CACHE` env var)
- [x] **Auto-transpile** TypeScript when imported from another file
- [x] Module-mode detection by extension (`.mjs`, `.mts`, `.ts`,
      `.tsx`, `.jsx`, `.cts`)

### Deferred to later versions

- Import maps (`x8.json`) — v2.x
- Integrity lockfile (`x8.lock`) — v2.x
- `x8 cache <url>` subcommand — v2.0
- npm/`node_modules` resolution — explicit non-goal for now

---

## v1.4.0 — Concurrency and workers ✅

**Status:** Released.

### Delivered

- [x] **`Worker(scriptPath)`** spawns a script on a dedicated OS
      thread with its own Boa context and tokio runtime
- [x] **`worker.postMessage(str)`** / **`worker.onmessage = fn`** for
      bidirectional messaging
- [x] **`self.postMessage(str)`** / **`self.onmessage = fn`** inside
      worker scripts
- [x] **`worker.terminate()`** to stop a worker explicitly
- [x] **`worker.onerror = fn`** for runtime errors from workers
- [x] Worker scripts can `import` modules and use `fetch`,
      independently of the main thread

### Deferred

- `new Worker(...)` constructor form — currently call without `new`
  (Boa native constructor wiring is non-trivial). Functionally
  equivalent.
- Structured cloning — currently messages are strings only. Pass
  `JSON.stringify(obj)` and `JSON.parse(msg)` to send objects.
- `MessageChannel`, `BroadcastChannel`, `SharedArrayBuffer`, `Atomics`
  — v2.x
- Parallel `Promise.all` over fetches (still sequential `block_on`)

---

## v1.5.0 — Permissions ✅

**Status:** Released.

### Delivered

- [x] `--allow-read` / `--deny-read` (gates `readFile`,
      `readFileSync`, file imports)
- [x] `--allow-write` / `--deny-write` (gates `writeFile`,
      `writeFileSync`)
- [x] `--allow-net` / `--deny-net` (gates `fetch`, HTTP imports)
- [x] `--allow-env` / `--deny-env` (reserved — no env API yet)
- [x] `--allow-run` / `--deny-run` (gates `Worker(...)`)
- [x] `--allow-all` / `--deny-all` (bulk operations)
- [x] Permissions inherited by workers from the spawning context
- [x] Default-allow in v1.x (backwards compatible); allow-only flags
      are no-ops to support forward-compatible scripts

### Deferred to v2.0

- Default-deny posture (breaking change — see v2.0)
- Path/host scoping: `--allow-read=/srv,/etc` — currently all-or-none
- Runtime `Permissions.request()` API
- `--audit` flag

---

## v1.6.0 — Embedding and FFI ✅

**Status:** Released.

### Delivered

- [x] Split crate into a `lib` + `bin` layout
- [x] `pub fn x8::run_cli(args: Vec<String>) -> ExitCode` for
      embedding the CLI behavior
- [x] Public `x8::Permissions` struct with `all_allowed()` /
      `all_denied()` constructors
- [x] Public API documented with rustdoc examples

### Deferred to v2.0+

- A higher-level `x8::Runtime` with structured `eval()` returning
  serializable values
- Crate published to crates.io (needs an API token to release)
- Native bindings for registering Rust functions as JS globals from
  outside the crate
- WASI plugins

---

## v2.0.0 — Production readiness ✅

**Status:** Released.

### Delivered

- [x] **Default-deny permissions** — `--allow-*` flags now required
      to use I/O, network, or workers (breaking change from v1.x)
- [x] **`x8 test`** — discovers `*.test.{ts,js}` and `*.spec.{ts,js}`
      recursively, runs them, reports pass/fail counts
- [x] **`x8 fmt`** — re-emits source via oxc Codegen, with
      `--write` to overwrite in place
- [x] **Test globals**: `test(name, fn)`, `assert(value, msg?)`,
      `assertEq(a, b)`
- [x] **Library/binary split** (v1.6 carryover): `x8::run_cli`,
      public `Permissions`

### Deferred — beyond v2.0

These items from the original v2.0 plan were honest stretches; v2.x
milestones can pick them up:

- Optional V8 backend behind `--features v8` (large undertaking)
- Snapshots for sub-millisecond cold start (depends on Boa snapshots)
- Single-file binaries via `x8 compile` (à la Bun) — requires a
  self-extracting binary format
- Pre-built release binaries on GitHub Releases (needs CI setup)
- Crates.io publication (needs API token)
- Source maps for v1.2 transpilation

---

## Beyond v2.0 — Exploratory

Ideas we are interested in but not ready to commit to:

- **AOT compilation** of JS to native via Cranelift (not just AOT
  bytecode like Boa already does — actual machine code).
- **Edge-runtime mode**: V8 Isolates–style multi-tenant sandboxing
  for serverless platforms.
- **First-class AI**: `import model from "ollama:llama3"`, vector
  search built into the standard library.
- **Deterministic execution mode**: reproducible runs for testing
  and replay debugging.
- **Capability-based modules**: each `import` declares the permissions
  it needs; users review and grant on install.

These are conversation starters, not promises.

---

## How to influence the roadmap

- **Open an issue** describing your use case. We weight roadmap
  decisions heavily toward concrete user needs.
- **Send a PR** for small items that aren't blocked on design.
- **Sponsor a milestone** — if you depend on a feature commercially,
  funded development can accelerate it.

The roadmap is intentionally aggressive on scope but conservative on
dates. We will ship milestones when they are good, not when a
calendar says they should be done.
