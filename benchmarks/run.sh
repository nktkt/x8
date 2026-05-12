#!/usr/bin/env bash
# benchmarks/run.sh — runs each .js benchmark under every installed runtime.
#
# Usage:
#   ./benchmarks/run.sh           # auto-detect node, deno, bun, x8
#   X8=./target/release/x8 ./benchmarks/run.sh
#
# Output is a markdown table on stdout.

set -u

DIR="$(cd "$(dirname "$0")" && pwd)"

# Resolve runtimes.
NODE_BIN="${NODE:-$(command -v node || true)}"
DENO_BIN="${DENO:-$(command -v deno || true)}"
BUN_BIN="${BUN:-$(command -v bun || true)}"
X8_BIN="${X8:-${DIR}/../target/release/x8}"
[ -x "$X8_BIN" ] || X8_BIN="${DIR}/../target/debug/x8"

# Pretty-print version strings.
ver() {
  local bin="$1"
  [ -z "$bin" ] && { echo "(missing)"; return; }
  [ ! -x "$bin" ] && { echo "(missing)"; return; }
  "$bin" --version 2>/dev/null | head -1 | tr -d '\n'
}

# Pull the ms= line out of a runtime's stdout.
extract_ms() {
  awk -F= '/^ms=/ {print $2; exit}'
}

run_one() {
  local bin="$1"
  local label="$2"
  local script="$3"
  if [ -z "$bin" ] || [ ! -x "$bin" ]; then
    echo "-"
    return
  fi
  local args=""
  case "$label" in
    deno) args="run --allow-read=." ;;
    x8)   args="--allow-read" ;;
    bun)  args="run" ;;
  esac
  # shellcheck disable=SC2086
  local ms
  ms=$( "$bin" $args "$script" 2>/dev/null | extract_ms )
  echo "${ms:-error}"
}

BENCHES=(fib loop strings json)

echo
echo "## Versions"
echo
echo "- node: $(ver "$NODE_BIN")"
echo "- deno: $(ver "$DENO_BIN")"
echo "- bun:  $(ver "$BUN_BIN")"
echo "- x8:   $(ver "$X8_BIN")"
echo
echo "## Results (lower is better, ms)"
echo
printf "| %-10s | %10s | %10s | %10s | %10s |\n" "benchmark" "node" "deno" "bun" "x8"
printf "|%s|%s:|%s:|%s:|%s:|\n" "------------" "-----------" "-----------" "-----------" "-----------"
for b in "${BENCHES[@]}"; do
  script="${DIR}/${b}.js"
  printf "| %-10s | %10s | %10s | %10s | %10s |\n" \
    "$b" \
    "$(run_one "$NODE_BIN" node "$script")" \
    "$(run_one "$DENO_BIN" deno "$script")" \
    "$(run_one "$BUN_BIN" bun "$script")" \
    "$(run_one "$X8_BIN" x8 "$script")"
done
echo
