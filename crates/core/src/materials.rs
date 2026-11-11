//! Materials from Datum Atlas (data/materials.json, built by tools/materials.py): a material region picks one
//! and defines its properties as names with units (ρ_steel, E_steel, σ_y,steel …). The values are kept in the
//! sheet when picked, so a calculation reads the same after the library changes.
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Atlas key, the name it gets on the sheet (before _ and the material's name), what it is, and its unit as
/// Atlas stores it (°C is an absolute temperature).
pub const KEYS: [(&str, &str, &str, &str); 19] = [
    ("rho", "rho", "density", "kg/m^3"),
    ("E", "E", "Young's modulus", "GPa"),
    ("G", "G", "shear modulus", "GPa"),
    ("nu", "nu", "Poisson's ratio", ""),
    ("sy", "sigma_y", "yield strength", "MPa"),
    ("ts", "sigma_ts", "tensile strength", "MPa"),
    ("sc", "sigma_c", "compressive strength", "MPa"),
    ("fat", "sigma_e", "fatigue strength at 10⁷ cycles", "MPa"),
    ("elong", "epsilon_f", "elongation at fracture", "%"),
    ("hv", "HV", "Vickers hardness", ""),
    ("tm", "T_m", "melting point", "°C"),
    ("tg", "T_g", "glass temperature", "°C"),
    ("tmax", "T_max", "maximum service temperature", "°C"),
    ("tmin", "T_min", "minimum service temperature", "°C"),
    ("lam", "lambda", "thermal conductivity", "W/(m*K)"),
    ("alpha", "alpha", "thermal expansion", "µm/(m*K)"),
    ("cp", "c_p", "specific heat", "J/(kg*K)"),
    ("res", "rho_e", "electrical resistivity", "µΩ*cm"),
    ("eps", "epsilon_r", "dielectric constant", ""),
];

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Source { pub title: String, pub kind: String, pub note: String }

/// One material as the library and a sheet keep it: each property a range (low, high, source id).
#[derive(Debug, Clone, Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Material { pub id: String, pub name: String, pub family: String, pub syn: Vec<String>, pub des: String, pub status: String, pub notes: String, pub props: HashMap<String, (f64, f64, String)> }

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Library { pub from: String, pub sources: HashMap<String, Source>, pub materials: Vec<Material> }

pub const JSON: &str = include_str!("../data/materials.json");

pub fn library() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| serde_json::from_str(JSON).unwrap_or_default())
}

pub fn find(id: &str) -> Option<&'static Material> { library().materials.iter().find(|m| m.id == id) }

/// The source a value names, as a title for the sheet (a sheet keeps its own copy, with the values).
pub fn source_title(id: &str) -> String { library().sources.get(id).map(|s| s.title.clone()).unwrap_or_else(|| id.to_string()) }

/// A material region's data: the material (kept as picked), the short name its properties take, and which end
/// of each range to use.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MatData { pub id: String, pub name: String, pub pick: String, pub mat: Option<Material>, pub sources: HashMap<String, String> }

/// The value used from a range: the middle (typical), the low end or the high end.
pub fn pick(lo: f64, hi: f64, how: &str) -> f64 { match how { "low" => lo.min(hi), "high" => lo.max(hi), _ => (lo + hi) / 2.0 } }

/// A short name a material's properties can take: letters, digits and _ after a letter (steel, Al6061, s_1).
pub fn valid_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_alphabetic()) && c.all(|x| x.is_alphanumeric() || x == '_') && n.chars().count() <= 24
}
