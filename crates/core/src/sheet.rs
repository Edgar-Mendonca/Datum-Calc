//! The calc sheet: regions top to bottom, the title block and the page; edit operations with
//! undo/redo, and .dcalc files (JSON with a format version).
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The .dcalc format written by this version. Older files are migrated in `Sheet::from_json`.
pub const FORMAT: u32 = 1;
const UNDO_LIMIT: usize = 200;
const MAX_FILE: u64 = 64_000_000;
/// Limits a sheet read from a file is held to (beyond them the rest is left out).
const MAX_REGIONS: usize = 5000;
const MAX_TEXT: usize = 20_000;
const MAX_PICTURE: usize = 8_000_000;
const MAX_ROWS: usize = 2000;
const MAX_COLS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Math,
    Text,
    Check,
    Solve,
    Plot,
    Table,
    Image,
}

/// One region of the sheet. `text` is what the user types (an equation, a paragraph, a check);
/// `unit` is the unit its result is shown in (empty: chosen automatically); `style` marks inputs,
/// headings and references; `data` holds what a kind needs beyond text (plot ranges, table cells, a picture).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Region {
    pub id: u32,
    pub kind: Kind,
    pub text: String,
    pub unit: String,
    pub style: String,
    pub data: serde_json::Value,
}

/// The block printed at the top of every calc sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TitleBlock {
    pub project: String,
    pub number: String,
    pub title: String,
    pub by: String,
    pub checked: String,
    pub date: String,
    pub rev: String,
    pub company: String,
    pub client: String,
    pub logo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    pub paper: String,
    /// Top, right, bottom, left in millimetres.
    pub margins: [f64; 4],
}

impl Default for Page {
    fn default() -> Self { Self { paper: "A4".into(), margins: [15.0, 15.0, 15.0, 20.0] } }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sheet {
    pub format: u32,
    pub name: String,
    pub title: TitleBlock,
    pub page: Page,
    /// "SI" or "US": the default units results are shown in.
    pub units: String,
    /// Significant figures for results.
    pub digits: u8,
    /// Show each equation again with its numbers put in (P = ρgQH/η = 998 · 9.807 · … = 19.57 kW).
    pub subst: bool,
    /// Show complex values as magnitude and angle (5∠53.13°) instead of 3 + 4j.
    pub polar: bool,
    pub regions: Vec<Region>,
    pub next_id: u32,
    /// Window settings saved with the sheet (zoom, panels).
    pub ui: serde_json::Value,
}

impl Default for Sheet {
    fn default() -> Self {
        Self { format: FORMAT, name: "Calc1".into(), title: TitleBlock::default(), page: Page::default(), units: "SI".into(), digits: 4, subst: false, polar: false, regions: vec![], next_id: 1, ui: serde_json::Value::Null }
    }
}

impl Sheet {
    pub fn new(name: &str) -> Self { Self { name: name.into(), ..Default::default() } }

    pub fn idx(&self, id: u32) -> Option<usize> { self.regions.iter().position(|r| r.id == id) }

    /// Reads a .dcalc file. A file that is not JSON fails with a clear message; a file whose regions
    /// are partly damaged opens with every region that can be read (the rest are dropped).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("the file is damaged ({e})"))?;
        let o = v.as_object().ok_or("the file holds no calc sheet")?;
        if o.get("format").and_then(|f| f.as_u64()).is_some_and(|f| f > FORMAT as u64) {
            return Err("it was saved by a newer Datum Calc. Update Datum Calc to open it.".into());
        }
        let mut s: Sheet = match serde_json::from_value(v.clone()) {
            Ok(s) => s,
            Err(_) => {
                // Field by field, so one damaged value does not lose the whole sheet.
                let mut s = Sheet::default();
                let get = |k: &str| o.get(k).cloned().unwrap_or(serde_json::Value::Null);
                if let Ok(n) = serde_json::from_value(get("name")) { s.name = n; }
                if let Ok(t) = serde_json::from_value(get("title")) { s.title = t; }
                if let Ok(p) = serde_json::from_value(get("page")) { s.page = p; }
                if let Ok(u) = serde_json::from_value(get("units")) { s.units = u; }
                if let Ok(d) = serde_json::from_value(get("digits")) { s.digits = d; }
                s.regions = get("regions").as_array().map(|a| a.iter().filter_map(|r| serde_json::from_value(r.clone()).ok()).collect()).unwrap_or_default();
                s
            }
        };
        s.format = FORMAT;
        s.normalize();
        Ok(s)
    }

    /// Keeps the invariants every reader and edit relies on: unique non-zero ids, next_id above them all,
    /// sane digits and units.
    pub fn normalize(&mut self) {
        self.regions.truncate(MAX_REGIONS);
        for r in &mut self.regions {
            if r.text.len() > MAX_TEXT { let mut k = MAX_TEXT; while !r.text.is_char_boundary(k) { k -= 1; } r.text.truncate(k); }
            if r.data.get("src").and_then(|v| v.as_str()).is_some_and(|v| v.len() > MAX_PICTURE) { r.data["src"] = serde_json::Value::Null; }
            if let Some(rows) = r.data.get_mut("rows").and_then(|v| v.as_array_mut()) { rows.truncate(MAX_ROWS); }
            if let Some(cols) = r.data.get_mut("cols").and_then(|v| v.as_array_mut()) { cols.truncate(MAX_COLS); }
        }
        let mut seen = std::collections::HashSet::new();
        let mut next = self.regions.iter().map(|r| r.id).max().unwrap_or(0).max(self.next_id.saturating_sub(1));
        for r in &mut self.regions {
            if r.id == 0 || !seen.insert(r.id) {
                next = next.saturating_add(1);
                r.id = next;
                seen.insert(r.id);
            }
        }
        self.next_id = next.saturating_add(1).max(1);
        self.digits = self.digits.clamp(2, 10);
        if self.units != "US" { self.units = "SI".into(); }
        if !self.page.margins.iter().all(|m| m.is_finite() && (0.0..=60.0).contains(m)) { self.page.margins = Page::default().margins; }
    }

    pub fn json(&self) -> String { serde_json::to_string_pretty(self).unwrap_or_default() }
}

#[derive(Debug, Clone, Deserialize)]
pub enum Op {
    /// A new region before position `at` (at the end when past it).
    Insert { at: usize, kind: Kind, text: String, #[serde(default)] data: serde_json::Value },
    /// Sets one field of a region: text, unit, style, kind or data (JSON).
    Set { id: u32, field: String, value: serde_json::Value },
    Delete { ids: Vec<u32> },
    Move { ids: Vec<u32>, up: bool },
    /// Sets a title block field.
    Title { field: String, value: String },
    /// Sets a sheet-level value: name, units, digits, paper, margins.
    SheetSet { field: String, value: serde_json::Value },
    /// Stores window settings under a key.
    Ui { key: String, value: serde_json::Value },
    Batch(Vec<Op>),
}

fn exec(s: &mut Sheet, op: Op) -> Result<(), String> {
    match op {
        Op::Insert { at, kind, text, data } => {
            if s.regions.len() >= 5000 { return Err("A sheet holds at most 5,000 regions.".into()); }
            let id = s.next_id;
            s.next_id += 1;
            let at = at.min(s.regions.len());
            s.regions.insert(at, Region { id, kind, text, data, ..Default::default() });
        }
        Op::Set { id, field, value } => {
            let i = s.idx(id).ok_or("That region no longer exists.")?;
            let r = &mut s.regions[i];
            let text = || value.as_str().map(String::from).ok_or_else(|| format!("{field} must be text."));
            match field.as_str() {
                "text" => { let t = text()?; if t.len() > MAX_TEXT { return Err("A region holds at most 20,000 characters.".into()); } r.text = t; }
                "unit" => r.unit = text()?,
                "style" => r.style = text()?,
                "kind" => r.kind = serde_json::from_value(value.clone()).map_err(|_| "Unknown region kind.")?,
                "data" => {
                    if value.get("src").and_then(|v| v.as_str()).is_some_and(|v| v.len() > MAX_PICTURE) { return Err("Choose a picture smaller than 6 MB.".into()); }
                    if value.get("rows").and_then(|v| v.as_array()).is_some_and(|v| v.len() > MAX_ROWS) { return Err("A table holds at most 2,000 rows.".into()); }
                    if value.get("cols").and_then(|v| v.as_array()).is_some_and(|v| v.len() > MAX_COLS) { return Err("A table holds at most 50 columns.".into()); }
                    r.data = value;
                }
                f => return Err(format!("Regions have no field {f}.")),
            }
        }
        Op::Delete { ids } => {
            if ids.iter().all(|id| s.idx(*id).is_none()) { return Err("Select a region first.".into()); }
            s.regions.retain(|r| !ids.contains(&r.id));
        }
        Op::Move { ids, up } => {
            let mut pos: Vec<usize> = ids.iter().filter_map(|id| s.idx(*id)).collect();
            pos.sort_unstable();
            if pos.is_empty() { return Err("Select a region first.".into()); }
            if up {
                if pos[0] == 0 { return Ok(()); }
                for i in pos { s.regions.swap(i - 1, i); }
            } else {
                if *pos.last().unwrap() + 1 >= s.regions.len() { return Ok(()); }
                for i in pos.into_iter().rev() { s.regions.swap(i, i + 1); }
            }
        }
        Op::Title { field, value } => {
            let t = &mut s.title;
            let f = match field.as_str() {
                "project" => &mut t.project, "number" => &mut t.number, "title" => &mut t.title, "by" => &mut t.by, "checked" => &mut t.checked,
                "date" => &mut t.date, "rev" => &mut t.rev, "company" => &mut t.company, "client" => &mut t.client, "logo" => &mut t.logo,
                f => return Err(format!("The title block has no field {f}.")),
            };
            *f = value;
        }
        Op::SheetSet { field, value } => match field.as_str() {
            "name" => { let n = value.as_str().unwrap_or("").trim(); if n.is_empty() { return Err("Give the sheet a name.".into()); } s.name = n.into(); }
            "units" => s.units = match value.as_str() { Some("US") => "US".into(), Some("SI") => "SI".into(), _ => return Err("Units are SI or US.".into()) },
            "subst" => s.subst = value.as_bool().ok_or("Show numbers is on or off.")?,
            "polar" => s.polar = value.as_bool().ok_or("Polar form is on or off.")?,
            "digits" => s.digits = value.as_u64().filter(|d| (2..=10).contains(d)).ok_or("Significant figures are 2 to 10.")? as u8,
            "paper" => s.page.paper = value.as_str().ok_or("Paper must be text.")?.into(),
            "margins" => s.page.margins = serde_json::from_value(value).map_err(|_| "Margins are four numbers in mm.")?,
            f => return Err(format!("The sheet has no setting {f}.")),
        },
        Op::Ui { key, value } => {
            if !s.ui.is_object() { s.ui = serde_json::json!({}); }
            s.ui[key] = value;
        }
        Op::Batch(ops) => for o in ops { exec(s, o)? },
    }
    Ok(())
}

/// An open sheet with its file and history.
pub struct Doc {
    pub s: Sheet,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    undo: Vec<Sheet>,
    redo: Vec<Sheet>,
}

impl Doc {
    pub fn new(s: Sheet) -> Self { Self { s, path: None, dirty: false, undo: vec![], redo: vec![] } }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    /// Changes whenever the history does, so the window knows to redraw.
    pub fn rev(&self) -> u64 { (self.undo.len() as u64) * 1_000_003 + (self.redo.len() as u64) * 7919 + self.s.next_id as u64 }

    /// Applies `op` to a copy and keeps it only if every part succeeded: a failed edit changes nothing.
    pub fn apply(&mut self, op: Op) -> Result<(), String> {
        let mut q = self.s.clone();
        exec(&mut q, op)?;
        q.normalize();
        if q == self.s { return Ok(()); }
        self.undo.push(std::mem::replace(&mut self.s, q));
        if self.undo.len() > UNDO_LIMIT { self.undo.remove(0); }
        self.redo.clear();
        self.dirty = true;
        Ok(())
    }

    pub fn undo(&mut self) {
        if let Some(s) = self.undo.pop() { self.redo.push(std::mem::replace(&mut self.s, s)); self.dirty = true; }
    }

    pub fn redo(&mut self) {
        if let Some(s) = self.redo.pop() { self.undo.push(std::mem::replace(&mut self.s, s)); self.dirty = true; }
    }

    /// Opens a .dcalc file. Files over 64 MB are refused (a sheet is a few hundred kB even with pictures);
    /// bytes that are not UTF-8 are replaced, so a damaged file still opens as far as it can.
    pub fn open(path: &Path) -> Result<Self, String> {
        let len = std::fs::metadata(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?.len();
        if len > MAX_FILE { return Err(format!("{} is {} MB; a Datum Calc sheet is never that large.", path.display(), len / 1_000_000)); }
        let bytes = std::fs::read(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
        let s = Sheet::from_json(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("This is not a Datum Calc sheet: {e}"))?;
        let mut d = Self::new(s);
        d.path = Some(path.into());
        Ok(d)
    }

    /// Saves safely: the sheet is written to a temporary file next to the target, which then replaces
    /// it in one step, so a crash or a full disk never leaves a half-written sheet.
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        let tmp = path.with_extension("dcalc.saving");
        let fail = |e: std::io::Error| format!("Could not save {}: {e}", path.display());
        std::fs::write(&tmp, self.s.json()).map_err(|e| { let _ = std::fs::remove_file(&tmp); fail(e) })?;
        std::fs::rename(&tmp, path).map_err(|e| { let _ = std::fs::remove_file(&tmp); fail(e) })?;
        self.path = Some(path.into());
        self.dirty = false;
        Ok(())
    }
}
