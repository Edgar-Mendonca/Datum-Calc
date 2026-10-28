//! Prints every calc pack worked out, in SI and US units, for checking them by eye.
//! cargo run -q -p datum-core --example packs
use datum_core::{eval, packs};

fn main() {
    for p in &packs::PACKS {
        let mut s = packs::sheet(p.id).expect("pack opens");
        for us in [false, true] {
            s.units = if us { "US".into() } else { "SI".into() };
            let o = eval::run(&s);
            println!("== {} ({})", s.name, s.units);
            for (r, g) in s.regions.iter().zip(&o.regions) {
                let line = r.text.split('#').next().unwrap_or("").trim();
                if g.result.is_empty() && g.error.is_none() { continue; }
                println!("  {line:<52} → {}{}{}", g.result, g.error.as_ref().map(|e| format!("  ERROR {}", e.msg)).unwrap_or_default(), if g.warn.is_empty() { String::new() } else { format!("  WARN {}", g.warn) });
            }
        }
    }
}
