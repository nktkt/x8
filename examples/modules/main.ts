// main.ts — demonstrates ES modules with top-level await.
//
// Run with:  x8 --allow-read examples/modules/main.ts

import { square, sum, mean } from "./math.ts";

const xs = [1, 2, 3, 4, 5];
console.log("input :", xs);
console.log("squares:", xs.map(square));
console.log("sum   :", sum(xs));
console.log("mean  :", mean(xs));

// Dynamic import works too.
const m = await import("./math.ts");
console.log("dynamic.square(10) =", m.square(10));
