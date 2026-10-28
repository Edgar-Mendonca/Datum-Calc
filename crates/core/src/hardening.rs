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
        0 | 1 => Op::Insert { at: r.below(s.regions.len() + 3), kind: KINDS[r.below(7)], text: r.text(b"x=12 m*+-/^()abc", 12) },
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
    d.apply(Op::Insert { at: 0, kind: Kind::Math, text: "a = 2 m".into() }).unwrap();
    let before = d.s.clone();
    let (u, r) = (d.can_undo(), d.can_redo());
    // The first op succeeds, the second fails: the batch as a whole must leave nothing behind.
    let bad = Op::Batch(vec![Op::Insert { at: 9, kind: Kind::Text, text: "x".into() }, Op::Set { id: 999, field: "text".into(), value: json!("y") }]);
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
    for k in 0..30 { d.apply(Op::Insert { at: k, kind: KINDS[k % 7], text: format!("x{k} = {k} m") }).unwrap(); }
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
    for k in 0..2000 { d.apply(Op::Insert { at: k, kind: Kind::Math, text: format!("x{k} = {k} kN*m") }).unwrap(); }
    let s = Sheet::from_json(&d.s.json()).unwrap();
    assert_eq!(s.regions.len(), 2000);
    let limit = if cfg!(debug_assertions) { 20.0 } else { 3.0 };
    assert!(t.elapsed().as_secs_f64() < limit, "2,000 inserts and a round trip took {:?}", t.elapsed());
}
