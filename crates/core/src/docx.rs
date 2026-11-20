//! Export to Word (0.2.10): the calc sheet as a .docx, with its title block, headings and text, equations Word can
//! edit (OMML), results, checks with PASS or FAIL, tables, plots and pictures. Regions hidden from print are left
//! out. The package is written stored (not compressed), which every Word version reads.
use crate::eval::SheetOut;
use crate::omml::{self, esc};
use crate::sheet::{Kind, Region, Sheet};

/// A picture for the document: PNG or JPEG bytes, and its size in pixels (for its proportions).
pub struct Picture { pub bytes: Vec<u8>, pub png: bool, pub w: u32, pub h: u32 }

fn run(t: &str, props: &str) -> String { format!("<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>", if props.is_empty() { String::new() } else { format!("<w:rPr>{props}</w:rPr>") }, esc(t)) }
fn para(style: &str, body: &str) -> String { format!("<w:p>{}{body}</w:p>", if style.is_empty() { String::new() } else { format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>") }) }

/// A table: the first row as a heading, thin borders.
fn table(rows: &[Vec<String>]) -> String {
    let cell = |t: &str, head: bool| format!("<w:tc><w:p>{}</w:p></w:tc>", run(t, if head { "<w:b/>" } else { "" }));
    let body: String = rows.iter().enumerate().map(|(i, r)| format!("<w:tr>{}</w:tr>", r.iter().map(|c| cell(c, i == 0)).collect::<String>())).collect();
    format!("<w:tbl><w:tblPr><w:tblStyle w:val=\"Grid\"/><w:tblW w:w=\"0\" w:type=\"auto\"/><w:tblBorders>{}</w:tblBorders></w:tblPr>{body}</w:tbl><w:p/>",
        ["top", "left", "bottom", "right", "insideH", "insideV"].iter().map(|b| format!("<w:{b} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"8D99A8\"/>")).collect::<String>())
}

/// A picture in its own paragraph, `cm` wide.
fn drawing(rel: &str, n: usize, p: &Picture, cm: f64) -> String {
    let w = (cm * 360_000.0) as u64;
    let h = (w as f64 * p.h.max(1) as f64 / p.w.max(1) as f64) as u64;
    format!("<w:p><w:r><w:drawing><wp:inline><wp:extent cx=\"{w}\" cy=\"{h}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\"/><a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"Picture {n}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{rel}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>")
}

/// A picture from a data URL (a picture region): its bytes, kind and size; None if it is not PNG or JPEG.
pub fn picture_from_data_url(src: &str) -> Option<Picture> {
    let (head, b64) = src.strip_prefix("data:")?.split_once(";base64,")?;
    let bytes = b64_decode(b64)?;
    let png = head == "image/png";
    if !png && head != "image/jpeg" { return None; }
    let (w, h) = if png { size_png(&bytes)? } else { size_jpeg(&bytes)? };
    Some(Picture { bytes, png, w, h })
}

fn size_png(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 24 || &b[1..4] != b"PNG" { return None; }
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

/// The document. `plots` are the drawn plots by region id (the window draws them).
pub fn document(s: &Sheet, o: &SheetOut, plots: &std::collections::HashMap<u32, Picture>) -> Vec<u8> {
    let mut body = String::new();
    let mut media: Vec<(String, &Picture)> = vec![];
    let mut owned: Vec<Picture> = vec![];
    // The title block as a small table.
    let t = &s.title;
    let tb: Vec<Vec<String>> = vec![
        vec!["Project".into(), t.project.clone(), "Calc number".into(), t.number.clone()],
        vec!["Title".into(), t.title.clone(), "Revision".into(), t.rev.clone()],
        vec!["Prepared by".into(), t.by.clone(), "Checked by".into(), t.checked.clone()],
        vec!["Company".into(), t.company.clone(), "Date".into(), t.date.clone()],
    ];
    body.push_str(&table(&tb));
    // Pictures of picture regions first, so their bytes live as long as the document.
    for r in s.regions.iter().filter(|r| r.kind == Kind::Image && !r.hide) {
        if let Some(p) = r.data.get("src").and_then(|v| v.as_str()).and_then(picture_from_data_url) { owned.push(p); }
    }
    let mut own = owned.iter();
    let mut n = 0;
    for (r, out) in s.regions.iter().zip(&o.regions) {
        if r.hide { continue; }
        match r.kind {
            Kind::Text => body.push_str(&text(r)),
            Kind::Image => {
                if let Some(p) = r.data.get("src").and_then(|v| v.as_str()).filter(|s| s.starts_with("data:image/png") || s.starts_with("data:image/jpeg")).and_then(|_| own.next()) {
                    n += 1;
                    let rel = format!("rIdImg{n}");
                    body.push_str(&drawing(&rel, n, p, 12.0));
                    media.push((rel, p));
                }
                if !r.text.trim().is_empty() { body.push_str(&para("Caption", &run(r.text.trim(), ""))); }
            }
            Kind::Table => {
                if !r.text.trim().is_empty() { body.push_str(&para("Caption", &run(r.text.trim(), ""))); }
                if let Some(tab) = &out.table {
                    let mut rows = vec![tab.cols.iter().map(|c| if c.unit.is_empty() { c.name.clone() } else { format!("{} ({})", c.name, c.unit) }).collect::<Vec<_>>()];
                    for i in 0..tab.rows { rows.push(tab.cols.iter().map(|c| c.cells.get(i).cloned().unwrap_or_default()).collect()); }
                    body.push_str(&table(&rows));
                }
            }
            _ => {
                body.push_str(&maths(r, out));
                if let (Some(_), Some(p)) = (&out.plot, plots.get(&r.id)) {
                    n += 1;
                    let rel = format!("rIdImg{n}");
                    body.push_str(&drawing(&rel, n, p, 15.0));
                    media.push((rel, p));
                }
            }
        }
    }
    let doc = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>",
        if s.page.paper == "Letter" { 12240 } else { 11906 }, if s.page.paper == "Letter" { 15840 } else { 16838 });
    let rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>{}</Relationships>",
        media.iter().enumerate().map(|(i, (rel, p))| format!("<Relationship Id=\"{rel}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{}.{}\"/>", i + 1, if p.png { "png" } else { "jpeg" })).collect::<String>());
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), CONTENT_TYPES.as_bytes().to_vec()),
        ("_rels/.rels".into(), ROOT_RELS.as_bytes().to_vec()),
        ("word/document.xml".into(), doc.into_bytes()),
        ("word/_rels/document.xml.rels".into(), rels.into_bytes()),
        ("word/styles.xml".into(), STYLES.as_bytes().to_vec()),
    ];
    for (i, (_, p)) in media.iter().enumerate() { files.push((format!("word/media/image{}.{}", i + 1, if p.png { "png" } else { "jpeg" }), p.bytes.clone())); }
    zip(&files)
}

/// A text region: a heading, a note, a reference or plain text, line by line.
fn text(r: &Region) -> String {
    let style = match r.style.as_str() { "heading" => "Heading1", "subheading" => "Heading2", "note" => "Note", "ref" => "Reference", _ => "" };
    r.text.lines().filter(|l| !l.trim().is_empty()).map(|l| para(style, &run(l, ""))).collect()
}

/// A line of maths: the equation as Word maths, its result, its note; a check's PASS or FAIL; an error as such.
fn maths(r: &Region, out: &crate::eval::RegionOut) -> String {
    let code = crate::eval::split_note(&r.text).0;
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
    if let Some(e) = &out.error { p.push_str(&run(&format!("   ⚠ {}", e.msg), "<w:color w:val=\"B42318\"/>")); }
    if !out.note.trim().is_empty() { p.push_str(&run(&format!("   — {}", out.note.trim()), "<w:i/><w:color w:val=\"5B6878\"/>")); }
    para(if out.input { "Input" } else { "" }, &p)
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
            h.extend(0u32.to_le_bytes()); // time and date
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
