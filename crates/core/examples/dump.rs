//! Writes the fixture the UI tests run against: a small sheet as the shell's `state` command returns it.
//! cargo run -q -p datum-core --example dump > tests_ui/fixture.json
use datum_core::{Doc, Kind, Op, Sheet};

fn main() {
    let mut d = Doc::new(Sheet::new("Pump power"));
    let rows: [(Kind, &str, &str); 6] = [
        (Kind::Text, "Pump power and motor size", "heading"),
        (Kind::Math, "Q = 120 m^3/h", "input"),
        (Kind::Math, "H = 45 m", "input"),
        (Kind::Math, "P = rho*g*Q*H/eta", ""),
        (Kind::Check, "check P <= P_motor", ""),
        (Kind::Text, "Ref: any fluid mechanics textbook (hydraulic power)", "ref"),
    ];
    for (k, (kind, text, style)) in rows.into_iter().enumerate() {
        d.apply(Op::Insert { at: k, kind, text: text.into() }).unwrap();
        d.apply(Op::Set { id: d.s.regions[k].id, field: "style".into(), value: style.into() }).unwrap();
    }
    d.apply(Op::Title { field: "project".into(), value: "Cooling water upgrade".into() }).unwrap();
    d.apply(Op::Title { field: "number".into(), value: "CALC-001".into() }).unwrap();
    let state = serde_json::json!({ "sheet": d.s, "path": null, "dirty": false, "undo": true, "redo": false, "rev": d.rev() });
    println!("{}", serde_json::json!({ "state": state }));
}
