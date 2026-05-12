# x8 examples

Runnable demonstrations of x8's features. Each file's header lists
the exact command to run it (including the permission flags it
needs).

| File | Demonstrates |
|---|---|
| [`hello.ts`](./hello.ts) | TypeScript types, template literals, `x8.version` |
| [`fetch-github.ts`](./fetch-github.ts) | `fetch`, top-level `await`, `Response.json()` |
| [`modules/main.ts`](./modules/main.ts) | Relative `import`, dynamic `import()` |
| [`worker-pool.ts`](./worker-pool.ts) | 4 parallel workers with bi-directional messaging |
| [`sample.test.ts`](./sample.test.ts) | The `x8 test` runner |

## Quick start

```sh
# No permissions needed (only console.log):
x8 examples/hello.ts

# Modules need read permission:
x8 --allow-read examples/modules/main.ts

# fetch needs network:
x8 --allow-net examples/fetch-github.ts nktkt/x8

# Workers need run + read:
x8 --allow-run --allow-read examples/worker-pool.ts

# Test runner:
x8 --allow-read test examples/sample.test.ts
```

If you'd rather just grant everything:

```sh
x8 --allow-all examples/worker-pool.ts
```
