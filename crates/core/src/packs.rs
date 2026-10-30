//! The calc packs: ready, referenced sheets embedded in the program (File › New from pack).
//! Each is a .dcalc file in `crates/core/packs`, and each is also a test (`tests::packs`).
use crate::sheet::Sheet;
use serde::Serialize;

pub struct Pack { pub id: &'static str, pub json: &'static str }

pub static PACKS: [Pack; 17] = [
    Pack { id: "pump-power", json: include_str!("../packs/pump-power.dcalc") },
    Pack { id: "npsh", json: include_str!("../packs/npsh.dcalc") },
    Pack { id: "pipe-dp", json: include_str!("../packs/pipe-dp.dcalc") },
    Pack { id: "operating-point", json: include_str!("../packs/operating-point.dcalc") },
    Pack { id: "compressor", json: include_str!("../packs/compressor.dcalc") },
    Pack { id: "hx-lmtd", json: include_str!("../packs/hx-lmtd.dcalc") },
    Pack { id: "pressure-vessel", json: include_str!("../packs/pressure-vessel.dcalc") },
    Pack { id: "beams", json: include_str!("../packs/beams.dcalc") },
    Pack { id: "bolt", json: include_str!("../packs/bolt.dcalc") },
    Pack { id: "motor", json: include_str!("../packs/motor.dcalc") },
    Pack { id: "shaft", json: include_str!("../packs/shaft.dcalc") },
    Pack { id: "tank", json: include_str!("../packs/tank.dcalc") },
    Pack { id: "thermal-expansion", json: include_str!("../packs/thermal-expansion.dcalc") },
    Pack { id: "pipe-wall", json: include_str!("../packs/pipe-wall.dcalc") },
    Pack { id: "column", json: include_str!("../packs/column.dcalc") },
    Pack { id: "cable-drop", json: include_str!("../packs/cable-drop.dcalc") },
    Pack { id: "ac-circuit", json: include_str!("../packs/ac-circuit.dcalc") },
];

/// A pack card for File › New from pack.
#[derive(Serialize)]
pub struct Card { pub id: &'static str, pub name: String, pub field: String, pub desc: String, pub checks: usize, pub regions: usize }

/// The Getting Started sheet (Help › Getting Started, File › Home): it teaches the app by example.
pub static GETTING_STARTED: Pack = Pack { id: "getting-started", json: include_str!("../packs/getting-started.dcalc") };

pub fn sheet(id: &str) -> Option<Sheet> {
    PACKS.iter().chain(std::iter::once(&GETTING_STARTED)).find(|p| p.id == id).and_then(|p| Sheet::from_json(p.json).ok())
}

pub fn cards() -> Vec<Card> {
    PACKS.iter().filter_map(|p| {
        let s = Sheet::from_json(p.json).ok()?;
        let pk = &s.ui["pack"];
        let txt = |k: &str| pk[k].as_str().unwrap_or("").to_string();
        Some(Card { id: p.id, name: s.name.clone(), field: txt("field"), desc: txt("desc"), checks: s.regions.iter().filter(|r| r.kind == crate::Kind::Check).count(), regions: s.regions.len() })
    }).collect()
}
