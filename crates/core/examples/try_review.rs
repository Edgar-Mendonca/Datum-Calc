use datum_core::{eval, sheet::{Kind, Sheet}};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut s = Sheet::new("t");
    for (i, l) in a.iter().enumerate() {
        let mut r = datum_core::sheet::Region::default(); r.id = i as u32 + 1; r.kind = if l.starts_with("check") { Kind::Check } else if l.contains('\n') { Kind::Program } else { Kind::Math };
        r.text = l.clone(); s.regions.push(r);
    }
    let o = eval::run(&s); if std::env::var("FULL").is_ok() { for r in &o.regions { eprintln!("{:?}", r.defs.iter().map(|d| d.value.clone()).collect::<Vec<_>>()); } }
    for r in o.regions { println!("{:<50} | {} {:?}\n   {}", r.result, r.error.as_ref().map(|e| e.msg.clone()).unwrap_or_default(), r.error.map(|e| (e.from, e.to)), r.latex); }
}
