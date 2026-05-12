// fetch-github.ts — fetch repo metadata from the GitHub API.
//
// Run with:  x8 --allow-net examples/fetch-github.ts

interface Repo {
  full_name: string;
  description: string;
  stargazers_count: number;
  language: string;
  forks_count: number;
}

const repo = args[0] ?? "nktkt/x8";
const url = `https://api.github.com/repos/${repo}`;

const res = await fetch(url, {
  headers: { "User-Agent": "x8-example" },
});

if (!res.ok) {
  console.error(`fetch failed: ${res.status} ${res.statusText}`);
  exit(1);
}

const data = (await res.json()) as Repo;
console.log(`${data.full_name}`);
console.log(`  ${data.description ?? "(no description)"}`);
console.log(`  language: ${data.language}`);
console.log(`  stars:    ${data.stargazers_count}`);
console.log(`  forks:    ${data.forks_count}`);
