//! `fid studio` — the workbench's local source.
//!
//! Studio is a static app that edits a product through a **source**: either this
//! server (files on disk, nothing leaves the machine) or GitHub's API (a phone,
//! a browser, no laptop). The app does not know which; both answer the same
//! small contract. This file is the first of them.
//!
//! The contract, all under `/api` and all behind a per-run token:
//!
//! | Route | Does |
//! |---|---|
//! | `GET  /api/dash` | the `fid dash --json` view |
//! | `GET  /api/schema` | JSON Schema for `fiducial.toml`, generated from the Rust types the CLI itself parses with |
//! | `GET  /api/files` | the files a workbench may edit |
//! | `GET  /api/file?path=` | one file's text |
//! | `PUT  /api/file?path=` | replace one file's text |
//! | `POST /api/derive[?check=1]` | run `fid derive` |
//! | `GET  /api/dev` · `POST /api/dev/start` · `POST /api/dev/stop` | the product's dev server, for the Preview pane |
//!
//! # Why hand-rolled HTTP on `std`
//!
//! The contract is eight routes serving one person on loopback. An async
//! runtime and a web framework would be the largest dependency in the CLI for
//! the smallest job in it. What matters is the *surface*, not the transport:
//! it is JSON over HTTP, so the GitHub source implements the same routes
//! from a browser and the app cannot tell them apart.
//!
//! # What keeps a local server safe
//!
//! It can write files and run commands, so it is treated as such:
//!
//! - **Loopback only**, and every request must carry the run's random token
//!   (`Authorization: Bearer`, or `?token=` for the first page load).
//! - **`Host` must be loopback** — a page on another site cannot reach this
//!   through DNS rebinding.
//! - **Paths are confined** to the product, refuse `..`, symlink escapes, `.git`,
//!   and secrets (`.env*`, `*.pem`, `*.key`), and writes are limited to text.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    commands::dash::Dash,
    config::{Config, CONFIG_FILE},
    lock::sha256_hex,
};

/// Largest request body accepted. A declaration is kilobytes; this is a ceiling
/// on abuse, not a target.
const MAX_BODY: usize = 4 * 1024 * 1024;
const LOG_LINES: usize = 400;

/// Directories never listed and never reachable.
const HIDDEN_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    ".next",
    ".turbo",
    ".wrangler",
];

/// The declarations Studio has a schema for. Each is what a form is generated
/// from, so adding a declaration to Studio is adding a name here.
pub const SCHEMAS: [&str; 2] = ["fiducial", "topology"];

/// The JSON Schema for a declaration, by name.
pub fn named_schema(name: &str) -> Option<Value> {
    let schema = match name {
        "fiducial" => schemars::schema_for!(Config),
        "topology" => schemars::schema_for!(crate::topology::Topology),
        _ => return None,
    };
    Some(serde_json::to_value(schema).expect("a schema is JSON"))
}

// ── Path confinement ──────────────────────────────────────────────────────────

/// Resolve `rel` inside `root`, or say why not. The one function every file
/// route goes through.
pub fn confine(root: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() || rel.contains('\0') {
        bail!("empty or malformed path");
    }
    let p = Path::new(rel);
    for c in p.components() {
        match c {
            Component::Normal(name) => {
                let n = name.to_string_lossy();
                if HIDDEN_DIRS.contains(&n.as_ref()) {
                    bail!("`{n}` is not editable here");
                }
                if is_secret(&n) {
                    bail!("`{n}` looks like a secret and is never served");
                }
            }
            Component::CurDir => {}
            _ => bail!("path must be relative to the product, with no `..`"),
        }
    }
    let joined = root.join(p);
    // A symlink inside the product must not lead out of it. Canonicalise the
    // deepest ancestor that exists (the file itself may be new).
    let mut probe = joined.as_path();
    while !probe.exists() {
        probe = probe.parent().context("path has no existing ancestor")?;
    }
    let real = probe.canonicalize()?;
    if !real.starts_with(root.canonicalize()?) {
        bail!("path leaves the product");
    }
    Ok(joined)
}

fn is_secret(name: &str) -> bool {
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name == ".dev.vars"
}

/// Text files a workbench can edit: everything that is not generated bulk.
pub fn list_files(root: &Path) -> Result<Vec<String>> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            let ft = e.file_type()?;
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                if !HIDDEN_DIRS.contains(&name.as_str()) {
                    walk(&path, root, out)?;
                }
            } else if !is_secret(&name) && is_text_ext(&name) {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn is_text_ext(name: &str) -> bool {
    const EXT: &[&str] = &[
        "toml", "md", "json", "css", "ts", "tsx", "js", "jsx", "mjs", "svelte", "astro", "html",
        "yml", "yaml", "sql", "rs", "txt", "mdx",
    ];
    name.rsplit_once('.')
        .is_some_and(|(_, e)| EXT.contains(&e.to_ascii_lowercase().as_str()))
}

// ── The dev server (for the Preview pane) ─────────────────────────────────────

/// Process group of the running dev server, so a signal handler (which can do
/// almost nothing safely) can still stop it. 0 means none.
static DEV_PGIDS: [std::sync::atomic::AtomicI32; TARGETS.len()] =
    [const { std::sync::atomic::AtomicI32::new(0) }; TARGETS.len()];

/// The servers Studio can run: the product's site, and its content editor.
/// Each is a slot, so previewing the site and editing content can run at once.
pub const TARGETS: [&str; 2] = ["web", "cms"];

/// Stop the dev server's whole group. `pnpm dev` is a shell, which starts pnpm,
/// which starts node, which starts the server: killing only the first leaves the
/// last one running and holding the port.
#[cfg(unix)]
fn kill_group(pgid: i32) {
    if pgid > 0 {
        // SAFETY: killpg takes plain integers and has no memory effects.
        unsafe { libc::killpg(pgid, libc::SIGTERM) };
    }
}

#[cfg(unix)]
extern "C" fn on_signal(_: libc::c_int) {
    for g in &DEV_PGIDS {
        kill_group(g.load(std::sync::atomic::Ordering::SeqCst));
    }
    // SAFETY: `_exit` is async-signal-safe; ordinary exit is not.
    unsafe { libc::_exit(0) };
}

/// Ctrl-C and `kill` must take the dev server with Studio, not orphan it.
#[cfg(unix)]
fn install_signal_handlers() {
    // SAFETY: the handler only calls async-signal-safe functions.
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
}

#[derive(Default)]
struct Dev {
    child: Option<Child>,
    log: VecDeque<String>,
    port: Option<u16>,
    command: String,
}

/// What to run for the preview. `[studio] dev` would be declared; until a
/// product does, the web app's own `dev` script is the answer.
fn dev_command(root: &Path, target: &str) -> Option<String> {
    let dir = format!("apps/{target}");
    let pkg = root.join(&dir).join("package.json");
    let has_dev = std::fs::read_to_string(&pkg)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .is_some_and(|v| v["scripts"]["dev"].is_string());
    has_dev.then(|| format!("pnpm --filter ./{dir} dev"))
}

/// `localhost:3000`, `127.0.0.1:5173` and `http://[::1]:4321/` all appear in
/// dev-server banners. Take the first port that follows a loopback host.
fn find_port(line: &str) -> Option<u16> {
    for host in ["localhost:", "127.0.0.1:", "[::1]:", "0.0.0.0:"] {
        if let Some(i) = line.find(host) {
            let digits: String = line[i + host.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if let Ok(p) = digits.parse::<u16>() {
                return Some(p);
            }
        }
    }
    None
}

fn pump(dev: Arc<Mutex<Dev>>, src: impl Read + Send + 'static) {
    thread::spawn(move || {
        for line in BufReader::new(src).lines().map_while(|l| l.ok()) {
            let mut d = dev.lock().unwrap();
            if d.port.is_none() {
                d.port = find_port(&line);
            }
            if d.log.len() == LOG_LINES {
                d.log.pop_front();
            }
            d.log.push_back(strip_ansi(&line));
        }
    });
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn dev_status(dev: &Arc<Mutex<Dev>>) -> Value {
    let mut d = dev.lock().unwrap();
    let running = match d.child.as_mut() {
        Some(c) => matches!(c.try_wait(), Ok(None)),
        None => false,
    };
    json!({
        "running": running,
        "command": d.command,
        "port": d.port,
        "url": d.port.map(|p| format!("http://localhost:{p}/")),
        "log": d.log.iter().collect::<Vec<_>>(),
    })
}

fn dev_start(root: &Path, target: &str, slot: usize, dev: &Arc<Mutex<Dev>>) -> Result<()> {
    if dev_status(dev)["running"] == json!(true) {
        return Ok(());
    }
    let cmd = dev_command(root, target).with_context(|| {
        format!("no dev server to run: the product has no `apps/{target}` with a `dev` script")
    })?;
    let mut command = Command::new("sh");
    command
        .args(["-c", &cmd])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Its own process group, so Stop can reach every process it started.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .spawn()
        .with_context(|| format!("starting `{cmd}`"))?;
    #[cfg(unix)]
    DEV_PGIDS[slot].store(child.id() as i32, std::sync::atomic::Ordering::SeqCst);
    let (out, err) = (child.stdout.take(), child.stderr.take());
    {
        let mut d = dev.lock().unwrap();
        *d = Dev {
            child: Some(child),
            command: cmd,
            ..Dev::default()
        };
    }
    if let Some(o) = out {
        pump(dev.clone(), o);
    }
    if let Some(e) = err {
        pump(dev.clone(), e);
    }
    Ok(())
}

fn dev_stop(slot: usize, dev: &Arc<Mutex<Dev>>) {
    let mut d = dev.lock().unwrap();
    if let Some(mut c) = d.child.take() {
        #[cfg(unix)]
        {
            kill_group(c.id() as i32);
            DEV_PGIDS[slot].store(0, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = c.kill();
        let _ = c.wait();
    }
    d.port = None;
}

// ── HTTP ──────────────────────────────────────────────────────────────────────

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    fn q(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().context("empty request")?.to_string();
    let target = parts.next().context("no target")?;
    let (path, qs) = target.split_once('?').unwrap_or((target, ""));
    let query = qs
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect();
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let len = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len > MAX_BODY {
        bail!("body too large");
    }
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    Ok(Request {
        method,
        path: percent_decode(path),
        query,
        headers,
        body,
    })
}

fn respond(stream: &mut TcpStream, status: u16, ctype: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn json_resp(stream: &mut TcpStream, status: u16, v: &Value) {
    respond(stream, status, "application/json", v.to_string().as_bytes());
}

fn loopback_host(host: &str) -> bool {
    let h = host.rsplit_once(':').map_or(host, |(h, _)| h);
    matches!(h, "localhost" | "127.0.0.1" | "[::1]")
}

struct State {
    root: PathBuf,
    token: String,
    /// One dev server per entry of [`TARGETS`].
    dev: Vec<Arc<Mutex<Dev>>>,
    /// The built Studio app, when one can be found. Public files, no secrets.
    app: Option<PathBuf>,
}

/// Where the built app is. An explicit `--app`, then `FID_STUDIO_APP`, then the
/// two places a build lands: installed as `@fiducial/studio`, or built in this
/// repository's workspace.
fn find_app(root: &Path, explicit: Option<PathBuf>) -> Option<PathBuf> {
    let from_env = std::env::var_os("FID_STUDIO_APP").map(PathBuf::from);
    let mut candidates: Vec<PathBuf> = explicit.into_iter().chain(from_env).collect();
    candidates.push(root.join("node_modules/@fiducial/studio/dist"));
    candidates.push(root.join("packages/studio/dist"));
    candidates
        .into_iter()
        .find(|d| d.join("index.html").is_file())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Serve a file of the built app. Unknown paths fall back to `index.html` so a
/// client-side route survives a reload; a path that tries to leave the app
/// directory does not.
fn serve_static(stream: &mut TcpStream, app: &Path, url_path: &str) {
    let rel = url_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let safe = Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)));
    let mut file = app.join(rel);
    if !safe || !file.is_file() {
        if !safe || rel.contains('.') {
            return respond(stream, 404, "text/plain", b"not found");
        }
        file = app.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(bytes) => respond(stream, 200, content_type(&file), &bytes),
        Err(_) => respond(stream, 404, "text/plain", b"not found"),
    }
}

fn handle(mut stream: TcpStream, st: Arc<State>) {
    let Ok(req) = read_request(&mut stream) else {
        return respond(&mut stream, 400, "text/plain", b"bad request");
    };
    if !req.header("host").is_some_and(loopback_host) {
        return respond(&mut stream, 403, "text/plain", b"forbidden host");
    }
    if req.method == "GET" && !req.path.starts_with("/api/") {
        return match &st.app {
            Some(app) => serve_static(&mut stream, app, &req.path),
            None => respond(
                &mut stream,
                404,
                "text/plain",
                b"Studio's app is not built here. Build packages/studio, or pass --app <dir>.",
            ),
        };
    }
    let bearer = req
        .header("authorization")
        .and_then(|a| a.strip_prefix("Bearer "));
    let authed = bearer == Some(st.token.as_str()) || req.q("token") == Some(st.token.as_str());
    if !authed {
        return respond(&mut stream, 401, "text/plain", b"missing or wrong token");
    }
    match route(&req, &st) {
        Ok((status, v)) => json_resp(&mut stream, status, &v),
        Err(e) => json_resp(&mut stream, 400, &json!({ "error": format!("{e:#}") })),
    }
}

fn route(req: &Request, st: &State) -> Result<(u16, Value)> {
    let root = st.root.as_path();
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/dash") => {
            let config = Config::load(&root.join(CONFIG_FILE))?;
            let dash = Dash::collect(root, &config)?;
            Ok((200, serde_json::to_value(dash)?))
        }
        ("GET", "/api/schema") => {
            let name = req.q("name").unwrap_or("fiducial");
            let schema = named_schema(name).with_context(|| {
                format!(
                    "no schema `{name}` (expected one of: {})",
                    SCHEMAS.join(", ")
                )
            })?;
            Ok((200, schema))
        }
        ("GET", "/api/files") => Ok((200, json!({ "files": list_files(root)? }))),
        ("GET", "/api/file") => {
            let rel = req.q("path").context("`path` is required")?;
            let text = std::fs::read_to_string(confine(root, rel)?)
                .with_context(|| format!("reading {rel}"))?;
            Ok((200, json!({ "path": rel, "text": text })))
        }
        ("PUT", "/api/file") => {
            let rel = req.q("path").context("`path` is required")?;
            if !is_text_ext(rel) {
                bail!("only text files are written");
            }
            let text = std::str::from_utf8(&req.body).context("body is not UTF-8")?;
            let path = confine(root, rel)?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, text).with_context(|| format!("writing {rel}"))?;
            Ok((
                200,
                json!({ "path": rel, "bytes": text.len(), "sha256": sha256_hex(text.as_bytes()) }),
            ))
        }
        ("POST", "/api/derive") => {
            let mut cmd = Command::new(std::env::current_exe()?);
            cmd.arg("derive").current_dir(root);
            if req.q("check").is_some() {
                cmd.arg("--check");
            }
            let out = cmd.output().context("running fid derive")?;
            Ok((
                200,
                json!({
                    "ok": out.status.success(),
                    "stdout": String::from_utf8_lossy(&out.stdout),
                    "stderr": String::from_utf8_lossy(&out.stderr),
                }),
            ))
        }
        ("GET", "/api/dev") => Ok((200, dev_status(&st.dev[target_slot(req)?]))),
        ("POST", "/api/dev/start") => {
            let slot = target_slot(req)?;
            dev_start(root, TARGETS[slot], slot, &st.dev[slot])?;
            Ok((200, dev_status(&st.dev[slot])))
        }
        ("POST", "/api/dev/stop") => {
            let slot = target_slot(req)?;
            dev_stop(slot, &st.dev[slot]);
            Ok((200, dev_status(&st.dev[slot])))
        }
        _ => Ok((404, json!({ "error": "no such route" }))),
    }
}

fn new_devs() -> Vec<Arc<Mutex<Dev>>> {
    TARGETS
        .iter()
        .map(|_| Arc::new(Mutex::new(Dev::default())))
        .collect()
}

/// `?target=cms`, defaulting to the site.
fn target_slot(req: &Request) -> Result<usize> {
    let t = req.q("target").unwrap_or("web");
    TARGETS.iter().position(|x| *x == t).with_context(|| {
        format!(
            "unknown target `{t}` (expected one of: {})",
            TARGETS.join(", ")
        )
    })
}

/// A token nobody can guess: kernel randomness where it exists, otherwise the
/// clock and pid hashed (loopback plus the `Host` check is the second line).
fn make_token() -> String {
    let mut buf = [0u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok()
    {
        return hex::encode(buf);
    }
    let seed = format!("{:?}{}", std::time::SystemTime::now(), std::process::id());
    sha256_hex(seed.as_bytes())[..32].to_string()
}

// ── Entry points ──────────────────────────────────────────────────────────────

/// `fid studio schema [name]` — print a declaration's JSON Schema.
///
/// Also what an editor's TOML language server wants for completion and
/// validation, so it is useful with no Studio running at all.
pub fn schema(name: &str) -> Result<()> {
    let schema = named_schema(name).with_context(|| {
        format!(
            "no schema `{name}` (expected one of: {})",
            SCHEMAS.join(", ")
        )
    })?;
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}

/// `fid studio` — serve the contract on loopback.
pub fn serve(port: u16, app: Option<PathBuf>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("binding 127.0.0.1:{port}"))?;
    let addr = listener.local_addr()?;
    #[cfg(unix)]
    install_signal_handlers();
    let root_for_app = root.clone();
    let st = Arc::new(State {
        root,
        token: make_token(),
        dev: new_devs(),
        app: find_app(&root_for_app, app),
    });
    match &st.app {
        Some(dir) => println!(
            "Studio  http://{addr}/#token={}\n(app: {} · Ctrl-C to stop)",
            st.token,
            dir.display()
        ),
        None => println!(
            "Studio source on http://{addr}/  ·  token {}\nThe app is not built (packages/studio, or --app <dir>); the API works without it.",
            st.token
        ),
    }
    for conn in listener.incoming().flatten() {
        let st = st.clone();
        thread::spawn(move || handle(conn, st));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confine_refuses_escape_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        assert!(confine(r, "fiducial.toml").is_ok());
        assert!(confine(r, "a/b/new.toml").is_ok());
        for bad in [
            "../x",
            "/etc/passwd",
            ".env",
            "apps/.env.local",
            ".git/config",
            "a/node_modules/x",
            "k.pem",
            "",
        ] {
            assert!(confine(r, bad).is_err(), "{bad} must be refused");
        }
    }

    #[cfg(unix)]
    #[test]
    fn confine_refuses_symlink_out() {
        let dir = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(out.path(), dir.path().join("link")).unwrap();
        assert!(confine(dir.path(), "link/secret.txt").is_err());
    }

    #[test]
    fn ports_are_found_in_banners() {
        assert_eq!(find_port("  ▲ Local: http://localhost:3000"), Some(3000));
        assert_eq!(find_port("Local: http://127.0.0.1:5173/"), Some(5173));
        assert_eq!(find_port("ready on http://[::1]:4321/"), Some(4321));
        assert_eq!(find_port("compiled in 2s"), None);
    }

    #[test]
    fn ansi_is_stripped() {
        assert_eq!(strip_ansi("\u{1b}[32mready\u{1b}[0m"), "ready");
    }

    #[test]
    fn the_schema_names_the_blocks() {
        let s = named_schema("fiducial").unwrap();
        let props = &s["properties"];
        for k in ["product", "capabilities", "design", "deploy"] {
            assert!(props.get(k).is_some(), "schema lacks `{k}`");
        }
    }

    #[test]
    fn dev_targets_are_named_and_unknown_ones_refused() {
        let req = |q: &str| Request {
            method: "GET".into(),
            path: "/api/dev".into(),
            query: q
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap();
                    (k.to_string(), v.to_string())
                })
                .collect(),
            headers: vec![],
            body: vec![],
        };
        assert_eq!(target_slot(&req("")).unwrap(), 0, "the site is the default");
        assert_eq!(target_slot(&req("target=cms")).unwrap(), 1);
        assert!(target_slot(&req("target=db")).is_err());
    }

    #[test]
    fn only_loopback_hosts_pass() {
        assert!(loopback_host("localhost:7777"));
        assert!(loopback_host("127.0.0.1:1"));
        assert!(!loopback_host("evil.example:7777"));
        assert!(!loopback_host("localhost.evil.com"));
    }

    #[test]
    fn server_enforces_token_and_serves_files() {
        let dir = tempfile::tempdir().unwrap();
        let app_dir = tempfile::tempdir().unwrap();
        std::fs::write(app_dir.path().join("index.html"), "<p>studio</p>").unwrap();
        std::fs::write(dir.path().join("a.toml"), "x = 1\n").unwrap();
        let st = Arc::new(State {
            root: dir.path().to_path_buf(),
            token: "t0k".into(),
            dev: new_devs(),
            app: Some(app_dir.path().to_path_buf()),
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let s2 = st.clone();
        thread::spawn(move || {
            for c in listener.incoming().flatten() {
                let s = s2.clone();
                thread::spawn(move || handle(c, s));
            }
        });
        let call = |method: &str, target: &str, auth: bool, host: &str, body: &str| {
            let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let a = if auth {
                "Authorization: Bearer t0k\r\n"
            } else {
                ""
            };
            write!(
                c,
                "{method} {target} HTTP/1.1\r\nHost: {host}\r\n{a}Content-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut out = String::new();
            c.read_to_string(&mut out).unwrap();
            out
        };
        let h = format!("localhost:{port}");
        assert!(call("GET", "/api/file?path=a.toml", false, &h, "").starts_with("HTTP/1.1 401"));
        assert!(
            call("GET", "/api/file?path=a.toml", true, "evil.example", "")
                .starts_with("HTTP/1.1 403")
        );
        let ok = call("GET", "/api/file?path=a.toml", true, &h, "");
        assert!(ok.contains("x = 1"), "{ok}");
        let put = call("PUT", "/api/file?path=b.toml", true, &h, "y = 2\n");
        assert!(put.starts_with("HTTP/1.1 200"), "{put}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.toml")).unwrap(),
            "y = 2\n"
        );
        assert!(
            call("GET", "/api/file?path=..%2Fetc%2Fpasswd", true, &h, "")
                .starts_with("HTTP/1.1 400")
        );
        assert!(call("GET", "/api/files", true, &h, "").contains("a.toml"));
        // The app itself is public; a client-side route falls back to the shell;
        // nothing outside the app directory is reachable.
        assert!(call("GET", "/", false, &h, "").contains("<p>studio</p>"));
        assert!(call("GET", "/declarations", false, &h, "").contains("<p>studio</p>"));
        assert!(call("GET", "/missing.js", false, &h, "").starts_with("HTTP/1.1 404"));
        assert!(call("GET", "/..%2Fa.toml", false, &h, "").starts_with("HTTP/1.1 404"));
    }

    /// The schemas the app bundles must be the ones the CLI parses with.
    #[test]
    fn the_bundled_schemas_are_current() {
        for name in SCHEMAS {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../packages/studio/src/schemas/{name}.schema.json"
            ));
            let committed: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|_| {
                    panic!(
                        "{} is missing: run `fid studio schema {name}`",
                        path.display()
                    )
                }))
                .unwrap();
            assert_eq!(
                committed,
                named_schema(name).unwrap(),
                "packages/studio/src/schemas/{name}.schema.json is stale: run `fid studio schema {name} > packages/studio/src/schemas/{name}.schema.json`"
            );
        }
    }
}
