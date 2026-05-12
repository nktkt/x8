//! x8 — a minimal, fast JavaScript runtime written in Rust.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use boa_engine::{
    js_string,
    object::ObjectInitializer,
    object::builtins::JsArray,
    property::Attribute,
    Context, JsError, JsNativeError, JsResult, JsString, JsValue, NativeFunction, Source,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "x8";

fn print_help() {
    println!("{NAME} {VERSION} — minimal JavaScript runtime written in Rust");
    println!();
    println!("USAGE:");
    println!("    x8 [OPTIONS] [SCRIPT] [-- ARGS...]");
    println!();
    println!("OPTIONS:");
    println!("    -e, --eval <CODE>    Evaluate inline JavaScript and exit");
    println!("    -V, --version        Print version information");
    println!("    -h, --help           Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    x8 script.js             Run a script file");
    println!("    x8 -e \"console.log(1+2)\"  Evaluate inline");
    println!("    x8                       Start an interactive REPL");
    println!();
    println!("BUILT-IN GLOBALS:");
    println!("    console.log/error/warn/info/debug");
    println!("    readFile(path) -> string");
    println!("    writeFile(path, content)");
    println!("    args                  Array of arguments after the script path");
    println!("    exit(code)            Exit the process with a status code");
    println!("    x8.version            Runtime version string");
}

fn format_value(value: &JsValue, ctx: &mut Context) -> String {
    match value.to_string(ctx) {
        Ok(s) => s.to_std_string_escaped(),
        Err(_) => "<unprintable>".to_string(),
    }
}

fn console_print(args: &[JsValue], ctx: &mut Context, to_stderr: bool) -> JsResult<JsValue> {
    let line = args
        .iter()
        .map(|v| format_value(v, ctx))
        .collect::<Vec<_>>()
        .join(" ");
    if to_stderr {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
    Ok(JsValue::undefined())
}

fn js_err(msg: impl Into<String>) -> JsError {
    JsNativeError::error().with_message(msg.into()).into()
}

fn read_file_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let path_val = args.first().ok_or_else(|| js_err("readFile: missing path"))?;
    let path = path_val.to_string(ctx)?.to_std_string_escaped();
    match fs::read_to_string(&path) {
        Ok(content) => Ok(JsValue::from(JsString::from(content.as_str()))),
        Err(e) => Err(js_err(format!("readFile({path}): {e}"))),
    }
}

fn write_file_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let path = args
        .first()
        .ok_or_else(|| js_err("writeFile: missing path"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    let content = args
        .get(1)
        .ok_or_else(|| js_err("writeFile: missing content"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    fs::write(&path, content).map_err(|e| js_err(format!("writeFile({path}): {e}")))?;
    Ok(JsValue::undefined())
}

fn exit_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let code = match args.first() {
        Some(v) => v.to_i32(ctx).unwrap_or(0),
        None => 0,
    };
    std::process::exit(code);
}

fn register_globals(ctx: &mut Context, script_args: &[String]) -> JsResult<()> {
    // console
    let console = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, a, c| console_print(a, c, false)),
            js_string!("log"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, a, c| console_print(a, c, true)),
            js_string!("error"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, a, c| console_print(a, c, true)),
            js_string!("warn"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, a, c| console_print(a, c, false)),
            js_string!("info"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, a, c| console_print(a, c, true)),
            js_string!("debug"),
            0,
        )
        .build();
    ctx.register_global_property(js_string!("console"), console, Attribute::all())?;

    // readFile, writeFile, exit
    ctx.register_global_callable(
        js_string!("readFile"),
        1,
        NativeFunction::from_fn_ptr(read_file_native),
    )?;
    ctx.register_global_callable(
        js_string!("writeFile"),
        2,
        NativeFunction::from_fn_ptr(write_file_native),
    )?;
    ctx.register_global_callable(
        js_string!("exit"),
        1,
        NativeFunction::from_fn_ptr(exit_native),
    )?;

    // args array
    let arr = JsArray::new(ctx);
    for (i, s) in script_args.iter().enumerate() {
        arr.set(i as u32, JsValue::from(JsString::from(s.as_str())), false, ctx)?;
    }
    ctx.register_global_property(js_string!("args"), arr, Attribute::all())?;

    // x8 namespace
    let x8_obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("version"),
            JsValue::from(JsString::from(VERSION)),
            Attribute::all(),
        )
        .property(
            js_string!("name"),
            JsValue::from(JsString::from(NAME)),
            Attribute::all(),
        )
        .build();
    ctx.register_global_property(js_string!("x8"), x8_obj, Attribute::all())?;

    Ok(())
}

fn run_source(source: &str, label: &str, ctx: &mut Context) -> ExitCode {
    match ctx.eval(Source::from_bytes(source.as_bytes())) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{NAME}: {label}: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_file(path: &Path, ctx: &mut Context) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{NAME}: cannot read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };
    run_source(&source, &path.display().to_string(), ctx)
}

fn run_repl(ctx: &mut Context) -> ExitCode {
    println!("{NAME} {VERSION} REPL — type .exit or Ctrl-D to quit");
    let mut rl = match DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("{NAME}: cannot start REPL: {e}");
            return ExitCode::from(1);
        }
    };
    loop {
        match rl.readline("x8> ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == ".exit" || trimmed == ".quit" {
                    break;
                }
                let _ = rl.add_history_entry(line.as_str());
                match ctx.eval(Source::from_bytes(line.as_bytes())) {
                    Ok(v) => {
                        if !v.is_undefined() {
                            let mut stdout = io::stdout().lock();
                            let _ = writeln!(stdout, "{}", format_value(&v, ctx));
                        }
                    }
                    Err(e) => eprintln!("Uncaught: {e}"),
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("{NAME}: REPL error: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

#[derive(Default)]
struct ParsedArgs {
    show_help: bool,
    show_version: bool,
    eval_code: Option<String>,
    script: Option<String>,
    script_args: Vec<String>,
    error: Option<String>,
}

fn parse_args(argv: Vec<String>) -> ParsedArgs {
    let mut out = ParsedArgs::default();
    let mut iter = argv.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => out.show_help = true,
            "-V" | "--version" => out.show_version = true,
            "-e" | "--eval" => match iter.next() {
                Some(code) => out.eval_code = Some(code),
                None => {
                    out.error = Some(format!("{arg} requires an argument"));
                    return out;
                }
            },
            "--" => {
                out.script_args.extend(iter.by_ref());
                break;
            }
            s if s.starts_with('-') && s != "-" => {
                out.error = Some(format!("unknown option: {s}"));
                return out;
            }
            _ => {
                out.script = Some(arg);
                out.script_args.extend(iter.by_ref());
                break;
            }
        }
    }
    out
}

fn main() -> ExitCode {
    let parsed = parse_args(env::args().collect());

    if let Some(err) = parsed.error {
        eprintln!("{NAME}: {err}");
        eprintln!("Try `{NAME} --help` for more information.");
        return ExitCode::from(2);
    }
    if parsed.show_help {
        print_help();
        return ExitCode::SUCCESS;
    }
    if parsed.show_version {
        println!("{NAME} {VERSION}");
        return ExitCode::SUCCESS;
    }

    let mut ctx = Context::default();
    if let Err(e) = register_globals(&mut ctx, &parsed.script_args) {
        eprintln!("{NAME}: failed to initialize runtime: {e}");
        return ExitCode::from(1);
    }

    if let Some(code) = parsed.eval_code {
        return run_source(&code, "[eval]", &mut ctx);
    }
    if let Some(script) = parsed.script {
        return run_file(Path::new(&script), &mut ctx);
    }
    run_repl(&mut ctx)
}
