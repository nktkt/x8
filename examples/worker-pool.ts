// worker-pool.ts — fan out 4 sum jobs to 4 worker threads in parallel.
//
// Run with:  x8 --allow-run --allow-read examples/worker-pool.ts

const WORKER_COUNT = 4;
const SCRIPT = "examples/worker-task.ts";

interface Result { n: number; sum: number; }

let pending = WORKER_COUNT;
let grand_total = 0;

for (let i = 0; i < WORKER_COUNT; i++) {
  const n = 100 * (i + 1);
  const w = Worker(SCRIPT);
  w.onmessage = (data: string) => {
    const r = JSON.parse(data) as Result;
    console.log(`  worker #${i}: sum 1..${r.n} = ${r.sum}`);
    grand_total += r.sum;
    pending--;
    w.terminate();
    if (pending === 0) {
      console.log(`grand total: ${grand_total}`);
    }
  };
  w.onerror = (msg: string) => {
    console.error(`  worker #${i} failed: ${msg}`);
    pending--;
    w.terminate();
  };
  w.postMessage(String(n));
}
