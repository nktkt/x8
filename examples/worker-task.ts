// worker-task.ts — worker script that sums 1..N on a background thread.
//
// Used by worker-pool.ts. Don't run directly.

self.onmessage = (msg: string) => {
  const n = parseInt(msg, 10);
  let sum = 0;
  for (let i = 1; i <= n; i++) sum += i;
  self.postMessage(JSON.stringify({ n, sum }));
};
