// Measures string split/join throughput: repeatedly splits a 10000-char base
// string by a delimiter, joins it back, and reads its length, over 10000
// iterations. Reports the final length and total wall-clock duration in ms.
// Portable across Node.js, Deno, Bun, and x8 (Boa-based) runtimes.

var base = "ab,cd,ef,gh,ij,".repeat(667).slice(0, 10000);

var iterations = 10000;
var lastLen = 0;

var start = Date.now();
for (var i = 0; i < iterations; i++) {
  var parts = base.split(",");
  var joined = parts.join(",");
  lastLen = joined.length;
}
var ms = Date.now() - start;

console.log("last_len=" + lastLen);
console.log("ms=" + ms);
