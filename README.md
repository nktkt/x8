# x8

A minimal, fast JavaScript runtime written in pure Rust.

`x8` is a small command-line JavaScript runtime built on top of the
[Boa](https://boajs.dev/) engine. It ships as a single binary with no
native dependencies, no V8, and no JIT — making it tiny to install and
trivial to embed.

## Why x8?

| | x8 | Node.js | Deno | Bun |
|---|---|---|---|---|
| Engine | Boa (pure Rust) | V8 (C++) | V8 (C++) | JavaScriptCore (C++) |
| Binary size | ~10 MB | ~80 MB | ~100 MB | ~90 MB |
| Native deps | none | libuv, OpenSSL, … | many | many |
| Build language | Rust | C++ | Rust | Zig |
| Goal | minimal & embeddable | full-featured | secure & standards | speed & DX |

x8 is **not** trying to replace Node, Deno, or Bun. It targets a
different niche: situations where you want a *tiny*, *self-contained*
JavaScript evaluator — for plugin systems, scripting layers, CTF
challenges, education, or just running quick snippets without the
heavyweight runtime tax.

## Features

- **Single static binary** — no `node_modules`, no shared libraries.
- **Pure-Rust JS engine** — built on Boa, fully memory-safe.
- **Familiar globals** — `console.log` family, `readFile`, `writeFile`,
  `args`, `exit`.
- **Three execution modes** — run a file, evaluate `-e <code>`, or drop
  into an interactive REPL.
- **Cross-platform** — anywhere Rust compiles (macOS, Linux, Windows).

## Installation

### From source

```sh
git clone https://github.com/nktkt/x8
cd x8
cargo build --release
# binary is at ./target/release/x8
```

### Cargo install

```sh
cargo install --path .
```

## Usage

### Run a script

```sh
x8 hello.js
```

```js
// hello.js
console.log("Hello, x8!");
```

### Evaluate inline code

```sh
x8 -e "console.log([1,2,3].map(x => x * x))"
# 1,4,9
```

### Interactive REPL

```sh
x8
# x8 1.0.0 REPL — type .exit or Ctrl-D to quit
# x8> 1 + 2
# 3
# x8> [1,2,3].reduce((a,b) => a+b)
# 6
```

### Pass arguments to the script

```sh
x8 myscript.js -- foo bar baz
```

```js
// myscript.js
console.log("got:", args);
// got: foo,bar,baz
```

## Built-in globals

| Name | Type | Description |
|---|---|---|
| `console.log(...args)` | function | Print to stdout (space-separated). |
| `console.error(...args)` | function | Print to stderr. |
| `console.warn(...args)` | function | Print to stderr. |
| `console.info(...args)` | function | Print to stdout. |
| `console.debug(...args)` | function | Print to stderr. |
| `readFile(path)` | function | Read a UTF-8 file synchronously. Returns the contents as a string. |
| `writeFile(path, content)` | function | Write a string to disk synchronously. |
| `exit(code?)` | function | Terminate the process with an optional exit code (defaults to `0`). |
| `args` | array | Arguments passed to the script. |
| `x8.version` | string | Runtime version (e.g. `"1.0.0"`). |
| `x8.name` | string | Runtime name (`"x8"`). |

## Examples

### Read a file and transform it

```js
const text = readFile("input.txt");
const upper = text.toUpperCase();
writeFile("output.txt", upper);
console.log("wrote", upper.length, "bytes");
```

### Compute Fibonacci

```js
const fib = n => n < 2 ? n : fib(n - 1) + fib(n - 2);
for (let i = 0; i < 10; i++) {
  console.log(`fib(${i}) =`, fib(i));
}
```

### Read CLI args

```js
if (args.length === 0) {
  console.error("usage: x8 greet.js -- <name>");
  exit(1);
}
console.log(`Hello, ${args[0]}!`);
```

## Language support

x8 inherits its JavaScript support from the Boa engine, which targets
the ECMAScript specification. Most modern syntax is supported:

- ES2015+ (let/const, arrow functions, classes, destructuring, spread)
- Template literals, default parameters, rest parameters
- Promises, generators, iterators
- `Map`, `Set`, `WeakMap`, `WeakSet`
- Regular expressions
- Modules (Boa's module loader; see Boa documentation)

For complete details on language coverage, see the
[Boa conformance report](https://boajs.dev/conformance).

## Roadmap

A short summary — see [ROADMAP.md](./ROADMAP.md) for the full plan with
deliverables, open questions, and risks for each milestone.

| Version | Theme | Highlights |
|---|---|---|
| **v1.0** ✅ | Initial release | Script/eval/REPL, console, fs, args |
| **v1.1** | Async I/O | `fetch`, timers, `Promise`, top-level await |
| **v1.2** | TypeScript | TS/JSX via SWC, source maps |
| **v1.3** | Modules | ES modules, HTTP imports, lockfile |
| **v1.4** | Concurrency | Web Workers, `MessageChannel` |
| **v1.5** | Permissions | Deno-style allow/deny flags |
| **v1.6** | Embedding | Stable Rust API, WASI plugins |
| **v2.0** | Production | Optional V8 backend, snapshots, `x8 compile` |

## Architecture

```
┌─────────────────────────────────────┐
│        x8 CLI (src/main.rs)         │
│  arg parsing · REPL · file loader   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│      Native globals (Rust)          │
│  console · readFile · writeFile     │
│  args · exit · x8.*                 │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Boa JS engine               │
│  parser · interpreter · GC          │
└─────────────────────────────────────┘
```

Native built-ins are registered as `NativeFunction` callables on the
global object. Each function shells out to `std::fs` / `std::process`
directly — there is no async event loop in v1.0.

## Contributing

Issues and pull requests are welcome. Please open an issue first for
larger changes so we can discuss the design.

When submitting a PR:

1. `cargo fmt` your code.
2. `cargo clippy --all-targets` should pass without warnings.
3. Add tests under `tests/` if your change is observable.

## License

MIT — see [LICENSE](./LICENSE).
