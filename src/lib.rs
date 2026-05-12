//! # x8 — a minimal, fast JavaScript runtime written in Rust.
//!
//! `x8` ships as a CLI but is also embeddable. The simplest entrypoint is
//! [`run_cli`], which takes a `Vec<String>` of CLI-style arguments and
//! returns a process exit code.
//!
//! ```no_run
//! let code = x8::run_cli(vec!["x8".into(), "-e".into(), "console.log(1+2)".into()]);
//! std::process::exit(u8::from(code) as i32);
//! ```
//!
//! For richer embedding, see [`Permissions`] (the public capability
//! struct passed through `run_cli` via `--allow-*` / `--deny-*` flags).
//! A higher-level `Runtime` API with structured eval/return is planned
//! for v2.0.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;


use boa_engine::{
    builtins::promise::PromiseState,
    js_string,
    job::{FutureJob, JobQueue, NativeJob},
    module::{Module, ModuleLoader, Referrer},
    object::{builtins::{JsArray, JsFunction, JsPromise}, ObjectInitializer},
    property::Attribute,
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction, Source,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use tokio::runtime::Runtime;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "x8";

// ============================================================================
// Async job queue: bridges tokio futures into Boa's job queue.
// ============================================================================

struct AsyncJobQueue {
    rt: Runtime,
    promise_jobs: RefCell<VecDeque<NativeJob>>,
    future_jobs: RefCell<Vec<FutureJob>>,
}

impl AsyncJobQueue {
    fn new(rt: Runtime) -> Self {
        Self {
            rt,
            promise_jobs: RefCell::new(VecDeque::new()),
            future_jobs: RefCell::new(Vec::new()),
        }
    }
}

impl JobQueue for AsyncJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _: &mut Context) {
        self.promise_jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _: &mut Context) {
        self.future_jobs.borrow_mut().push(future);
    }

    fn run_jobs(&self, context: &mut Context) {
        loop {
            let mut did_work = false;

            // Drain all pending sync jobs first.
            loop {
                let job = self.promise_jobs.borrow_mut().pop_front();
                match job {
                    Some(job) => {
                        if let Err(e) = job.call(context) {
                            eprintln!("Uncaught (in promise): {e}");
                        }
                    }
                    None => break,
                }
            }

            // Drain pending futures.
            let futures: Vec<FutureJob> =
                std::mem::take(&mut *self.future_jobs.borrow_mut());
            if !futures.is_empty() {
                did_work = true;
                for fut in futures {
                    let job = self.rt.block_on(fut);
                    self.promise_jobs.borrow_mut().push_back(job);
                }
            }

            // Drain worker events (non-blocking).
            let drained: Vec<WorkerEvent> = WORKER_EVENTS_RX.with(|rx| {
                let mut out = Vec::new();
                if let Some(rx) = rx.borrow().as_ref() {
                    while let Ok(ev) = rx.try_recv() {
                        out.push(ev);
                    }
                }
                out
            });
            if !drained.is_empty() {
                for event in drained {
                    if let Err(e) = handle_worker_event(event, context) {
                        eprintln!("Uncaught (in worker handler): {e}");
                    }
                }
                continue;
            }

            if did_work {
                continue;
            }

            // No more work. If there are still active workers, block on the event channel.
            let any_workers = WORKERS.with(|w| !w.borrow().is_empty());
            if !any_workers {
                break;
            }
            let event = WORKER_EVENTS_RX.with(|rx| {
                rx.borrow().as_ref().and_then(|r| r.recv().ok())
            });
            match event {
                Some(event) => {
                    if let Err(e) = handle_worker_event(event, context) {
                        eprintln!("Uncaught (in worker handler): {e}");
                    }
                }
                None => break,
            }
        }
    }
}

// ============================================================================
// Permissions
// ============================================================================

/// Capability set controlling what scripts are allowed to do.
///
/// The CLI accepts `--allow-*` and `--deny-*` flags that build a
/// `Permissions` value before the runtime starts. In v1.x the default
/// is "allow all"; in v2.0 the default will flip to "deny all".
#[derive(Clone, Copy, Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub net: bool,
    pub env: bool,
    pub run: bool,
}

impl Permissions {
    /// All capabilities enabled (v1.x default).
    pub fn all_allowed() -> Self {
        Self {
            read: true,
            write: true,
            net: true,
            env: true,
            run: true,
        }
    }

    /// All capabilities disabled (v2.0 default).
    pub fn all_denied() -> Self {
        Self {
            read: false,
            write: false,
            net: false,
            env: false,
            run: false,
        }
    }
}

thread_local! {
    static PERMISSIONS: RefCell<Permissions> = RefCell::new(Permissions::all_allowed());
}

fn install_permissions(p: Permissions) {
    PERMISSIONS.with(|c| *c.borrow_mut() = p);
}

fn perms() -> Permissions {
    PERMISSIONS.with(|c| *c.borrow())
}

fn require_perm(category: &str, ok: bool) -> JsResult<()> {
    if !ok {
        return Err(js_err(format!(
            "permission denied: {category} (rerun without --deny-{category})"
        )));
    }
    Ok(())
}

// ============================================================================
// Module loader (file:// and https:// imports, with on-disk HTTP cache).
// ============================================================================

fn x8_cache_dir() -> PathBuf {
    if let Ok(dir) = env::var("X8_CACHE") {
        return PathBuf::from(dir);
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("x8").join("deps");
    }
    PathBuf::from(".x8-cache")
}

fn cache_filename(url: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    let ext = if url.ends_with(".ts") || url.contains(".ts?") {
        ".ts"
    } else if url.ends_with(".tsx") {
        ".tsx"
    } else if url.ends_with(".mjs") {
        ".mjs"
    } else {
        ".js"
    };
    format!("{:016x}{}", h.finish(), ext)
}

fn fetch_http_cached(url: &str, cache_dir: &Path) -> Result<String, String> {
    let _ = fs::create_dir_all(cache_dir);
    let cache_path = cache_dir.join(cache_filename(url));
    if cache_path.exists() {
        if let Ok(s) = fs::read_to_string(&cache_path) {
            return Ok(s);
        }
    }
    let body = reqwest::blocking::get(url)
        .map_err(|e| format!("http GET {url}: {e}"))?
        .text()
        .map_err(|e| format!("http body {url}: {e}"))?;
    let _ = fs::write(&cache_path, &body);
    Ok(body)
}

fn resolve_specifier(referrer: &Referrer, specifier: &str) -> Result<String, String> {
    // Absolute http/https URL.
    if specifier.starts_with("https://") || specifier.starts_with("http://") {
        return Ok(specifier.to_string());
    }
    // Relative to referrer.
    let referrer_path = referrer.path();
    let referrer_str = referrer_path.and_then(|p| p.to_str()).unwrap_or("");
    // If referrer is an HTTP URL, resolve relatively against it (string join).
    if referrer_str.starts_with("https://") || referrer_str.starts_with("http://") {
        if let Some(idx) = referrer_str.rfind('/') {
            return Ok(format!("{}/{}", &referrer_str[..idx], specifier));
        }
        return Ok(format!("{referrer_str}/{specifier}"));
    }
    // File-system relative.
    if specifier.starts_with("./") || specifier.starts_with("../") || specifier.starts_with('/') {
        let base = referrer_path
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        let joined = base.join(specifier);
        return Ok(joined.to_string_lossy().to_string());
    }
    Err(format!(
        "bare specifier not supported: {specifier} (use a relative path or full URL)"
    ))
}

struct X8ModuleLoader {
    cache_dir: PathBuf,
    modules: RefCell<HashMap<String, Module>>,
}

impl X8ModuleLoader {
    fn new() -> Self {
        Self {
            cache_dir: x8_cache_dir(),
            modules: RefCell::new(HashMap::new()),
        }
    }
}

impl ModuleLoader for X8ModuleLoader {
    fn load_imported_module(
        &self,
        referrer: Referrer,
        specifier: JsString,
        finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
        context: &mut Context,
    ) {
        let spec_str = specifier.to_std_string_escaped();

        let result = (|| -> JsResult<Module> {
            let resolved = resolve_specifier(&referrer, &spec_str).map_err(js_err)?;
            if let Some(m) = self.modules.borrow().get(&resolved) {
                return Ok(m.clone());
            }
            let (source_text, source_path) =
                if resolved.starts_with("https://") || resolved.starts_with("http://") {
                    require_perm("net", perms().net)?;
                    let body =
                        fetch_http_cached(&resolved, &self.cache_dir).map_err(js_err)?;
                    (body, PathBuf::from(&resolved))
                } else {
                    require_perm("read", perms().read)?;
                    let path = PathBuf::from(&resolved);
                    let body = fs::read_to_string(&path)
                        .map_err(|e| js_err(format!("load {resolved}: {e}")))?;
                    (body, path)
                };
            let js = if is_typescript_path(&source_path) {
                transpile(&source_text, &source_path)
                    .map_err(|e| js_err(format!("transpile {resolved}: {e}")))?
            } else {
                source_text
            };
            let source = Source::from_bytes(js.as_bytes()).with_path(&source_path);
            let module = Module::parse(source, None, context)?;
            self.modules
                .borrow_mut()
                .insert(resolved, module.clone());
            Ok(module)
        })();

        finish_load(result, context);
    }

    fn register_module(&self, specifier: JsString, module: Module) {
        let key = specifier.to_std_string_escaped();
        self.modules.borrow_mut().insert(key, module);
    }

    fn get_module(&self, specifier: JsString) -> Option<Module> {
        let key = specifier.to_std_string_escaped();
        self.modules.borrow().get(&key).cloned()
    }
}

// ============================================================================
// Shared runtime state (accessed from native function callbacks).
// ============================================================================

thread_local! {
    static QUEUE: RefCell<Option<Rc<AsyncJobQueue>>> = const { RefCell::new(None) };
    static CANCELLED_TIMERS: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

static NEXT_TIMER_ID: AtomicU32 = AtomicU32::new(1);

fn install_queue(queue: Rc<AsyncJobQueue>) {
    QUEUE.with(|q| *q.borrow_mut() = Some(queue));
}

fn queue() -> Rc<AsyncJobQueue> {
    QUEUE.with(|q| q.borrow().as_ref().expect("job queue uninstalled").clone())
}

fn next_timer_id() -> u32 {
    NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed)
}

fn cancel_timer(id: u32) {
    CANCELLED_TIMERS.with(|c| {
        c.borrow_mut().insert(id);
    });
}

fn timer_is_cancelled(id: u32) -> bool {
    CANCELLED_TIMERS.with(|c| c.borrow_mut().remove(&id))
}

// ============================================================================
// Helpers
// ============================================================================

fn js_err(msg: impl Into<String>) -> JsError {
    JsNativeError::error().with_message(msg.into()).into()
}

fn format_value(value: &JsValue, ctx: &mut Context) -> String {
    match value.to_string(ctx) {
        Ok(s) => s.to_std_string_escaped(),
        Err(_) => "<unprintable>".to_string(),
    }
}

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
    println!("PERMISSIONS (opt-in deny in v1.x, default-deny in v2.0):");
    println!("    --allow-all          Allow all capabilities (no-op in v1.x)");
    println!("    --allow-read/write/net/env/run   Allow that capability");
    println!("    --deny-all           Deny all capabilities");
    println!("    --deny-read/write/net/env/run    Deny that capability");
    println!();
    println!("EXAMPLES:");
    println!("    x8 script.js                Run a script file");
    println!("    x8 -e \"console.log(1+2)\"     Evaluate inline");
    println!("    x8                          Start an interactive REPL");
    println!();
    println!("BUILT-IN GLOBALS:");
    println!("    console.log/error/warn/info/debug");
    println!("    readFile(path) / writeFile(path, content)");
    println!("    setTimeout(fn, ms) / clearTimeout(id)");
    println!("    setInterval(fn, ms) / clearInterval(id)");
    println!("    queueMicrotask(fn)");
    println!("    fetch(url, opts?) -> Promise<Response>");
    println!("    args, exit(code), x8.version");
}

// ============================================================================
// Console
// ============================================================================

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

// ============================================================================
// File system
// ============================================================================

fn read_file_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    require_perm("read", perms().read)?;
    let path = args
        .first()
        .ok_or_else(|| js_err("readFile: missing path"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    fs::read_to_string(&path)
        .map(|s| JsValue::from(JsString::from(s.as_str())))
        .map_err(|e| js_err(format!("readFile({path}): {e}")))
}

fn write_file_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    require_perm("write", perms().write)?;
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

// ============================================================================
// Process
// ============================================================================

fn exit_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let code = match args.first() {
        Some(v) => v.to_i32(ctx).unwrap_or(0),
        _none => 0,
    };
    std::process::exit(code);
}

// ============================================================================
// Timers
// ============================================================================

fn set_timeout_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let cb_val = args
        .first()
        .cloned()
        .ok_or_else(|| js_err("setTimeout: missing callback"))?;
    let cb_obj = cb_val
        .as_callable()
        .ok_or_else(|| js_err("setTimeout: callback is not callable"))?
        .clone();
    let ms = args
        .get(1)
        .and_then(|v| v.to_i32(ctx).ok())
        .unwrap_or(0)
        .max(0) as u64;
    let id = next_timer_id();
    let future: FutureJob = Box::pin(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        NativeJob::new(move |ctx| -> JsResult<JsValue> {
            if timer_is_cancelled(id) {
                return Ok(JsValue::undefined());
            }
            cb_obj.call(&JsValue::undefined(), &[], ctx)?;
            Ok(JsValue::undefined())
        })
    });
    queue().enqueue_future_job(future, ctx);
    Ok(JsValue::from(id))
}

fn clear_timeout_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if let Some(v) = args.first() {
        let id = v.to_u32(ctx).unwrap_or(0);
        cancel_timer(id);
    }
    Ok(JsValue::undefined())
}

fn schedule_interval(id: u32, cb: JsObject, ms: u64, ctx: &mut Context) {
    let cb_for_job = cb.clone();
    let cb_for_reschedule = cb;
    let future: FutureJob = Box::pin(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        NativeJob::new(move |ctx| -> JsResult<JsValue> {
            if timer_is_cancelled(id) {
                return Ok(JsValue::undefined());
            }
            cb_for_job.call(&JsValue::undefined(), &[], ctx)?;
            schedule_interval(id, cb_for_reschedule, ms, ctx);
            Ok(JsValue::undefined())
        })
    });
    queue().enqueue_future_job(future, ctx);
}

fn set_interval_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let cb_val = args
        .first()
        .cloned()
        .ok_or_else(|| js_err("setInterval: missing callback"))?;
    let cb_obj = cb_val
        .as_callable()
        .ok_or_else(|| js_err("setInterval: callback is not callable"))?
        .clone();
    let ms = args
        .get(1)
        .and_then(|v| v.to_i32(ctx).ok())
        .unwrap_or(0)
        .max(1) as u64;
    let id = next_timer_id();
    schedule_interval(id, cb_obj, ms, ctx);
    Ok(JsValue::from(id))
}

fn clear_interval_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    clear_timeout_native(&JsValue::undefined(), args, ctx)
}

fn queue_microtask_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let cb_val = args
        .first()
        .cloned()
        .ok_or_else(|| js_err("queueMicrotask: missing callback"))?;
    let cb_obj = cb_val
        .as_callable()
        .ok_or_else(|| js_err("queueMicrotask: callback is not callable"))?
        .clone();
    let job = NativeJob::new(move |ctx| -> JsResult<JsValue> {
        cb_obj.call(&JsValue::undefined(), &[], ctx)?;
        Ok(JsValue::undefined())
    });
    queue().enqueue_promise_job(job, ctx);
    Ok(JsValue::undefined())
}

// ============================================================================
// fetch
// ============================================================================

struct FetchResult {
    status: u16,
    status_text: String,
    url: String,
    body: String,
    headers: Vec<(String, String)>,
}

fn response_text_native(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = this
        .as_object()
        .ok_or_else(|| js_err("Response.text: invalid this"))?;
    let body = obj.get(js_string!("_body"), ctx)?;
    Ok(JsPromise::resolve(body, ctx).into())
}

fn response_json_native(this: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = this
        .as_object()
        .ok_or_else(|| js_err("Response.json: invalid this"))?;
    let body = obj.get(js_string!("_body"), ctx)?;
    let body_str = body.to_string(ctx)?.to_std_string_escaped();
    let json_global = ctx.global_object().get(js_string!("JSON"), ctx)?;
    let json_obj = json_global
        .as_object()
        .ok_or_else(|| js_err("JSON is not an object"))?
        .clone();
    let parse = json_obj.get(js_string!("parse"), ctx)?;
    let parse_fn = parse
        .as_callable()
        .ok_or_else(|| js_err("JSON.parse is not callable"))?
        .clone();
    let parsed = parse_fn.call(
        &JsValue::from(json_obj),
        &[JsValue::from(JsString::from(body_str.as_str()))],
        ctx,
    )?;
    Ok(JsPromise::resolve(parsed, ctx).into())
}

fn build_response(result: &FetchResult, ctx: &mut Context) -> JsResult<JsValue> {
    let mut h = ObjectInitializer::new(ctx);
    for (k, v) in &result.headers {
        h.property(
            JsString::from(k.as_str()),
            JsValue::from(JsString::from(v.as_str())),
            Attribute::all(),
        );
    }
    let headers = h.build();

    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("ok"),
            JsValue::from(result.status < 400),
            Attribute::all(),
        )
        .property(
            js_string!("status"),
            JsValue::from(result.status),
            Attribute::all(),
        )
        .property(
            js_string!("statusText"),
            JsValue::from(JsString::from(result.status_text.as_str())),
            Attribute::all(),
        )
        .property(
            js_string!("url"),
            JsValue::from(JsString::from(result.url.as_str())),
            Attribute::all(),
        )
        .property(js_string!("headers"), headers, Attribute::all())
        .property(
            js_string!("_body"),
            JsValue::from(JsString::from(result.body.as_str())),
            Attribute::all(),
        )
        .function(
            NativeFunction::from_fn_ptr(response_text_native),
            js_string!("text"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(response_json_native),
            js_string!("json"),
            0,
        )
        .build();

    Ok(JsValue::from(obj))
}

fn fetch_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    require_perm("net", perms().net)?;
    let url = args
        .first()
        .ok_or_else(|| js_err("fetch: missing URL"))?
        .to_string(ctx)?
        .to_std_string_escaped();

    // Optional method/body/headers from second arg.
    let mut method = "GET".to_string();
    let mut body: Option<String> = None;
    let mut header_pairs: Vec<(String, String)> = Vec::new();
    if let Some(opts_val) = args.get(1) {
        if let Some(opts) = opts_val.as_object() {
            if let Ok(m) = opts.get(js_string!("method"), ctx) {
                if !m.is_undefined() {
                    method = m.to_string(ctx)?.to_std_string_escaped();
                }
            }
            if let Ok(b) = opts.get(js_string!("body"), ctx) {
                if !b.is_undefined() && !b.is_null() {
                    body = Some(b.to_string(ctx)?.to_std_string_escaped());
                }
            }
            if let Ok(h) = opts.get(js_string!("headers"), ctx) {
                if let Some(h_obj) = h.as_object() {
                    // Iterate own enumerable string keys.
                    let keys = h_obj.own_property_keys(ctx)?;
                    for key in keys {
                        if let boa_engine::property::PropertyKey::String(s) = &key {
                            let v = h_obj.get(key.clone(), ctx)?;
                            header_pairs.push((
                                s.to_std_string_escaped(),
                                v.to_string(ctx)?.to_std_string_escaped(),
                            ));
                        }
                    }
                }
            }
        }
    }

    let resolve_slot: Rc<RefCell<Option<JsFunction>>> = Default::default();
    let reject_slot: Rc<RefCell<Option<JsFunction>>> = Default::default();
    let r_clone = resolve_slot.clone();
    let rj_clone = reject_slot.clone();

    let promise = JsPromise::new(
        move |resolvers, _ctx| {
            *r_clone.borrow_mut() = Some(resolvers.resolve.clone());
            *rj_clone.borrow_mut() = Some(resolvers.reject.clone());
            Ok(JsValue::undefined())
        },
        ctx,
    );

    let resolve = resolve_slot
        .borrow()
        .as_ref()
        .expect("resolver not captured")
        .clone();
    let reject = reject_slot
        .borrow()
        .as_ref()
        .expect("reject not captured")
        .clone();

    let url_for_err = url.clone();
    let future: FutureJob = Box::pin(async move {
        let outcome: Result<FetchResult, String> = async {
            let client = reqwest::Client::new();
            let parsed_method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|e| format!("bad method: {e}"))?;
            let mut req = client.request(parsed_method, &url);
            for (k, v) in header_pairs.iter() {
                req = req.header(k, v);
            }
            if let Some(b) = body {
                req = req.body(b);
            }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let status_text = resp
                .status()
                .canonical_reason()
                .unwrap_or("")
                .to_string();
            let final_url = resp.url().to_string();
            let mut headers_out = Vec::new();
            for (k, v) in resp.headers().iter() {
                headers_out.push((k.to_string(), v.to_str().unwrap_or("").to_string()));
            }
            let body_text = resp.text().await.map_err(|e| e.to_string())?;
            Ok(FetchResult {
                status,
                status_text,
                url: final_url,
                body: body_text,
                headers: headers_out,
            })
        }
        .await;

        NativeJob::new(move |ctx| -> JsResult<JsValue> {
            match outcome {
                Ok(result) => {
                    let resp_val = build_response(&result, ctx)?;
                    resolve.call(&JsValue::undefined(), &[resp_val], ctx)?;
                }
                Err(e) => {
                    let msg = JsValue::from(JsString::from(
                        format!("fetch({url_for_err}): {e}").as_str(),
                    ));
                    reject.call(&JsValue::undefined(), &[msg], ctx)?;
                }
            }
            Ok(JsValue::undefined())
        })
    });

    queue().enqueue_future_job(future, ctx);
    Ok(promise.into())
}

// ============================================================================
// Workers (basic threaded ES module workers with string messaging)
// ============================================================================

enum WorkerCmd {
    Message(String),
    Terminate,
}

enum WorkerEvent {
    Message { worker_id: u32, data: String },
    Error { worker_id: u32, message: String },
    Done { worker_id: u32 },
}

struct WorkerHandle {
    sender: mpsc::Sender<WorkerCmd>,
    js_obj: JsObject,
}

thread_local! {
    static WORKERS: RefCell<HashMap<u32, WorkerHandle>> = RefCell::new(HashMap::new());
    static WORKER_EVENTS_TX: RefCell<Option<mpsc::Sender<WorkerEvent>>> = const { RefCell::new(None) };
    static WORKER_EVENTS_RX: RefCell<Option<mpsc::Receiver<WorkerEvent>>> = const { RefCell::new(None) };
    static IS_WORKER: RefCell<Option<(u32, mpsc::Sender<WorkerEvent>)>> = const { RefCell::new(None) };
}

static NEXT_WORKER_ID: AtomicU32 = AtomicU32::new(1);

fn init_worker_events() {
    let (tx, rx) = mpsc::channel();
    WORKER_EVENTS_TX.with(|c| *c.borrow_mut() = Some(tx));
    WORKER_EVENTS_RX.with(|c| *c.borrow_mut() = Some(rx));
}

fn main_event_sender() -> mpsc::Sender<WorkerEvent> {
    WORKER_EVENTS_TX.with(|c| {
        c.borrow()
            .as_ref()
            .expect("worker events not initialized")
            .clone()
    })
}

fn worker_post_message_native(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let this_obj = this
        .as_object()
        .ok_or_else(|| js_err("Worker.postMessage: invalid this"))?;
    let id_val = this_obj.get(js_string!("_id"), ctx)?;
    let id = id_val.to_u32(ctx).unwrap_or(0);
    let data = args
        .first()
        .ok_or_else(|| js_err("Worker.postMessage: missing message"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    let sender = WORKERS.with(|w| w.borrow().get(&id).map(|h| h.sender.clone()));
    match sender {
        Some(s) => s
            .send(WorkerCmd::Message(data))
            .map_err(|e| js_err(format!("postMessage: {e}")))?,
        None => return Err(js_err(format!("Worker {id} has terminated"))),
    }
    Ok(JsValue::undefined())
}

fn worker_terminate_native(
    this: &JsValue,
    _: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let this_obj = this
        .as_object()
        .ok_or_else(|| js_err("Worker.terminate: invalid this"))?;
    let id_val = this_obj.get(js_string!("_id"), ctx)?;
    let id = id_val.to_u32(ctx).unwrap_or(0);
    let sender = WORKERS.with(|w| w.borrow_mut().remove(&id).map(|h| h.sender));
    if let Some(s) = sender {
        let _ = s.send(WorkerCmd::Terminate);
    }
    Ok(JsValue::undefined())
}

fn worker_self_post_message_native(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let data = args
        .first()
        .ok_or_else(|| js_err("self.postMessage: missing message"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    let (id, tx) = IS_WORKER.with(|c| {
        c.borrow()
            .clone()
            .expect("self.postMessage called outside worker")
    });
    let _ = tx.send(WorkerEvent::Message {
        worker_id: id,
        data,
    });
    Ok(JsValue::undefined())
}

fn worker_main(
    id: u32,
    path: PathBuf,
    cmd_rx: mpsc::Receiver<WorkerCmd>,
    event_tx: mpsc::Sender<WorkerEvent>,
    permissions: Permissions,
) {
    install_permissions(permissions);
    // Tokio runtime + Boa context for this worker.
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::Error {
                worker_id: id,
                message: format!("worker {id}: tokio init failed: {e}"),
            });
            let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
            return;
        }
    };
    let job_queue = Rc::new(AsyncJobQueue::new(rt));
    install_queue(job_queue.clone());

    let module_loader = Rc::new(X8ModuleLoader::new());

    let mut ctx = match Context::builder()
        .job_queue(job_queue)
        .module_loader(module_loader)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::Error {
                worker_id: id,
                message: format!("worker {id}: context build: {e}"),
            });
            let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
            return;
        }
    };

    // Mark this thread as a worker.
    IS_WORKER.with(|c| *c.borrow_mut() = Some((id, event_tx.clone())));

    // Set up worker globals.
    if let Err(e) = register_globals(&mut ctx, &[]) {
        let _ = event_tx.send(WorkerEvent::Error {
            worker_id: id,
            message: format!("worker {id}: globals init: {e}"),
        });
        let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
        return;
    }

    // Build `self` object with postMessage; onmessage is set by JS.
    let self_obj = ObjectInitializer::new(&mut ctx)
        .function(
            NativeFunction::from_fn_ptr(worker_self_post_message_native),
            js_string!("postMessage"),
            1,
        )
        .property(
            js_string!("onmessage"),
            JsValue::null(),
            Attribute::all(),
        )
        .build();
    let _ = ctx.register_global_property(js_string!("self"), self_obj.clone(), Attribute::all());

    // Load and evaluate the worker script.
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            let _ = event_tx.send(WorkerEvent::Error {
                worker_id: id,
                message: format!("worker {id}: read {}: {e}", path.display()),
            });
            let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
            return;
        }
    };
    let source = if is_typescript_path(&path) {
        match transpile(&raw, &path) {
            Ok(js) => js,
            Err(e) => {
                let _ = event_tx.send(WorkerEvent::Error {
                    worker_id: id,
                    message: format!("worker {id}: transpile: {e}"),
                });
                let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
                return;
            }
        }
    } else {
        raw
    };

    if is_module_path(&path) {
        let module = match Module::parse(
            Source::from_bytes(source.as_bytes()).with_path(&path),
            None,
            &mut ctx,
        ) {
            Ok(m) => m,
            Err(e) => {
                let _ = event_tx.send(WorkerEvent::Error {
                    worker_id: id,
                    message: format!("worker {id}: parse: {e}"),
                });
                let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
                return;
            }
        };
        let p = module.load_link_evaluate(&mut ctx);
        queue().run_jobs(&mut ctx);
        if let PromiseState::Rejected(reason) = p.state() {
            let s = reason
                .to_string(&mut ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let _ = event_tx.send(WorkerEvent::Error {
                worker_id: id,
                message: format!("worker {id}: {s}"),
            });
        }
    } else if let Err(e) = ctx.eval(Source::from_bytes(source.as_bytes())) {
        let _ = event_tx.send(WorkerEvent::Error {
            worker_id: id,
            message: format!("worker {id}: {e}"),
        });
    }

    // Message loop.
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            WorkerCmd::Terminate => break,
            WorkerCmd::Message(data) => {
                let onmessage = match self_obj.get(js_string!("onmessage"), &mut ctx) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(callable) = onmessage.as_callable() {
                    let msg = JsValue::from(JsString::from(data.as_str()));
                    let _ = callable.call(
                        &JsValue::from(self_obj.clone()),
                        &[msg],
                        &mut ctx,
                    );
                    queue().run_jobs(&mut ctx);
                }
            }
        }
    }

    let _ = event_tx.send(WorkerEvent::Done { worker_id: id });
}

fn new_worker_native(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    require_perm("run", perms().run)?;
    let path_str = args
        .first()
        .ok_or_else(|| js_err("Worker: missing path"))?
        .to_string(ctx)?
        .to_std_string_escaped();
    let path = PathBuf::from(&path_str);

    let id = NEXT_WORKER_ID.fetch_add(1, Ordering::Relaxed);
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let event_tx = main_event_sender();
    let inherited_perms = perms();

    let worker_obj = ObjectInitializer::new(ctx)
        .property(js_string!("_id"), JsValue::from(id), Attribute::all())
        .property(js_string!("onmessage"), JsValue::null(), Attribute::all())
        .property(js_string!("onerror"), JsValue::null(), Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(worker_post_message_native),
            js_string!("postMessage"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(worker_terminate_native),
            js_string!("terminate"),
            0,
        )
        .build();

    WORKERS.with(|w| {
        w.borrow_mut().insert(
            id,
            WorkerHandle {
                sender: cmd_tx,
                js_obj: worker_obj.clone(),
            },
        );
    });

    thread::spawn(move || worker_main(id, path, cmd_rx, event_tx, inherited_perms));

    Ok(JsValue::from(worker_obj))
}

fn handle_worker_event(event: WorkerEvent, ctx: &mut Context) -> JsResult<JsValue> {
    match event {
        WorkerEvent::Message { worker_id, data } => {
            let obj = WORKERS.with(|w| w.borrow().get(&worker_id).map(|h| h.js_obj.clone()));
            if let Some(obj) = obj {
                let onmessage = obj.get(js_string!("onmessage"), ctx)?;
                if let Some(callable) = onmessage.as_callable() {
                    let msg = JsValue::from(JsString::from(data.as_str()));
                    callable.call(&JsValue::from(obj), &[msg], ctx)?;
                }
            }
        }
        WorkerEvent::Error { worker_id, message } => {
            let obj = WORKERS.with(|w| w.borrow().get(&worker_id).map(|h| h.js_obj.clone()));
            if let Some(obj) = obj {
                let onerror = obj.get(js_string!("onerror"), ctx)?;
                if let Some(callable) = onerror.as_callable() {
                    let msg = JsValue::from(JsString::from(message.as_str()));
                    callable.call(&JsValue::from(obj), &[msg], ctx)?;
                } else {
                    eprintln!("{NAME}: {message}");
                }
            } else {
                eprintln!("{NAME}: {message}");
            }
        }
        WorkerEvent::Done { worker_id } => {
            WORKERS.with(|w| {
                w.borrow_mut().remove(&worker_id);
            });
        }
    }
    Ok(JsValue::undefined())
}

// ============================================================================
// Register globals
// ============================================================================

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

    macro_rules! reg {
        ($name:literal, $arity:expr, $func:expr) => {
            ctx.register_global_callable(
                js_string!($name),
                $arity,
                NativeFunction::from_fn_ptr($func),
            )?;
        };
    }

    reg!("readFile", 1, read_file_native);
    reg!("readFileSync", 1, read_file_native);
    reg!("writeFile", 2, write_file_native);
    reg!("writeFileSync", 2, write_file_native);
    reg!("exit", 1, exit_native);
    reg!("setTimeout", 2, set_timeout_native);
    reg!("clearTimeout", 1, clear_timeout_native);
    reg!("setInterval", 2, set_interval_native);
    reg!("clearInterval", 1, clear_interval_native);
    reg!("queueMicrotask", 1, queue_microtask_native);
    reg!("fetch", 1, fetch_native);
    reg!("Worker", 1, new_worker_native);

    // args
    let arr = JsArray::new(ctx);
    for (i, s) in script_args.iter().enumerate() {
        arr.set(
            i as u32,
            JsValue::from(JsString::from(s.as_str())),
            false,
            ctx,
        )?;
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

// ============================================================================
// Execution
// ============================================================================

fn run_source(source: &str, label: &str, ctx: &mut Context) -> ExitCode {
    match ctx.eval(Source::from_bytes(source.as_bytes())) {
        Ok(_) => {
            queue().run_jobs(ctx);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{NAME}: {label}: {e}");
            ExitCode::from(1)
        }
    }
}

fn is_typescript_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("ts" | "tsx" | "jsx" | "mts" | "cts")
    )
}

fn transpile(source: &str, path: &Path) -> Result<String, String> {
    use oxc_allocator::Allocator;
    use oxc_codegen::Codegen;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;
    use oxc_transformer::{TransformOptions, Transformer};

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| {
        SourceType::default()
            .with_typescript(true)
            .with_module(true)
    });
    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if !parser_ret.errors.is_empty() {
        let msg: Vec<String> = parser_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(msg.join("\n"));
    }
    let mut program = parser_ret.program;

    let semantic_ret = SemanticBuilder::new().with_enum_eval(true).build(&program);
    if !semantic_ret.errors.is_empty() {
        let msg: Vec<String> = semantic_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(msg.join("\n"));
    }
    let scoping = semantic_ret.semantic.into_scoping();

    let mut options = TransformOptions::default();
    options.jsx.runtime = oxc_transformer::JsxRuntime::Classic;
    options.jsx.pragma = Some("h".to_string());
    options.jsx.pragma_frag = Some("Fragment".to_string());
    let transformer_ret = Transformer::new(&allocator, path, &options)
        .build_with_scoping(scoping, &mut program);
    if !transformer_ret.errors.is_empty() {
        let msg: Vec<String> = transformer_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(msg.join("\n"));
    }
    Ok(Codegen::new().build(&program).code)
}

fn is_module_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("mjs" | "mts" | "ts" | "tsx" | "jsx" | "cts")
    )
}

fn run_module(source: String, path: &Path, ctx: &mut Context) -> ExitCode {
    let parsed = match Module::parse(
        Source::from_bytes(source.as_bytes()).with_path(path),
        None,
        ctx,
    ) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{NAME}: {}: parse error: {e}", path.display());
            return ExitCode::from(1);
        }
    };
    let promise = parsed.load_link_evaluate(ctx);
    queue().run_jobs(ctx);
    match promise.state() {
        PromiseState::Pending => {
            eprintln!("{NAME}: {}: module evaluation never resolved", path.display());
            ExitCode::from(1)
        }
        PromiseState::Fulfilled(_) => ExitCode::SUCCESS,
        PromiseState::Rejected(reason) => {
            let s = reason
                .to_string(ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|_| "<unprintable error>".to_string());
            eprintln!("{NAME}: {}: {s}", path.display());
            ExitCode::from(1)
        }
    }
}

fn run_file(path: &Path, ctx: &mut Context) -> ExitCode {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{NAME}: cannot read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };
    let source = if is_typescript_path(path) {
        match transpile(&raw, path) {
            Ok(js) => js,
            Err(e) => {
                eprintln!("{NAME}: {}: transpile error\n{e}", path.display());
                return ExitCode::from(1);
            }
        }
    } else {
        raw
    };
    if is_module_path(path) {
        run_module(source, path, ctx)
    } else {
        run_source(&source, &path.display().to_string(), ctx)
    }
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
                        queue().run_jobs(ctx);
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

// ============================================================================
// Arg parsing
// ============================================================================

#[derive(Default)]
struct ParsedArgs {
    show_help: bool,
    show_version: bool,
    eval_code: Option<String>,
    script: Option<String>,
    script_args: Vec<String>,
    error: Option<String>,
    permissions: Option<Permissions>,
}

fn parse_args(argv: Vec<String>) -> ParsedArgs {
    let mut out = ParsedArgs::default();
    let mut perms = Permissions::all_allowed();
    let mut perms_touched = false;
    let mut iter = argv.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => out.show_help = true,
            "-V" | "--version" => out.show_version = true,
            "-e" | "--eval" => match iter.next() {
                Some(code) => out.eval_code = Some(code),
                _none => {
                    out.error = Some(format!("{arg} requires an argument"));
                    return out;
                }
            },
            // Permission flags. In v1.x, default is allow-all; --deny-* subtracts.
            "--allow-all" | "--allow-read" | "--allow-write" | "--allow-net"
            | "--allow-env" | "--allow-run" => {
                perms_touched = true;
                // No-op in v1.x: default already allows everything.
            }
            "--deny-all" => {
                perms = Permissions {
                    read: false,
                    write: false,
                    net: false,
                    env: false,
                    run: false,
                };
                perms_touched = true;
            }
            "--deny-read" => {
                perms.read = false;
                perms_touched = true;
            }
            "--deny-write" => {
                perms.write = false;
                perms_touched = true;
            }
            "--deny-net" => {
                perms.net = false;
                perms_touched = true;
            }
            "--deny-env" => {
                perms.env = false;
                perms_touched = true;
            }
            "--deny-run" => {
                perms.run = false;
                perms_touched = true;
            }
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
    if perms_touched {
        out.permissions = Some(perms);
    }
    out
}

// ============================================================================
// Main
// ============================================================================

/// Entry point used by the `x8` binary.
///
/// Pass the full argument vector (including `argv[0]`). Returns the
/// process exit code; the caller is responsible for surfacing it via
/// [`std::process::exit`] or returning it from `fn main`.
pub fn run_cli(args: Vec<String>) -> ExitCode {
    let parsed = parse_args(args);

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

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{NAME}: failed to build tokio runtime: {e}");
            return ExitCode::from(1);
        }
    };
    if let Some(p) = parsed.permissions {
        install_permissions(p);
    }
    init_worker_events();
    let job_queue = Rc::new(AsyncJobQueue::new(rt));
    install_queue(job_queue.clone());

    let module_loader = Rc::new(X8ModuleLoader::new());

    let mut ctx = match Context::builder()
        .job_queue(job_queue)
        .module_loader(module_loader)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{NAME}: failed to build context: {e}");
            return ExitCode::from(1);
        }
    };
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
