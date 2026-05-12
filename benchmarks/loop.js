// Measures wall-clock time of a tight numeric loop performing 10M
// iterations of Math.sqrt(i) * Math.sin(i) accumulation. Intended as a
// cross-runtime micro-benchmark (Node.js, Deno, Bun, x8) for raw
// arithmetic + Math intrinsic throughput. No I/O, no imports.

const N = 10000000;
const start = Date.now();

let sum = 0;
for (let i = 0; i < N; i++) {
  sum += Math.sqrt(i) * Math.sin(i);
}

const ms = Date.now() - start;

console.log("sum=" + sum.toFixed(2));
console.log("ms=" + ms);
