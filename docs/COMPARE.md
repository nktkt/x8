# x8 vs Node.js, Deno, and Bun

x8 is not trying to be faster than V8. It can't be — it runs on Boa, a pure-Rust interpreter with no JIT, and is meaningfully slower than any production-grade JavaScript engine on CPU-bound work. What x8 offers instead is minimalism (a ~10 MB binary with no C++ dependencies), embeddability (drops into any Rust project as a crate), and a pure-Rust trust boundary that's attractive for sandboxing untrusted scripts. If you need raw throughput, pick Node, Deno, or Bun. If you need a small, auditable JS runtime written entirely in a memory-safe language, keep reading.

## Feature comparison

| Feature | x8 (v2.0) | Node.js | Deno | Bun |
| --- | --- | --- | --- | --- |
| Engine | Boa (Rust, no JIT) | V8 (JIT) | V8 (JIT) | JavaScriptCore (JIT) |
| TypeScript | yes (strip only, no checking) | no (loader needed) | yes (type-checked) | yes (strip, optional check) |
| JSX | yes (classic, h/Fragment) | no (loader needed) | yes | yes |
| ES Modules | yes | yes | yes | yes |
| HTTP imports | yes (cached to ~/.cache/x8/deps) | no | yes | partial (via loader) |
| npm / node_modules | no | yes | yes (compat layer) | yes (native) |
| fetch | partial (no streaming) | yes | yes | yes |
| Web Workers | partial (string messages only) | yes (worker_threads) | yes | yes |
| SharedArrayBuffer / Atomics | no | yes | yes | yes |
| Permissions model | yes (default-deny, allow/deny flags) | no | yes (allow flags) | no |
| Test runner | yes (`x8 test`) | yes (`node --test`) | yes (`deno test`) | yes (`bun test`) |
| Formatter | yes (`x8 fmt`) | no | yes (`deno fmt`) | no |
| Compile to single binary | planned (v2.1) | no (pkg/nexe third-party) | yes (`deno compile`) | yes (`bun build --compile`) |
| Snapshots | no | yes | yes | yes |
| Native addons | no | yes (N-API) | yes (FFI) | yes (N-API, FFI) |
| Binary size | ~10 MB | ~90 MB | ~100 MB | ~90 MB |
| Startup time | very fast (no JIT warmup) | fast | fast | fast |
| Throughput (CPU-bound) | significantly slower | high | high | very high |
| Platform support | anywhere Rust compiles | broad | broad | limited (no Windows native) |
| License | MIT | MIT | MIT | MIT |
| Repository | github.com/nktkt/x8 | github.com/nodejs/node | github.com/denoland/deno | github.com/oven-sh/bun |

## When to pick x8

- **Embedding JS in a Rust application.** x8 is a crate. There's no FFI to V8, no separate runtime to ship, no C++ toolchain on the build host. If your host program is already Rust, x8 keeps the entire trust boundary in safe Rust.
- **Running untrusted scripts in a sandbox.** Default-deny permissions plus a memory-safe interpreter means a malicious script can't reach the network, disk, or environment without an explicit flag, and can't exploit a JIT bug because there is no JIT.
- **Small CI scripts and tooling.** A 10 MB binary that starts instantly, supports TypeScript and JSX out of the box, and has a built-in test runner and formatter is convenient when you don't want a 100 MB runtime or a `node_modules` directory in your CI image.
- **Reproducible, dependency-light glue code.** HTTP imports plus content-addressed caching in `~/.cache/x8/deps` mean a script's dependencies are pinned by URL with no package manager state to manage.

## When NOT to pick x8

- **Production HTTP servers under real load.** Boa is an interpreter. Per-request throughput is going to be a fraction of what Node, Deno, or Bun deliver. Use one of those.
- **Anything CPU-bound at scale.** Image processing, parsers, crypto, number crunching, large JSON transforms — all will be much slower than on a JIT engine. There is no SharedArrayBuffer/Atomics either, so you can't parallelize tight loops across workers.
- **Anything that needs npm.** x8 does not resolve `node_modules` and does not implement the Node built-in modules. Any meaningful slice of the npm ecosystem is out of reach. If you depend on Express, Next, Prisma, etc., this is the wrong runtime.
- **Streaming I/O or large payloads over `fetch`.** The fetch implementation is a subset: text and JSON bodies, headers, method, body — no `ReadableStream`, no backpressure. Don't proxy large uploads through x8.
- **Workloads needing native addons or FFI.** x8 has neither.
- **Multi-threaded shared-memory workloads.** Workers exist but only exchange string messages. There is no shared memory.

## Future

Compile-to-single-binary is planned for v2.1. Streaming fetch, richer Worker messaging, and a broader Web API surface are tracked in [ROADMAP.md](../ROADMAP.md). x8 is not going to grow a JIT or an npm compatibility layer — those goals belong to other runtimes and trying to match them would defeat the point of x8.
