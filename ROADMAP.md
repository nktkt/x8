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

## v1.3.0 — ES modules and HTTP imports

**Theme:** Make code shareable and composable.

### Deliverables

- [ ] **Static `import` / `export`** support beyond what Boa already
      provides — wire up a real module loader
- [ ] **Dynamic `import()`** returning a Promise
- [ ] **Relative imports**: `./foo.js`, `../bar/baz.ts`
- [ ] **HTTP imports**: `import x from "https://esm.sh/lodash"` with
      an on-disk cache (`~/.cache/x8/deps/`)
- [ ] **Import maps** (`x8.json` or `import_map.json`) for aliasing
      and pinning versions
- [ ] **Integrity locking** (`x8.lock`) — content hashes for every
      remote import, verified on subsequent runs
- [ ] **`x8 cache <url>`** subcommand to pre-fetch dependencies
- [ ] Permissions: HTTP imports gated behind `--allow-net` (off by
      default once permissions land — see v1.5)

### Open questions

- npm compatibility: do we resolve `node_modules` like Bun, or stay
  HTTP-only like Deno's original design? Probably **both, opt-in**.

---

## v1.4.0 — Concurrency and workers

**Theme:** Use multiple cores without leaving the runtime.

### Deliverables

- [ ] **Web Workers** API — spawn a worker from a script URL, get a
      `Worker` instance with `postMessage` / `onmessage`
- [ ] **`MessageChannel`** and **`BroadcastChannel`**
- [ ] **`structuredClone`** for cross-worker data transfer
- [ ] **`SharedArrayBuffer`** and `Atomics` (Boa support permitting)
- [ ] Each worker runs in its own `tokio` runtime on a dedicated
      thread
- [ ] CPU-bound `Promise.all` actually parallelizes across cores when
      operands are worker calls

### Risks

- Boa is not currently thread-safe across `Context` instances. We
  will need one `Context` per worker, which is the natural design but
  has implications for shared object identity.

---

## v1.5.0 — Permissions

**Theme:** Make x8 safe to run untrusted code in.

Inspired by Deno's permission model, but more granular.

### Deliverables

- [ ] `--allow-read[=<paths>]` / `--deny-read`
- [ ] `--allow-write[=<paths>]` / `--deny-write`
- [ ] `--allow-net[=<hosts>]` / `--deny-net`
- [ ] `--allow-env[=<vars>]` / `--deny-env`
- [ ] `--allow-run[=<cmds>]` / `--deny-run`
- [ ] `--allow-all` (escape hatch, off by default)
- [ ] Runtime permission API: `Permissions.request({ name: "net",
      host: "..." })` returning a Promise
- [ ] Audit log via `--audit` flag (writes every permission check to
      stderr)

### Notes

- The permission model becomes the **default-deny** posture in v2.0;
  in v1.5 it ships as opt-in to preserve backwards compatibility.

---

## v1.6.0 — Embedding and FFI

**Theme:** x8 as a Rust library.

### Deliverables

- [ ] Stable public Rust API (`x8::Runtime`, `x8::Module`, …)
- [ ] Crate published to crates.io as `x8-runtime`
- [ ] **Native bindings**: register Rust functions/structs as JS
      globals from outside the crate (today this requires forking)
- [ ] **WASI plugins**: load WASM modules as scriptable extensions —
      a safer alternative to FFI for sandboxed plugin systems
- [ ] Documentation site with examples

---

## v2.0.0 — Pluggable engine and stable API

**Theme:** Production readiness.

### Deliverables

- [ ] **Default-deny permissions** (breaking change from v1.x)
- [ ] **Optional V8 backend** behind `--features v8` for hot paths —
      same JS APIs, different engine
- [ ] **Stable embedding API** with a semver guarantee
- [ ] **Snapshots** — pre-compile a startup heap so cold start is
      ~milliseconds
- [ ] **Built-in test runner**: `x8 test`
- [ ] **Built-in formatter**: `x8 fmt`
- [ ] **Single-file binaries**: `x8 compile script.js -o myapp` (à la
      Bun)
- [ ] Pre-built binaries for macOS, Linux, Windows on GitHub Releases

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
