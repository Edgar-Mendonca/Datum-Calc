//! Units: dimensions, the unit table, unit expressions such as `kN*m`, `m^3/h` or `W/(m^2*K)`,
//! and the choice of unit a result is shown in.
//!
//! Every value is held in SI base units with a dimension vector. Angle is a soft tag: it is kept so
//! results can be shown in rad/s or rpm, but it never makes two values incompatible (an angle is a
//! ratio). Temperatures given in °C or °F are absolute (held in kelvin, `abs`); their differences are
//! plain kelvin, so 20 °C − 10 °C = 10 K and a rise of 10 K reads as 10 Δ°C or 18 Δ°F.

/// Exponents of length, mass, time, temperature, current, amount, luminous intensity, angle.
pub type Dim = [i8; 8];
pub const NONE: Dim = [0; 8];
#[allow(clippy::too_many_arguments)]
const fn d(l: i8, m: i8, t: i8, k: i8, a: i8, n: i8, j: i8, ang: i8) -> Dim { [l, m, t, k, a, n, j, ang] }
pub const TEMP: Dim = d(0, 0, 0, 1, 0, 0, 0, 0);
pub const ANGLE: Dim = d(0, 0, 0, 0, 0, 0, 0, 1);

/// Same dimensions, ignoring the angle tag.
pub fn same(a: &Dim, b: &Dim) -> bool { a[..7] == b[..7] }
pub fn mul(a: &Dim, b: &Dim) -> Dim { std::array::from_fn(|i| a[i].saturating_add(b[i])) }
pub fn div(a: &Dim, b: &Dim) -> Dim { std::array::from_fn(|i| a[i].saturating_sub(b[i])) }
pub fn pow(a: &Dim, n: i8) -> Dim { std::array::from_fn(|i| a[i].saturating_mul(n)) }
pub fn is_none(a: &Dim) -> bool { a[..7] == [0; 7] }

/// A unit: its symbol, its factor to SI and its dimension. `offset` is added after the factor for
/// absolute temperatures (°C: K = °C + 273.15).
#[derive(Debug, Clone, Copy)]
pub struct Unit { pub sym: &'static str, pub factor: f64, pub dim: Dim, pub offset: f64, pub name: &'static str, pub group: &'static str }

const L: Dim = d(1, 0, 0, 0, 0, 0, 0, 0);
const M: Dim = d(0, 1, 0, 0, 0, 0, 0, 0);
const T: Dim = d(0, 0, 1, 0, 0, 0, 0, 0);
const I: Dim = d(0, 0, 0, 0, 1, 0, 0, 0);
const N_: Dim = d(0, 0, 0, 0, 0, 1, 0, 0);
const J_: Dim = d(0, 0, 0, 0, 0, 0, 1, 0);
const AREA: Dim = d(2, 0, 0, 0, 0, 0, 0, 0);
const VOL: Dim = d(3, 0, 0, 0, 0, 0, 0, 0);
const FORCE: Dim = d(1, 1, -2, 0, 0, 0, 0, 0);
const PRESS: Dim = d(-1, 1, -2, 0, 0, 0, 0, 0);
const ENERGY: Dim = d(2, 1, -2, 0, 0, 0, 0, 0);
const POWER: Dim = d(2, 1, -3, 0, 0, 0, 0, 0);
const FREQ: Dim = d(0, 0, -1, 0, 0, 0, 0, 0);
const ANGVEL: Dim = d(0, 0, -1, 0, 0, 0, 0, 1);
const SPEED: Dim = d(1, 0, -1, 0, 0, 0, 0, 0);
const FLOW: Dim = d(3, 0, -1, 0, 0, 0, 0, 0);
const MFLOW: Dim = d(0, 1, -1, 0, 0, 0, 0, 0);
const DVISC: Dim = d(-1, 1, -1, 0, 0, 0, 0, 0);
const KVISC: Dim = d(2, 0, -1, 0, 0, 0, 0, 0);
const CHARGE: Dim = d(0, 0, 1, 0, 1, 0, 0, 0);
const VOLT: Dim = d(2, 1, -3, 0, -1, 0, 0, 0);
const OHM: Dim = d(2, 1, -3, 0, -2, 0, 0, 0);
const SIEMENS: Dim = d(-2, -1, 3, 0, 2, 0, 0, 0);
const FARAD: Dim = d(-2, -1, 4, 0, 2, 0, 0, 0);
const HENRY: Dim = d(2, 1, -2, 0, -2, 0, 0, 0);
const WEBER: Dim = d(2, 1, -2, 0, -1, 0, 0, 0);
const TESLA: Dim = d(0, 1, -2, 0, -1, 0, 0, 0);
const DENS: Dim = d(-3, 1, 0, 0, 0, 0, 0, 0);

const PI: f64 = std::f64::consts::PI;
const IN: f64 = 0.0254;
const FT: f64 = 0.3048;
const LB: f64 = 0.453_592_37;
const GN: f64 = 9.806_65;
const LBF: f64 = LB * GN;
const GAL: f64 = 231.0 * IN * IN * IN;
const BTU: f64 = 1_055.055_852_62;
const HP: f64 = 550.0 * FT * LBF;

macro_rules! units {
    ($($sym:expr, $f:expr, $dim:expr, $name:expr, $group:expr;)*) => { &[$(Unit { sym: $sym, factor: $f, dim: $dim, offset: 0.0, name: $name, group: $group }),*] };
}

/// The unit table (factors from NIST SP 811, 2008, Appendix B). Symbols are matched exactly;
/// aliases come after the main symbol and share its factor.
pub static UNITS: &[Unit] = units![
    // Length
    "m", 1.0, L, "metre", "Length"; "km", 1e3, L, "kilometre", "Length"; "cm", 1e-2, L, "centimetre", "Length"; "mm", 1e-3, L, "millimetre", "Length";
    "µm", 1e-6, L, "micrometre", "Length"; "um", 1e-6, L, "micrometre", "Length"; "nm", 1e-9, L, "nanometre", "Length";
    "in", IN, L, "inch", "Length"; "ft", FT, L, "foot", "Length"; "yd", 0.9144, L, "yard", "Length"; "mi", 1609.344, L, "mile", "Length";
    "dm", 0.1, L, "decimetre", "Length";
    "nmi", 1852.0, L, "nautical mile", "Length"; "mil", 2.54e-5, L, "mil (thou)", "Length";
    // Mass
    "kg", 1.0, M, "kilogram", "Mass"; "g", 1e-3, M, "gram", "Mass"; "mg", 1e-6, M, "milligram", "Mass"; "t", 1e3, M, "tonne", "Mass"; "tonne", 1e3, M, "tonne", "Mass";
    "lb", LB, M, "pound (mass)", "Mass"; "lbm", LB, M, "pound (mass)", "Mass"; "oz", LB / 16.0, M, "ounce", "Mass"; "slug", LBF / FT, M, "slug", "Mass";
    "ton_us", 2000.0 * LB, M, "short ton", "Mass"; "grain", LB / 7000.0, M, "grain", "Mass";
    "Mg", 1e3, M, "megagram", "Mass"; "µg", 1e-9, M, "microgram", "Mass";
    // Time
    "s", 1.0, T, "second", "Time"; "ms", 1e-3, T, "millisecond", "Time"; "µs", 1e-6, T, "microsecond", "Time"; "us", 1e-6, T, "microsecond", "Time";
    "min", 60.0, T, "minute", "Time"; "h", 3600.0, T, "hour", "Time"; "hr", 3600.0, T, "hour", "Time"; "day", 86400.0, T, "day", "Time";
    "week", 604_800.0, T, "week", "Time"; "yr", 31_557_600.0, T, "year (365.25 days)", "Time";
    "ns", 1e-9, T, "nanosecond", "Time";
    // Temperature (absolute scales have an offset; K and °R are the same as their differences)
    "K", 1.0, TEMP, "kelvin", "Temperature"; "°R", 5.0 / 9.0, TEMP, "degree Rankine", "Temperature"; "degR", 5.0 / 9.0, TEMP, "degree Rankine", "Temperature";
    "Δ°C", 1.0, TEMP, "degree Celsius (difference)", "Temperature"; "delta°C", 1.0, TEMP, "degree Celsius (difference)", "Temperature"; "deltaC", 1.0, TEMP, "degree Celsius (difference)", "Temperature";
    "Δ°F", 5.0 / 9.0, TEMP, "degree Fahrenheit (difference)", "Temperature"; "delta°F", 5.0 / 9.0, TEMP, "degree Fahrenheit (difference)", "Temperature"; "deltaF", 5.0 / 9.0, TEMP, "degree Fahrenheit (difference)", "Temperature";
    // Current, amount, light
    "A", 1.0, I, "ampere", "Electric"; "mA", 1e-3, I, "milliampere", "Electric"; "kA", 1e3, I, "kiloampere", "Electric";
    "mol", 1.0, N_, "mole", "Amount"; "kmol", 1e3, N_, "kilomole", "Amount"; "mmol", 1e-3, N_, "millimole", "Amount"; "lbmol", LB * 1e3, N_, "pound-mole", "Amount";
    "cd", 1.0, J_, "candela", "Light";
    // Angle
    "rad", 1.0, ANGLE, "radian", "Angle"; "deg", PI / 180.0, ANGLE, "degree", "Angle"; "°", PI / 180.0, ANGLE, "degree", "Angle";
    "rev", 2.0 * PI, ANGLE, "revolution", "Angle"; "arcmin", PI / 10800.0, ANGLE, "arc minute", "Angle"; "arcsec", PI / 648_000.0, ANGLE, "arc second", "Angle";
    "gon", PI / 200.0, ANGLE, "gon (grad)", "Angle";
    // Area
    "ha", 1e4, AREA, "hectare", "Area"; "acre", 4_046.856_422_4, AREA, "acre (international)", "Area";
    // Volume
    "L", 1e-3, VOL, "litre", "Volume"; "l", 1e-3, VOL, "litre", "Volume"; "mL", 1e-6, VOL, "millilitre", "Volume"; "ml", 1e-6, VOL, "millilitre", "Volume";
    "gal", GAL, VOL, "US gallon", "Volume"; "gal_uk", 4.546_09e-3, VOL, "imperial gallon", "Volume"; "bbl", 42.0 * GAL, VOL, "barrel (oil)", "Volume";
    "cc", 1e-6, VOL, "cubic centimetre", "Volume"; "qt", GAL / 4.0, VOL, "US liquid quart", "Volume"; "pt", GAL / 8.0, VOL, "US liquid pint", "Volume"; "fl_oz", GAL / 128.0, VOL, "US fluid ounce", "Volume";
    // Force
    "N", 1.0, FORCE, "newton", "Force"; "mN", 1e-3, FORCE, "millinewton", "Force"; "kN", 1e3, FORCE, "kilonewton", "Force"; "MN", 1e6, FORCE, "meganewton", "Force";
    "kgf", GN, FORCE, "kilogram-force", "Force"; "lbf", LBF, FORCE, "pound-force", "Force"; "kip", 1000.0 * LBF, FORCE, "kip", "Force"; "dyn", 1e-5, FORCE, "dyne", "Force";
    "tf", 1000.0 * GN, FORCE, "tonne-force", "Force"; "ozf", LBF / 16.0, FORCE, "ounce-force", "Force";
    // Pressure and stress
    "Pa", 1.0, PRESS, "pascal", "Pressure"; "kPa", 1e3, PRESS, "kilopascal", "Pressure"; "MPa", 1e6, PRESS, "megapascal", "Pressure"; "GPa", 1e9, PRESS, "gigapascal", "Pressure";
    "hPa", 1e2, PRESS, "hectopascal", "Pressure"; "bar", 1e5, PRESS, "bar", "Pressure"; "mbar", 1e2, PRESS, "millibar", "Pressure"; "atm", 101_325.0, PRESS, "standard atmosphere", "Pressure";
    "psi", LBF / (IN * IN), PRESS, "pound-force per square inch", "Pressure"; "ksi", 1000.0 * LBF / (IN * IN), PRESS, "kip per square inch", "Pressure";
    "psf", LBF / (FT * FT), PRESS, "pound-force per square foot", "Pressure"; "torr", 101_325.0 / 760.0, PRESS, "torr", "Pressure";
    "mmHg", 133.322_387_415, PRESS, "millimetre of mercury (conventional)", "Pressure"; "inHg", 3386.389, PRESS, "inch of mercury (conventional)", "Pressure";
    "mH2O", GN * 1000.0, PRESS, "metre of water (conventional)", "Pressure"; "inH2O", GN * 1000.0 * IN, PRESS, "inch of water (conventional)", "Pressure";
    "at", GN * 1e4, PRESS, "technical atmosphere (kgf/cm²)", "Pressure"; "psia", LBF / (IN * IN), PRESS, "pound-force per square inch (absolute)", "Pressure";
    // Energy, work, torque
    "J", 1.0, ENERGY, "joule", "Energy"; "kJ", 1e3, ENERGY, "kilojoule", "Energy"; "MJ", 1e6, ENERGY, "megajoule", "Energy"; "GJ", 1e9, ENERGY, "gigajoule", "Energy";
    "Wh", 3600.0, ENERGY, "watt hour", "Energy"; "kWh", 3.6e6, ENERGY, "kilowatt hour", "Energy"; "MWh", 3.6e9, ENERGY, "megawatt hour", "Energy";
    "cal", 4.184, ENERGY, "calorie (thermochemical)", "Energy"; "kcal", 4184.0, ENERGY, "kilocalorie (thermochemical)", "Energy";
    "BTU", BTU, ENERGY, "British thermal unit (IT)", "Energy"; "Btu", BTU, ENERGY, "British thermal unit (IT)", "Energy"; "therm", 1e5 * BTU, ENERGY, "therm", "Energy";
    "eV", 1.602_176_634e-19, ENERGY, "electronvolt", "Energy"; "erg", 1e-7, ENERGY, "erg", "Energy";
    "MMBTU", 1e6 * BTU, ENERGY, "million BTU", "Energy"; "TJ", 1e12, ENERGY, "terajoule", "Energy";
    // Power
    "W", 1.0, POWER, "watt", "Power"; "mW", 1e-3, POWER, "milliwatt", "Power"; "kW", 1e3, POWER, "kilowatt", "Power"; "MW", 1e6, POWER, "megawatt", "Power"; "GW", 1e9, POWER, "gigawatt", "Power";
    "hp", HP, POWER, "horsepower (mechanical)", "Power"; "hp_metric", 735.498_75, POWER, "horsepower (metric)", "Power"; "PS", 735.498_75, POWER, "horsepower (metric)", "Power";
    "TR", 12_000.0 * BTU / 3600.0, POWER, "ton of refrigeration", "Power"; "VA", 1.0, POWER, "volt-ampere", "Power"; "kVA", 1e3, POWER, "kilovolt-ampere", "Power"; "MVA", 1e6, POWER, "megavolt-ampere", "Power";
    "var", 1.0, POWER, "volt-ampere reactive", "Power"; "kvar", 1e3, POWER, "kilovolt-ampere reactive", "Power";
    // Frequency and rotation
    "Hz", 1.0, FREQ, "hertz", "Frequency"; "kHz", 1e3, FREQ, "kilohertz", "Frequency"; "MHz", 1e6, FREQ, "megahertz", "Frequency";
    "rpm", 2.0 * PI / 60.0, ANGVEL, "revolution per minute", "Rotation"; "rps", 2.0 * PI, ANGVEL, "revolution per second", "Rotation";
    "GHz", 1e9, FREQ, "gigahertz", "Frequency";
    // Speed
    "mph", 1609.344 / 3600.0, SPEED, "mile per hour", "Speed"; "kn", 1852.0 / 3600.0, SPEED, "knot", "Speed"; "knot", 1852.0 / 3600.0, SPEED, "knot", "Speed";
    "kph", 1000.0 / 3600.0, SPEED, "kilometre per hour", "Speed"; "fpm", FT / 60.0, SPEED, "foot per minute", "Speed"; "fps", FT, SPEED, "foot per second", "Speed";
    // Flow
    "gpm", GAL / 60.0, FLOW, "US gallon per minute", "Flow"; "cfm", FT * FT * FT / 60.0, FLOW, "cubic foot per minute", "Flow"; "lpm", 1e-3 / 60.0, FLOW, "litre per minute", "Flow";
    "bpd", 42.0 * GAL / 86400.0, FLOW, "barrel per day", "Flow"; "MGD", 1e6 * GAL / 86400.0, FLOW, "million US gallons per day", "Flow";
    "tph", 1000.0 / 3600.0, MFLOW, "tonne per hour", "Mass flow";
    "gph", GAL / 3600.0, FLOW, "US gallon per hour", "Flow";
    // Viscosity
    "P", 0.1, DVISC, "poise", "Viscosity"; "cP", 1e-3, DVISC, "centipoise", "Viscosity"; "St", 1e-4, KVISC, "stokes", "Viscosity"; "cSt", 1e-6, KVISC, "centistokes", "Viscosity";
    // Electric
    "V", 1.0, VOLT, "volt", "Electric"; "mV", 1e-3, VOLT, "millivolt", "Electric"; "kV", 1e3, VOLT, "kilovolt", "Electric";
    "Ω", 1.0, OHM, "ohm", "Electric"; "ohm", 1.0, OHM, "ohm", "Electric"; "kΩ", 1e3, OHM, "kilohm", "Electric"; "kohm", 1e3, OHM, "kilohm", "Electric"; "MΩ", 1e6, OHM, "megohm", "Electric"; "mΩ", 1e-3, OHM, "milliohm", "Electric";
    "S", 1.0, SIEMENS, "siemens", "Electric"; "C", 1.0, CHARGE, "coulomb", "Electric"; "Ah", 3600.0, CHARGE, "ampere hour", "Electric"; "mAh", 3.6, CHARGE, "milliampere hour", "Electric";
    "F", 1.0, FARAD, "farad", "Electric"; "µF", 1e-6, FARAD, "microfarad", "Electric"; "uF", 1e-6, FARAD, "microfarad", "Electric"; "nF", 1e-9, FARAD, "nanofarad", "Electric"; "pF", 1e-12, FARAD, "picofarad", "Electric";
    "H", 1.0, HENRY, "henry", "Electric"; "mH", 1e-3, HENRY, "millihenry", "Electric"; "Wb", 1.0, WEBER, "weber", "Electric"; "T", 1.0, TESLA, "tesla", "Electric"; "mT", 1e-3, TESLA, "millitesla", "Electric";
    "µA", 1e-6, I, "microampere", "Electric"; "uA", 1e-6, I, "microampere", "Electric"; "GΩ", 1e9, OHM, "gigohm", "Electric"; "µH", 1e-6, HENRY, "microhenry", "Electric";
    "G", 1e-4, TESLA, "gauss", "Electric"; "µT", 1e-6, TESLA, "microtesla", "Electric";
    "lm", 1.0, J_, "lumen", "Light"; "lx", 1.0, d(-2, 0, 0, 0, 0, 0, 1, 0), "lux", "Light";
    // Density
    "sg", 1000.0, DENS, "specific gravity (× water at 1000 kg/m³)", "Density";
    "pcf", LB / (FT * FT * FT), DENS, "pound per cubic foot", "Density";
    // Dimensionless
    "%", 0.01, NONE, "percent", "Ratio"; "ppm", 1e-6, NONE, "parts per million", "Ratio"; "ppb", 1e-9, NONE, "parts per billion", "Ratio";
];

/// Absolute temperature scales: K = factor × value + offset.
pub static ABS_TEMPS: &[Unit] = &[
    Unit { sym: "°C", factor: 1.0, dim: TEMP, offset: 273.15, name: "degree Celsius", group: "Temperature" },
    Unit { sym: "degC", factor: 1.0, dim: TEMP, offset: 273.15, name: "degree Celsius", group: "Temperature" },
    Unit { sym: "°F", factor: 5.0 / 9.0, dim: TEMP, offset: 459.67 * 5.0 / 9.0, name: "degree Fahrenheit", group: "Temperature" },
    Unit { sym: "degF", factor: 5.0 / 9.0, dim: TEMP, offset: 459.67 * 5.0 / 9.0, name: "degree Fahrenheit", group: "Temperature" },
];

pub fn lookup(sym: &str) -> Option<&'static Unit> {
    UNITS.iter().chain(ABS_TEMPS).find(|u| u.sym == sym)
}

/// A parsed unit expression: its factor to SI, its dimension, and an absolute temperature offset
/// (only for a lone °C or °F).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitExpr { pub factor: f64, pub dim: Dim, pub offset: f64, pub text: String }

/// Parses a unit expression on its own, such as `kN*m`, `m^3/h`, `W/(m^2*K)`, `m/s²` or `N·m`.
pub fn parse(s: &str) -> Result<UnitExpr, String> {
    let toks = crate::parse::lex(s).map_err(|e| e.msg)?;
    let mut p = crate::parse::UnitParser { toks: &toks, i: 0, known: &[] };
    let u = p.expr().ok_or_else(|| format!("“{s}” is not a unit."))?;
    if p.i < toks.len() { return Err(format!("“{s}” is not a unit.")); }
    Ok(u)
}

/// Preferred units to show a dimension in, smallest first; the largest whose value is ≥ 1 is used.
/// Whether a dimension has named units to show it in for a unit system.
pub fn has_ladder(dim: &Dim, us: bool) -> bool { ladder(dim, us).is_some() }

fn ladder(dim: &Dim, us: bool) -> Option<&'static [&'static str]> {
    let k = |x: Dim| same(dim, &x);
    Some(match () {
        _ if k(L) => if us { &["in", "ft", "mi"] } else { &["mm", "m", "km"] },
        _ if k(M) => if us { &["lb"] } else { &["g", "kg", "t"] },
        _ if k(T) => &["ms", "s", "min", "h"],
        _ if k(AREA) => if us { &["in^2", "ft^2"] } else { &["mm^2", "m^2"] },
        _ if k(VOL) => if us { &["in^3", "ft^3"] } else { &["mm^3", "L", "m^3"] },
        _ if k(FORCE) => if us { &["lbf", "kip"] } else { &["N", "kN", "MN"] },
        _ if k(PRESS) => if us { &["psi", "ksi"] } else { &["Pa", "kPa", "MPa", "GPa"] },
        _ if k(ENERGY) && dim[7] == 0 => if us { &["BTU"] } else { &["J", "kJ", "MJ", "GJ"] },
        // A torque: a power over a speed (J/rad), or a force times a length.
        _ if k(ENERGY) => if us { &["lbf*in", "lbf*ft", "kip*ft"] } else { &["N*mm", "N*m", "kN*m", "MN*m"] },
        _ if k(POWER) => if us { &["hp"] } else { &["W", "kW", "MW", "GW"] },
        _ if k(FREQ) && dim[7] != 0 => &["rad/s"],
        _ if k(FREQ) => &["Hz", "kHz", "MHz"],
        _ if k(SPEED) => if us { &["ft/s"] } else { &["m/s"] },
        _ if k(FLOW) => if us { &["gpm"] } else { &["m^3/h"] },
        _ if k(MFLOW) => if us { &["lb/h"] } else { &["kg/s"] },
        _ if k(DENS) => if us { &["lb/ft^3"] } else { &["kg/m^3"] },
        _ if k(DVISC) => &["Pa*s"],
        _ if k(KVISC) => &["m^2/s"],
        _ if k(I) => &["mA", "A", "kA"],
        _ if k(VOLT) => &["mV", "V", "kV"],
        _ if k(OHM) => &["mΩ", "Ω", "kΩ", "MΩ"],
        _ if k(CHARGE) => &["C"],
        _ if k(FARAD) => &["pF", "nF", "µF", "F"],
        _ if k(HENRY) => &["mH", "H"],
        _ if k(TEMP) => if us { &["Δ°F"] } else { &["K"] },
        _ if k(N_) => &["mol", "kmol"],
        _ if k(d(1, 0, -2, 0, 0, 0, 0, 0)) => if us { &["ft/s^2"] } else { &["m/s^2"] },
        _ if k(d(0, 1, 0, 0, 0, 0, 0, 0)) => &["kg"],
        _ if k(d(-1, 1, -2, 0, 0, 0, 0, 0)) => &["Pa"],
        _ if k(d(1, 1, -3, -1, 0, 0, 0, 0)) => if us { &["BTU/(h*ft*°R)"] } else { &["W/(m*K)"] },
        _ if k(d(0, 1, -3, -1, 0, 0, 0, 0)) => if us { &["BTU/(h*ft^2*°R)"] } else { &["W/(m^2*K)"] },
        _ if k(d(2, 0, -2, -1, 0, 0, 0, 0)) => if us { &["BTU/(lb*°R)"] } else { &["J/(kg*K)", "kJ/(kg*K)"] },
        _ if k(d(2, 1, -2, -1, 0, -1, 0, 0)) => &["J/(mol*K)"],
        _ if k(d(2, 0, -2, 0, 0, 0, 0, 0)) => if us { &["BTU/lb"] } else { &["J/kg", "kJ/kg"] },
        _ if k(d(4, 0, 0, 0, 0, 0, 0, 0)) => if us { &["in^4"] } else { &["mm^4", "m^4"] },
        _ if k(d(-1, 0, 0, 0, 0, 0, 0, 0)) => &["1/m"],
        _ if k(d(0, 1, -2, 0, 0, 0, 0, 0)) => if us { &["lbf/ft"] } else { &["N/m", "kN/m"] },
        _ if k(d(1, 1, -2, 0, 0, 0, 0, 0)) => &["N"],
        _ => return None,
    })
}

/// The unit to show a value in: the user's choice if it fits, else the ladder for the unit system,
/// else SI base units. Returns (factor, offset, label).
pub fn display(si: f64, dim: &Dim, abs: bool, want: &str, us: bool) -> (f64, f64, String) {
    if !want.trim().is_empty() {
        if let Ok(u) = parse(want) {
            if same(&u.dim, dim) {
                // An absolute temperature shown in °C/°F uses the offset; a difference uses Δ.
                if u.offset != 0.0 && !abs { return (u.factor, 0.0, if u.factor == 1.0 { "Δ°C" } else { "Δ°F" }.into()); }
                return (u.factor, if abs { u.offset } else { 0.0 }, pretty(&u.text));
            }
        }
    }
    if abs && same(dim, &TEMP) {
        return if us { (5.0 / 9.0, 459.67 * 5.0 / 9.0, "°F".into()) } else { (1.0, 273.15, "°C".into()) };
    }
    if is_none(dim) {
        return if dim[7] != 0 && want.is_empty() { (PI / 180.0, 0.0, "°".into()) } else { (1.0, 0.0, String::new()) };
    }
    if let Some(l) = ladder(dim, us) {
        let mut pick = l[0];
        for s in l { if let Ok(u) = parse(s) { if si.abs() / u.factor >= 1.0 { pick = s; } } }
        if let Ok(u) = parse(pick) { return (u.factor, 0.0, pretty(pick)); }
    }
    (1.0, 0.0, base_label(dim))
}

/// SI base units for a dimension with no named unit, such as kg·m²/s³.
pub fn base_label(dim: &Dim) -> String {
    const SYM: [&str; 8] = ["m", "kg", "s", "K", "A", "mol", "cd", "rad"];
    let part = |pos: bool| -> Vec<String> {
        (0..7).filter(|&i| if pos { dim[i] > 0 } else { dim[i] < 0 }).map(|i| { let e = dim[i].unsigned_abs(); if e == 1 { SYM[i].to_string() } else { format!("{}{}", SYM[i], sup(e as i32)) } }).collect()
    };
    let (n, dn) = (part(true), part(false));
    match (n.is_empty(), dn.len()) {
        (true, 0) => String::new(),
        (true, _) => format!("1/{}", dn.join("·")),
        (false, 0) => n.join("·"),
        (false, 1) => format!("{}/{}", n.join("·"), dn[0]),
        (false, _) => format!("{}/({})", n.join("·"), dn.join("·")),
    }
}

fn sup(n: i32) -> String {
    n.to_string().chars().map(|c| match c { '-' => '⁻', '0' => '⁰', '1' => '¹', '2' => '²', '3' => '³', '4' => '⁴', '5' => '⁵', '6' => '⁶', '7' => '⁷', '8' => '⁸', _ => '⁹' }).collect()
}

/// `m^3/h` → `m³/h`, `kN*m` → `kN·m`.
pub fn pretty(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '^' => {
                let mut n = String::new();
                if chars.peek() == Some(&'-') { n.push(chars.next().unwrap()); }
                while let Some(&d) = chars.peek() { if d.is_ascii_digit() { n.push(d); chars.next(); } else { break } }
                out.push_str(&n.parse::<i32>().map(sup).unwrap_or(n));
            }
            '*' => out.push('·'),
            _ => out.push(c),
        }
    }
    out
}

/// Every unit for the units list in the window: (symbol, name, group, dimension label).
pub fn list() -> Vec<(&'static str, &'static str, &'static str, String)> {
    UNITS.iter().chain(ABS_TEMPS).map(|u| (u.sym, u.name, u.group, base_label(&u.dim))).collect()
}

/// US customary symbols (the rest are SI or neutral, such as s, A, V, rpm, %).
const US_SYMS: [&str; 41] = ["in", "ft", "yd", "mi", "mil", "lb", "lbm", "oz", "slug", "ton_us", "grain", "°R", "degR", "Δ°F", "delta°F", "deltaF", "°F", "degF", "acre", "gal", "bbl",
    "lbf", "kip", "psi", "ksi", "psf", "psia", "inHg", "inH2O", "BTU", "Btu", "therm", "MMBTU", "hp", "TR", "mph", "fpm", "fps", "gpm", "cfm", "pcf"];
const NEUTRAL: [&str; 50] = ["s", "ms", "µs", "us", "ns", "min", "h", "hr", "day", "week", "yr", "A", "mA", "kA", "µA", "uA", "V", "mV", "kV", "rpm", "rps", "rad", "deg", "°",
    // Electrical and viscosity units are used the same way in both systems.
    "Ω", "ohm", "kΩ", "kohm", "MΩ", "mΩ", "GΩ", "S", "C", "Ah", "mAh", "F", "µF", "uF", "nF", "pF", "H", "mH", "µH", "Wb", "T", "mT", "µT", "G", "cP", "cSt"];

/// Which system a unit expression belongs to: Some(true) for US customary, Some(false) for SI and
/// metric, None when it only uses units common to both (s, A, V, rpm, %).
pub fn system(text: &str) -> Option<bool> {
    let toks = crate::parse::lex(text).ok()?;
    let mut sys = None;
    for (t, _) in toks {
        if let crate::parse::Tok::Id(w) = t {
            if US_SYMS.contains(&w.as_str()) { return Some(true); }
            if !NEUTRAL.contains(&w.as_str()) && w != "%" && w != "ppm" && w != "ppb" { sys = Some(false); }
        }
    }
    sys
}
