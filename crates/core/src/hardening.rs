//! The hardening standard (FAMILY §11): failed edits change nothing, random sheets and edits never
//! panic and keep their invariants, damaged files open or fail clearly, and a speed gate.
use crate::sheet::*;
use serde_json::json;

/// A small xorshift generator, so the tests need no extra crate and always run the same.
pub(crate) struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    pub fn below(&mut self, n: usize) -> usize { (self.next() % n.max(1) as u64) as usize }
    pub fn text(&mut self, alphabet: &[u8], max: usize) -> String { (0..self.below(max + 1)).map(|_| alphabet[self.below(alphabet.len())] as char).collect() }
}

const KINDS: [Kind; 7] = [Kind::Math, Kind::Text, Kind::Check, Kind::Solve, Kind::Plot, Kind::Table, Kind::Image];

fn random_op(r: &mut Rng, s: &Sheet) -> Op {
    let id = if s.regions.is_empty() || r.below(8) == 0 { r.next() as u32 } else { s.regions[r.below(s.regions.len())].id };
    match r.below(9) {
        0 | 1 => Op::Insert { at: r.below(s.regions.len() + 3), kind: KINDS[r.below(7)], text: r.text(b"x=12 m*+-/^()abc", 12), data: Default::default() },
        2 => Op::Set { id, field: ["text", "unit", "style", "kind", "data", "nope"][r.below(6)].into(), value: [json!("kN"), json!(3), json!("math"), json!(null), json!({"a": 1})][r.below(5)].clone() },
        3 => Op::Delete { ids: vec![id] },
        4 => Op::Move { ids: vec![id], up: r.below(2) == 0 },
        5 => Op::Title { field: ["project", "by", "rev", "bad"][r.below(4)].into(), value: r.text(b"AB 12", 6) },
        6 => Op::SheetSet { field: ["name", "units", "digits", "paper", "margins", "x"][r.below(6)].into(), value: [json!("US"), json!(""), json!(5), json!(99), json!([1, 2, 3, 4])][r.below(5)].clone() },
        7 => Op::Ui { key: "zoom".into(), value: json!(r.below(200)) },
        _ => Op::Batch((0..r.below(4)).map(|_| Op::Delete { ids: vec![r.next() as u32] }).collect()),
    }
}

fn invariants(s: &Sheet) {
    let mut ids: Vec<u32> = s.regions.iter().map(|r| r.id).collect();
    assert!(ids.iter().all(|&i| i > 0 && i < s.next_id), "ids below next_id");
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), s.regions.len(), "ids are unique");
    assert!((2..=10).contains(&s.digits));
    assert!(s.units == "SI" || s.units == "US");
}

#[test]
fn failed_edit_changes_nothing() {
    let mut d = Doc::new(Sheet::new("T"));
    d.apply(Op::Insert { at: 0, kind: Kind::Math, text: "a = 2 m".into(), data: Default::default() }).unwrap();
    let before = d.s.clone();
    let (u, r) = (d.can_undo(), d.can_redo());
    // The first op succeeds, the second fails: the batch as a whole must leave nothing behind.
    let bad = Op::Batch(vec![Op::Insert { at: 9, kind: Kind::Text, text: "x".into(), data: Default::default() }, Op::Set { id: 999, field: "text".into(), value: json!("y") }]);
    assert!(d.apply(bad).is_err());
    for op in [Op::Set { id: 1, field: "colour".into(), value: json!("red") }, Op::Title { field: "nope".into(), value: "x".into() }, Op::SheetSet { field: "digits".into(), value: json!(40) }, Op::Delete { ids: vec![77] }] {
        assert!(d.apply(op).is_err());
    }
    assert_eq!(d.s, before);
    assert_eq!((d.can_undo(), d.can_redo()), (u, r));
}

#[test]
fn random_edits_keep_invariants_and_undo_restores() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..60 {
        let mut d = Doc::new(Sheet::new("R"));
        let mut history = vec![d.s.clone()];
        for _ in 0..80 {
            let op = random_op(&mut r, &d.s);
            let rev = d.rev();
            if d.apply(op).is_ok() && d.rev() != rev { history.push(d.s.clone()); }
            invariants(&d.s);
            let back = Sheet::from_json(&d.s.json()).unwrap();
            assert_eq!(back, d.s, "save and open give the same sheet");
        }
        // Undo walks back through every state, in order.
        while d.can_undo() {
            history.pop();
            d.undo();
            assert_eq!(&d.s, history.last().unwrap());
        }
    }
}

#[test]
fn damaged_files_open_or_fail_clearly() {
    let mut d = Doc::new(Sheet::new("Damaged"));
    for k in 0..30 { d.apply(Op::Insert { at: k, kind: KINDS[k % 7], text: format!("x{k} = {k} m"), data: Default::default() }).unwrap(); }
    let good = d.s.json();
    let mut r = Rng(42);
    for n in 0..2000 {
        let mut b = good.clone().into_bytes();
        match n % 3 {
            0 => b.truncate(r.below(b.len())),
            1 => for _ in 0..1 + r.below(6) { let i = r.below(b.len()); b[i] = b"{}[]\",:0x\\\n"[r.below(11)]; },
            _ => { let i = r.below(b.len()); let j = (i + r.below(200)).min(b.len()); b.drain(i..j); }
        }
        let text = String::from_utf8_lossy(&b);
        match Sheet::from_json(&text) { Ok(s) => invariants(&s), Err(e) => assert!(!e.is_empty()) }
    }
    // A damaged region is dropped; the rest of the sheet survives.
    let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
    v["regions"][3]["kind"] = json!("hologram");
    v["regions"][5] = json!(17);
    v["title"]["by"] = json!(["not", "text"]);
    let s = Sheet::from_json(&v.to_string()).unwrap();
    assert_eq!(s.regions.len(), 28);
    assert_eq!(s.name, "Damaged");
    // Duplicate and zero ids are renumbered.
    v = serde_json::from_str(&good).unwrap();
    v["regions"][1]["id"] = json!(1);
    v["regions"][2]["id"] = json!(0);
    v["next_id"] = json!(0);
    invariants(&Sheet::from_json(&v.to_string()).unwrap());
    assert!(Sheet::from_json(r#"{"format": 99}"#).unwrap_err().contains("newer"));
    assert!(Sheet::from_json("[1, 2]").is_err());
    assert!(Sheet::from_json("").is_err());
}

#[test]
fn speed_gate_large_sheet() {
    let t = std::time::Instant::now();
    let mut d = Doc::new(Sheet::new("Big"));
    for k in 0..2000 { d.apply(Op::Insert { at: k, kind: Kind::Math, text: format!("x{k} = {k} kN*m"), data: Default::default() }).unwrap(); }
    let s = Sheet::from_json(&d.s.json()).unwrap();
    assert_eq!(s.regions.len(), 2000);
    let limit = if cfg!(debug_assertions) { 20.0 } else { 3.0 };
    assert!(t.elapsed().as_secs_f64() < limit, "2,000 inserts and a round trip took {:?}", t.elapsed());
}

#[test]
fn random_text_never_panics_the_parser_or_evaluator() {
    let mut r = Rng(0xDA7A_CA1C);
    let pieces: [&str; 40] = ["x", "y", "=", "+", "-", "*", "/", "^", "(", ")", ",", "..", "1", "0", "2.5", "1e308", "-1e-308", "m", "kg", "s", "°C", "°F", "kN*m", "m^3/h",
        "sqrt(", "sin(", "if(", "max(", "f(", "check ", "solve ", "plot ", " for x in ", "<=", ">", "σ", "_", "²", "%", " "];
    let mut texts: Vec<String> = (0..3000).map(|_| (0..1 + r.below(12)).map(|_| pieces[r.below(pieces.len())]).collect()).collect();
    texts.extend((0..1000).map(|_| r.text(b"xy=+-*/^().,0123456789 me<>_!$#[]{}\\\"'", 24)));
    for chunk in texts.chunks(8) {
        let mut s = Sheet::new("fuzz");
        s.regions.push(Region { id: 1, kind: Kind::Math, text: "f(x) = x^2".into(), ..Default::default() });
        for (i, t) in chunk.iter().enumerate() {
            let _ = crate::parse::statement(t, &[]);
            s.regions.push(Region { id: i as u32 + 2, kind: if t.starts_with("check") { Kind::Check } else { Kind::Math }, text: t.clone(), ..Default::default() });
        }
        let o = crate::eval::run(&s);
        assert_eq!(o.regions.len(), s.regions.len());
        for g in &o.regions {
            if let Some(e) = &g.error { assert!(!e.msg.is_empty() && e.from <= e.to && e.to <= 200, "{e:?}"); }
        }
    }
}

#[test]
fn evaluation_speed_gate() {
    let mut s = Sheet::new("Big");
    s.regions.push(Region { id: 1, kind: Kind::Math, text: "x0 = 1 m".into(), ..Default::default() });
    for k in 1..1500 { s.regions.push(Region { id: k + 1, kind: Kind::Math, text: format!("x{k} = x{} * 1.001 + 2 mm", k - 1), ..Default::default() }); }
    s.regions.push(Region { id: 9000, kind: Kind::Math, text: "v = 0, 0.002 .. 100".into(), ..Default::default() });
    s.regions.push(Region { id: 9001, kind: Kind::Math, text: "w = sum(sqrt(v)*2)".into(), ..Default::default() });
    let t = std::time::Instant::now();
    let o = crate::eval::run(&s);
    assert_eq!(o.errors, 0);
    let limit = if cfg!(debug_assertions) { 10.0 } else { 1.0 };
    assert!(t.elapsed().as_secs_f64() < limit, "1,500 lines took {:?}", t.elapsed());
}

/// A random expression from the grammar: numbers with units, names, operators, functions, brackets.
fn random_expr(r: &mut Rng, depth: u32) -> String {
    const UNITS: [&str; 14] = ["m", "mm", "kg", "s", "N", "kN*m", "m^3/h", "MPa", "°C", "°F", "K", "rpm", "W/(m^2*K)", "%"];
    const NUMS: [&str; 10] = ["0", "1", "2.5", "-3", "1e-9", "1e9", "1e300", "1e-300", "0.0001", "42"];
    const FUNCS: [&str; 16] = ["sqrt", "sin", "cos", "ln", "log", "exp", "abs", "min", "max", "round", "sum", "mean", "len", "if", "hypot", "f"];
    if depth == 0 || r.below(4) == 0 {
        return match r.below(4) { 0 => NUMS[r.below(10)].into(), 1 => format!("{} {}", NUMS[r.below(10)], UNITS[r.below(14)]), 2 => ["a", "b", "v", "g", "pi", "zz"][r.below(6)].into(), _ => format!("({})", random_expr(r, depth.saturating_sub(1))) };
    }
    match r.below(4) {
        0 | 1 => format!("{} {} {}", random_expr(r, depth - 1), ["+", "-", "*", "/", "^", "<=", ">"][r.below(7)], random_expr(r, depth - 1)),
        2 => { let f = FUNCS[r.below(16)]; let n = if f == "if" { 3 } else if matches!(f, "min" | "max" | "round" | "hypot") { 2 } else { 1 }; format!("{f}({})", (0..n).map(|_| random_expr(r, depth - 1)).collect::<Vec<_>>().join(", ")) }
        _ => format!("-{}", random_expr(r, depth - 1)),
    }
}

#[test]
fn random_expressions_never_panic() {
    let mut r = Rng(0x5EED_CA1C_0001);
    for round in 0..600 {
        let mut s = Sheet::new("fuzz");
        let mut push = |kind: Kind, text: String, data: serde_json::Value| { let id = s.regions.len() as u32 + 1; s.regions.push(Region { id, kind, text, data, ..Default::default() }); };
        push(Kind::Math, "a = 2 m".into(), json!(null));
        push(Kind::Math, "b = 3".into(), json!(null));
        push(Kind::Math, "v = 0, 0.5 .. 3".into(), json!(null));
        push(Kind::Math, "f(x) = x^2 + b".into(), json!(null));
        if round % 5 == 0 {
            let cell = |r: &mut Rng| [json!("1"), json!("2 m"), json!(""), json!("x"), json!(3), json!(null), json!("1e400")][r.below(7)].clone();
            let rows: Vec<serde_json::Value> = (0..r.below(5)).map(|_| json!([cell(&mut r), cell(&mut r)])).collect();
            push(Kind::Table, String::new(), json!({ "cols": [{ "name": "c1", "unit": (["", "m", "zz", "°C"][r.below(4)]) }, { "name": "c2", "formula": (["", "c1*2", "c1 +", "sum(c1)"][r.below(4)]) }], "rows": rows }));
        }
        for _ in 0..8 {
            let e = random_expr(&mut r, 4);
            let text = match r.below(6) {
                0 => format!("check {e} <= {}", random_expr(&mut r, 2)),
                1 => format!("solve {e} = {} for x in {} .. {}", random_expr(&mut r, 2), random_expr(&mut r, 1), random_expr(&mut r, 1)),
                2 => format!("plot {e} for x in 0 .. {}", random_expr(&mut r, 1)),
                3 => format!("w{} = {e}  # note", r.below(5)),
                4 => format!("v{} = {} .. {}", r.below(3), random_expr(&mut r, 1), random_expr(&mut r, 1)),
                _ => format!("{e} ="),
            };
            let kind = if text.starts_with("check") { Kind::Check } else { Kind::Math };
            push(kind, text, json!(null));
        }
        for us in [false, true] {
            let o = crate::eval::run_with(&s, &crate::eval::Fmt { digits: [2, 4, 10][round % 3], eng: round % 7 == 0, us });
            assert_eq!(o.regions.len(), s.regions.len());
            for g in &o.regions {
                assert!(g.result.len() < 2000 && g.latex.len() < 20_000, "{}", g.latex);
                if let Some(e) = &g.error { assert!(!e.msg.is_empty() && e.from <= e.to); }
                if let Some(p) = &g.plot { assert!(p.series.iter().all(|s| s.ys.len() == p.xs.len())); }
            }
        }
    }
}

#[test]
fn units_latex_and_numbers_never_panic() {
    let mut r = Rng(77);
    for _ in 0..20_000 {
        let t = r.text("m/^*()23-ksNPaW°CF·²³µΩ%h_ ".as_bytes(), 14);
        if let Ok(u) = crate::units::parse(&t) { assert!(u.factor.is_finite() || u.factor == 0.0, "{t}"); }
        let _ = crate::latex::unit(&crate::units::pretty(&t));
        let _ = crate::units::system(&t);
    }
    let specials = [0.0, -0.0, f64::MIN_POSITIVE, 5e-324, 1e-300, 1e-12, 0.999_999_5, 999_999.5, 1e21, f64::MAX, -f64::MAX, f64::NAN, f64::INFINITY, -1e-7];
    for x in specials.iter().copied().chain((0..20_000).map(|_| f64::from_bits(r.next()))) {
        for d in [2, 4, 10] { for eng in [false, true] {
            let (t, l) = crate::eval::fmt_num(x, d, eng);
            assert!(!t.is_empty() && !l.is_empty() && t.len() < 60, "{x:e} → {t}");
        } }
    }
}

#[test]
fn damaged_tables_and_pictures_open_and_work_out() {
    let mut r = Rng(9);
    let good = json!({ "format": 1, "name": "T", "regions": [
        { "id": 1, "kind": "table", "text": "", "data": { "cols": [{ "name": "Q", "unit": "m^3/h" }, { "name": "H", "unit": "m", "formula": "" }], "rows": [["0", "60"], ["50", "55"]] } },
        { "id": 2, "kind": "image", "text": "caption", "data": { "src": "data:image/png;base64,AAAA", "width": 60 } },
        { "id": 3, "kind": "math", "text": "x = interp(25 m^3/h, Q, H)" } ] }).to_string();
    for n in 0..3000 {
        let mut b = good.clone().into_bytes();
        for _ in 0..1 + r.below(4) { let i = r.below(b.len()); b[i] = b"{}[]\",:0x-9\\ "[r.below(13)]; }
        if n % 4 == 0 { b.truncate(r.below(b.len())); }
        if let Ok(s) = Sheet::from_json(&String::from_utf8_lossy(&b)) { let o = crate::eval::run(&s); assert_eq!(o.regions.len(), s.regions.len()); }
    }
}
