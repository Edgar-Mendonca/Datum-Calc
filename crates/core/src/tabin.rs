//! Data into a table (0.2.7): text pasted from a spreadsheet, or a CSV file. Finds the separator (tab, semicolon
//! or comma) and a decimal comma, reads a heading row ("Q (m3/h)", "H [m]") into names and units, and keeps the
//! cells as text for the table to work out.
use serde_json::{json, Value};

const MAX_ROWS: usize = 2000;
const MAX_COLS: usize = 50;

/// One line split at the separator, with "quoted, cells" and "" for a quote inside quotes.
fn split(line: &str, sep: char) -> Vec<String> {
    let (mut out, mut cur, mut q) = (vec![], String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => { cur.push('"'); it.next(); }
            '"' => q = !q,
            c if c == sep && !q => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// A number written with a decimal comma (1,25 or -0,5e3) in the decimal-point form.
fn point(c: &str, comma: bool) -> String {
    if comma && c.matches(',').count() == 1 && !c.contains('.') && c.chars().all(|ch| ch.is_ascii_digit() || matches!(ch, ',' | '-' | '+' | 'e' | 'E')) { c.replace(',', ".") } else { c.to_string() }
}

fn is_number(c: &str) -> bool { c.parse::<f64>().is_ok_and(|v| v.is_finite()) }

/// A unit as typed in a heading (m3/h, m², kg/m3) in the sheet's form (m^3/h); None if it is not a unit.
fn unit(u: &str) -> Option<String> {
    let u = u.trim();
    if u.is_empty() || u == "-" { return Some(String::new()); }
    let mut s = String::new();
    let cs: Vec<char> = u.chars().collect();
    for (i, c) in cs.iter().enumerate() {
        // A digit right after a letter is a power: m3 → m^3.
        if c.is_ascii_digit() && i > 0 && cs[i - 1].is_alphabetic() { s.push('^'); }
        s.push(*c);
    }
    crate::units::parse(&s).ok().map(|_| s)
}

/// A heading as a name and a unit: "Q (m3/h)", "H [m]", "eta"; the name made a valid name (spaces to _).
fn heading(h: &str) -> (String, Option<String>) {
    let h = h.trim();
    let (name, u) = match h.find(['(', '[']) {
        Some(i) if h.ends_with([')', ']']) => (&h[..i], Some(&h[i + 1..h.len() - 1])),
        _ => (h, None),
    };
    let mut n: String = name.trim().chars().map(|c| if c.is_whitespace() || c == '-' || c == '.' { '_' } else { c }).filter(|c| c.is_alphanumeric() || *c == '_').collect();
    while n.contains("__") { n = n.replace("__", "_"); }
    let n = n.trim_matches('_').to_string();
    let n = if n.is_empty() || n.chars().next().is_some_and(|c| c.is_alphabetic()) { n } else { format!("c{n}") };
    (n, u.map(|u| u.to_string()))
}

/// The table in the text: {cols: [{name, unit, formula}], rows: [[cell]], notes: [what was changed or not read]}.
pub fn parse(text: &str) -> Result<Value, String> {
    let text = text.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return Err("There is no data in this text.".into()); }
    let head = &lines[..lines.len().min(20)];
    let count = |c: char| head.iter().map(|l| l.matches(c).count()).min().unwrap_or(0);
    let sep = if text.contains('\t') { '\t' } else if count(';') > 0 { ';' } else if count(',') > 0 { ',' } else { '\t' };
    let comma = sep != ',';
    let mut rows: Vec<Vec<String>> = lines.iter().map(|l| split(l, sep).iter().map(|c| point(c, comma)).collect()).collect();
    let mut notes = vec![];
    // A heading row has some text that is not a number; a second row of units may follow it.
    let first_text = rows[0].iter().any(|c| !c.is_empty() && !is_number(c));
    let mut names: Vec<(String, Option<String>)> = vec![];
    if first_text && rows.len() > 1 {
        names = rows.remove(0).iter().map(|h| heading(h)).collect();
        let unit_row = rows.first().is_some_and(|r| r.iter().any(|c| !c.is_empty()) && r.iter().all(|c| c.is_empty() || (!is_number(c) && unit(c.trim_matches(['(', ')', '[', ']'])).is_some())));
        if unit_row {
            let u = rows.remove(0);
            for (k, n) in names.iter_mut().enumerate() { if let Some(c) = u.get(k).filter(|c| !c.is_empty()) { n.1 = Some(c.trim_matches(['(', ')', '[', ']']).to_string()); } }
        }
    }
    let width = rows.iter().map(|r| r.len()).chain([names.len()]).max().unwrap_or(0).min(MAX_COLS);
    if rows.iter().any(|r| r.len() > MAX_COLS) || names.len() > MAX_COLS { notes.push(format!("Only the first {MAX_COLS} columns were read.")); }
    if rows.len() > MAX_ROWS { notes.push(format!("Only the first {MAX_ROWS} rows were read (there are {}).", rows.len())); rows.truncate(MAX_ROWS); }
    let mut taken: Vec<String> = vec![];
    let cols: Vec<Value> = (0..width).map(|k| {
        let (n, u) = names.get(k).cloned().unwrap_or_default();
        let mut name = if n.is_empty() { format!("c{}", k + 1) } else { n };
        let base = name.clone();
        let mut i = 2;
        while taken.contains(&name) { name = format!("{base}_{i}"); i += 1; }
        taken.push(name.clone());
        let u = match u.as_deref().map(|t| (t, unit(t))) {
            Some((_, Some(s))) => s,
            Some((t, None)) => { notes.push(format!("The unit “{t}” of {name} was not recognised: type it in the Unit row.")); String::new() }
            None => String::new(),
        };
        json!({ "name": name, "unit": u, "formula": "" })
    }).collect();
    for r in &mut rows { r.resize(width, String::new()); }
    Ok(json!({ "cols": cols, "rows": rows, "notes": notes }))
}
