// sample.test.ts — illustrates the `x8 test` runner.
//
// Run with:  x8 test examples/sample.test.ts

test("addition", () => {
  assertEq(1 + 2, 3);
});

test("array map", () => {
  const r = [1, 2, 3].map(x => x * x);
  assertEq(JSON.stringify(r), "[1,4,9]");
});

test("truthy", () => {
  assert(42 > 0);
});

test("string split", () => {
  assertEq(JSON.stringify("a,b,c".split(",")), `["a","b","c"]`);
});
