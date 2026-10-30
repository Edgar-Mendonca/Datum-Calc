#![allow(clippy::approx_constant)]
//! Engine tests: every unit factor against NIST SP 811 (2008) Appendix B, temperatures, dimension
//! errors, display choice, the parser, evaluation, LaTeX and solving. Expected values are worked
//! out by hand (or quoted from the source) before running.
use crate::eval::{self, fmt_num, Fmt};
use crate::sheet::{Kind, Region, Sheet};
use crate::units::{self, lookup};

fn close(a: f64, b: f64, rel: f64) -> bool { (a - b).abs() <= rel * b.abs().max(1e-300) }

/// Each unit's factor to SI, as published (NIST SP 811 App. B.8/B.9, exact where NIST says exact).
#[test]
fn unit_factors_match_nist() {
    let table: &[(&str, f64)] = &[
        ("m", 1.0), ("km", 1000.0), ("cm", 0.01), ("mm", 0.001), ("µm", 1e-6), ("um", 1e-6), ("nm", 1e-9), ("in", 0.0254), ("ft", 0.3048), ("yd", 0.9144), ("mi", 1609.344), ("nmi", 1852.0), ("mil", 2.54e-5),
        ("kg", 1.0), ("g", 1e-3), ("mg", 1e-6), ("t", 1000.0), ("tonne", 1000.0), ("lb", 0.45359237), ("lbm", 0.45359237), ("oz", 2.834952e-2), ("slug", 1.459390e1), ("ton_us", 9.071847e2), ("grain", 6.479891e-5),
        ("s", 1.0), ("ms", 1e-3), ("µs", 1e-6), ("us", 1e-6), ("min", 60.0), ("h", 3600.0), ("hr", 3600.0), ("day", 86400.0), ("week", 604800.0), ("yr", 3.15576e7),
        ("K", 1.0), ("°R", 5.0 / 9.0), ("degR", 5.0 / 9.0), ("Δ°C", 1.0), ("delta°C", 1.0), ("deltaC", 1.0), ("Δ°F", 5.0 / 9.0), ("delta°F", 5.0 / 9.0), ("deltaF", 5.0 / 9.0),
        ("A", 1.0), ("mA", 1e-3), ("kA", 1e3), ("mol", 1.0), ("kmol", 1e3), ("mmol", 1e-3), ("lbmol", 453.59237), ("cd", 1.0),
        ("rad", 1.0), ("deg", 1.745329e-2), ("°", 1.745329e-2), ("rev", 6.283185), ("arcmin", 2.908882e-4), ("arcsec", 4.848137e-6),
        ("ha", 1e4), ("acre", 4.046856e3), ("L", 1e-3), ("l", 1e-3), ("mL", 1e-6), ("ml", 1e-6), ("gal", 3.785412e-3), ("gal_uk", 4.54609e-3), ("bbl", 1.589873e-1),
        ("N", 1.0), ("mN", 1e-3), ("kN", 1e3), ("MN", 1e6), ("kgf", 9.80665), ("lbf", 4.448222), ("kip", 4.448222e3), ("dyn", 1e-5),
        ("Pa", 1.0), ("kPa", 1e3), ("MPa", 1e6), ("GPa", 1e9), ("hPa", 100.0), ("bar", 1e5), ("mbar", 100.0), ("atm", 1.01325e5), ("psi", 6.894757e3), ("ksi", 6.894757e6),
        ("psf", 4.788026e1), ("torr", 1.333224e2), ("mmHg", 1.333224e2), ("inHg", 3.386389e3), ("mH2O", 9.80665e3), ("inH2O", 2.490889e2),
        ("J", 1.0), ("kJ", 1e3), ("MJ", 1e6), ("GJ", 1e9), ("Wh", 3600.0), ("kWh", 3.6e6), ("MWh", 3.6e9), ("cal", 4.184), ("kcal", 4184.0), ("BTU", 1.055056e3), ("Btu", 1.055056e3),
        ("therm", 1.055056e8), ("eV", 1.602177e-19), ("erg", 1e-7),
        ("W", 1.0), ("mW", 1e-3), ("kW", 1e3), ("MW", 1e6), ("GW", 1e9), ("hp", 7.456999e2), ("hp_metric", 7.354988e2), ("PS", 7.354988e2), ("TR", 3.516853e3),
        ("VA", 1.0), ("kVA", 1e3), ("MVA", 1e6), ("var", 1.0), ("kvar", 1e3),
        ("Hz", 1.0), ("kHz", 1e3), ("MHz", 1e6), ("rpm", 1.047198e-1), ("rps", 6.283185),
        ("mph", 4.4704e-1), ("kn", 5.144444e-1), ("knot", 5.144444e-1), ("kph", 2.777778e-1), ("fpm", 5.08e-3), ("fps", 3.048e-1),
        ("gpm", 6.309020e-5), ("cfm", 4.719474e-4), ("lpm", 1.666667e-5), ("bpd", 1.840131e-6), ("MGD", 4.381264e-2), ("tph", 2.777778e-1),
        ("P", 0.1), ("cP", 1e-3), ("St", 1e-4), ("cSt", 1e-6),
        ("V", 1.0), ("mV", 1e-3), ("kV", 1e3), ("Ω", 1.0), ("ohm", 1.0), ("kΩ", 1e3), ("kohm", 1e3), ("MΩ", 1e6), ("mΩ", 1e-3), ("S", 1.0), ("C", 1.0), ("Ah", 3600.0), ("mAh", 3.6),
        ("F", 1.0), ("µF", 1e-6), ("uF", 1e-6), ("nF", 1e-9), ("pF", 1e-12), ("H", 1.0), ("mH", 1e-3), ("Wb", 1.0), ("T", 1.0), ("mT", 1e-3),
        ("sg", 1000.0), ("%", 0.01), ("ppm", 1e-6), ("ppb", 1e-9),
        ("dm", 0.1), ("Mg", 1e3), ("µg", 1e-9), ("ns", 1e-9), ("gon", 1.570796e-2), ("cc", 1e-6), ("qt", 9.463529e-4), ("pt", 4.731765e-4), ("fl_oz", 2.957353e-5),
        ("tf", 9.80665e3), ("ozf", 2.780139e-1), ("at", 9.80665e4), ("psia", 6.894757e3), ("MMBTU", 1.055056e9), ("TJ", 1e12), ("GHz", 1e9), ("gph", 1.051503e-6),
        ("µA", 1e-6), ("uA", 1e-6), ("GΩ", 1e9), ("µH", 1e-6), ("G", 1e-4), ("µT", 1e-6), ("lm", 1.0), ("lx", 1.0), ("pcf", 1.601846e1),
    ];
    // NIST gives 7 significant figures for inexact factors.
    for (sym, want) in table {
        let u = lookup(sym).unwrap_or_else(|| panic!("{sym} is missing"));
        assert!(close(u.factor, *want, 5e-7), "{sym}: {} vs {want}", u.factor);
    }
    // Every unit in the table is tested.
    for u in units::UNITS { assert!(table.iter().any(|t| t.0 == u.sym), "{} has no factor test", u.sym); }
    assert!(units::UNITS.len() + units::ABS_TEMPS.len() >= 180, "about 200 units");
}

#[test]
fn unit_dimensions() {
    let dim = |s: &str| units::parse(s).unwrap().dim;
    assert_eq!(dim("N"), dim("kg*m/s^2"));
    assert_eq!(dim("Pa"), dim("N/m^2"));
    assert_eq!(dim("J"), dim("N*m"));
    assert_eq!(dim("W"), dim("J/s"));
    assert_eq!(dim("V"), dim("W/A"));
    assert_eq!(dim("Ω"), dim("V/A"));
    assert_eq!(dim("C"), dim("A*s"));
    assert_eq!(dim("F"), dim("C/V"));
    assert_eq!(dim("H"), dim("Wb/A"));
    assert_eq!(dim("T"), dim("Wb/m^2"));
    assert_eq!(dim("S"), dim("A/V"));
    assert_eq!(dim("gpm"), dim("m^3/s"));
    assert_eq!(dim("cP"), dim("Pa*s"));
    assert_eq!(dim("cSt"), dim("m^2/s"));
    assert_eq!(dim("kWh"), dim("J"));
    assert_eq!(dim("psi"), dim("lbf/in^2"));
    assert!(units::same(&dim("rpm"), &dim("Hz")), "rotation and frequency differ only by the angle tag");
}

#[test]
fn unit_expressions() {
    let f = |s: &str| units::parse(s).unwrap().factor;
    assert!(close(f("m^3/h"), 1.0 / 3600.0, 1e-12));
    assert!(close(f("kN*m"), 1000.0, 1e-12));
    assert!(close(f("W/(m^2*K)"), 1.0, 1e-12));
    assert!(close(f("W/m^2/K"), 1.0, 1e-12));
    assert!(close(f("lbf/in^2"), lookup("psi").unwrap().factor, 1e-12));
    assert!(close(f("ft^3/min"), lookup("cfm").unwrap().factor, 1e-12));
    assert!(close(f("m/s²"), 1.0, 1e-12));
    assert!(close(f("m³"), 1.0, 1e-12));
    assert!(close(f("N·m"), 1.0, 1e-12));
    assert!(close(f("BTU/(h*ft^2*°R)"), 5.678263, 1e-6), "W/(m²·K) per BTU/(h·ft²·°F), NIST");
    assert!(close(f("kg/m^3"), 1.0, 1e-12));
    assert!(close(f("s^-1"), 1.0, 1e-12));
    assert!(units::parse("m^3/hx").is_err());
    assert!(units::parse("").is_err());
    assert!(units::parse("°C^2").is_err(), "an absolute temperature has no powers");
}

fn sheet(lines: &[&str]) -> Sheet {
    let mut s = Sheet::new("t");
    for (i, l) in lines.iter().enumerate() {
        let kind = if l.starts_with("check") { Kind::Check } else { Kind::Math };
        s.regions.push(Region { id: i as u32 + 1, kind, text: l.to_string(), ..Default::default() });
    }
    s
}
fn run(lines: &[&str]) -> eval::SheetOut { eval::run(&sheet(lines)) }
/// The SI value of the last region's result.
fn val(lines: &[&str]) -> f64 {
    let s = sheet(lines);
    let o = eval::run(&s);
    if let Some(e) = o.regions.iter().find_map(|r| r.error.clone()) { panic!("{lines:?}: {}", e.msg); }
    let mut env = eval::Env::new();
    let mut known = vec![];
    let mut last = f64::NAN;
    for l in lines {
        match crate::parse::statement(l, &known).unwrap() {
            crate::parse::Stmt::Assign { name, e, .. } => { let v = env.eval(&e).unwrap(); last = v.x(); env.vars.insert(eval::canon(&name), v); known.push(name); }
            crate::parse::Stmt::Show { e } => last = env.eval(&e).unwrap().x(),
            crate::parse::Stmt::Func { name, params, e } => { env.funcs.insert(eval::canon(&name), (params, e)); }
            _ => {}
        }
    }
    last
}
fn result(lines: &[&str]) -> String { let o = run(lines); let r = o.regions.last().unwrap(); r.error.as_ref().map(|e| format!("ERR {}", e.msg)).unwrap_or(r.result.clone()) }
fn error(lines: &[&str]) -> String { run(lines).regions.last().unwrap().error.clone().map(|e| e.msg).unwrap_or_default() }

#[test]
fn temperatures() {
    assert!(close(val(&["T = 20 °C"]), 293.15, 1e-12));
    assert!(close(val(&["T = 68 °F"]), 293.15, 1e-12));
    assert!(close(val(&["T = -40 °C"]), 233.15, 1e-12));
    assert!(close(val(&["T = -40 °F"]), 233.15, 1e-12), "−40 is the same on both scales");
    assert!(close(val(&["T = 0 degF"]), 255.372_222, 1e-6));
    assert!(close(val(&["T = 491.67 °R"]), 273.15, 1e-12));
    assert_eq!(result(&["T = 20 °C"]), "20 °C");
    assert_eq!(result(&["T1 = 20 °C", "T2 = 80 °C", "dT = T2 - T1"]), "60 K", "a difference is not a temperature");
    assert_eq!(result(&["T1 = 20 °C", "T2 = 30 °C", "Tm = (T1 + T2)/2"]), "25 °C", "a mean of temperatures stays a temperature");
    assert_eq!(result(&["T1 = 20 °C", "T2 = T1 + 15 K"]), "35 °C");
    assert_eq!(result(&["T1 = 20 °C", "T2 = T1 + 15 Δ°C"]), "35 °C");
    let mut s = sheet(&["T1 = 20 °C", "T2 = 80 °C", "dT = T2 - T1"]);
    s.regions[2].unit = "°F".into();
    assert_eq!(eval::run(&s).regions[2].result, "108 Δ°F", "a difference shown in °F is a Δ°F");
    s.regions[1].unit = "°F".into();
    assert_eq!(eval::run(&s).regions[1].result, "176 °F");
    s.regions[1].unit = "K".into();
    assert_eq!(eval::run(&s).regions[1].result, "353.1 K");
    let mut s = sheet(&["T = 100 °C"]);
    s.units = "US".into();
    assert_eq!(eval::run(&s).regions[0].result, "212 °F");
}

#[test]
fn dimension_errors_are_caught_on_their_line() {
    let o = run(&["a = 2 m", "b = 3 s", "c = a + b", "d = a * 2"]);
    let e = o.regions[2].error.as_ref().unwrap();
    assert!(e.msg.contains("a length") && e.msg.contains("a time"), "{}", e.msg);
    assert_eq!((e.from, e.to), (4, 9), "the error points at a + b");
    assert!(o.regions[3].error.is_none(), "the rest of the sheet still works");
    assert_eq!(o.regions[3].result, "4 m");
    assert!(error(&["x = 2 m", "y = sin(x)"]).contains("angle"));
    assert!(error(&["x = 2 m", "y = ln(x)"]).contains("plain number"));
    assert!(error(&["x = 2 m", "y = 2^x"]).contains("plain number"));
    assert!(error(&["x = 2 m", "check x < 3 s"]).contains("compare"));
    assert!(error(&["x = sqrt(2 m)"]).contains("no units"));
    assert_eq!(result(&["A = 4 m^2", "x = sqrt(A)"]), "2 m");
    assert_eq!(result(&["T = 8 N*m/rad", "x = cbrt(T/(1 N/m^2))"]), "2 m", "the soft angle tag never blocks a root");
    assert_eq!(result(&["T = 4 N*m/rad", "x = (T*1 m/(1 N))^0.5"]), "2 m");
    assert_eq!(result(&["V = 8 m^3", "x = cbrt(V)"]), "2 m");
    assert_eq!(result(&["A = 4 m^2", "x = A^0.5"]), "2 m");
}

#[test]
fn display_picks_a_good_unit() {
    assert_eq!(result(&["P = 18400 W*1"]), "18.4 kW");
    assert_eq!(result(&["p = 2500000 Pa*1"]), "2.5 MPa");
    assert_eq!(result(&["p = 250 Pa*1"]), "250 Pa");
    assert_eq!(result(&["F = 2 kg * 9.81 m/s^2"]), "19.62 N");
    assert_eq!(result(&["L = 0.25 m*1"]), "250 mm");
    assert_eq!(result(&["L = 2500 m*1"]), "2.5 km");
    assert_eq!(result(&["Q = 120 m^3/h"]), "120 m³/h");
    assert_eq!(result(&["v = 3 m/s"]), "3 m/s");
    assert_eq!(result(&["x = 3 m * 2 N / (4 s)"]), "1.5 W");
    assert_eq!(result(&["k = 0.5 W/(m*K)"]), "0.5 W/(m·K)");
    assert_eq!(result(&["U = 450 W/(m^2*K)"]), "450 W/(m²·K)");
    assert_eq!(result(&["w = 2 kg*m^2/s"]), "2 kg·m²/s");
    assert_eq!(result(&["w = 2 kg*m^2/s*1"]), "2 m²·kg/s");
    assert_eq!(result(&["r = 3 m / (2 m)"]), "1.5");
    assert_eq!(result(&["θ = 30 deg"]), "30 deg");
    assert_eq!(result(&["θ = 30 deg*1"]), "30°");
    assert_eq!(result(&["I = 8356 cm^4"]), "8356 cm⁴", "an input keeps the unit it was typed in");
    assert_eq!(result(&["D = 1200 mm"]), "1200 mm");
    let mut s = sheet(&["D = 1200 mm", "L = 3 ft"]);
    s.units = "US".into();
    let o = eval::run(&s);
    assert_eq!((o.regions[0].result.as_str(), o.regions[1].result.as_str()), ("3.937 ft", "3 ft"), "a metric input converts in a US sheet; a US input stays");
    let mut s = sheet(&["R = 0.193 ohm/km"]);
    s.units = "US".into();
    assert_eq!(eval::run(&s).regions[0].result, "0.193 ohm/km", "with no US unit for it, an input keeps its own");
    assert_eq!(result(&["n = 1500 rpm"]), "1500 rpm");
    assert_eq!(result(&["n = 1500 rpm*1"]), "157.1 rad/s");
    let mut s = sheet(&["n = 1500 rpm"]);
    s.regions[0].unit = "rpm".into();
    assert_eq!(eval::run(&s).regions[0].result, "1500 rpm");
    let mut s = sheet(&["P = 18.4 kW"]);
    s.regions[0].unit = "hp".into();
    assert_eq!(eval::run(&s).regions[0].result, "24.67 hp");
    s.regions[0].unit = "m".into();
    let o = eval::run(&s);
    assert!(o.regions[0].warn.contains("cannot show"), "{}", o.regions[0].warn);
    assert_eq!(o.regions[0].result, "18.4 kW");
    let mut s = sheet(&["p = 1 MPa", "F = 10 kN", "L = 3 ft"]);
    s.units = "US".into();
    let o = eval::run(&s);
    assert_eq!(o.regions[0].result, "145 psi");
    assert_eq!(o.regions[1].result, "2.248 kip");
    assert_eq!(o.regions[2].result, "3 ft");
}

#[test]
fn number_formatting() {
    assert_eq!(fmt_num(18.4, 4, false).0, "18.4");
    assert_eq!(fmt_num(19.6133, 4, false).0, "19.61");
    assert_eq!(fmt_num(0.00012345, 4, false).0, "123.5×10⁻⁶");
    assert_eq!(fmt_num(1234567.0, 4, false).0, "1.235×10⁶");
    assert_eq!(fmt_num(999999.9, 3, false).0, "1×10⁶");
    assert_eq!(fmt_num(12500.0, 3, true).0, "12.5×10³");
    assert_eq!(fmt_num(-0.5, 4, false).0, "-0.5");
    assert_eq!(fmt_num(0.0, 4, false).0, "0");
    assert_eq!(fmt_num(100.0, 4, false).0, "100");
    assert_eq!(fmt_num(274_863.3, 4, false).0, "274\u{2009}900", "large numbers keep their significant figures, grouped");
    assert_eq!(fmt_num(274_863.3, 4, false).1, "274\\,900");
    assert_eq!(fmt_num(29_007.5, 4, false).0, "29\u{2009}010");
    assert_eq!(fmt_num(1234.6, 4, false).0, "1235", "four digits are not grouped");
    assert_eq!(fmt_num(-56_789.0, 5, false).0, "-56\u{2009}789");
    assert_eq!(fmt_num(1e6, 4, false).1, "1 \\times 10^{6}");
    assert_eq!(fmt_num(f64::NAN, 4, false).0, "—");
}

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(val(&["x = 2 + 3 * 4"]), 14.0);
    assert_eq!(val(&["x = (2 + 3) * 4"]), 20.0);
    assert_eq!(val(&["x = 2^3^2"]), 512.0, "powers group from the right");
    assert_eq!(val(&["x = -2^2"]), -4.0);
    assert_eq!(val(&["x = 2*-3"]), -6.0);
    assert_eq!(val(&["x = 10 / 4 / 5"]), 0.5);
    assert_eq!(val(&["x = 7 - 2 - 1"]), 4.0);
    assert_eq!(val(&["x = 3²"]), 9.0);
    assert_eq!(val(&["x = 2**3"]), 8.0);
    assert_eq!(val(&["x = 1.5e3 + .5"]), 1500.5);
    assert_eq!(val(&["x = 6 × 7 ÷ 2"]), 21.0);
    assert!(close(val(&["x = sin(30 deg)"]), 0.5, 1e-12));
    assert!(close(val(&["x = cos(pi)"]), -1.0, 1e-12));
    assert!(close(val(&["x = cos(π)"]), -1.0, 1e-12));
    assert!(close(val(&["x = tan(45 °)"]), 1.0, 1e-12));
    assert!(close(val(&["x = asin(0.5)"]), std::f64::consts::PI / 6.0, 1e-12));
    assert!(close(val(&["x = ln(e)"]), 1.0, 1e-12));
    assert!(close(val(&["x = log(1000)"]), 3.0, 1e-12));
    assert!(close(val(&["x = exp(0)"]), 1.0, 1e-12));
    assert_eq!(val(&["x = abs(-3 m)"]), 3.0);
    assert_eq!(val(&["x = min(3, 1, 2)"]), 1.0);
    assert_eq!(val(&["x = max(3 m, 400 mm)"]), 3.0);
    assert_eq!(val(&["x = round(2.345, 2)"]), 2.35);
    assert_eq!(val(&["x = floor(2.7)"]), 2.0);
    assert_eq!(val(&["x = ceil(2.1)"]), 3.0);
    assert_eq!(val(&["x = if(2 > 1, 10, 20)"]), 10.0);
    assert_eq!(val(&["x = if(2 < 1, 10, 20)"]), 20.0);
    assert_eq!(val(&["x = if(1 > 0, 1, sqrt(-1))"]), 1.0, "if only works out the branch it takes");
    assert_eq!(val(&["x = hypot(3, 4)"]), 5.0);
    assert!(close(val(&["x = atan2(1, 1)"]), std::f64::consts::FRAC_PI_4, 1e-12));
    assert!(close(val(&["x = 5 %"]), 0.05, 1e-12));
}

#[test]
fn names_greek_and_subscripts() {
    assert_eq!(val(&["sigma_allow = 250 MPa", "x = σ_allow / 2"]), 125e6, "sigma and σ are the same name");
    assert_eq!(val(&["ρ = 1000 kg/m^3", "x = rho * 2"]), 2000.0);
    assert_eq!(val(&["eta_motor = 0.9", "x = η_motor"]), 0.9);
    assert_eq!(val(&["P_in' = 3", "x = P_in' * 2"]), 6.0);
    assert_eq!(val(&["g = 10 m/s^2", "x = g"]), 10.0, "a name on the sheet takes the place of a constant");
    assert!(close(val(&["x = g"]), 9.80665, 1e-12));
    assert!(close(val(&["x = R"]), 8.314462618, 1e-12));
    assert!(close(val(&["x = σ_SB"]), 5.670374419e-8, 1e-12));
}

#[test]
fn units_after_numbers() {
    assert!(close(val(&["Q = 120 m^3/h"]), 120.0 / 3600.0, 1e-12));
    assert!(close(val(&["M = 2.5 kN*m"]), 2500.0, 1e-12));
    assert!(close(val(&["h = 1.2 W/(m^2*K)"]), 1.2, 1e-12));
    assert!(close(val(&["x = 2 m * 3"]), 6.0, 1e-12), "* followed by a number ends the unit");
    assert!(close(val(&["t = 4", "x = 2*t"]), 8.0, 1e-12), "a unit must follow the number directly");
    assert!(close(val(&["t = 4", "x = 2 m*t"]), 8.0, 1e-12), "a name on the sheet is not taken as a unit");
    assert!(close(val(&["x = 2 m*t"]), 2000.0, 1e-12), "without a name t is the tonne");
    assert!(close(val(&["x = 1000 kg/m^3*g"]), 9806.65, 1e-12), "g after a unit is gravity, not grams");
    assert!(close(val(&["f(t) = 2 m*t", "x = f(3)"]), 6.0, 1e-12), "a parameter is not a unit");
    assert_eq!(result(&["plot 2 m*t for t in 0 .. 1"]), "", "the plotted variable is not a unit");
    assert!(run(&["plot 2 m*t for t in 0 .. 1"]).regions[0].plot.as_ref().unwrap().yunit == "m");
    assert!(close(val(&["x = 10 m/s / 2"]), 5.0, 1e-12));
    assert!(close(val(&["v = 3 m/s", "x = 9 m / v"]), 3.0, 1e-12));
    assert!(error(&["x = 2 metres"]).contains("not a unit"));
}

#[test]
fn user_functions_and_vectors() {
    assert_eq!(val(&["f(x) = 3*x^2 + 2", "y = f(2)"]), 14.0);
    assert_eq!(val(&["A(d) = pi*d^2/4", "y = A(2 m)"]), std::f64::consts::PI);
    assert_eq!(val(&["h(a, b) = sqrt(a^2 + b^2)", "y = h(3 m, 4 m)"]), 5.0);
    assert!(error(&["f(x) = x", "y = f(1, 2)"]).contains("takes 1"));
    assert!(error(&["f(x) = f(x)", "y = f(1)"]).contains("too deeply"));
    let o = run(&["x = 0, 0.5 .. 2", "s = sum(x)", "m = mean(x)", "n = len(x)", "y = x^2", "mx = max(y)"]);
    assert_eq!(o.regions[0].result, "[0, 0.5, 1, 1.5, 2]");
    assert_eq!(o.regions[1].result, "5");
    assert_eq!(o.regions[2].result, "1");
    assert_eq!(o.regions[3].result, "5");
    assert_eq!(o.regions[5].result, "4");
    assert_eq!(result(&["x = 0 .. 3"]), "[0, 1, 2, 3]");
    assert_eq!(result(&["L = 0, 1 m .. 3 m"]), "[0, 1, 2, 3] m");
    assert!(result(&["x = 0, 0.1 .. 10"]).starts_with("[0, 0.1, 0.2, … 10]"));
    assert!(error(&["x = 0, -1 .. 10"]).contains("step"));
    assert_eq!(result(&["xs = 0 .. 2", "x = interp(1.5, xs, xs^2)"]), "2.5");
}

#[test]
fn checks_pass_fail_and_utilisation() {
    let o = run(&["sigma = 205 MPa", "sigma_allow = 250 MPa", "check sigma <= sigma_allow"]);
    let c = o.regions[2].check.as_ref().unwrap();
    assert!(c.pass);
    assert!(close(c.ratio.unwrap(), 0.82, 1e-12));
    assert_eq!((c.lhs.as_str(), c.rhs.as_str()), ("205 MPa", "250 MPa"));
    let o = run(&["P = 18.4 kW", "P_motor = 15 kW", "check P_motor >= P"]);
    let c = o.regions[2].check.as_ref().unwrap();
    assert!(!c.pass);
    assert!(close(c.ratio.unwrap(), 18.4 / 15.0, 1e-12), "utilisation is demand over capacity");
    assert_eq!((o.checks, o.failed), (1, 1));
    let o = run(&["a = 300 mm", "check a <= 0.3 m"]);
    assert!(o.regions[1].check.as_ref().unwrap().pass, "equal within rounding passes ≤");
    assert_eq!(o.regions[1].check.as_ref().unwrap().rhs, "300 mm", "the right side is shown in the left side's unit");
    assert!(error(&["check 3"]).contains("compares"));
}

#[test]
fn errors_say_where_and_why() {
    let o = run(&["x = y + 1", "y = 2"]);
    assert!(o.regions[0].error.as_ref().unwrap().msg.contains("further down"), "{:?}", o.regions[0].error);
    let o = run(&["a = 1 m + 1 s", "b = a * 2"]);
    assert!(o.regions[1].error.as_ref().unwrap().msg.contains("error on its own line"));
    assert!(error(&["x = z * 2"]).contains("not defined"));
    assert!(error(&["x = (2 + 3"]).contains("“)”"));
    assert!(error(&["x = 2 + 3)"]).contains("bracket"));
    assert!(error(&["x = 2 +"]).contains("missing"));
    assert!(error(&["x = 2 $"]).contains("“$”"));
    assert!(error(&["x = 1 / 0"]).contains("zero"));
    assert!(error(&["x = sqrt(-4)"]).contains("negative"));
    assert!(error(&["x = foo(2)"]).contains("not a function"));
    assert!(error(&["x = 2 3"]).contains("operator"));
    assert!(error(&["x = kN"]).contains("is a unit"));
    let o = run(&["x = (2 + 3"]);
    let e = o.regions[0].error.as_ref().unwrap();
    assert_eq!((e.from, e.to), (10, 10), "a missing bracket points at the end");
}

#[test]
fn solve_finds_roots() {
    let o = run(&["solve x^2 = 2 for x in 0 .. 3"]);
    assert!(o.regions[0].error.is_none(), "{:?}", o.regions[0].error);
    assert!(close(val(&["x = 0"]), 0.0, 1.0));
    let mut env = eval::Env::new();
    let _ = &mut env;
    assert_eq!(o.regions[0].result, "1.414");
    assert_eq!(o.vars[0].name, "x");
    // Colebrook: ε/D = 0.0001, Re = 1e5 → f ≈ 0.0185 (Moody chart; Haaland gives 0.01848).
    let o = run(&["eps = 0.0001", "Re = 100000", "solve 1/sqrt(f) = -2*log(eps/3.7 + 2.51/(Re*sqrt(f))) for f in 0.005 .. 0.1", "y = f * 2"]);
    assert!(o.regions[2].error.is_none(), "{:?}", o.regions[2].error);
    assert_eq!(o.regions[2].result, "0.01851");
    assert_eq!(o.regions[3].result, "0.03703");
    let o = run(&["solve x^2 = -1 for x in 0 .. 3"]);
    assert!(o.regions[0].error.as_ref().unwrap().msg.starts_with("No root between 0 and 3"), "{:?}", o.regions[0].error);
    let o = run(&["solve x^2 = 4 m^2 for x in 0 m .. 10 m"]);
    assert_eq!(o.regions[0].result, "2 m");
    let mut f = |x: f64| x.cos() - x;
    assert!(close(crate::solve::root(&mut f, 0.0, 1.0).unwrap(), 0.739_085_133_215_160_6, 1e-12));
}

#[test]
fn plots_sample_their_curves() {
    let o = run(&["H_pump(Q) = 60 m - 0.002 m/(m^3/h)^2 * Q^2", "H_sys(Q) = 20 m + 0.001 m/(m^3/h)^2 * Q^2", "plot H_pump(Q), H_sys(Q) for Q in 0 m^3/h .. 150 m^3/h"]);
    let p = o.regions[2].plot.as_ref().unwrap_or_else(|| panic!("{:?}", o.regions[2].error));
    assert_eq!(p.xs.len(), 121);
    assert_eq!(p.xunit, "m³/h");
    assert!(close(p.xs[120], 150.0, 1e-12));
    assert_eq!(p.series.len(), 2);
    assert!(close(p.series[0].ys[0], 60.0, 1e-12) && p.yunit == "m", "{} {}", p.series[0].ys[0], p.yunit);
    assert!(close(p.series[1].ys[120], 42.5, 1e-9));
}

#[test]
fn latex_shows_what_is_computed() {
    let t = |s: &str| crate::latex::stmt(&crate::parse::statement(s, &[]).unwrap());
    assert_eq!(t("P = rho*g*Q*H/eta"), "P = \\frac{\\rho \\cdot g \\cdot Q \\cdot H}{\\eta}");
    assert_eq!(t("sigma_allow = 250 MPa"), "\\sigma_{\\mathrm{allow}} = 250\\,\\mathrm{MPa}");
    assert_eq!(t("Q = 120 m^3/h"), "Q = 120\\,\\mathrm{m^{3}/h}");
    assert_eq!(t("x = sqrt(a^2 + b^2)"), "x = \\sqrt{{a}^{2} + {b}^{2}}");
    assert_eq!(t("x = (a + b)/(c - d)"), "x = \\frac{a + b}{c - d}");
    assert_eq!(t("x = (a + b)*c"), "x = \\left(a + b\\right) \\cdot c");
    assert_eq!(t("x = a - (b - c)"), "x = a - \\left(b - c\\right)");
    assert_eq!(t("check sigma <= sigma_allow"), "\\sigma \\le \\sigma_{\\mathrm{allow}}");
    assert_eq!(t("f(x) = 3*x^2"), "f\\left(x\\right) = 3 \\cdot {x}^{2}");
    assert_eq!(t("T = 20 °C"), "T = 20\\,\\mathrm{{^\\circ}C}");
    assert_eq!(t("x = sin(30 deg)"), "x = \\sin\\left(30\\,\\mathrm{deg}\\right)");
    assert_eq!(t("Re = rho*v*D/mu"), "\\mathrm{Re} = \\frac{\\rho \\cdot v \\cdot D}{\\mu}");
    assert_eq!(t("x = 1.5e-5"), "x = 1.5 \\times 10^{-5}");
    assert_eq!(t("x = 0.0015"), "x = 0.0015");
    assert_eq!(crate::latex::unit("W/(m²·K)"), "\\mathrm{W/(m^{2}\\cdot K)}");
    assert_eq!(crate::latex::unit("m·s⁻¹"), "\\mathrm{m\\cdot s^{-1}}");
    let o = run(&["P = 18400 W*1"]);
    assert_eq!(o.regions[0].result_tex, "18.4\\,\\mathrm{kW}");
}

#[test]
fn whole_sheet_pump_power() {
    // Hydraulic power P = ρ·g·Q·H / η: 1000 × 9.80665 × (120/3600) × 45 / 0.75 = 19 613.3 W.
    let mut s = sheet(&["rho = 1000 kg/m^3", "Q = 120 m^3/h", "H = 45 m", "eta = 0.75", "P = rho*g*Q*H/eta", "P_motor = 22 kW", "check P <= P_motor"]);
    s.regions.insert(0, Region { id: 99, kind: Kind::Text, text: "Pump power".into(), ..Default::default() });
    let o = eval::run(&s);
    assert_eq!(o.regions[5].result, "19.61 kW");
    assert!(o.regions[2].input && !o.regions[5].input);
    let c = o.regions[7].check.as_ref().unwrap();
    assert!(c.pass && close(c.ratio.unwrap(), 19613.3 / 22000.0, 1e-5));
    assert_eq!(o.vars.len(), 6);
    assert_eq!(o.vars[4].name, "P");
    assert_eq!(o.errors, 0);
    let f = Fmt { digits: 3, eng: false, us: true, polar: false };
    let o = eval::run_with(&s, &f);
    assert_eq!(o.regions[5].result, "26.3 hp");
}

#[test]
fn tables_define_columns_and_work_out_formulas() {
    let mut s = sheet(&["rho = 1000 kg/m^3"]);
    s.regions.push(Region { id: 2, kind: Kind::Table, data: serde_json::json!({
        "cols": [{ "name": "Q", "unit": "m^3/h" }, { "name": "H", "unit": "m" }, { "name": "P_h", "unit": "kW", "formula": "rho*g*Q*H" }],
        "rows": [["0", "60"], ["50", "55"], ["100", "40"], ["150", "15 m"], ["", ""]] }), ..Default::default() });
    s.regions.push(Region { id: 3, kind: Kind::Math, text: "H_at(q) = interp(q, Q, H)".into(), ..Default::default() });
    s.regions.push(Region { id: 4, kind: Kind::Math, text: "H_75 = H_at(75 m^3/h)".into(), ..Default::default() });
    let o = eval::run(&s);
    let t = o.regions[1].table.as_ref().unwrap();
    assert!(o.regions[1].error.is_none(), "{:?}", o.regions[1].error);
    assert_eq!(t.rows, 4, "trailing empty rows are left out");
    assert_eq!(t.cols[0].cells, ["0", "50", "100", "150"]);
    assert_eq!(t.cols[0].unit, "m³/h");
    // ρgQH: 1000 × 9.80665 × (100/3600) × 40 = 10 896 W.
    assert_eq!(t.cols[2].cells[2], "10.9");
    assert_eq!(t.cols[2].unit, "kW");
    assert_eq!(o.regions[3].result, "47.5 m", "interpolated halfway between 55 m and 40 m");
    assert_eq!(o.vars.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["ρ", "Q", "H", "P_h", "H_75"]);
    // A cell with the wrong kind of unit is an error on that column, not a crash.
    s.regions[1].data["rows"][1][1] = serde_json::json!("55 s");
    let o = eval::run(&s);
    assert!(o.regions[1].error.as_ref().unwrap().msg.starts_with("H: Row 2"), "{:?}", o.regions[1].error);
    assert!(o.regions[3].error.as_ref().unwrap().msg.contains("error on its own line"));
    // Damaged table data never panics.
    for bad in [serde_json::json!(null), serde_json::json!({"cols": 3}), serde_json::json!({"cols": [{"name": "x"}], "rows": [[1, 2]]}), serde_json::json!({"cols": [{"name": "y", "formula": "=("}], "rows": [["1"]]})] {
        s.regions[1].data = bad;
        let _ = eval::run(&s);
    }
}

#[test]
fn notes_after_hash_describe_a_line() {
    let o = run(&["Q = 120 m^3/h  # design flow, from the process data sheet", "x = Q*2 # twice"]);
    assert_eq!(o.regions[0].note, "design flow, from the process data sheet");
    assert_eq!(o.regions[0].latex, "Q = 120\\,\\mathrm{m^{3}/h}");
    assert_eq!(o.regions[1].result, "240 m³/h");
    let o = run(&["x = 2 +  # broken"]);
    assert_eq!(o.regions[0].error.as_ref().unwrap().from, 7, "error positions count from the start of the line");
}

/// Every calc pack is a test: its results are compared with a hand calculation worked out before
/// running (the same formulas evaluated independently), quoted with the source in each pack.
mod packs {
    use super::close;
    use crate::eval::{self, Fmt};

    fn run(id: &str) -> (eval::SheetOut, eval::Env) {
        let s = crate::packs::sheet(id).unwrap_or_else(|| panic!("{id} does not open"));
        let r = eval::run_env(&s, &Fmt { digits: 4, eng: false, us: false, polar: false });
        let errs: Vec<String> = r.0.regions.iter().filter_map(|o| o.error.as_ref().map(|e| format!("region {}: {}", o.id, e.msg))).collect();
        assert!(errs.is_empty(), "{id}: {errs:?}");
        assert!(r.0.regions.iter().all(|o| o.warn.is_empty()), "{id}: a unit cannot show its value");
        r
    }
    fn v(env: &eval::Env, name: &str) -> f64 { env.vars.get(&eval::canon(name)).unwrap_or_else(|| panic!("{name} missing")).x() }
    fn checks(o: &eval::SheetOut) -> Vec<(bool, f64)> { o.regions.iter().filter_map(|r| r.check.as_ref().map(|c| (c.pass, c.ratio.unwrap_or(f64::NAN)))).collect() }

    #[test]
    fn all_packs_open_and_list() {
        let cards = crate::packs::cards();
        assert_eq!(cards.len(), 19);
        for c in &cards { assert!(!c.name.is_empty() && !c.desc.is_empty() && !c.field.is_empty() && c.checks > 0, "{}", c.id); }
    }

    #[test]
    fn getting_started_works_out_with_no_errors() {
        // The sheet that teaches the app must itself be right: M = 12 × 6²/8 = 54 kN·m; Wallis's cubic root 2.0946.
        let (o, e) = run("getting-started");
        assert!(close(v(&e, "M"), 54_000.0, 1e-9));
        assert!(close(v(&e, "r"), 2.094_551_48, 1e-7));
        assert!(close(v(&e, "H_75"), 37.5, 1e-9));
        assert_eq!(o.regions.iter().find(|r| r.name == "dT").unwrap().result, "15 K");
        // T = 2.5 kN × 0.2 m = 500 N·m; I = 230/(8 + 6j) = 230 (8 − 6j)/100 = 18.4 − 13.8j A; |I| = 23 A.
        assert_eq!(o.regions.iter().find(|r| r.name == "T").unwrap().result, "500 N·m");
        assert_eq!(o.regions.iter().find(|r| r.name == "I").unwrap().result, "(18.4 − 13.8j) A");
        assert!(close(v(&e, "I_rms"), 23.0, 1e-12));
        assert!(checks(&o)[0].0);
    }

    #[test]
    fn shaft() {
        // ω = 1480 × 2π/60 = 154.99 rad/s; T = 75 000/154.99 = 483.9 N·m; τ = 16 × 483.9/(π 0.06³) = 11.41 MPa;
        // d_min = ∛(16 × 483.9/(π 40e6)) = 39.50 mm; θ = 483.9 × 0.8/(79.3e9 × 1.2723e-6) = 3.837e-3 rad = 0.2198°.
        let (o, e) = run("shaft");
        assert!(close(v(&e, "T"), 483.917_06, 1e-6));
        assert!(close(v(&e, "tau"), 11.410_043e6, 1e-6));
        assert!(close(v(&e, "d_min"), 39.496_658e-3, 1e-6));
        assert!(close(v(&e, "theta"), 3.836_920_8e-3, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "0.2198°"), "the twist shows in degrees");
        assert!(o.regions.iter().any(|r| r.result == "483.9 N·m"));
        assert!(checks(&o).iter().all(|c| c.0));
        assert!(close(checks(&o)[2].1, 0.879_357, 1e-5));
    }

    #[test]
    fn tank() {
        // V = π 3²/4 × 4 = 28.27 m³; V_w = 25.45 m³; m = 25.45 t; p = 1000 × 9.80665 × 3.6 = 35.30 kPa; t = 25.45/20 = 1.272 h.
        let (o, e) = run("tank");
        assert!(close(v(&e, "V"), 28.274_334, 1e-6));
        assert!(close(v(&e, "m_l"), 25_446.900, 1e-6));
        assert!(close(v(&e, "p_b"), 35_303.94, 1e-6));
        assert!(close(v(&e, "t_fill"), 1.272_345 * 3600.0, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "1.272 h"), "a time of an hour or more shows in hours");
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.636_17, 1e-4));
    }

    #[test]
    fn thermal_expansion() {
        // ΔT = 165 K; ΔL = 12e-6 × 50 × 165 = 99 mm; σ = 200e9 × 12e-6 × 165 = 396 MPa; A = π/4 (168.3² − 154.1²) = 3596 mm²; F = 1424 kN.
        let (o, e) = run("thermal-expansion");
        assert!(close(v(&e, "dT"), 165.0, 1e-9));
        assert!(close(v(&e, "dL"), 0.099, 1e-9));
        assert!(close(v(&e, "sigma_r"), 396e6, 1e-9));
        assert!(close(v(&e, "F_r"), 1_423_863.79, 1e-6));
        let c = checks(&o);
        assert!(c[0].0 && close(c[0].1, 0.825, 1e-9));
        assert!(!c[1].0, "the fully restrained case fails, as the pack explains");
    }

    #[test]
    fn pipe_wall() {
        // t = 4.0 MPa × 168.3/(2 (138 + 4.0 × 0.4)) = 2.411 mm; + 1.5 = 3.911 mm; /(1 − 0.125) = 4.470 mm ≤ 7.11 mm.
        let (o, e) = run("pipe-wall");
        assert!(close(v(&e, "t"), 2.411_174_8e-3, 1e-6));
        assert!(close(v(&e, "t_req"), 4.469_914e-3, 1e-6));
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.628_680, 1e-5));
    }

    #[test]
    fn column() {
        // P_cr = π² × 200e9 × 603.8e-8/4² = 744.9 kN; σ = 138.5 MPa; r = 33.50 mm; λ = 119.4; 200/(744.9/3) = 0.805.
        let (o, e) = run("column");
        assert!(close(v(&e, "P_cr"), 744_908.39, 1e-6));
        assert!(close(v(&e, "sigma_cr"), 138.458_81e6, 1e-6));
        assert!(close(v(&e, "lambda"), 119.400_07, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "744.9 kN"));
        assert!(checks(&o).iter().all(|c| c.0));
        assert!(close(checks(&o)[0].1, 0.805_468, 1e-5));
    }

    #[test]
    fn cable_drop() {
        // sin φ = √(1 − 0.86²) = 0.5103; ΔV = √3 × 132.5 × 0.08 km × (0.193 × 0.86 + 0.08 × 0.5103) Ω/km = 3.797 V = 0.949 %.
        let (o, e) = run("cable-drop");
        assert!(close(v(&e, "dV"), 3.796_858_6, 1e-6));
        assert!(close(v(&e, "dV_pct"), 0.009_492_15, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "0.9492 %"));
        assert!(o.regions.iter().any(|r| r.result == "PASS (0.9492 % ≤ 3 %)"), "a check reads in the unit its name is shown in");
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.316_405, 1e-5));
    }

    #[test]
    fn ac_circuit() {
        // ω = 100π = 314.16 rad/s; Z_L = 8 + 9.4248j Ω; I_L = 230/Z_L = 12.040 − 14.184j A, |I_L| = 18.605 A; PF_L = 8/|Z_L| = 0.64713;
        // Z_C = −21.2207j Ω; Y = 1/Z_L + jωC = 0.052347 − 0.014546j S; I = 230 Y = 12.0398 − 3.34556j A, |I| = 12.4960 A, ∠ −15.529°;
        // S = V I* = 2769.15 + 769.48j VA; PF = 2769.15/2873.98 = 0.96349.
        let (o, e) = run("ac-circuit");
        let z = |n: &str| { let x = e.vars.get(&eval::canon(n)).unwrap(); (x.v[0], x.im.first().copied().unwrap_or(0.0)) };
        assert!(close(z("I").0, 12.039_801_5, 1e-8) && close(z("I").1, -3.345_562_35, 1e-8));
        assert!(close(z("S").0, 2_769.154_35, 1e-8) && close(z("S").1, 769.479_34, 1e-8));
        assert!(close(v(&e, "I_L_rms"), 18.604_953_5, 1e-8));
        assert!(close(v(&e, "PF_L"), 0.647_128_8, 1e-6));
        assert!(close(v(&e, "I_rms"), 12.495_983_7, 1e-8));
        assert!(close(v(&e, "phi").to_degrees(), -15.529_293, 1e-6));
        assert!(close(v(&e, "PF"), 0.963_493_7, 1e-7));
        for (want, why) in [("(8 + 9.425j) Ω", "rectangular with its unit"), ("(12.04 − 3.346j) A", "the supply current"), ("-21.22j Ω", "pure imaginary, no brackets"), ("314.2 rad/s", "ω in its unit"), ("2.769 kW", "real power")] {
            assert!(o.regions.iter().any(|r| r.result == want), "{why}: {want}");
        }
        assert!(checks(&o).iter().all(|c| c.0));
        let mut s = crate::packs::sheet("ac-circuit").unwrap();
        s.polar = true;
        assert!(eval::run(&s).regions.iter().any(|r| r.result == "12.5∠-15.53° A"), "polar form");
    }

    #[test]
    fn insulated_pipe() {
        // r₁ = 84.15 mm, r₂ = 134.15 mm; R_ins = ln(1.59418)/(2π 0.040) = 1.85558 K·m/W; R_o = 1/(2π 0.13415 × 10) = 0.118640 K·m/W;
        // q = 160/1.97422 = 81.045 W/m; Q = 8.1045 kW over 100 m; T_s = 20 + 81.045 × 0.11864 = 29.615 °C.
        let (o, e) = run("insulated-pipe");
        assert!(close(v(&e, "R_ins"), 1.855_578_16, 1e-8));
        assert!(close(v(&e, "R_o"), 0.118_639_54, 1e-8));
        assert!(close(v(&e, "q"), 81.044_760_3, 1e-8));
        assert!(close(v(&e, "Q"), 8_104.476_03, 1e-8));
        assert!(close(v(&e, "T_s") - 273.15, 29.615_113_1, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "29.62 °C") && o.regions.iter().any(|r| r.result == "81.04 W/m"));
        let c = checks(&o);
        assert!(c[0].0 && c[0].1.is_nan(), "no utilisation for absolute temperatures");
        assert!(c[1].0 && close(c[1].1, 0.810_447_6, 1e-6));
    }

    #[test]
    fn fillet_weld() {
        // f_vw,d = 490/(√3 × 0.9 × 1.25) = 251.468 MPa; F_w,Rd = 251.468 × 6 = 1508.81 N/mm; F_w,Ed = 250 000/300 = 833.33 N/mm;
        // F_Rd = 1508.81 × 300 = 452.64 kN; utilisation 0.5523; length limit max(30, 36) = 36 mm.
        let (o, e) = run("fillet-weld");
        assert!(close(v(&e, "f_vw_d"), 251.468_117e6, 1e-8));
        assert!(close(v(&e, "F_w_Rd"), 1_508_808.70, 1e-8));
        assert!(close(v(&e, "F_Rd"), 452_642.61, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "1509 N/mm") && o.regions.iter().any(|r| r.result == "PASS (150 mm ≥ 36 mm)"));
        let c = checks(&o);
        assert!(c.iter().all(|c| c.0) && close(c[0].1, 0.552_312_1, 1e-6));
    }

    #[test]
    fn pump_power() {
        // P_h = 998 × 9.80665 × (120/3600) × 45 = 14 680.6 W; /0.75 = 19 574.1 W; /0.93 = 21 047.4 W; ×1.15 = 22 510.2 W.
        let (o, e) = run("pump-power");
        assert!(close(v(&e, "P_h"), 14_680.555, 1e-6));
        assert!(close(v(&e, "P_shaft"), 19_574.073, 1e-6));
        assert!(close(v(&e, "P_in"), 21_047.391, 1e-6));
        assert!(close(v(&e, "P_req"), 22_510.184, 1e-6));
        assert_eq!(checks(&o).len(), 1);
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.750_34, 1e-4));
        assert!(o.regions.iter().any(|r| r.result == "14.68 kW"));
    }

    #[test]
    fn npsh() {
        // NPSHa = (101 325 − 7 385)/(992.2 × 9.80665) + 2 − 0.8 = 10.855 m ≥ 4.5 + 1 m.
        let (o, e) = run("npsh");
        assert!(close(v(&e, "NPSH_a"), 10.854_52, 1e-6));
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.506_70, 1e-4));
    }

    #[test]
    fn pipe_pressure_drop() {
        // v = 0.03333/(π 0.1541²/4) = 1.7872 m/s; Re = 274 863; Colebrook (ε/D = 2.92×10⁻⁴) f = 0.017067
        // (the Moody chart reads about 0.017); Δp = (f L/D + K) ρ v²/2 = 57.68 kPa; h_L = 5.894 m.
        let (o, e) = run("pipe-dp");
        assert!(close(v(&e, "v"), 1.787_243, 1e-6));
        assert!(close(v(&e, "Re"), 274_863.3, 1e-6));
        assert!(close(v(&e, "f"), 0.017_066_69, 1e-6));
        assert!(close(v(&e, "dp"), 57_680.44, 1e-5));
        assert!(close(v(&e, "h_L"), 5.893_55, 1e-5));
        assert!(checks(&o)[0].0);
    }

    #[test]
    fn operating_point() {
        // Between 120 and 160 m³/h the pump line is 46 − 13 (q − 120)/40; with 20 + 0.0012 q² they meet at q = 133.85 m³/h, H = 41.50 m.
        let (o, e) = run("operating-point");
        assert!(close(v(&e, "Q_op") * 3600.0, 133.849_63, 1e-6));
        assert!(close(v(&e, "H_op"), 41.498_87, 1e-6));
        assert!(checks(&o)[0].0);
        let plot = o.regions.iter().find_map(|r| r.plot.clone()).unwrap();
        assert_eq!(plot.series.len(), 2);
        assert!(close(plot.series[0].ys[0], 62.0, 1e-9) && close(plot.series[1].ys[0], 20.0, 1e-9));
    }

    #[test]
    fn compressor() {
        // (n−1)/n = 0.27/(1.27 × 0.78) = 0.272562; H_p = 8314.46/18.5 × 303.15/0.272562 × (3^0.272562 − 1) = 174.51 kJ/kg;
        // T2 = 303.15 × 3^0.272562 = 135.83 °C; P_gas = 2.5 × 174.51/0.78 = 559.3 kW; P_shaft = 570.7 kW.
        let (o, e) = run("compressor");
        assert!(close(v(&e, "x"), 0.272_562_08, 1e-7));
        assert!(close(v(&e, "H_p"), 174_505.2, 1e-6));
        assert!(close(v(&e, "T_2") - 273.15, 135.830_6, 1e-6));
        assert!(close(v(&e, "P_shaft"), 570_726.07, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "135.8 °C"), "the discharge temperature shows in °C");
        assert!(checks(&o).iter().all(|c| c.0));
    }

    #[test]
    fn heat_exchanger() {
        // Q = 3 × 2200 × 50 = 330 kW; m_c = 330 000/(4180 × 20) = 3.947 kg/s; LMTD = 30/ln(75/45) = 58.73 K; A = 330 000/(350 × 58.73) = 16.05 m².
        let (o, e) = run("hx-lmtd");
        assert!(close(v(&e, "Q"), 330_000.0, 1e-9));
        assert!(close(v(&e, "m_c"), 3.947_368, 1e-6));
        assert!(close(v(&e, "LMTD"), 58.728_456, 1e-6));
        assert!(close(v(&e, "A_req"), 16.054_52, 1e-6));
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.802_73, 1e-4));
    }

    #[test]
    fn pressure_vessel() {
        // σ_h = 1.5 × 1200/(2 × 10.5) = 85.71 MPa; σ_l = 42.86 MPa; t = 1.5 × 600/(138 × 0.85 − 0.9) + 1.5 = 9.232 mm.
        let (o, e) = run("pressure-vessel");
        assert!(close(v(&e, "sigma_h"), 85.714_286e6, 1e-6));
        assert!(close(v(&e, "sigma_l"), 42.857_143e6, 1e-6));
        assert!(close(v(&e, "t_req"), 9.231_959e-3, 1e-6));
        assert_eq!(checks(&o).iter().filter(|c| c.0).count(), 3);
    }

    #[test]
    fn beams() {
        // M = 12 × 6²/8 = 54 kN·m; σ = 54e3/557.1e-6 = 96.93 MPa; δ = 5 × 12e3 × 6⁴/(384 × 200e9 × 8356e-8) = 12.12 mm ≤ 24 mm.
        // Cantilever: M = 25 kN·m; δ = 10e3 × 2.5³/(3 × 200e9 × 8356e-8) = 3.117 mm ≤ 13.9 mm.
        let (o, e) = run("beams");
        assert!(close(v(&e, "M_max"), 54_000.0, 1e-9));
        assert!(close(v(&e, "sigma"), 96.930_533e6, 1e-6));
        assert!(close(v(&e, "delta"), 12.117_042e-3, 1e-6));
        assert!(close(v(&e, "delta_c"), 3.116_523e-3, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "54 kN·m"), "moments show in kN·m");
        assert!(checks(&o).iter().all(|c| c.0));
        assert!(close(checks(&o)[0].1, 0.504_88, 1e-4));
    }

    #[test]
    fn bolt() {
        // A_s = π/4 (16 − 1.8764)² = 156.67 mm² (ISO table: 157); F_i = 0.75 × 156.67 × 580 = 68.15 kN; T = 0.2 × 68.15 × 0.016 = 218.1 N·m;
        // F_b = 68.15 + 0.25 × 30 = 75.65 kN ≤ 90.87 kN.
        let (o, e) = run("bolt");
        assert!(close(v(&e, "A_s"), 156.668_14e-6, 1e-6));
        assert!(close(v(&e, "F_i"), 68_150.64, 1e-6));
        assert!(close(v(&e, "T"), 218.082_06, 1e-6));
        assert!(close(v(&e, "sigma_b"), 482.871_9e6, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "218.1 N·m"));
        assert!(checks(&o).iter().all(|c| c.0));
    }

    #[test]
    fn motor() {
        // P_in = 75/0.95 = 78.95 kW; I = 78 947/(√3 × 400 × 0.86) = 132.5 A; S = 91.80 kVA; Q = 46.84 kvar; I_start = 927.5 A.
        let (o, e) = run("motor");
        assert!(close(v(&e, "I_FL"), 132.500_83, 1e-6));
        assert!(close(v(&e, "S"), 91_799.27, 1e-6));
        assert!(close(v(&e, "Q_r"), 46_844.62, 1e-6));
        assert!(close(v(&e, "I_start"), 927.505_8, 1e-6));
        assert!(o.regions.iter().any(|r| r.result == "91.8 kVA") && o.regions.iter().any(|r| r.result == "46.84 kvar"));
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 0.828_13, 1e-4));
    }
}

#[test]
fn equations_with_numbers_substituted() {
    let mut s = sheet(&["rho = 998 kg/m^3", "Q = 120 m^3/h", "H = 45 m", "eta = 0.75", "P = rho*g*Q*H/eta", "A = pi*(2 m)^2/4", "s_x = -3 m", "y = s_x^2", "check P <= 30 kW"]);
    assert!(eval::run(&s).regions[4].subst_tex.is_empty(), "off unless the sheet shows numbers");
    s.subst = true;
    let o = eval::run(&s);
    assert_eq!(o.regions[4].subst_tex, "\\frac{998\\,\\mathrm{kg/m^{3}} \\cdot 9.807\\,\\mathrm{m/s^{2}} \\cdot 120\\,\\mathrm{m^{3}/h} \\cdot 45\\,\\mathrm{m}}{0.75}");
    assert!(o.regions[0].subst_tex.is_empty(), "an input is not repeated");
    assert!(o.regions[5].subst_tex.is_empty(), "π alone is not a substitution");
    assert_eq!(o.regions[7].subst_tex, "{\\left(-3\\,\\mathrm{m}\\right)}^{2}", "a negative value is bracketed under a power");
    assert!(o.regions[8].subst_tex.contains("\\le 30\\,\\mathrm{kW}"));
    let mut s = sheet(&["P = 75 kW", "n = 1480 rpm", "T = P/n", "x = T*2"]);
    s.regions[2].unit = "N*m".into();
    s.subst = true;
    assert_eq!(eval::run(&s).regions[3].subst_tex, "483.9\\,\\mathrm{N\\cdot m} \\cdot 2", "a value is put in with the unit it is shown in");
    let s2 = Sheet { subst: true, ..Default::default() };
    assert!(Sheet::from_json(&s2.json()).unwrap().subst, "the setting is saved with the sheet");
}

#[test]
fn torque_reads_in_newton_metres() {
    // Issue 2: 120 N × 0.35 m = 42 N·m, not 42 J; w L²/8 = 12 kN/m × 36 m²/8 = 54 kN·m; P/n = 75 kW/(1480 × 2π/60 s⁻¹) = 483.9 N·m.
    assert_eq!(result(&["F = 120 N", "d = 0.35 m", "T = F*d"]), "42 N·m");
    assert_eq!(result(&["F = 120 N", "d = 0.35 m", "T = F*d/2 + 1 N*m"]), "22 N·m", "a moment keeps its reading through plain factors and sums");
    assert_eq!(result(&["M = 12 kN/m * (6 m)^2/8"]), "54 kN·m");
    assert_eq!(result(&["T = 75 kW/(1480 rpm)"]), "483.9 N·m");
    assert_eq!(result(&["F = 20 N", "T = F*2 mm"]), "40 N·mm");
    assert_eq!(result(&["E = 2 kW * 3 h"]), "21.6 MJ", "a power times a time is an energy");
    let mut s = sheet(&["F = 100 lbf", "T = F*3 ft"]);
    s.units = "US".into();
    assert_eq!(eval::run(&s).regions[1].result, "300 lbf·ft");
    let o = run(&["F = 120 N", "d = 0.35 m", "T = F*d", "E = 2 kW * 3 h"]);
    assert_eq!(o.regions[2].units[..3], ["N·m", "kN·m", "N·mm"], "torque units first for a moment");
    assert!(o.regions[3].units.contains(&"N·m".to_string()) && o.regions[3].units[0] == "J", "and offered for any energy");
    // A torque typed with its unit keeps the reading; a torque times an angle is work (review of 0.1.8).
    assert_eq!(result(&["M2 = 50 N*m", "M3 = M2*2"]), "100 N·m");
    assert_eq!(result(&["M2 = 50 N*m", "M2 + M2"]), "100 N·m");
    assert_eq!(result(&["E = 50 J", "E*2"]), "100 J", "an energy typed in J stays an energy");
    assert_eq!(result(&["F = 20 kN", "d = 1 m", "M = F*d", "W = M*(90 deg)"]), "31.42 kJ");
    let mut s = sheet(&["T = 120 N * 0.35 m"]);
    s.regions[0].unit = "kN·m".into();
    assert_eq!(eval::run(&s).regions[0].result, "0.042 kN·m");
}

#[test]
fn complex_numbers() {
    // Issue 1. (3 + 4j)(1 − 2j) = 3 − 6j + 4j + 8 = 11 − 2j; (3 + 4j)/(1 − 2j) = (3 + 4j)(1 + 2j)/5 = (−5 + 10j)/5 = −1 + 2j.
    assert_eq!(result(&["z = 3 + 4j", "z*(1 - 2j)"]), "11 − 2j");
    assert_eq!(result(&["z = 3 + 4j", "z/(1 - 2j)"]), "-1 + 2j");
    assert_eq!(result(&["j*j"]), "-1", "j² is real");
    assert_eq!(result(&["i^2"]), "-1", "i is the same unit");
    assert_eq!(result(&["exp(j*pi) + 1"]), "0", "Euler's identity, the round-off tidied");
    assert_eq!(result(&["z = 3 + 4j", "abs(z)"]), "5");
    assert_eq!(result(&["z = 3 + 4j", "arg(z)"]), "53.13°");
    assert_eq!(result(&["z = 3 + 4j", "conj(z)"]), "3 − 4j");
    assert_eq!(result(&["z = 3 + 4j", "re(z) + im(z)"]), "7");
    assert_eq!(result(&["sqrt(-3 - 4j)"]), "1 − 2j", "principal root");
    assert_eq!(result(&["z = 3 + 4j", "z^2"]), "-7 + 24j");
    assert_eq!(result(&["z = 3 + 4j", "ln(z)"]), "1.609 + 0.9273j");
    assert_eq!(result(&["polar(2, 90 deg)"]), "2j");
    assert_eq!(result(&["Z = (3 + 4j) kΩ"]), "(3 + 4j) kΩ", "a unit after brackets, kept as typed");
    assert_eq!(result(&["V = 10 V", "Z = (3 + 4j) kΩ", "I = V/Z"]), "(1.2 − 1.6j) mA");
    assert_eq!(result(&["X = 5j mA"]), "5j mA");
    assert_eq!(result(&["x = 0j", "check x < 1"]), "PASS (0 < 1)", "0j is real");
    assert!(run(&["Z = (8 + 6j) Ω"]).regions[0].input, "a complex number written out is an input");
    assert!(!run(&["x = 2 + 3"]).regions[0].input, "a sum still shows its result");
    assert_eq!(result(&["x = 1, 2 .. 3", "x*(1 + 1j)"]), "[1 + 1j, 2 + 2j, 3 + 3j]");
    assert_eq!(result(&["x = 1, 2 .. 3", "sum(x*1j)"]), "6j");
    assert_eq!(result(&["j = 3", "2*j + 4j"]), "6 + 4j", "a name on the sheet takes j's place; 4j stays imaginary");
    // Real-only places say so.
    for (lines, msg) in [
        (&["z = 3 + 4j", "check z <= 5"][..], "A check needs a real value"),
        (&["z = 3 + 4j", "z < 5"][..], "Complex values cannot be compared"),
        (&["z = 3 + 4j", "sin(z)"][..], "sin needs real values"),
        (&["x = 0 .. 1j"][..], "A range needs a real value"),
        (&["3 + 4j Ω"][..], "Cannot add a pure number and"),
        (&["z = 1j", "z/(0j)"][..], "Division by zero"),
        (&["(2 + 1j) m ^ (1j)"][..], "A complex power needs a plain number"),
        (&["F = 10 kN", "A = 2 m^2", "(F/A) MPa"][..], "A unit after brackets needs a plain number inside"),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
    let o = run(&["z = 1j", "plot z*x for x in 0 .. 1", "solve z*x = 1 for x in 0 .. 2"]);
    assert!(o.regions[1].error.as_ref().unwrap().msg.starts_with("A plot shows real values"));
    assert!(o.regions[2].error.as_ref().unwrap().msg.starts_with("solve needs a real value"));
    // Polar form, and the substituted line brackets a complex value.
    let mut s = sheet(&["Z = (3 + 4j) Ω", "I = 10 V/Z"]);
    s.polar = true;
    s.subst = true;
    let o = eval::run(&s);
    assert_eq!(o.regions[0].result, "5∠53.13° Ω");
    assert_eq!(o.regions[1].result, "2∠-53.13° A");
    s.polar = false;
    let o = eval::run(&s);
    assert_eq!(o.regions[1].subst_tex, "\\frac{10\\,\\mathrm{V}}{\\left(3 + 4\\,\\mathrm{j}\\right)\\,\\mathrm{\\Omega }}");
    assert_eq!(o.regions[0].latex, "Z = \\left(3 + 4\\,\\mathrm{j}\\right)\\,\\mathrm{\\Omega }");
    // j drawn upright when it is the imaginary unit; a parameter, a plot variable or a name on the sheet stays italic.
    let o = run(&["w = 2", "Z = 3 + j*w", "f(j) = j^2", "plot i*w for i in 0 .. 1", "j = 5", "k = j*2"]);
    let tex: Vec<&str> = o.regions.iter().map(|r| r.latex.as_str()).collect();
    assert_eq!(tex[1..], ["Z = 3 + \\mathrm{j} \\cdot w", "f\\left(j\\right) = {j}^{2}", "i \\cdot w \\quad \\text{for } i \\in \\left[0, 1\\right]", "j = 5", "k = j \\cdot 2"]);
    assert!(!crate::sheet::Sheet::from_json(&serde_json::to_string(&s).unwrap()).unwrap().polar);
}
