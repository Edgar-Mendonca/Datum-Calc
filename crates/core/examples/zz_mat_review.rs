use datum_core::{eval, sheet::{Kind, Region, Sheet}};
fn sh(data: serde_json::Value, lines: &[&str]) -> Sheet {
    let mut s = Sheet::new("m");
    s.regions.push(Region { id: 1, kind: Kind::Material, data, ..Default::default() });
    for (i, l) in lines.iter().enumerate() { s.regions.push(Region { id: i as u32 + 2, kind: if l.starts_with("check") { Kind::Check } else { Kind::Math }, text: l.to_string(), ..Default::default() }); }
    s
}
fn dump(s: &Sheet, f: &eval::Fmt) {
    let o = eval::run_with(s, f);
    for r in &o.regions {
        if let Some(m) = &r.material { for x in &m.rows { println!("  {:14} {:28} {:>12} {:12} {}", x.tex, x.what, x.value, x.unit, x.range); } }
        if let Some(e) = &r.error { println!("  ERR {}", e.msg); }
        if !r.result.is_empty() || !r.warn.is_empty() { println!("  [{}] {} warn={} subst={}", r.id, r.result, r.warn, r.subst_tex); }
    }
}
fn main() {
    let si = eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: false };
    let us = eval::Fmt { digits: 4, eng: false, us: true, polar: false, exact: false };
    let ex = eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: true };
    for id in ["steel-s355", "pa66", "nr", "pu-flex"] {
        println!("== {id} SI"); dump(&sh(serde_json::json!({"id": id, "name": "m"}), &[]), &si);
        println!("== {id} US"); dump(&sh(serde_json::json!({"id": id, "name": "m"}), &[]), &us);
    }
    println!("== exact"); dump(&sh(serde_json::json!({"id": "nr", "name": "m"}), &["E_m =", "epsilon_f_m ="]), &ex);
    println!("== redefine above/below");
    let mut s = sh(serde_json::json!({"id": "steel-s355", "name": "steel"}), &["E_steel = 1 GPa", "x = E_steel"]);
    s.regions.insert(0, Region { id: 9, kind: Kind::Math, text: "E_steel = 3 GPa".into(), ..Default::default() });
    dump(&s, &si);
    println!("== two math redefs"); dump(&sh(serde_json::json!({}), &["a = 1", "a = 2"]), &si);
    println!("== collision e_steel/steel");
    let mut s = sh(serde_json::json!({"id": "steel-s355", "name": "steel"}), &[]);
    s.regions.push(Region { id: 5, kind: Kind::Material, data: serde_json::json!({"id": "steel-s355", "name": "e_steel"}), ..Default::default() });
    s.regions.push(Region { id: 6, kind: Kind::Math, text: "rho_e_steel =".into(), ..Default::default() });
    dump(&s, &si);
    println!("== unicode name"); dump(&sh(serde_json::json!({"id": "steel-s355", "name": "ü"}), &["E_ü ="]), &si);
    println!("== name g"); dump(&sh(serde_json::json!({"id": "steel-s355", "name": "g"}), &["E_g =", "x = 2*g"]), &si);
    println!("== subst"); let mut s = sh(serde_json::json!({"id": "steel-s355", "name": "steel"}), &["m = rho_steel*0.5 m^3", "dT = alpha_steel*100 K", "r = rho_e_steel*1 m/(1 mm^2)", "t = T_max_steel - 20 °C", "u = T_max_steel"]); s.subst = true; dump(&s, &si);
    dump(&s, &us);
}
