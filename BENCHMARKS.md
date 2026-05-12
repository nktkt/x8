# Benchmarks

A small set of CPU-bound micro-benchmarks comparing **x8** against
Node.js and Bun. The point of this document is not to claim x8 is
fast — it isn't. The point is to be **honest** about the cost x8
trades for its design choices (pure-Rust interpreter, no JIT, ~10 MB
binary).

If you need raw throughput, pick a V8 or JavaScriptCore based
runtime. If you need a small, embeddable, pure-Rust JS interpreter,
the numbers below should help you decide whether that trade-off works
for your workload.

## Methodology

- Each benchmark is a single JavaScript file in [`benchmarks/`](./benchmarks/).
- Each script uses **only** `Date.now()`, `console.log`, and standard
  ES2015+ syntax. No I/O, no fetch, no imports — so the same file
  runs unchanged under every runtime.
- Each runtime is invoked once per benchmark via
  [`benchmarks/run.sh`](./benchmarks/run.sh):
  - `node benchmarks/<bench>.js`
  - `bun run benchmarks/<bench>.js`
  - `x8 --allow-read benchmarks/<bench>.js`
- The reported number is the value of `ms=` printed by each script
  (wall-clock duration of the workload itself, not process startup).
- These are **single-run** numbers. They are stable to about ±5% on
  the test machine but you should expect variance up to ±15% between
  runs on a busy laptop. For real engineering decisions, run them
  yourself.

## Environment

The numbers below were collected on:

- Hardware: Apple Silicon Mac (M-series)
- macOS: Darwin 25.3.0
- Node.js v25.1.0
- Bun 1.3.1
- x8 2.0.0 (release build with LTO, profile = `release`)
- Deno: not installed on the test machine — column reads `-`. If you
  have it, `run.sh` will pick it up automatically.

## Results

Lower is better. All values in milliseconds.

| Benchmark | Node 25 | Bun 1.3 | x8 2.0 | x8 vs Bun |
| --- | ---: | ---: | ---: | ---: |
| `fib.js` (naive `fib(35)`) | 44 | 30 | 10189 | ~340x slower |
| `loop.js` (10M iterations of `Math.sqrt * Math.sin`) | 341 | 33 | 3099 | ~94x slower |
| `strings.js` (10k `split` + `join` of a 10k-char string) | 1003 | 340 | 43044 | ~127x slower |
| `json.js` (100 cycles of `stringify` + `parse` over 1000 records) | 41 | 32 | 988 | ~31x slower |

## What these numbers actually mean

- **Recursive workloads** (`fib`) hit x8 the hardest. Boa is a
  tree-walking interpreter with no inlining; every recursive call is
  pure interpretation overhead, while V8/JSC inline aggressively.
- **Tight numeric loops** (`loop`) are about two orders of magnitude
  slower under x8. The math primitives themselves are fast; the
  interpreter dispatch isn't.
- **String manipulation** (`strings`) shows x8's worst result here.
  Boa's `JsString` is rope-style and allocation-heavy; `split` /
  `join` of large strings exposes that.
- **JSON** (`json`) is x8's best showing. The hot path goes through
  Boa's native `JSON.stringify` / `JSON.parse`, where most of the
  work happens in optimized Rust code rather than the interpreter.

## When the cost is fine

These numbers look ugly in isolation. They matter less when:

- The script runs for milliseconds and most of the time is in
  built-in Rust functions (file I/O, HTTP requests, `JSON`).
- You're scripting glue logic where ~1 second of overhead is
  invisible to the user.
- The script is short-lived and startup time dominates (x8 is
  competitive on startup because there's no JIT warm-up).
- You're embedding x8 to sandbox plugin scripts, where a 30–100x
  slowdown is acceptable for a 10 MB pure-Rust binary you can audit.

## When the cost is not fine

- CPU-bound batch processing of large data.
- Hot paths in a production HTTP server.
- Anything that recursively pattern-matches large structures in JS
  (compilers, parsers written in JS).
- Workloads where Bun/Node already use 50%+ of available CPU — you'll
  saturate orders of magnitude sooner with x8.

## Reproducing

```sh
cargo build --release
./benchmarks/run.sh
```

`run.sh` auto-detects whichever of `node`, `deno`, `bun`, and the x8
release binary are on `$PATH`. Override with environment variables:

```sh
X8=/usr/local/bin/x8 NODE=/path/to/node ./benchmarks/run.sh
```

## Future

When the [V8 backend](./ROADMAP.md) (deferred from v2.0) lands, x8
should close most of this gap on CPU-bound work — at the cost of the
pure-Rust trust boundary and a much larger binary. Until then, the
numbers above are the floor.
