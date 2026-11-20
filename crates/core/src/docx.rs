//! Export to a document (0.2.10): the calc sheet as a .docx (Office Open XML), with its title block, headings and
//! text, equations a word processor can edit (OMML), results, checks with PASS or FAIL, tables, plots and pictures. Regions hidden from print are left
//! out. The package is written stored (not compressed), which every reader of the format accepts.
use crate::eval::{split_note, SheetOut};
use crate::omml::{self, esc};
use crate::sheet::{Kind, Region, Sheet};
use crate::units;
use std::collections::HashMap;

/// A picture for the document: PNG or JPEG bytes, and its size in pixels (for its proportions).
pub struct Picture { pub bytes: Vec<u8>, pub png: bool, pub w: u32, pub h: u32 }

fn run(t: &str, props: &str) -> String { format!("<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>", if props.is_empty() { String::new() } else { format!("<w:rPr>{props}</w:rPr>") }, esc(t)) }
fn para(style: &str, body: &str) -> String { format!("<w:p>{}{body}</w:p>", if style.is_empty() { String::new() } else { format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>") }) }

/// A table with thin borders; `head`: the first row is a heading (bold, repeated on each page).
fn table(rows: &[Vec<String>], head: bool) -> String {
    let cell = |t: &str, b: bool| format!("<w:tc><w:p>{}</w:p></w:tc>", run(t, if b { "<w:b/>" } else { "" }));
    let body: String = rows.iter().enumerate().map(|(i, r)| {
        let h = head && i == 0;
        format!("<w:tr>{}{}</w:tr>", if h { "<w:trPr><w:tblHeader/></w:trPr>" } else { "" }, r.iter().map(|c| cell(c, h)).collect::<String>())
    }).collect();
    format!("<w:tbl><w:tblPr><w:tblStyle w:val=\"Grid\"/><w:tblW w:w=\"0\" w:type=\"auto\"/><w:tblBorders>{}</w:tblBorders></w:tblPr>{body}</w:tbl><w:p/>",
        ["top", "left", "bottom", "right", "insideH", "insideV"].iter().map(|b| format!("<w:{b} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"8D99A8\"/>")).collect::<String>())
}

/// A picture in its own paragraph, `w` EMU wide (its height from its proportions, at most `hmax`).
fn drawing(rel: &str, n: usize, p: &Picture, w: u64, hmax: u64) -> String {
    let mut h = (w as f64 * p.h.max(1) as f64 / p.w.max(1) as f64) as u64;
    let w = if h > hmax { let w2 = (w as f64 * hmax as f64 / h as f64) as u64; h = hmax; w2 } else { w };
    format!("<w:p><w:pPr><w:keepNext/></w:pPr><w:r><w:drawing><wp:inline><wp:extent cx=\"{w}\" cy=\"{h}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\"/><a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"Picture {n}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{rel}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>")
}

/// A picture from a data URL: its bytes, kind and size, told from the bytes themselves; None if it is not PNG or JPEG.
pub fn picture_from_data_url(src: &str) -> Option<Picture> {
    let (_, b64) = src.strip_prefix("data:")?.split_once(";base64,")?;
    picture(b64_decode(b64)?)
}

/// A picture from its bytes: PNG or JPEG, with its size in pixels.
pub fn picture(bytes: Vec<u8>) -> Option<Picture> {
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    if !png && !bytes.starts_with(&[0xFF, 0xD8]) { return None; }
    let (w, h) = if png { size_png(&bytes)? } else { size_jpeg(&bytes)? };
    (w > 0 && h > 0).then_some(Picture { bytes, png, w, h })
}

fn size_png(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 24 { return None; }
    Some((u32::from_be_bytes(b[16..20].try_into().ok()?), u32::from_be_bytes(b[20..24].try_into().ok()?)))
}

fn size_jpeg(b: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF { i += 1; continue; }
        let m = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if matches!(m, 0xC0..=0xC3) { return Some((u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32, u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32)); }
        i += 2 + len;
    }
    None
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| match c { b'A'..=b'Z' => Some(c - b'A'), b'a'..=b'z' => Some(c - b'a' + 26), b'0'..=b'9' => Some(c - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None };
    let (mut out, mut acc, mut bits) = (Vec::with_capacity(s.len() * 3 / 4), 0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = acc << 6 | val(c)? as u32;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((acc >> bits) as u8); }
    }
    Some(out)
}

/// Paper sizes in twips (1/20 pt): A4, A3, Letter, Legal.
fn paper(name: &str) -> (u32, u32) {
    match name { "A3" => (16838, 23811), "Letter" => (12240, 15840), "Legal" => (12240, 20160), _ => (11906, 16838) }
}

/// The pictures of a document, numbered in order, each with its relationship.
#[derive(Default)]
struct Media<'a> { list: Vec<&'a Picture> }
impl<'a> Media<'a> {
    fn add(&mut self, p: &'a Picture, w: u64, hmax: u64) -> String {
        self.list.push(p);
        let n = self.list.len();
        drawing(&format!("rIdImg{n}"), n, p, w, hmax)
    }
}

fn cap(t: &str) -> String { if t.trim().is_empty() { String::new() } else { para("Caption", &run(t.trim(), "")) } }

/// The document. `pics` are pictures by region id, drawn by the window: plots, and picture regions turned upright;
/// a picture region the window did not send is read from its own data. The title block's logo is read from its data.
pub fn document(s: &Sheet, o: &SheetOut, pics: &HashMap<u32, Picture>) -> Vec<u8> {
    // The sheet's own units, so its lines read again the same way.
    units::custom_clear();
    for (sym, f, d) in &o.units { units::custom_add(sym, *f, *d); }
    let (pw, ph) = paper(&s.page.paper);
    let tw = |mm: f64| (mm.max(0.0) * 56.693) as u32;
    let [mt, mr, mb, ml] = s.page.margins;
    let text_w = (pw.saturating_sub(tw(mr) + tw(ml)).max(1440) as u64) * 635;
    let text_h = (ph.saturating_sub(tw(mt) + tw(mb)).max(1440) as u64) * 635;
    let own: Vec<(u32, Picture)> = s.regions.iter().filter(|r| r.kind == Kind::Image && !r.hide && !pics.contains_key(&r.id))
        .filter_map(|r| Some((r.id, r.data.get("src")?.as_str().and_then(picture_from_data_url)?))).collect();
    let logo = picture_from_data_url(&s.title.logo);
    let mut media = Media::default();
    let mut body = String::new();
    // The title block as a small table, the logo above it.
    if let Some(l) = &logo { body.push_str(&media.add(l, text_w / 5, 1_080_000)); }
    let t = &s.title;
    let mut tb: Vec<Vec<String>> = vec![
        vec!["Project".into(), t.project.clone(), "Calc number".into(), t.number.clone()],
        vec!["Title".into(), if t.title.trim().is_empty() { s.name.clone() } else { t.title.clone() }, "Revision".into(), t.rev.clone()],
        vec!["Prepared by".into(), t.by.clone(), "Checked by".into(), t.checked.clone()],
        vec!["Company".into(), t.company.clone(), "Date".into(), t.date.clone()],
    ];
    if !t.client.trim().is_empty() { tb.push(vec!["Client".into(), t.client.clone(), String::new(), String::new()]); }
    body.push_str(&table(&tb, false));
    let shown = || s.regions.iter().zip(&o.regions).filter(|(r, _)| !r.hide);
    // The summary of checks, when the sheet prints one.
    if s.ui.pointer("/print/summary").and_then(|v| v.as_bool()) == Some(true) {
        let checks: Vec<_> = shown().filter_map(|(r, x)| Some((r, x.check.as_ref()?))).collect();
        let failed = checks.iter().filter(|(_, c)| !c.pass).count();
        body.push_str(&para("Heading1", &run("Summary of checks", "")));
        body.push_str(&para("", &run(&match (checks.len(), failed) { (0, _) => "This sheet has no checks.".into(), (n, 0) => format!("All {n} checks pass."), (n, f) => format!("{f} of {n} checks fail.") }, "")));
        if !checks.is_empty() {
            let mut rows = vec![vec!["Check".into(), "Values".into(), "Utilisation".into(), "Result".into()]];
            for (r, c) in checks {
                let op = match c.op.as_str() { "<=" => "≤", ">=" => "≥", "!=" => "≠", o => o };
                rows.push(vec![split_note(&r.text).0.trim().into(), format!("{} {op} {}", c.lhs, c.rhs), c.ratio.map(|u| format!("{u:.2}")).unwrap_or_default(), (if c.pass { "PASS" } else { "FAIL" }).into()]);
            }
            body.push_str(&table(&rows, true));
        }
    }
    for (r, out) in shown().filter(|(r, _)| r.style != "ref") {
        match r.kind {
            Kind::Text => body.push_str(&text(r)),
            Kind::Image => {
                if let Some(p) = pics.get(&r.id).or_else(|| own.iter().find(|(id, _)| *id == r.id).map(|x| &x.1)) {
                    let pct = r.data.get("width").and_then(|v| v.as_f64()).unwrap_or(60.0).clamp(5.0, 100.0);
                    body.push_str(&media.add(p, (text_w as f64 * pct / 100.0) as u64, text_h * 2 / 3));
                }
                body.push_str(&cap(&r.text));
            }
            Kind::Table => {
                body.push_str(&cap(&r.text));
                if let Some(tab) = out.table.as_ref().filter(|t| !t.cols.is_empty()) {
                    let mut rows = vec![tab.cols.iter().map(|c| if c.unit.is_empty() { c.name.clone() } else { format!("{} ({})", c.name, c.unit) }).collect::<Vec<_>>()];
                    for i in 0..tab.rows { rows.push(tab.cols.iter().map(|c| c.cells.get(i).cloned().unwrap_or_default()).collect()); }
                    body.push_str(&table(&rows, true));
                    for c in tab.cols.iter().filter(|c| !c.error.is_empty()) { body.push_str(&para("", &err(&format!("{}: {}", c.name, c.error)))); }
                }
                if let Some(e) = &out.error { body.push_str(&para("", &err(&e.msg))); }
            }
            Kind::Material => {
                if let Some(m) = &out.material {
                    body.push_str(&para("Heading2", &run(&m.title, "")));
                    let info = [m.des.as_str(), m.notes.as_str()].iter().filter(|x| !x.trim().is_empty()).cloned().collect::<Vec<_>>().join(". ");
                    if !info.is_empty() { body.push_str(&para("Note", &run(&info, ""))); }
                    let mut rows = vec![vec!["Property".into(), "Value".into(), "Range".into(), "Source".into()]];
                    for x in &m.rows { rows.push(vec![x.what.clone(), format!("{} {}", x.value, x.unit).trim().into(), x.range.clone(), x.source.clone()]); }
                    if m.rows.is_empty() { rows.push(vec![String::new(); 4]); }
                    body.push_str(&table(&rows, true));
                    for src in &m.sources { body.push_str(&para("Reference", &run(src, ""))); }
                }
                if let Some(e) = &out.error { body.push_str(&para("", &err(&e.msg))); }
            }
            _ => {
                body.push_str(&maths(r, out));
                if let (Some(_), Some(p)) = (&out.plot, pics.get(&r.id)) { body.push_str(&media.add(p, text_w, text_h * 2 / 3)); }
            }
        }
    }
    // References at the end, as printed.
    let refs: String = shown().filter(|(r, _)| r.style == "ref").map(|(r, _)| text(r)).collect();
    if !refs.is_empty() { body.push_str(&para("Heading1", &run("References", ""))); body.push_str(&refs); }
    units::custom_clear();
    let doc = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"{pw}\" w:h=\"{ph}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>",
        tw(mt), tw(mr), tw(mb), tw(ml));
    let ext = |p: &Picture| if p.png { "png" } else { "jpeg" };
    let rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>{}</Relationships>",
        media.list.iter().enumerate().map(|(i, p)| format!("<Relationship Id=\"rIdImg{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{n}.{}\"/>", ext(p), n = i + 1)).collect::<String>());
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), CONTENT_TYPES.as_bytes().to_vec()),
        ("_rels/.rels".into(), ROOT_RELS.as_bytes().to_vec()),
        ("word/document.xml".into(), doc.into_bytes()),
        ("word/_rels/document.xml.rels".into(), rels.into_bytes()),
        ("word/styles.xml".into(), STYLES.as_bytes().to_vec()),
    ];
    for (i, p) in media.list.iter().enumerate() { files.push((format!("word/media/image{}.{}", i + 1, ext(p)), p.bytes.clone())); }
    zip(&files)
}

fn err(msg: &str) -> String { run(&format!("⚠ {msg}"), "<w:color w:val=\"B42318\"/>") }

/// A text region: a heading, a note, a reference or plain text, line by line.
fn text(r: &Region) -> String {
    let style = match r.style.as_str() { "heading" => "Heading1", "subheading" => "Heading2", "note" => "Note", "ref" => "Reference", _ => "" };
    r.text.lines().filter(|l| !l.trim().is_empty()).map(|l| para(style, &run(l, ""))).collect()
}

/// A line of maths: the equation as OMML maths, its result, its note; a check's PASS or FAIL; an error as such.
fn maths(r: &Region, out: &crate::eval::RegionOut) -> String {
    let code = split_note(&r.text).0;
    let eq = if r.kind == Kind::Program { None } else { crate::parse::statement(code, &[]).ok().and_then(|st| omml::stmt(&st)) };
    let mut p = match eq {
        Some(m) => {
            // The result in the same equation: = 19.57 kW.
            let res = if !out.result.is_empty() && out.check.is_none() && !out.input { format!("{}{}", omml::rp(" = "), omml::rp(&out.result)) } else { String::new() };
            format!("<m:oMath>{m}{res}</m:oMath>")
        }
        // Lines that are not one equation (solve, fit, ode, a program) as their text, then their result.
        None => {
            let lines: Vec<&str> = code.lines().collect();
            let mut t = lines.iter().enumerate().map(|(i, l)| format!("{}{}", if i > 0 { "<w:r><w:br/></w:r>" } else { "" }, run(l, "<w:rFonts w:ascii=\"Consolas\" w:hAnsi=\"Consolas\"/>"))).collect::<String>();
            if !out.result.is_empty() { t.push_str(&run(&format!("   ⇒ {}", out.result), "")); }
            t
        }
    };
    if let Some(c) = &out.check {
        let ok = if c.pass { "PASS" } else { "FAIL" };
        p.push_str(&run(&format!("   {ok}"), &format!("<w:b/><w:color w:val=\"{}\"/>", if c.pass { "1E7B34" } else { "B42318" })));
        if let Some(u) = c.ratio { p.push_str(&run(&format!("  utilisation {:.2}", u), "")); }
    }
    if let Some(e) = &out.error { p.push_str(&run("   ", "")); p.push_str(&err(&e.msg)); }
    if !out.note.trim().is_empty() { p.push_str(&run(&format!("   — {}", out.note.trim()), "<w:i/><w:color w:val=\"5B6878\"/>")); }
    para(if r.style == "input" { "Input" } else { "" }, &p)
}

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>";
const ROOT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"21\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"80\" w:line=\"264\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"240\" w:after=\"80\"/><w:outlineLvl w:val=\"0\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"28\"/><w:color w:val=\"1B2733\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"160\" w:after=\"60\"/><w:outlineLvl w:val=\"1\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"23\"/><w:color w:val=\"1B2733\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Note\"><w:name w:val=\"Note\"/><w:basedOn w:val=\"Normal\"/><w:rPr><w:i/><w:color w:val=\"5B6878\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Reference\"><w:name w:val=\"Reference\"/><w:basedOn w:val=\"Normal\"/><w:rPr><w:sz w:val=\"18\"/><w:color w:val=\"5B6878\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Caption\"><w:name w:val=\"caption\"/><w:basedOn w:val=\"Normal\"/><w:rPr><w:i/><w:sz w:val=\"19\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Input\"><w:name w:val=\"Input\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"FFF6E0\"/></w:pPr></w:style><w:style w:type=\"table\" w:styleId=\"Grid\"><w:name w:val=\"Grid\"/><w:tblPr><w:tblCellMar><w:left w:w=\"80\" w:type=\"dxa\"/><w:right w:w=\"80\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr></w:style></w:styles>";

/// A zip file of the given files, stored (no compression), with CRC-32 checks.
pub fn zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let crc_table: Vec<u32> = (0..256u32).map(|n| (0..8).fold(n, |c, _| if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 })).collect();
    let crc = |b: &[u8]| !b.iter().fold(!0u32, |c, x| crc_table[((c ^ *x as u32) & 0xFF) as usize] ^ (c >> 8));
    let (mut out, mut central) = (vec![], vec![]);
    for (name, data) in files {
        let (off, c, len, nlen) = (out.len() as u32, crc(data), data.len() as u32, name.len() as u16);
        let head = |sig: u32, central: bool| {
            let mut h = vec![];
            h.extend(sig.to_le_bytes());
            if central { h.extend(20u16.to_le_bytes()); }
            h.extend(20u16.to_le_bytes()); // version needed
            h.extend(0x0800u16.to_le_bytes()); // UTF-8 names
            h.extend(0u16.to_le_bytes()); // stored
            h.extend(0x0021_0000u32.to_le_bytes()); // time 00:00, date 1 Jan 1980 (the earliest a zip can hold)
            h.extend(c.to_le_bytes());
            h.extend(len.to_le_bytes());
            h.extend(len.to_le_bytes());
            h.extend(nlen.to_le_bytes());
            h.extend(0u16.to_le_bytes());
            // Comment length, disk number, internal attributes (6 bytes), external attributes (4), then the offset.
            if central { h.extend([0u8; 6]); h.extend(0u32.to_le_bytes()); h.extend(off.to_le_bytes()); }
            h.extend(name.as_bytes());
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend(data);
        central.extend(head(0x0201_4b50, true));
    }
    let (start, size, n) = (out.len() as u32, central.len() as u32, files.len() as u16);
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(n.to_le_bytes());
    out.extend(n.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out
}
