# x8 FAQ

Frequently asked questions about [x8](https://github.com/nktkt/x8), a minimal JavaScript runtime written in pure Rust.

## General

### Why does x8 exist when Node/Deno/Bun already do this?

x8 is not trying to replace Node, Deno, or Bun. It targets a different niche: an embeddable, single-binary runtime (~10 MB, no native deps) you can drop into a Rust application or a constrained environment without pulling in V8 and its C++ toolchain. If you need raw throughput for a server, use Node/Deno/Bun. If you need a small, pure-Rust JS runtime you can ship or embed cleanly, that is what x8 is for.

### How fast is it compared to Node, Deno, and Bun?

Honestly: a lot slower. x8 is built on [Boa](https://github.com/boa-dev/boa), which is a pure-Rust interpreter with no JIT, so on most workloads you should expect roughly **10-50x slower** than V8-based runtimes. For scripting, glue code, config evaluation, plugins, and lightweight tooling this is usually fine; for CPU-bound JS or hot HTTP servers it is not the right tool.

### Why no V8? Wouldn't that be faster?

Yes, V8 would be dramatically faster. The reason x8 does not use it is that V8 brings in a C++ build toolchain, platform-specific quirks, and pushes the binary past 80 MB. Pure Rust is the design goal: one `cargo build`, no native dependencies, a small static binary, and trivial cross-compilation. That tradeoff is the whole point of the project.

### Is x8 trying to replace Node, Deno, or Bun?

No. Those runtimes are mature, fast, and have huge ecosystems. x8 is intentionally minimal and aimed at the embeddable / small-footprint slot that the bigger runtimes do not really serve.

## Packages and modules

### Does x8 support npm packages?

Not in v2.0. x8 supports HTTP imports (`import x from "https://..."`) and relative file imports (`./foo.ts`), but there is no `node_modules` resolution, no `package.json` lookup, and no npm registry client. Many pure-JS/TS libraries that publish ESM builds to a CDN like esm.sh or jsr.io will work; libraries that depend on Node built-ins or native addons will not.

### Can I import TypeScript and JSX directly?

Yes. x8 uses [oxc](https://github.com/oxc-project/oxc) to strip TS types and transform JSX at load time, so `.ts`, `.tsx`, and `.jsx` files run without a separate build step. There is no type checking — types are erased, not verified.

### Why TypeScript via oxc, not SWC?

Both work, but oxc has a smaller dependency tree, faster compile times for x8 itself, and similar correctness for the type-stripping and JSX transforms x8 actually uses. SWC is great but pulls in more than x8 needs.

### Are there source maps?

Not yet. Source map generation for the TS/JSX transform is on the v2.x roadmap. For now, stack traces point at the transformed code, which is usually close enough to the original for simple cases but can be confusing for heavy JSX.

## Permissions

### Why does my script error with "permission denied"?

Since v2.0, x8 is **default-deny** for capabilities like file system access, network, and environment variables — the same model as Deno. You need to opt in explicitly with flags such as `--allow-read`, `--allow-write`, `--allow-net`, `--allow-env`, or `--allow-all` for everything. This is a deliberate breaking change from v1.x.

### Why are the example fetch calls failing without --allow-net?

Same reason: `fetch` is gated behind the network permission as of v2.0. Run your script with `--allow-net` (or scope it, e.g. `--allow-net=api.github.com`) and it will work. Older examples written for v1.x assumed permissions were granted by default.

## Web APIs

### Does x8 support browser APIs like `document` or `window`?

No. x8 is a server-side runtime. It implements `fetch`, `URL`, `TextEncoder`/`TextDecoder`, `console`, timers, and a basic Web Worker, but there is no DOM, no `window`, no `document`, and no rendering. Code that assumes a browser environment will not run.

### Can I embed x8 in my Rust application?

Yes. The simplest entry point today is `x8::run_cli`, which lets you invoke x8 as if from the command line from inside a Rust process. A richer embedding API (custom module loaders, host functions, permission hooks) is planned for v2.x.

## Workers

### How do I share state between Web Workers?

You don't share state directly — only strings via `postMessage`. There is no `SharedArrayBuffer`, no transferable objects, and no structured clone of arbitrary values. The idiomatic pattern is `worker.postMessage(JSON.stringify(data))` on the send side and `JSON.parse(e.data)` on the receive side.

### Does `Worker(...)` use `new` or just call it?

Just call it, no `new`: `const w = Worker("./worker.ts")`. Wiring up the proper `new Worker(...)` constructor form is on the v2.x todo list; for now the function-call form is what works.
