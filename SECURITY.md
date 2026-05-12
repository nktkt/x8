# Security Policy

## Supported Versions

Only the latest released tag receives security fixes. Earlier tags are
best-effort: if a fix is trivial to backport I may do so, but please do not
rely on it. Pin to the current tag and upgrade when advisories are published.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for security problems.

Preferred channel: open a private security advisory on GitHub.

  Repo > Security > Advisories > New draft security advisory
  https://github.com/nktkt/x8/security/advisories/new

If that is not available to you, contact the repo owner directly through
their GitHub profile (https://github.com/nktkt) and request a private channel.

When reporting, include:

- x8 version (tag or commit)
- Platform and Rust toolchain
- A minimal reproducer (script, import URL, or input)
- Impact you observed (sandbox bypass, RCE, panic, etc.)

## Scope

In scope (please report):

- Sandbox escape: code running under `--deny-all` or a restricted
  `--allow-*` set gaining a capability it was not granted (file, net,
  env, ffi, subprocess, etc.).
- Arbitrary code execution triggered by loading a malicious import URL,
  module, or source map, beyond what the permission set allows.
- Panics, aborts, or memory-safety issues reachable from
  attacker-controlled script input or attacker-controlled module
  contents.
- Privilege escalation between isolates or workers within a single x8
  process.

Out of scope:

- A script author crashing or hanging their own runtime (e.g. infinite
  loops, OOM from their own allocations, `Deno.exit`-style misuse).
- Performance regressions, high memory use, or DoS that only affects
  the operator running the script.
- Deviations from the ECMAScript or WinterCG specifications, unless the
  deviation itself enables a permission bypass or memory-safety bug.
- Issues in third-party crates that do not affect x8 as configured;
  please report those upstream.

## Disclosure Timeline

Standard embargo is **90 days** from the date I acknowledge the report.
This is configurable: if a fix lands sooner we will coordinate an earlier
release, and if the issue is unusually complex I may request an extension
before day 90. If I go silent for more than 14 days after acknowledgement,
you are free to disclose.

## Bug Bounty

There is no bug bounty. x8 is maintained by a single person in their spare
time. What I can offer:

- Acknowledgement in `CHANGELOG.md` and the published advisory (opt-in;
  let me know the name/handle/link you want, or ask to stay anonymous).
- A prompt, honest reply.

## Permission Model

As of x8 v2.0, the runtime **default-denies all capabilities**. Permissions
are the primary security boundary; the JavaScript engine sandbox alone is
not considered sufficient to run untrusted code.

When running code you do not fully trust:

- Prefer `--deny-all` and add only the specific `--allow-*` flags the
  script requires.
- Avoid `--allow-all`, broad `--allow-read` / `--allow-net` without a
  scope list, and `--allow-ffi` / `--allow-run` for untrusted input.
- Treat any permission prompt bypass, or any capability available
  without the matching `--allow-*` flag, as an in-scope security bug.
