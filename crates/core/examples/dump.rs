//! Writes the fixture the UI tests run against: a small sheet as the shell's `state` command returns it.
//! cargo run -q -p datum-core --example dump > tests_ui/fixture.json
use datum_core::{eval, Doc, Kind, Op, Sheet};

fn main() {
    // With a pack id (and "us"), dump that pack instead: cargo run --example dump -- shaft us
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(mut s) = args.first().and_then(|id| datum_core::packs::sheet(id)) {
        if args.iter().any(|a| a == "us") { s.units = "US".into(); }
        if args.iter().any(|a| a == "numbers") { s.subst = true; }
        let state = serde_json::json!({ "sheet": s, "calc": eval::run(&s), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
        println!("{}", serde_json::json!({ "state": state, "packs": datum_core::packs::cards() }));
        return;
    }
    let mut d = Doc::new(Sheet::new("Pump power"));
    let rows: [(Kind, &str, &str); 11] = [
        (Kind::Text, "Pump power and motor size", "heading"),
        (Kind::Math, "Q = 120 m^3/h", "input"),
        (Kind::Math, "H = 45 m", "input"),
        (Kind::Math, "rho = 1000 kg/m^3", "input"),
        (Kind::Math, "eta = 0.75", "input"),
        (Kind::Math, "P = rho*g*Q*H/eta", ""),
        (Kind::Math, "P_motor = 22 kW", ""),
        (Kind::Check, "check P <= P_motor", ""),
        (Kind::Math, "x = P + Q", ""),
        (Kind::Plot, "plot P*Q_/Q for Q_ in 0 m^3/h .. 150 m^3/h", ""),
        (Kind::Text, "Ref: any fluid mechanics textbook (hydraulic power)", "ref"),
    ];
    for (k, (kind, text, style)) in rows.into_iter().enumerate() {
        d.apply(Op::Insert { at: k, kind, text: text.into(), data: Default::default() }).unwrap();
        d.apply(Op::Set { id: d.s.regions[k].id, field: "style".into(), value: style.into() }).unwrap();
    }
    d.apply(Op::Title { field: "project".into(), value: "Cooling water upgrade".into() }).unwrap();
    d.apply(Op::Title { field: "number".into(), value: "CALC-001".into() }).unwrap();
    // A table and a picture before the reference.
    let at = d.s.regions.len() - 1;
    let table = serde_json::json!({ "cols": [{ "name": "Q_d", "unit": "m^3/h", "formula": "" }, { "name": "H_d", "unit": "m", "formula": "" }, { "name": "P_d", "unit": "kW", "formula": "rho*g*Q_d*H_d" }], "rows": [["0", "60"], ["50", "55"], ["100", "40"]] });
    let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    d.apply(Op::Insert { at, kind: Kind::Table, text: "Pump curve".into(), data: table }).unwrap();
    d.apply(Op::Insert { at: at + 1, kind: Kind::Image, text: "Free-body diagram".into(), data: serde_json::json!({ "src": png, "width": 40 }) }).unwrap();
    let state = serde_json::json!({ "sheet": d.s, "calc": eval::run(&d.s), "path": null, "dirty": false, "undo": true, "redo": false, "rev": d.rev() });
    // The AC circuit pack in polar form with its numbers: complex results drawn in the window (0.1.8).
    let mut ac = datum_core::packs::sheet("ac-circuit").expect("the AC pack opens");
    (ac.polar, ac.subst) = (true, true);
    let ac = serde_json::json!({ "sheet": ac, "calc": eval::run(&ac), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    // The RC filter pack: Bode plots on a log frequency axis (0.1.11).
    let rc = datum_core::packs::sheet("rc-filter").expect("the RC pack opens");
    let rc = serde_json::json!({ "sheet": rc, "calc": eval::run(&rc), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    // Exact maths with View › Fractions on (0.2.0), a matrix (0.2.4), a program (0.2.5) and properties (0.2.6).
    let mut alg = Sheet::new("Algebra");
    for (k, t) in ["expand((x+1)^2)", "factor(2*x^2 - x - 1)", "y = expand((x+1)^3)", "factor(y)", "a = 1/3", "simplify((x^2 - 1)/(x - 1))", "K = [[2, -1], [-1, 2]] kN/m", "u = lsolve(K, [10, 0] kN)"].iter().enumerate() {
        alg.regions.push(datum_core::Region { id: k as u32 + 1, kind: Kind::Math, text: t.to_string(), ..Default::default() });
    }
    alg.regions.push(datum_core::Region { id: 9, kind: Kind::Program, text: "s = 0\nfor i in 1 .. 10\n  if i > 4\n    break\n  end\n  s = s + i\nend".into(), ..Default::default() });
    alg.regions.push(datum_core::Region { id: 10, kind: Kind::Math, text: "h_1 = water_h(400 °C, 40 bar)".into(), ..Default::default() });
    alg.regions.push(datum_core::Region { id: 11, kind: Kind::Math, text: "c_p = gas_cp(CO2, 500 K)".into(), ..Default::default() });
    alg.next_id = 12;
    alg.exact = true;
    let alg = serde_json::json!({ "sheet": alg, "calc": eval::run(&alg), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    // A material region (0.2.1): S355 as "steel", used by the lines below.
    let mut mat = Sheet::new("Materials");
    mat.regions.push(datum_core::Region { id: 1, kind: Kind::Material, data: serde_json::json!({ "id": "steel-s355", "name": "steel", "pick": "typical" }), ..Default::default() });
    for (k, t) in ["sigma = 200 MPa", "check sigma <= sigma_y_steel/1.1", "m = rho_steel*0.5 m^3"].iter().enumerate() {
        mat.regions.push(datum_core::Region { id: k as u32 + 2, kind: if t.starts_with("check") { Kind::Check } else { Kind::Math }, text: t.to_string(), ..Default::default() });
    }
    mat.next_id = 5;
    let mat = serde_json::json!({ "sheet": mat, "calc": eval::run(&mat), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    // Data in and a fit (0.2.7): a table read from text, a fitted curve and the measured points on one plot.
    let tab = datum_core::tabin::parse("Q (m3/h)\tH [m]\n0\t50.3\n40\t46.5\n80\t37.4\n120\t20.9\n160\t-1\n", true).unwrap_or_default();
    let mut fit = Sheet::new("Fit");
    fit.regions.push(datum_core::Region { id: 1, kind: Kind::Table, text: "Pump curve".into(), data: serde_json::json!({ "cols": tab["cols"], "rows": tab["rows"] }), ..Default::default() });
    fit.regions.push(datum_core::Region { id: 2, kind: Kind::Math, text: "fit H_f(q) = a + b*q + c*q^2 to Q, H".into(), ..Default::default() });
    fit.regions.push(datum_core::Region { id: 3, kind: Kind::Plot, text: "plot H_f(q), points(Q, H) for q in 0 m^3/h .. 170 m^3/h".into(), ..Default::default() });
    fit.regions.push(datum_core::Region { id: 4, kind: Kind::Math, text: "ode y'' = -y/(0.01 s^2) for t in 0 .. 1 s from y = 1 cm, y' = 0".into(), ..Default::default() });
    fit.next_id = 5;
    let fit = serde_json::json!({ "sheet": fit, "calc": eval::run(&fit), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    // Folding, hiding and export (0.2.10): a folded heading over a line, a line hidden in print, a plot.
    let mut ex = Sheet::new("Export");
    let rows: [(Kind, &str, &str, bool, bool); 6] = [
        (Kind::Text, "Folded part", "heading", true, false), (Kind::Math, "a_fold = 2 m", "", false, false),
        (Kind::Text, "Shown part", "heading", false, false), (Kind::Math, "b = 3 m", "", false, false),
        (Kind::Math, "secret = 99 kg", "", false, true), (Kind::Plot, "plot b*x for x in 0 .. 2", "", false, false),
    ];
    for (k, (kind, text, style, fold, hide)) in rows.into_iter().enumerate() {
        ex.regions.push(datum_core::Region { id: k as u32 + 1, kind, text: text.into(), style: style.into(), fold, hide, ..Default::default() });
    }
    ex.next_id = 7;
    let ex = serde_json::json!({ "sheet": ex, "calc": eval::run(&ex), "path": null, "dirty": false, "undo": false, "redo": false, "rev": 1 });
    let mut tabin = tab.clone();
    tabin["caption"] = "pump".into();
    println!("{}", serde_json::json!({ "state": state, "packs": datum_core::packs::cards(), "ac": ac, "rc": rc, "alg": alg, "mat": mat, "fit": fit, "tabin": tabin, "ex": ex }));
}
