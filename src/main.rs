//! x8 binary: thin wrapper around the [`x8`] library crate.
//!
//! All runtime logic lives in `src/lib.rs`. This file only forwards
//! command-line arguments and the resulting exit code.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    x8::run_cli(env::args().collect())
}
