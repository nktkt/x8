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
x8 --allow-read script.js
# or grant everything (matches v1.x behavior):
x8 --allow-all script.js
```

Since v2.0, x8 starts with **all capabilities denied**. Add
`--allow-read`, `--allow-write`, `--allow-net`, `--allow-run`, or
`--allow-all` to grant what your script needs.

```sh
x8 hello.js  # works for scripts that only use console.log etc.
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

## Subcommands

### `x8 test [paths...]`

Discovers `*.test.{ts,js}` and `*.spec.{ts,js}` files recursively
under the given paths (current directory if no paths are given) and
runs each. Inside test files, three globals are provided:

```ts
test("addition works", () => {
  assertEq(1 + 2, 3);
});

test("array map", () => {
  const r = [1, 2, 3].map(n => n * 2);
  assertEq(JSON.stringify(r), "[2,4,6]");
});

test("truthy", () => {
  assert(42 > 0);
});
```

```sh
x8 --allow-read test ./tests
```

Exit code is the number of failures (or `0` on success).

### `x8 fmt [--write] [paths...]`

Re-emits source via the oxc code generator. By default prints the
formatted output to stdout. Pass `--write` (or `-w`) to overwrite
each file in place.

```sh
x8 fmt src/component.tsx
x8 fmt --write src/*.ts
```

`x8 fmt` does **not** require any permission flags — it does not
execute the parsed source.

## Built-in globals

| Name | Type | Description |
|---|---|---|
| `console.log(...args)` | function | Print to stdout (space-separated). |
| `console.error(...args)` | function | Print to stderr. |
| `console.warn(...args)` | function | Print to stderr. |
| `console.info(...args)` | function | Print to stdout. |
| `console.debug(...args)` | function | Print to stderr. |
| `readFile(path)` / `readFileSync(path)` | function | Read a UTF-8 file. Returns its contents as a string. |
| `writeFile(path, content)` / `writeFileSync` | function | Write a string to disk. |
| `setTimeout(fn, ms)` | function | Schedule `fn` to run after `ms` milliseconds. Returns a timer ID. |
| `clearTimeout(id)` | function | Cancel a pending `setTimeout`. |
| `setInterval(fn, ms)` | function | Run `fn` repeatedly every `ms` milliseconds. Returns a timer ID. |
| `clearInterval(id)` | function | Cancel a `setInterval`. |
| `queueMicrotask(fn)` | function | Run `fn` on the microtask queue. |
| `fetch(url, opts?)` | function | Returns a `Promise<Response>`. Subset of the WHATWG Fetch spec. |
| `exit(code?)` | function | Terminate the process with an optional exit code (defaults to `0`). |
| `args` | array | Arguments passed to the script. |
| `x8.version` | string | Runtime version (e.g. `"1.1.0"`). |
| `x8.name` | string | Runtime name (`"x8"`). |

### Fetch response shape

```js
const res = await fetch(url, { method, headers, body });
// res.ok          - boolean (status < 400)
// res.status      - number  (200, 404, …)
// res.statusText  - string  ("OK", "Not Found", …)
// res.url         - string  (final URL after redirects)
// res.headers     - object  (header name → value)
// res.text()      - Promise<string>
// res.json()      - Promise<any>
```

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

## TypeScript and JSX

Files with `.ts`, `.tsx`, `.jsx`, `.mts`, or `.cts` extensions are
automatically transpiled with [oxc](https://oxc.rs) before execution.

```sh
x8 script.ts        # types stripped, then run
x8 component.jsx    # JSX transformed, then run
```

**JSX** uses the classic runtime with `h` as the element factory and
`Fragment` as the fragment tag. You must define them yourself:

```js
const h = (tag, props, ...children) => ({ tag, props: props || {}, children });
const Fragment = "Fragment";

const tree = <div className="x">hello <span>world</span></div>;
console.log(tree);
```

**Limitations** (in v1.2):

- No type checking — types are stripped, not verified. Use `tsc
  --noEmit` for type validation.
- No source maps yet (runtime errors point at transpiled JS).

## Modules

Files with extensions `.mjs`, `.mts`, `.ts`, `.tsx`, `.jsx`, or `.cts`
are run as **ES modules**. They support `import` / `export`, dynamic
`import()`, and **top-level `await`**.

### Relative imports

```ts
// math.ts
export const square = (n: number) => n * n;
```
```ts
// main.ts
import { square } from "./math.ts";
console.log(square(5));   // 25
```

### HTTP(S) imports

```ts
import { something } from "https://example.com/lib.js";
```

Downloaded modules are cached under `~/.cache/x8/deps/` (override
with the `X8_CACHE` environment variable). Subsequent runs read from
the cache without making a network request.

### Dynamic imports

```ts
const helpers = await import("./helpers.ts");
helpers.run();
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
| **v1.1** ✅ | Async I/O | `fetch`, timers, `Promise`, microtasks |
| **v1.2** ✅ | TypeScript | TS/JSX via oxc, type stripping |
| **v1.3** ✅ | Modules | ES modules, dynamic import, HTTP imports |
| **v1.4** ✅ | Concurrency | Workers on dedicated threads |
| **v1.5** ✅ | Permissions | Allow/deny flags, inherited by workers |
| **v1.6** ✅ | Embedding | lib/bin split, `x8::run_cli`, public `Permissions` |
| **v2.0** ✅ | Production | Default-deny perms, `x8 test`, `x8 fmt` |

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
