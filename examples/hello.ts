// hello.ts — TypeScript types stripped at run time by oxc.
//
// Run with:  x8 examples/hello.ts

interface Greeting {
  recipient: string;
  emoji: string;
}

const greet = (g: Greeting): string => `${g.emoji} Hello, ${g.recipient}!`;

const names = ["world", "x8", "Rust"];
for (const name of names) {
  console.log(greet({ recipient: name, emoji: "*" }));
}

console.log(`Running on x8 v${x8.version}`);
