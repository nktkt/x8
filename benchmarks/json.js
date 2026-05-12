// Benchmark: measures JSON.stringify + JSON.parse round-trip throughput.
// Builds a deterministic array of 1000 records, then performs 100 iterations
// of stringify followed by parse, reporting the final payload size in bytes
// and total wall-clock duration in milliseconds.

var records = [];
for (var i = 0; i < 1000; i++) {
  var tags = [
    "tag" + (i % 7),
    "tag" + ((i + 1) % 11),
    "tag" + ((i + 2) % 13),
    "tag" + ((i + 3) % 17),
    "tag" + ((i + 4) % 19)
  ];
  records.push({
    id: i,
    name: "user_" + (i % 997),
    email: "user" + (i % 997) + "@example.com",
    tags: tags,
    score: (i * 2654435761) % 1000000
  });
}

var start = Date.now();
var bytes = 0;
var parsed = null;
for (var k = 0; k < 100; k++) {
  var s = JSON.stringify(records);
  bytes = s.length;
  parsed = JSON.parse(s);
}
var duration = Date.now() - start;

console.log("bytes=" + bytes);
console.log("ms=" + duration);
