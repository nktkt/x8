// Measures wall-clock time (ms) to compute fib(35) once using naive recursion.
// Portable across Node.js, Deno, Bun, and x8 (Boa-based) runtimes.

function fib(n) {
  if (n < 2) return n;
  return fib(n - 1) + fib(n - 2);
}

var start = Date.now();
var result = fib(35);
var ms = Date.now() - start;

console.log("result=" + result);
console.log("ms=" + ms);
