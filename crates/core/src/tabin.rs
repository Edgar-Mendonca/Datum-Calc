//! Data into a table (0.2.7): text pasted from a spreadsheet, or a CSV file. Finds the separator (tab, semicolon
//! or comma, by the count most lines share) and whether commas are decimal commas, reads a heading row
//! ("Q (m3/h)", "H [m]") into names and units, and keeps the cells as text for the table to work out.
use serde_json::{json, Value};

const MAX_ROWS: usize = 2000;
const MAX_COLS: usize = 50;

/// The rows of the text split at the separator; quotes may hold the separator, "" and line breaks.
fn rows_of(text: &str, sep: Option<char>) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut cur, mut q) = (vec![], vec![], String::new(), false);
    let mut it = text.chars().peekable();
    let end_row = |row: &mut Vec<String>, cur: &mut String, rows: &mut Vec<Vec<String>>| {
        row.push(std::mem::take(cur).trim().to_string());
        let r = std::mem::take(row);
        if r.iter().any(|c| !c.is_empty()) { rows.push(r); }
    };
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => { cur.push('"'); it.next(); }
            '"' => q = !q,
            '\n' if !q => end_row(&mut row, &mut cur, &mut rows),
            c if Some(c) == sep && !q => row.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    end_row(&mut row, &mut cur, &mut rows);
    rows
}

/// How many times a character stands outside quotes on a line.
fn count_out(line: &str, sep: char) -> usize {
    let mut q = false;
    line.chars().filter(|c| { if *c == '"' { q = !q; } !q && *c == sep }).count()
}

fn is_number(c: &str) -> bool { c.parse::<f64>().is_ok_and(|v| v.is_finite()) }

/// A cell that is data: a number, or a number with a unit after it ("12 m").
fn is_value(c: &str) -> bool {
    if is_number(c) { return true; }
    let i = c.find(|ch: char| ch.is_alphabetic() || ch == '°' || ch == '%').unwrap_or(c.len());
    i > 0 && is_number(c[..i].trim()) && crate::units::parse(c[i..].trim()).is_ok()
}

/// A number with its grouping and decimal separators in the plain form: 1.234,5 → 1234.5 with decimal commas;
/// 1,234 → 1234 when commas group thousands.
fn plain(c: &str, decimal_comma: bool, thousands_comma: bool) -> String {
    let t = c.trim();
    let digits = |s: &str| !s.is_empty() && s.chars().all(|ch| ch.is_ascii_digit());
    let (sign, body) = t.strip_prefix('-').map(|b| ("-", b)).unwrap_or(("", t));
    if decimal_comma {
        // Groups of three before the comma, set off by points or spaces: 1.234.567,89 or 1 234,5.
        let (int, frac) = body.split_once(',').unwrap_or((body, ""));
        let groups: Vec<&str> = int.split(['.', ' ', '\u{a0}']).collect();
        let grouped = groups.len() > 1 && digits(groups[0]) && groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3 && digits(g)) && (frac.is_empty() || digits(frac));
        if grouped { return format!("{sign}{}{}", groups.concat(), if frac.is_empty() { String::new() } else { format!(".{frac}") }); }
        if body.matches(',').count() == 1 && !body.contains('.') && digits(int) && digits(frac) { return format!("{sign}{int}.{frac}"); }
    }
    if thousands_comma && body.contains(',') {
        let groups: Vec<&str> = body.split('.').next().unwrap_or("").split(',').collect();
        if digits(groups[0]) && groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3 && digits(g)) { return format!("{sign}{}", body.replace(',', "")); }
    }
    t.to_string()
}

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
/// `pasted` is text copied from a spreadsheet, whose columns are always set off by tabs: without a tab it is
/// one column (and 1,5 a decimal comma).
pub fn parse(text: &str, pasted: bool) -> Result<Value, String> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return Err("There is no data in this text.".into()); }
    let mut notes = vec![];
    // The separator: the one whose most common count (above zero) the most lines share.
    let head = &lines[..lines.len().min(50)];
    let mode = |c: char| -> (usize, usize) {
        let mut tally = std::collections::HashMap::new();
        for l in head { let n = count_out(l, c); if n > 0 { *tally.entry(n).or_insert(0usize) += 1; } }
        tally.into_iter().max_by_key(|(n, k)| (*k, *n)).map(|(n, k)| (k, n)).unwrap_or((0, 0))
    };
    let cands: Vec<char> = if pasted { vec!['\t'] } else { vec!['\t', ';', ','] };
    let best = cands.iter().map(|c| (*c, mode(*c))).filter(|(_, (k, _))| *k * 2 >= head.len().min(4) || *k >= 2).max_by_key(|(c, (k, _))| (*k, *c == '\t', *c == ';'));
    let sep = best.map(|b| b.0);
    // Lines above the table that lack its separator (a title, a date) are left out.
    let mut skip = 0;
    if let Some((c, _)) = best { while skip < lines.len() && count_out(lines[skip], c) == 0 { skip += 1; } }
    if skip > 0 { notes.push(format!("Left out {skip} line{} above the table.", if skip == 1 { "" } else { "s" })); }
    let body: String = text.lines().filter(|l| !l.trim().is_empty()).skip(skip).collect::<Vec<_>>().join("\n");
    let mut rows = rows_of(&body, sep);
    if rows.is_empty() { return Err("There is no data in this text.".into()); }
    // Decimal commas: with semicolons, or in one column, or tabs with numbers like 1,5 or 0,25 (not 1,234).
    let cells: Vec<&String> = rows.iter().flatten().collect();
    let three = |c: &str| { let t = c.trim_start_matches('-'); t.split_once(',').is_some_and(|(a, b)| a.chars().all(|x| x.is_ascii_digit()) && b.len() == 3 && b.chars().all(|x| x.is_ascii_digit()) && !a.is_empty() && a.len() <= 3) };
    let decimal = |c: &str| { let t = c.trim_start_matches('-'); t.matches(',').count() == 1 && !t.contains('.') && t.split_once(',').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a.chars().all(|x| x.is_ascii_digit()) && b.chars().all(|x| x.is_ascii_digit())) && !three(c) };
    let comma_cells = cells.iter().filter(|c| c.contains(',')).count();
    let decimal_comma = sep != Some(',') && (sep == Some(';') || cells.iter().any(|c| decimal(c)) || (sep.is_none() && comma_cells > 0 && cells.iter().filter(|c| c.contains(',')).all(|c| !three(c))));
    let thousands_comma = !decimal_comma && sep != Some(',') && comma_cells > 0 && cells.iter().filter(|c| c.contains(',')).all(|c| three(c) || !c.chars().any(|x| x.is_ascii_digit()));
    if thousands_comma && cells.iter().any(|c| three(c)) { notes.push("Commas between groups of three digits were read as thousands (1,234 as 1234).".into()); }
    for r in &mut rows { for c in r.iter_mut() { *c = plain(c, decimal_comma, thousands_comma); } }
    // A heading row has some text that is not a value; a second row of units may follow it.
    let first_text = rows[0].iter().any(|c| !c.is_empty() && !is_value(c));
    let mut names: Vec<(String, Option<String>)> = vec![];
    if first_text {
        names = rows.remove(0).iter().map(|h| heading(h)).collect();
        let unit_row = rows.first().is_some_and(|r| r.iter().any(|c| !c.is_empty()) && r.iter().all(|c| c.is_empty() || (!is_number(c) && unit(c.trim_matches(['(', ')', '[', ']'])).is_some())));
        if unit_row {
            let u = rows.remove(0);
            for (k, n) in names.iter_mut().enumerate() { if let Some(c) = u.get(k).filter(|c| !c.is_empty()) { n.1 = Some(c.trim_matches(['(', ')', '[', ']']).to_string()); } }
        }
    }
    let width = rows.iter().map(|r| r.len()).chain([names.len()]).max().unwrap_or(0).clamp(1, MAX_COLS);
    if rows.iter().any(|r| r.len() > MAX_COLS) || names.len() > MAX_COLS { notes.push(format!("Only the first {MAX_COLS} columns were read.")); }
    if rows.len() > MAX_ROWS { notes.push(format!("Only the first {MAX_ROWS} rows were read (there are {}).", rows.len())); rows.truncate(MAX_ROWS); }
    if rows.is_empty() { rows.push(vec![String::new(); width]); }
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
