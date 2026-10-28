//! Datum Calc desktop shell: owns the open sheet, files, options and recovery, and talks to the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod update;

use datum_core::{eval, Doc, Op, Sheet};
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    /// Minutes between recovery saves; 0 turns them off.
    autosave: u32,
    /// Offer beta (pre-release) versions in Check for updates.
    beta: bool,
    /// auto, light or dark.
    theme: String,
    /// Reopen the last sheet at start.
    open_last: bool,
    /// Defaults for new sheets: unit system (SI or US), significant figures, paper.
    units: String,
    digits: u8,
    paper: String,
    /// Defaults for the title block of new sheets.
    company: String,
    by: String,
    logo: String,
    /// Number format: "auto" (plain, engineering above 1e6) or "eng" (always engineering).
    number: String,
}

impl Default for Options {
    fn default() -> Self {
        Self { autosave: 5, beta: false, theme: "auto".into(), open_last: false, units: "SI".into(), digits: 4, paper: "A4".into(), company: String::new(), by: String::new(), logo: String::new(), number: "auto".into() }
    }
}

struct App {
    doc: Doc,
    opts: Options,
}

struct St(Mutex<App>);

impl St {
    /// The open sheet. A command that panicked must not lock every later one out, so a poisoned lock is reused.
    fn lock(&self) -> std::sync::MutexGuard<'_, App> { self.0.lock().unwrap_or_else(|e| e.into_inner()) }
}
type R<T> = Result<T, String>;

/// What the window needs after every change: the sheet and the file state.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Out {
    sheet: Sheet,
    /// The sheet worked out: results, maths, checks, errors and variables.
    calc: eval::SheetOut,
    path: Option<String>,
    dirty: bool,
    undo: bool,
    redo: bool,
    rev: u64,
}

fn out(a: &App) -> Out {
    let d = &a.doc;
    let f = eval::Fmt { digits: d.s.digits, eng: a.opts.number == "eng", us: d.s.units == "US" };
    Out { sheet: d.s.clone(), calc: eval::run_with(&d.s, &f), path: d.path.as_ref().map(|p| p.display().to_string()), dirty: d.dirty, undo: d.can_undo(), redo: d.can_redo(), rev: d.rev() }
}

fn today() -> String { chrono::Local::now().format("%d-%b-%Y").to_string() }

/// A new sheet gets the user's defaults for units, paper and the title block.
fn fresh(a: &App, mut s: Sheet) -> Sheet {
    let o = &a.opts;
    s.units = o.units.clone();
    s.digits = o.digits;
    s.page.paper = o.paper.clone();
    if s.title.company.is_empty() { s.title.company = o.company.clone(); }
    if s.title.by.is_empty() { s.title.by = o.by.clone(); }
    if s.title.logo.is_empty() { s.title.logo = o.logo.clone(); }
    if s.title.date.is_empty() { s.title.date = today(); }
    if s.title.rev.is_empty() { s.title.rev = "A".into(); }
    s.normalize();
    s
}

fn dir(app: &AppHandle, data: bool) -> Option<PathBuf> {
    let d = if data { app.path().app_data_dir() } else { app.path().app_config_dir() }.ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

#[tauri::command]
fn state(st: State<St>) -> Out { out(&st.lock()) }

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Out> {
    let mut a = st.lock();
    a.doc.apply(op)?;
    Ok(out(&a))
}

#[tauri::command]
fn history(st: State<St>, redo: bool) -> Out {
    let mut a = st.lock();
    if redo { a.doc.redo() } else { a.doc.undo() }
    out(&a)
}

#[tauri::command]
fn new_sheet(st: State<St>) -> Out {
    let mut a = st.lock();
    let s = fresh(&a, Sheet::new("Calc1"));
    a.doc = Doc::new(s);
    out(&a)
}

/// The calc packs for File › New from pack.
#[tauri::command]
fn packs() -> Vec<datum_core::packs::Card> { datum_core::packs::cards() }

#[tauri::command]
fn new_from_pack(st: State<St>, id: String) -> R<Out> {
    let s = datum_core::packs::sheet(&id).ok_or("That calc pack is not in this version.")?;
    let mut a = st.lock();
    let mut s = fresh(&a, s);
    s.units = "SI".into();
    a.doc = Doc::new(s);
    Ok(out(&a))
}

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Out>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Calc", &["dcalc"])) else { return Ok(None) };
    let d = Doc::open(&path)?;
    remember(&app, &path);
    let mut a = st.lock();
    a.doc = d;
    Ok(Some(out(&a)))
}

#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, as_new: bool) -> R<Option<Out>> {
    let (known, name) = {
        let a = st.lock();
        (a.doc.path.clone().filter(|_| !as_new), a.doc.s.name.clone())
    };
    let path = match known {
        Some(p) => p,
        None => match app.dialog().file().add_filter("Datum Calc", &["dcalc"]).set_file_name(format!("{name}.dcalc")).blocking_save_file() {
            Some(f) => f.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let mut a = st.lock();
    if as_new || a.doc.path.is_none() {
        if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) { a.doc.s.name = stem; }
    }
    a.doc.save(&path)?;
    remember(&app, &path);
    clear_recovery(&app);
    Ok(Some(out(&a)))
}

/// Saves a file the window made (CSV, SVG, PNG data URL or HTML text).
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

fn base64_decode(s: &str) -> R<Vec<u8>> {
    let val = |c: u8| match c { b'A'..=b'Z' => Some(c - b'A'), b'a'..=b'z' => Some(c - b'a' + 26), b'0'..=b'9' => Some(c - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = acc << 6 | val(c).ok_or("The picture data is damaged.")? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Reads a picture (logo or image region) and returns it as a data URL.
#[tauri::command]
async fn pick_image(app: AppHandle) -> R<Option<String>> {
    let Some(path) = pick(&app, "Pictures", &["png", "jpg", "jpeg", "svg", "gif"]) else { return Ok(None) };
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() > 4_000_000 {
        return Err("Choose a picture smaller than 4 MB.".into());
    }
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() { "svg" => "image/svg+xml", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif", _ => "image/png" };
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for ch in bytes.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            b64.push(if k <= ch.len() { T[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
        }
    }
    Ok(Some(format!("data:{mime};base64,{b64}")))
}

fn options_file(app: &AppHandle) -> Option<PathBuf> { dir(app, false).map(|d| d.join("options.json")) }

#[tauri::command]
fn options(st: State<St>) -> Options { st.lock().opts.clone() }

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, opts: Options) -> Out {
    if let Some(f) = options_file(&app) {
        let _ = std::fs::write(f, serde_json::to_string(&opts).unwrap_or_default());
    }
    let mut a = st.lock();
    a.opts = opts;
    out(&a)
}

// ---------- recovery: a copy of unsaved work, kept until it is saved or discarded ----------
#[derive(Serialize, Deserialize)]
struct Recovery { name: String, path: Option<String>, saved: String }

fn recovery_files(app: &AppHandle) -> Option<(PathBuf, PathBuf)> { dir(app, true).map(|d| (d.join("recovery.dcalc"), d.join("recovery.json"))) }

fn clear_recovery(app: &AppHandle) {
    if let Some((a, b)) = recovery_files(app) {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
}

#[tauri::command]
fn autosave(app: AppHandle, st: State<St>) {
    let a = st.lock();
    if let (true, Some((sheet, meta))) = (a.doc.dirty, recovery_files(&app)) {
        let info = Recovery { name: a.doc.s.name.clone(), path: a.doc.path.as_ref().map(|p| p.display().to_string()), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = std::fs::write(sheet, a.doc.s.json());
        let _ = std::fs::write(meta, serde_json::to_string(&info).unwrap_or_default());
    }
}

#[tauri::command]
fn recovery(app: AppHandle) -> Option<Recovery> {
    let (sheet, meta) = recovery_files(&app)?;
    sheet.exists().then_some(())?;
    serde_json::from_str(&std::fs::read_to_string(meta).ok()?).ok()
}

#[tauri::command]
fn recover(app: AppHandle, st: State<St>, keep: bool) -> R<Option<Out>> {
    let info = recovery(app.clone());
    let files = recovery_files(&app);
    if !keep {
        clear_recovery(&app);
        return Ok(None);
    }
    let (sheet, _) = files.ok_or("No recovered sheet was found.")?;
    let mut d = Doc::open(&sheet)?;
    d.path = info.and_then(|i| i.path).map(PathBuf::from);
    d.dirty = true;
    let mut a = st.lock();
    a.doc = d;
    Ok(Some(out(&a)))
}

#[derive(Serialize)]
struct Recent { path: String, name: String, modified: String }

fn recent_paths(app: &AppHandle) -> Vec<String> {
    options_file(app).map(|f| f.with_file_name("recent.json")).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(app: &AppHandle, path: &Path) {
    let p = path.display().to_string();
    let mut v = recent_paths(app);
    v.retain(|x| *x != p);
    v.insert(0, p);
    v.truncate(15);
    if let Some(f) = options_file(app).map(|f| f.with_file_name("recent.json")) {
        let _ = std::fs::write(f, serde_json::to_string(&v).unwrap_or_default());
    }
}

#[tauri::command]
fn recent(app: AppHandle) -> Vec<Recent> {
    recent_paths(&app)
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            let modified = chrono::DateTime::<chrono::Local>::from(m).format("%d-%b-%y %H:%M").to_string();
            let name = Path::new(&p).file_stem()?.to_string_lossy().into_owned();
            Some(Recent { path: p, name, modified })
        })
        .collect()
}

/// A .dcalc path passed on the command line (double-clicked in Explorer), or the last sheet.
#[tauri::command]
fn startup_file(app: AppHandle, st: State<St>) -> Option<String> {
    let last = || if st.lock().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
    std::env::args().skip(1).find(|a| a.to_lowercase().ends_with(".dcalc")).or_else(last)
}

/// Opens one of the project's GitHub pages or the family site in the browser (only those).
#[tauri::command]
fn open_url(url: String) -> R<()> {
    if !url.starts_with("https://github.com/Edgar-Mendonca/Datum-Calc") && !url.starts_with("https://edgar-mendonca.github.io/Datum/") {
        return Err("Only the Datum Calc GitHub pages can be opened.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// Every unit, constant and function, for the Units and Help lists.
#[derive(Serialize)]
struct Lists { units: Vec<(&'static str, &'static str, &'static str, String)>, constants: Vec<(&'static str, &'static str)>, functions: Vec<(&'static str, &'static str)> }

#[tauri::command]
fn lists() -> Lists {
    Lists { units: datum_core::units::list(), constants: eval::CONSTANTS.to_vec(), functions: eval::FUNCTIONS.to_vec() }
}

#[tauri::command]
fn version() -> &'static str { env!("CARGO_PKG_VERSION") }

#[tauri::command]
fn quit(app: AppHandle) {
    clear_recovery(&app);
    app.exit(0);
}

#[tauri::command]
async fn check_update(st: State<'_, St>) -> R<update::Info> {
    let beta = st.lock().opts.beta;
    tauri::async_runtime::spawn_blocking(move || update::check(beta)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn install_update(app: AppHandle, url: String, sums: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || update::install(&app, &url, sums.as_deref())).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let opts: Options = options_file(app.handle()).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let mut a = App { doc: Doc::new(Sheet::new("Calc1")), opts };
            a.doc = Doc::new(fresh(&a, Sheet::new("Calc1")));
            app.manage(St(Mutex::new(a)));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().lock().doc.dirty {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, apply, history, new_sheet, packs, new_from_pack, open, save, save_file, pick_image,
            lists, options, set_options, autosave, recovery, recover, recent, startup_file, version, open_url, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Calc failed to start");
}
