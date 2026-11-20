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
        ("V", 1.0), ("mV", 1e-3), ("kV", 1e3), ("Ω", 1.0), ("ohm", 1.0), ("kΩ", 1e3), ("kohm", 1e3), ("MΩ", 1e6), ("mΩ", 1e-3), ("µΩ", 1e-6), ("nΩ", 1e-9), ("S", 1.0), ("C", 1.0), ("Ah", 3600.0), ("mAh", 3.6),
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
        let kind = if l.starts_with("check") { Kind::Check } else if l.contains('\n') { Kind::Program } else { Kind::Math };
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
    assert_eq!(result(&["w = 2 kg*m^2/s*1"]), "2 J·s", "a kind with no unit of its own reads in named units");
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
    let f = Fmt { digits: 3, eng: false, us: true, polar: false, exact: false };
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
        let r = eval::run_env(&s, &Fmt { digits: 4, eng: false, us: false, polar: false, exact: false });
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
        assert_eq!(cards.len(), 30);
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
    fn rc_filter() {
        // τ = 1.6 kΩ × 100 nF = 160 µs; f_c = 1/(2π τ) = 994.72 Hz; H(5 kHz) = 1/(1 + j 2π 5000 τ) = 0.038072 − 0.19137j;
        // |H| = 0.19512 → −14.194 dB; ∠ −78.748°; attenuation 14.194 dB ≥ 12 dB.
        let (o, e) = run("rc-filter");
        assert!(close(v(&e, "f_c"), 994.718_394, 1e-8));
        let h = e.vars.get("H_s").unwrap();
        assert!(close(h.v[0], 0.038_071_761, 1e-7) && close(h.im[0], -0.191_369_543, 1e-7));
        assert!(close(v(&e, "G_s"), -14.193_970_4, 1e-8));
        assert!(close(v(&e, "phi_s").to_degrees(), -78.748_274_3, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "994.7 Hz") && o.regions.iter().any(|r| r.result == "0.03807 − 0.1914j"), "a ratio, not an angle");
        assert!(checks(&o)[0].0);
        let plots: Vec<_> = o.regions.iter().filter_map(|r| r.plot.as_ref()).collect();
        assert!(plots.len() == 2 && plots.iter().all(|p| p.logx && p.xunit == "Hz"));
        let p = plots[0];
        assert!(close(p.xs[0], 10.0, 1e-12) && close(*p.xs.last().unwrap(), 100_000.0, 1e-9) && close(p.xs[30], 100.0, 1e-9), "evenly spaced in decades");
        assert!(close(p.series[0].ys[60], 20.0 * (1.0 / (1.0f64 + (1000.0 / 994.718_394f64).powi(2)).sqrt()).log10(), 1e-6), "−3 dB near the cut-off");
        assert_eq!(plots[1].yunit, "°");
    }

    #[test]
    fn flange_bolts() {
        // H = π/4 0.2² 2e6 = 62 831.9 N; H_p = 2 0.008 π 0.2 3 2e6 = 60 318.6 N; W_m1 = 123 150.4 N; W_m2 = π 0.008 0.2 69e6 = 346 831.8 N;
        // A_m = max(716.0, 2016.5) mm² = 2016.5 mm²; A_b = 12 × 225.2 = 2702.4 mm²; utilisation 0.74618.
        let (o, e) = run("flange-bolts");
        assert!(close(v(&e, "W_m1"), 123_150.432, 1e-8) && close(v(&e, "W_m2"), 346_831.829, 1e-8));
        assert!(close(v(&e, "A_m"), 2016.464122e-6, 1e-8) && close(v(&e, "A_b"), 2702.4e-6, 1e-12));
        assert!(checks(&o)[0].0 && close(checks(&o)[0].1, 2016.464122 / 2702.4, 1e-8));
    }

    #[test]
    fn tank_heating() {
        // m = 4990 kg; UA = 5 × 18 = 90 W/K; τ = 4990 × 4182/90 = 231 868.7 s; t = τ ln((50 000 − 90×5)/(50 000 − 90×50)) = 19 771.4 s = 5.4921 h;
        // without losses 4990 × 4182 × 45/50 000 = 18 781.4 s = 5.2170 h: the losses add about 5 %.
        let (o, e) = run("tank-heating");
        assert!(close(v(&e, "tau"), 231_868.666_7, 1e-9));
        assert!(close(v(&e, "t"), 19_771.426_09, 1e-8) && close(v(&e, "t_0"), 18_781.362, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "5.492 h") && checks(&o)[0].0);
    }

    #[test]
    fn three_phase() {
        // I_1 = 75 000/(√3 400 0.82) = 132.016 A; S_1 = 91.463 kVA; Q_1 = 52.350 kvar; Q_c = 75 000 (0.69800 − 0.32868) = 27.699 kvar;
        // I_2 = 113.951 A; ω = 314.16 rad/s; C = 27 699/(3 × 314.16 × 400²) = 183.68 µF.
        let (o, e) = run("three-phase");
        assert!(close(v(&e, "I_1"), 132.016_068, 1e-8) && close(v(&e, "I_2"), 113.950_711, 1e-8));
        assert!(close(v(&e, "Q_c"), 27_699.014_14, 1e-8) && close(v(&e, "C"), 183.684_792e-6, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "314.2 rad/s") && o.regions.iter().any(|r| r.result == "183.7 µF"));
        assert!(checks(&o)[0].0);
    }

    #[test]
    fn spring() {
        // C = 8; K_B = 34/29 = 1.17241; k = 81e9 × 0.003⁴/(8 × 0.024³ × 8) = 7415.77 N/m; y = 20.227 mm; τ = K_B 8 150 0.024/(π 0.003³) = 398.07 MPa;
        // S_ut = 2211/3^0.145 = 1885.41 MPa; τ_allow = 848.44 MPa (Shigley Table 10-6, 45 %).
        let (o, e) = run("spring");
        assert!(close(v(&e, "k"), 7415.771484, 1e-9) && close(v(&e, "tau"), 398.070294e6, 1e-8));
        assert!(close(v(&e, "S_ut"), 1885.411539e6, 1e-8) && close(v(&e, "y"), 20.22716e-3, 1e-7));
        assert!(o.regions.iter().any(|r| r.result == "7.416 N/mm") && checks(&o)[0].0 && close(checks(&o)[0].1, 0.469_182, 1e-5));
    }

    #[test]
    fn belt_drive() {
        // θ = π − 2 asin(240/1600) = 2.840456 rad (162.75°); v = 1450 × 2π/60 × 0.08 = 12.1475 m/s; ΔT = 617.411 N; e^(0.3θ) = 2.344652;
        // T_2 = 459.161 N; T_1 = 1076.572 N; n_2 = 580 rpm; L = 2.497646 m.
        let (o, e) = run("belt-drive");
        assert!(close(v(&e, "theta"), 2.840_456_108, 1e-9) && close(v(&e, "v"), 12.147_491_59, 1e-9));
        assert!(close(v(&e, "T_1"), 1076.572291, 1e-8) && close(v(&e, "T_2"), 459.160873, 1e-8));
        assert!(close(v(&e, "L"), 2.497_645_943, 1e-9));
        assert!(o.regions.iter().any(|r| r.result == "580 rpm") && o.regions.iter().any(|r| r.result == "162.7°"));
        // With its numbers put in, the angle in the exponent reads in radians.
        let mut s = crate::packs::sheet("belt-drive").unwrap();
        s.subst = true;
        let r = eval::run(&s).regions.into_iter().find(|r| r.name == "R").unwrap();
        assert!(r.subst_tex.contains("2.84\\,\\mathrm{rad}") && !r.subst_tex.contains("circ"), "{}", r.subst_tex);
        assert!(checks(&o)[0].0);
    }

    #[test]
    fn parallel_pipes() {
        // K_i = f L/D ρ/(2A²): K_1 = 8.08948e7, K_2 = 1.55529e8 Pa/(m³/s)²; Q_1/Q_2 = √(K_2/K_1) = 1.38659; Q_1 = 58.099 m³/h,
        // Q_2 = 41.901 m³/h; Δp = K_1 Q_1² = 21.069 kPa; v_1 = 2.0548 m/s, v_2 = 2.3155 m/s.
        let (o, e) = run("parallel-pipes");
        assert!(close(v(&e, "Q_1") * 3600.0, 58.099_058_17, 1e-8) && close(v(&e, "Q_2") * 3600.0, 41.900_941_83, 1e-8));
        assert!(close(v(&e, "dp_12"), 21_069.487_2, 1e-8) && close(v(&e, "v_2"), 2.315_535_421, 1e-8));
        assert!(o.regions.iter().any(|r| r.result == "Q_1 = 58.1 m³/h, Q_2 = 41.9 m³/h") && checks(&o)[0].0);
    }

    #[test]
    fn tank_drain() {
        // Exact: √h = √h₀ − (C_d A_o/A_t) √(2g)/2 · t; A_o = 1.9635e-3 m²; empties at t_e = 786.94 s; after 10 min
        // h = 0.112859 m ≥ 0.1 m; the outflow then is C_d A_o √(2gh) = 1.8104 L/s.
        let (o, e) = run("tank-drain");
        assert!(close(v(&e, "h_10"), 0.112_859_097_128_226, 1e-8) && close(v(&e, "h_x"), 0.112_859_097_128_226, 1e-12));
        assert!(close(v(&e, "t_e"), 786.935_862_178, 1e-12) && checks(&o)[0].0);
        let q = 0.62 * 0.001_963_495_408_49 * (2.0 * 9.80665 * 0.112_859_097_128f64).sqrt();
        assert!(close(v(&e, "Q_10"), q, 1e-7) && o.regions.iter().all(|r| r.error.is_none()));
        assert!(o.regions.iter().any(|r| r.plot.is_some()));
    }

    #[test]
    fn pump_test() {
        // Least squares (numpy.polyfit) through the six test points: a = 61.992857 m, b = 0.0094 m/(m³/h),
        // c = −0.00178857 m/(m³/h)², R² = 0.999991; k = 23/90² = 0.00283951 m/(m³/h)²; the curves meet at
        // Q = 101.787 m³/h, H = 44.419 m (scipy brentq).
        let (o, e) = run("pump-test");
        let h = 3600.0;
        assert!(close(v(&e, "a"), 61.992_857_142_86, 1e-9) && close(v(&e, "b") / h, 0.0094, 1e-8) && close(v(&e, "c") / (h * h), -0.001_788_571_428_57, 1e-9));
        assert!(close(v(&e, "Q_op") * h, 101.787_026_624_8, 1e-9) && close(v(&e, "H_op"), 44.418_984_216, 1e-9) && checks(&o)[0].0);
        assert!(o.regions.iter().all(|r| r.error.is_none()) && o.regions.iter().any(|r| r.result.ends_with("R² = 1.0000")));
        assert!(o.regions.iter().any(|r| r.plot.as_ref().is_some_and(|p| p.series.len() == 3 && p.series[2].points && p.series[2].xs.len() == 6)));
    }

    #[test]
    fn rankine() {
        // Independent IAPWS-IF97 values (another implementation): h₁ = 3214.374 kJ/kg, s₁ = 6.771192 kJ/(kg·K) at 40 bar,
        // 400 °C; h₂s = 2144.459; h₂ = h₁ − 0.85 (h₁ − h₂s) = 2304.946, x₂ = 0.883390, 45.81 °C; h₃ = 191.812,
        // v₃ = 1.010261 L/kg; w_p = v₃ Δp/0.75 = 5.37459; q_in = 3017.187; η = (909.427 − 5.375)/3017.187 = 0.299634;
        // P = 20 × 904.05 = 18.081 MW; Q_cond = 42.263 MW. Steam tables give 3213.6 kJ/kg and 6.7690 at 40 bar, 400 °C.
        let (o, e) = run("rankine");
        assert!(close(v(&e, "h_1"), 3_214_373.508_87, 1e-9) && close(v(&e, "s_1"), 6_771.191_993_87, 1e-9));
        assert!(close(v(&e, "h_2s"), 2_144_459.331_47, 1e-8) && close(v(&e, "x_2"), 0.883_389_726_14, 1e-8));
        assert!(close(v(&e, "T_2"), 318.957_548_207, 1e-9) && close(v(&e, "h_3"), 191_812.295_194, 1e-8));
        assert!(close(v(&e, "eta_th"), 0.299_634_254_08, 1e-8) && close(v(&e, "P_net"), 18_081_049.29, 1e-8));
        assert!(close(v(&e, "Q_cond"), 42_262_683.26, 1e-8) && checks(&o)[0].0);
        assert!(o.regions.iter().all(|r| r.error.is_none()) && o.regions.iter().any(|r| r.result == "18.08 MW"));
    }

    #[test]
    fn pipe_regimes() {
        // A = π 0.0266²/4 = 5.5572e-4 m²; Q_1 = 0.05 m³/h: v = 0.024993 m/s, Re = 663.48 (laminar), f = 64/Re = 0.096461,
        // Δp = f L/D ρ v²/2 = 22.606 Pa. Q_2 = 3 m³/h: v = 1.49957 m/s, Re = 39 808.7, Colebrook with ε/D = 0.0016917 gives
        // f = 0.0264692 (Haaland 0.0262), Δp = 22 331.7 Pa ≤ 30 kPa.
        let (o, e) = run("pipe-regimes");
        assert!(close(v(&e, "Re_1"), 663.478_000_02, 1e-8) && close(v(&e, "f_1"), 0.096_461_374_75, 1e-8));
        assert!(close(v(&e, "f_2"), 0.026_469_227_09, 1e-8) && close(v(&e, "dp_1"), 22.606_381_36, 1e-8));
        assert!(close(v(&e, "dp_2"), 22_331.678_32, 1e-8) && checks(&o)[0].0);
        assert!(o.regions.iter().all(|r| r.error.is_none()) && o.regions.iter().any(|r| r.plot.as_ref().is_some_and(|p| p.series[0].ys.iter().all(|y| y.is_finite()))));
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
        (&["z = 3 + 4j", "min(z, 1)"][..], "min needs real values"),
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

#[test]
fn angular_frequency_and_named_units() {
    // ω = 2π × 50 Hz = 314.16 rad/s = 3000 rpm; 50 Hz = 3000 rpm (one revolution a cycle); 1480 rpm = 24.667 Hz.
    assert_eq!(result(&["f = 50 Hz", "w = 2*pi*f"]), "314.2 rad/s");
    assert_eq!(result(&["w = 2*pi/(20 ms)"]), "314.2 rad/s");
    assert_eq!(result(&["f = 50 Hz", "w = 2*pi*f", "w/(2*pi)"]), "50 Hz");
    let shown = |lines: &[&str], unit: &str| { let mut s = sheet(lines); s.regions.last_mut().unwrap().unit = unit.into(); eval::run(&s).regions.last().unwrap().result.clone() };
    assert_eq!(shown(&["f = 50 Hz"], "rpm"), "3000 rpm");
    assert_eq!(shown(&["f = 50 Hz", "w = 2*pi*f"], "rpm"), "3000 rpm");
    assert_eq!(shown(&["f = 50 Hz", "w = 2*pi*f"], "Hz"), "50 Hz");
    assert_eq!(shown(&["f = 50 Hz"], "rad/s"), "314.2 rad/s");
    assert_eq!(shown(&["n = 1480 rpm"], "Hz"), "24.67 Hz");
    // Nothing worked out from ω is an angle: Q = ωL/R and 1/(1 + jωτ) are plain numbers.
    assert_eq!(result(&["f = 50 Hz", "w = 2*pi*f", "Q = w*30 mH/(10 Ω)"]), "0.9425");
    assert_eq!(result(&["f = 50 Hz", "w = 2*pi*f", "X = w*30 mH"]), "9.425 Ω");
    assert!(run(&["f = 50 Hz"]).regions[0].units.contains(&"rad/s".to_string()));
    // Review of 0.1.11: 1/(2π) undoes the radians (the textbook natural frequency), ω on a plot's x axis, ω and f on one plot.
    assert_eq!(result(&["k = 1000 N/m", "m = 0.1 kg", "f_n = 1/(2*pi)*sqrt(k/m)"]), "15.92 Hz");
    assert_eq!(shown(&["tau = 1 ms", "f_c = 1/(2*pi)/tau"], "Hz"), "159.2 Hz");
    assert_eq!(result(&["f = 50 Hz", "w = 2*pi*f", "w*(1/(2*pi))"]), "50 Hz");
    let o = run(&["f = 50 Hz", "w = 2*pi*f", "plot x*1 s for x in w .. 2*w", "plot w, f for x in 0 .. 1"]);
    let (p, q) = (o.regions[2].plot.as_ref().unwrap(), o.regions[3].plot.as_ref().unwrap());
    assert!(close(p.xs[0], 314.159_265, 1e-8) && p.xunit == "rad/s", "{:?}", &p.xs[..2]);
    assert!(q.yunit == "rad/s" && close(q.series[1].ys[0], 314.159_265, 1e-8), "f drawn as 2π f on a rad/s axis");
    // Every automatic label is a unit that can be typed back, with the same dimension.
    for e in 0..625 {
        let p = |k: u32| (e / 5u32.pow(k) % 5) as i8 - 2;
        let dim = [p(0), p(1), p(2), p(3), 0, 0, 0, 0];
        if dim == [0; 8] { continue; }
        let l = units::label(&dim);
        let u = units::parse(&l).unwrap_or_else(|e| panic!("{dim:?} → {l}: {e}"));
        assert!(units::same(&u.dim, &dim), "{dim:?} → {l}");
    }
    // Kinds with no unit of their own read in named SI units, not base units.
    for (line, want) in [("q = 81 W/m", "81 W/m"), ("R = 1.86 K*m/W*1", "1.86 m·K/W"), ("G = 3 kg/(m^2*s)*1", "3 kg/(m²·s)"), ("k = 5 W/K*1", "5 W/K"),
        ("p = 4 N*s*1", "4 N·s"), ("E = 2 V/m*1", "2 V/m"), ("I = 2 kg*m^2*1", "2 kg·m²"), ("x = 2 W/m^2*1", "2 W/m²")] {
        assert_eq!(result(&[line]), want, "{line}");
        assert!(units::parse(&result(&[line]).split_once(' ').unwrap().1.replace('·', "*")).is_ok(), "{line}: the label is a unit that can be typed");
    }
}

#[test]
fn complex_numbers_for_ac_analysis() {
    // 10 Ω∠30° = 8.660 + 5j Ω; par(10, −20j) = −200j/(10 − 20j) = 8 − 4j; cosh(1 + j) = 0.83373 + 0.98890j; sin(2j) = 3.6269j;
    // db(1/(1 + j)) = 20 log₁₀(0.70711) = −3.0103.
    assert_eq!(result(&["Z = 10 Ω∠30°"]), "(8.66 + 5j) Ω");
    assert_eq!(result(&["V = 230 V/_-30 deg"]), "(199.2 − 115j) V", "/_ is the typed form of ∠");
    assert_eq!(result(&["2*10∠90°"]), "20j", "∠ binds before ×");
    assert_eq!(result(&["V = 230∠-30"]), "199.2 − 115j", "a bare number after ∠ is in degrees");
    assert_eq!(result(&["10^2∠30°"]), "86.6 + 50j", "∠ does not bind inside a power");
    assert!(error(&["plot x for x in -10 °C .. 50 °C log"]).starts_with("A log scale needs a range above zero"));
    assert_eq!(result(&["par(10 Ω, -20j Ω)"]), "(8 − 4j) Ω");
    assert_eq!(result(&["par(2 Ω, 2 Ω, 1 Ω)"]), "500 mΩ");
    assert_eq!(result(&["cosh(1 + 1j)"]), "0.8337 + 0.9889j");
    assert_eq!(result(&["sin(2j)"]), "3.627j");
    assert_eq!(result(&["db(1/(1 + 1j))"]), "-3.01");
    assert_eq!(error(&["par(1 Ω, 0 Ω)"]), "par of a zero value (a short circuit).");
    assert!(error(&["db(2 V)"]).starts_with("db needs a plain number"));
    assert!(error(&["plot x for x in 0 Hz .. 1 kHz log"]).starts_with("A log scale needs a range above zero"));
    assert_eq!(run(&["Z = 10 Ω∠30°"]).regions[0].latex, "Z = 10\\,\\mathrm{\\Omega }\\angle 30\\,\\mathrm{{^\\circ}}");
    // A table of impedances over frequency: a complex formula column, and complex cells typed with the column's unit.
    let mut s = sheet(&["L = 30 mH"]);
    s.regions.push(Region { id: 9, kind: Kind::Table, data: serde_json::json!({ "cols": [{ "name": "f_t", "unit": "Hz" }, { "name": "Z_t", "formula": "10 Ω + j*2*pi*f_t*L" }, { "name": "Z_m", "unit": "Ω" }], "rows": [["50", "", "3 + 4j"], ["100", "", "1j"]] }), ..Default::default() });
    let o = eval::run(&s);
    let t = o.regions[1].table.as_ref().unwrap();
    assert!(o.regions[1].error.is_none(), "{:?}", o.regions[1].error);
    assert_eq!(t.cols[1].cells, ["10 + 9.425j", "10 + 18.85j"]);
    assert_eq!(t.cols[1].unit, "Ω");
    assert_eq!(t.cols[2].cells, ["3 + 4j", "1j"]);
}

#[test]
fn exact_algebra() {
    // Worked by hand: (x + 1)² = x² + 2x + 1; (x² − 1)/(x − 1) = x + 1; 2x² − x − 1 = (2x + 1)(x − 1);
    // 6x²y + 9xy = 3xy(2x + 3); x³ − 6x² + 11x − 6 = (x − 1)(x − 2)(x − 3); x⁴ − 1 = (x + 1)(x − 1)(x² + 1).
    for (line, want) in [
        ("expand((x+1)^2)", "x² + 2x + 1"), ("expand((x+1)*(x-1))", "x² − 1"), ("simplify((x^2-1)/(x-1))", "x + 1"),
        ("simplify(3*(x+2) - x)", "2x + 6"), ("simplify(x/2 + x/3)", "(5/6)x"), ("simplify((x^2+2*x+1)/(x^2-1))", "(x + 1)/(x − 1)"),
        ("factor(x^2-1)", "(x + 1)(x − 1)"), ("factor(2*x^2 - x - 1)", "(2x + 1)(x − 1)"), ("factor(6*x^2*y + 9*x*y)", "3xy(2x + 3)"),
        ("factor(x^3 - 6*x^2 + 11*x - 6)", "(x − 1)(x − 2)(x − 3)"), ("factor(x^4 - 1)", "(x + 1)(x − 1)(x² + 1)"), ("factor(x^2 + 1)", "x² + 1"),
        ("factor(3*x^2 - 3)", "3(x + 1)(x − 1)"), ("factor(-x^2 + 1)", "-(x + 1)(x − 1)"), ("factor((x+1)^3)", "(x + 1)³"),
        ("expand(pi*(r+1)^2)", "πr² + 2πr + π"), ("expand(0.1*x + 0.2*x)", "(3/10)x"), ("simplify(x^2*y/(x*y^2))", "x/y"), ("factor(0)", "0"),
    ] { assert_eq!(result(&[line]), want, "{line}"); }
    // Defined plain numbers are put in; an expression kept in a name is used by later lines.
    assert_eq!(result(&["a = 2", "expand((x+a)^2)"]), "x² + 4x + 4");
    assert_eq!(result(&["y = expand((x+1)^3)", "factor(y)"]), "(x + 1)³");
    assert_eq!(run(&["factor(2*x^2 - x - 1)"]).regions[0].result_tex, "\\left(2x + 1\\right)\\left(x - 1\\right)");
    // What exact maths refuses, and says so.
    for (lines, msg) in [
        (&["simplify(2 m*x)"][..], "Exact maths works on numbers, names, + − × ÷ and whole powers, not units."),
        (&["simplify(sin(x))"][..], "Exact maths works on numbers, names, + − × ÷ and whole powers, not the function sin."),
        (&["2*expand(x)"][..], "expand gives an expression, not a value"),
        (&["y = expand(x)", "y + 1"][..], "y is an expression, not a value"),
        (&["L = 2 m", "expand(L*x)"][..], "L has units"),
        (&["expand(x^0.5)"][..], "Exact maths needs a whole-number power."),
        (&["simplify(1/(x - x))"][..], "Division by zero."),
        (&["expand((x+1)^200)"][..], "The numbers grow too large for exact maths."),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}

#[test]
fn exact_algebra_holds_for_random_polynomials() {
    use crate::sym::{factor, Poly, Q, RatF};
    let mut seed = 0x0200_5EEDu64;
    let mut rnd = |n: i128| { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed % (2 * n as u64 + 1)) as i128 - n };
    let x = Poly::symbol("x");
    let y = Poly::symbol("y");
    for round in 0..1000 {
        // A product of linear factors with small roots, times a random quadratic, in x (and sometimes y).
        let mut p = Poly::constant(Q::int(rnd(3).max(1)));
        for _ in 0..(round % 4) { p = p.mul(&x.scale(Q::int(rnd(2).max(1))).unwrap().add(&Poly::constant(Q::int(rnd(4)))).unwrap()).unwrap(); }
        let mut q = x.mul(&x).unwrap().add(&Poly::constant(Q::int(rnd(5)))).unwrap();
        if round % 3 == 0 { q = q.add(&y.scale(Q::int(rnd(3))).unwrap()).unwrap(); }
        let pq = p.mul(&q).unwrap();
        // factor, multiplied back, is the polynomial.
        let f = factor(&pq).unwrap();
        let mut back = Poly::constant(f.c).mul(&Poly(std::iter::once((f.mono.clone(), Q::ONE)).collect())).unwrap();
        for (part, k) in &f.parts { back = back.mul(&part.pow(*k).unwrap()).unwrap(); }
        assert_eq!(back, pq, "round {round}: factor does not multiply back");
        // (p·q)/q is p.
        if !q.is_zero() {
            let r = RatF::poly(pq.clone()).div(&RatF::poly(q.clone())).unwrap();
            assert_eq!(r, RatF::poly(p.clone()), "round {round}: (p q)/q is not p");
        }
    }
}

#[test]
fn fractions_shown_exactly() {
    let shown = |lines: &[&str]| { let mut s = sheet(lines); s.exact = true; eval::run(&s).regions.last().unwrap().result.clone() };
    assert_eq!(shown(&["x = 1/3"]), "1/3");
    assert_eq!(shown(&["0.1 + 0.2"]), "3/10", "the decimal round-off of 3/10 is still 3/10");
    let mut s = sheet(&["x = 7/12*1 m"]);
    (s.exact, s.regions[0].unit) = (true, "m".into());
    assert_eq!(eval::run(&s).regions[0].result, "(7/12) m", "in the unit chosen, bracketed before it");
    assert_eq!(shown(&["z = 274863"]), "274\u{2009}863", "whole numbers in full, grouped with a thin space");
    assert_eq!(shown(&["sqrt(2)"]), "1.414", "not a fraction: decimals as before");
    assert_eq!(shown(&["pi"]), "3.142");
    assert_eq!(shown(&["w = 2/3 + 1j/4"]), "2/3 + (1/4)j");
    assert_eq!(shown(&["check 1/3 <= 1/2"]), "PASS (1/3 ≤ 1/2)");
    assert_eq!(result(&["x = 1/3"]), "0.3333", "off by default: engineers keep decimals");
    assert_eq!(crate::sym::fraction(0.333_333_333_333, 10_000), None, "a decimal typed short of 1/3 is not 1/3");
}

#[test]
fn exact_algebra_stays_quick() {
    // The worst cases a sheet can write: a constant with very many divisors, a large power, many symbols.
    let t = std::time::Instant::now();
    let lines = ["factor(963761198400*x^2 - 963761198400)", "expand((x+1)^64)", "expand((x+y+z)^12)", "factor(x^24 - 1)", "simplify((x^20 - 1)/(x^5 - 1))", "expand((a+b+c+d)^9)"];
    for l in lines { let o = run(&[l]); assert!(o.regions[0].error.is_none() || o.regions[0].error.as_ref().unwrap().msg.contains("too"), "{l}: {:?}", o.regions[0].error); }
    let limit = if cfg!(debug_assertions) { 10.0 } else { 1.0 };
    assert!(t.elapsed().as_secs_f64() < limit, "took {:?}", t.elapsed());
    assert!(result(&["factor(x^24 - 1)"]).matches("(").count() >= 2);
}

#[test]
fn exact_algebra_review_of_0_2_0() {
    // Inputs that filled memory or wrapped the exponent: refused at once (the limit is a power of 1000).
    let t = std::time::Instant::now();
    for line in ["simplify((((((x^64)^64)^64)^64)^64)/(x+1))", "expand(((((((x^64)^64)^64)^64)^64)^64))", "expand(((((((x^64)^64)^64)^64)^64)^64)*x)", "expand((x+1)^1000)", "factor(10^30*x + 1/10^9)"] {
        let e = error(&[line]);
        assert!(e.contains("too large"), "{line}: {e}");
    }
    assert!(t.elapsed().as_secs_f64() < if cfg!(debug_assertions) { 10.0 } else { 1.0 }, "took {:?}", t.elapsed());
    assert_eq!(result(&["expand(x^1000)"]), "x¹⁰⁰⁰");
    assert_eq!(error(&["expand(1e20*x)"]), "Numbers above 10¹⁵ are too large for exact maths.");
    // Text that reads as meant: brackets around a sum or a product on either side of /, whole-number constants.
    for (line, want) in [("factor((x+1)/(x-1))", "(x + 1)/(x − 1)"), ("factor(x*y/(x+y))", "xy/(x + y)"), ("factor(6/(4*x^2-4))", "3/(2(x + 1)(x − 1))"),
        ("factor(x/2)", "(1/2)x"), ("factor((x+1)/(2*x-2))", "(x + 1)/(2(x − 1))"), ("simplify((x+1)/(x*y))", "(x + 1)/(xy)"), ("simplify(1/(2*x))", "1/(2x)")] {
        assert_eq!(result(&[line]), want, "{line}");
    }
    // A name redefined as a value drops its expression; names the sheet would refuse are refused, not symbols.
    assert_eq!(result(&["y = expand((x+1)^2)", "y = 3", "expand(y)"]), "3");
    assert!(error(&["expand((a+1)^2)", "a = 2"][..1]).is_empty() && run(&["expand((a+1)^2)", "a = 2"]).regions[0].error.as_ref().unwrap().msg.contains("defined further down"));
    assert!(error(&["b = 1/0", "expand((b+1)^2)"]).contains("has an error on its own line"));
    assert!(error(&["f(t) = t^2", "expand(f + 1)"]).contains("f is a function"));
    assert!(error(&["expand(sin)"]).contains("sin is a function"));
    // A fraction before a unit is bracketed in text.
    let mut s = sheet(&["q = 2/3*1 m"]);
    (s.exact, s.regions[0].unit) = (true, "m".into());
    assert_eq!(eval::run(&s).regions[0].result, "(2/3) m");
}

mod materials {
    use super::close;
    use crate::eval;
    use crate::sheet::{Kind, Region, Sheet};

    fn with_material(data: serde_json::Value, lines: &[&str]) -> Sheet {
        let mut s = Sheet::new("m");
        s.regions.push(Region { id: 1, kind: Kind::Material, data, ..Default::default() });
        for (i, l) in lines.iter().enumerate() { s.regions.push(Region { id: i as u32 + 2, kind: if l.starts_with("check") { Kind::Check } else { Kind::Math }, text: l.to_string(), ..Default::default() }); }
        s
    }

    #[test]
    fn the_library_loads_and_every_unit_reads() {
        let lib = crate::materials::library();
        assert_eq!(lib.materials.len(), 252, "{}", lib.from);
        assert!(lib.from.starts_with("Datum Atlas"));
        for (_, _, _, unit) in crate::materials::KEYS { assert!(unit.is_empty() || crate::units::parse(unit).is_ok(), "{unit}"); }
        for m in &lib.materials { for (_, _, src) in m.props.values() { assert!(lib.sources.contains_key(src), "{}: {src}", m.id); } }
    }

    #[test]
    fn s355_defines_its_properties_with_units() {
        // Atlas S355: ρ 7850–7870 kg/m³, E 200–210 GPa, σy 345–375 MPa, α 11.5–12.5 µm/(m·K), Tmax 350–400 °C, ρe 16–18 µΩ·cm.
        let s = with_material(serde_json::json!({ "id": "steel-s355", "name": "steel" }), &["check 200 MPa <= sigma_y_steel/1.1", "m = rho_steel*1 m^3"]);
        let (o, e) = eval::run_env(&s, &eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: false });
        let v = |n: &str| e.vars.get(&eval::canon(n)).unwrap_or_else(|| panic!("{n}")).x();
        assert!(close(v("rho_steel"), 7860.0, 1e-12) && close(v("E_steel"), 205e9, 1e-12) && close(v("sigma_y_steel"), 360e6, 1e-12));
        assert!(close(v("alpha_steel"), 12e-6, 1e-12) && close(v("T_max_steel"), 375.0 + 273.15, 1e-12) && close(v("rho_e_steel"), 17e-8, 1e-12));
        assert!(e.vars.get(&eval::canon("T_max_steel")).unwrap().abs, "an absolute temperature");
        let m = o.regions[0].material.as_ref().unwrap();
        assert_eq!(m.title, "Structural steel, EN S355");
        assert!(m.draft && m.rows.len() >= 10 && m.sources.len() == 1);
        let row = |w: &str| m.rows.iter().find(|r| r.what == w).unwrap();
        assert_eq!((row("yield strength").value.as_str(), row("yield strength").unit.as_str(), row("yield strength").range.as_str()), ("360", "MPa", "345 – 375"));
        assert_eq!(row("thermal expansion").unit, "µm/(m·K)");
        assert!(o.regions[1].check.as_ref().unwrap().pass);
        assert_eq!(o.regions[2].result, "7.86 t");
        assert!(o.vars.iter().any(|v| v.name == eval::canon("sigma_y_steel") && v.value == "360" && v.unit == "MPa"), "listed in Variables");
        // Low and high ends.
        let low = |pk: &str| eval::run_env(&with_material(serde_json::json!({ "id": "steel-s355", "name": "s", "pick": pk }), &[]), &eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: false }).1.vars[&eval::canon("sigma_y_s")].x();
        assert!(close(low("low"), 345e6, 1e-12) && close(low("high"), 375e6, 1e-12));
    }

    #[test]
    fn a_sheet_keeps_its_values() {
        // A kept material wins over the library: the sheet reads as it did when the material was picked.
        let kept = serde_json::json!({ "id": "steel-s355", "name": "steel", "mat": { "id": "steel-s355", "name": "Structural steel, EN S355", "family": "Metals", "status": "draft", "props": { "E": [100, 100, "atlas-typ"] } }, "sources": { "atlas-typ": "Kept title" } });
        let o = eval::run(&with_material(kept, &["E_steel ="]));
        assert_eq!(o.regions[1].result, "100 GPa");
        assert_eq!(o.regions[0].material.as_ref().unwrap().sources, ["Kept title"]);
        // A material this version does not have, but the sheet kept, still works.
        let gone = serde_json::json!({ "id": "no-such", "name": "x", "mat": { "id": "no-such", "name": "Old alloy", "props": { "rho": [2700, 2700, "src"] } } });
        assert_eq!(eval::run(&with_material(gone, &["rho_x ="])).regions[1].result, "2700 kg/m³");
    }

    #[test]
    fn what_a_material_region_says_when_it_cannot() {
        let err = |d: serde_json::Value| eval::run(&with_material(d, &[])).regions[0].error.as_ref().map(|e| e.msg.clone()).unwrap_or_default();
        assert_eq!(err(serde_json::json!({})), "Choose a material from the list.");
        assert!(err(serde_json::json!({ "id": "no-such", "name": "x" })).contains("is not in this version's materials"));
        assert!(err(serde_json::json!({ "id": "steel-s355", "name": "" })).starts_with("Give the material a short name"));
        assert!(err(serde_json::json!({ "id": "steel-s355", "name": "2bad" })).starts_with("Give the material a short name"));
        // Used above the region: defined further down.
        let mut s = with_material(serde_json::json!({ "id": "steel-s355", "name": "steel" }), &[]);
        s.regions.insert(0, Region { id: 9, kind: Kind::Math, text: "E_steel =".into(), ..Default::default() });
        assert!(eval::run(&s).regions[0].error.as_ref().unwrap().msg.contains("defined further down"));
    }

    #[test]
    fn review_of_0_2_1() {
        // A second material with the same short name warns: its values replace the first one's from here.
        let mut s = with_material(serde_json::json!({ "id": "steel-s355", "name": "steel" }), &[]);
        s.regions.push(Region { id: 7, kind: Kind::Material, data: serde_json::json!({ "id": "ss-304", "name": "steel" }), ..Default::default() });
        let o = eval::run(&s);
        assert!(o.regions[0].warn.is_empty() && o.regions[1].warn.starts_with("Names ending _steel are already defined above"), "{:?}", o.regions[1].warn);
        // Elongation keeps its % on a US sheet; hardness shows HV.
        let mut s = with_material(serde_json::json!({ "id": "steel-s355", "name": "steel" }), &[]);
        s.units = "US".into();
        let o = eval::run(&s);
        let row = o.regions[0].material.as_ref().unwrap().rows.iter().find(|r| r.what == "elongation at fracture").unwrap().clone();
        assert_eq!((row.value.as_str(), row.unit.as_str()), ("21", "%"));
        let hv = crate::materials::library().materials.iter().find(|m| m.props.contains_key("hv")).unwrap();
        let o = eval::run(&with_material(serde_json::json!({ "id": hv.id, "name": "h" }), &[]));
        assert_eq!(o.regions[0].material.as_ref().unwrap().rows.iter().find(|r| r.what == "Vickers hardness").unwrap().unit, "HV");
    }

    #[test]
    fn every_material_works_out_in_si_and_us() {
        for m in &crate::materials::library().materials {
            for us in [false, true] {
                let mut s = with_material(serde_json::json!({ "id": m.id, "name": "m" }), &[]);
                if us { s.units = "US".into(); }
                let o = eval::run(&s);
                assert!(o.regions[0].error.is_none(), "{}: {:?}", m.id, o.regions[0].error);
                let mo = o.regions[0].material.as_ref().unwrap();
                assert!(!mo.rows.is_empty() && mo.rows.iter().all(|r| !r.value.is_empty() && !r.value.contains("NaN") && r.value != "—"), "{}: {:?}", m.id, mo.rows);
            }
        }
    }
}

#[test]
fn several_equations_solved_together() {
    // x + y = 3, x − y = 1 → (2, 1); x² + y² = 25 with y = x + 1 → (3, 4) from (1, 1); beam reactions R_A + R_B = F,
    // 4 R_B = 1 F → 750 N, 250 N; series resonance 1/(2π √(LC)) = 1 s⁻¹·… with L = 2 mH, C = 100 µF → 355.88 Hz.
    assert_eq!(result(&["solve x + y = 3, x - y = 1 for x = 0, y = 0"]), "x = 2, y = 1");
    assert_eq!(result(&["solve x^2 + y^2 = 25, y = x + 1 for x = 1, y = 1"]), "x = 3, y = 4");
    // Unknowns of very different sizes (GPa beside a strain, MPa beside mm²), guesses far off, and an equation that
    // is exactly zero at the guess with small quantities (y = x³ ≈ 8×10⁻²⁷, not held to one SI unit).
    assert_eq!(result(&["solve E_s*eps = 200 MPa, eps = 0.001 for E_s = 100 GPa, eps = 0.002"]), "E_s = 200 GPa, eps = 0.001");
    assert_eq!(result(&["solve sigma = F/A_s, F = 2 kN, A_s = 10 mm^2 for sigma = 1 MPa, F = 1 N, A_s = 1 mm^2"]), "σ = 200 MPa, F = 2 kN, A_s = 10 mm²");
    assert_eq!(result(&["solve x^3 - y = 0, x + y = 2e-9 for x = 1e-3, y = 1e-9"]), "x = 2×10⁻⁹, y = 8×10⁻²⁷");
    assert_eq!(result(&["F = 1000 N", "solve R_A + R_B = F, R_B*4 m = F*1 m for R_A = 0 N, R_B = 0 N"]), "R_A = 750 N, R_B = 250 N");
    assert_eq!(result(&["C = 100 µF", "solve 2*pi*f_r*sqrt(L_r*C) = 1, L_r = 2 mH for f_r = 50 Hz, L_r = 1 mH", "f_r ="]), "355.9 Hz");
    assert_eq!(result(&["solve exp(t) = 5 for t = 0"]), "t = 1.609", "one equation from a guess");
    assert_eq!(result(&["solve x + y = 3, x - y = 1 for x = 0, y = 0", "x*y ="]), "2", "the unknowns are defined below");
    let o = run(&["solve x + y = 3, x - y = 1 for x = 0, y = 0"]);
    assert!(o.vars.iter().any(|v| v.name == "x") && o.vars.iter().any(|v| v.name == "y"), "both in the Variables panel");
    assert!(o.regions[0].latex.contains("\\begin{cases}") && o.regions[0].result_tex.starts_with("\\Rightarrow"));
    // What it says when it cannot.
    for (lines, msg) in [
        (&["solve x + y = 1 for x = 0, y = 0"][..], "1 equation for 2 unknowns"),
        (&["solve x + y = 1, 2*x + 2*y = 2 for x = 0, y = 0"][..], "The equations do not fix every unknown"),
        (&["solve x^2 + 1 = 0 for x = 1"][..], "No solution found from these guesses"),
        (&["solve a + b = 1, a - b = 2 m for a = 0, b = 0"][..], "Cannot compare a pure number and a length"),
        (&["solve x = 1, y = 2 for x in 0 .. 1"][..], "Several equations need a starting guess"),
        (&["solve x = 1j for x = 0"][..], "solve needs a real value"),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
    // A failed solve leaves earlier values as they were; its unknowns are marked as errors further down.
    assert!(error(&["solve x^2 + 1 = 0 for x = 1", "x + 1"]).contains("has an error on its own line"));
    assert_eq!(result(&["x = 7", "solve x^2 + 1 = 0 for x = 1", "x + 1 ="]), "8");
}

#[test]
fn vectors_and_matrices() {
    // A = [[2, 1], [1, 3]]: det 5, inverse [[0.6, −0.2], [−0.2, 0.4]], A x = [3, 5] → x = [0.8, 1.4]; A² = [[5, 5], [5, 10]].
    let a = "A = [[2, 1], [1, 3]]";
    assert_eq!(result(&[a, "det(A)"]), "5");
    assert_eq!(result(&[a, "inv(A)"]), "[[0.6, -0.2], [-0.2, 0.4]]");
    assert_eq!(result(&[a, "A^-1"]), "[[0.6, -0.2], [-0.2, 0.4]]");
    assert_eq!(result(&[a, "lsolve(A, [3, 5])"]), "[0.8, 1.4]");
    assert_eq!(result(&[a, "A*lsolve(A, [3, 5])"]), "[3, 5]", "multiplies back");
    assert_eq!(result(&[a, "A^2"]), "[[5, 5], [5, 10]]");
    assert_eq!(result(&[a, "A*[[1, 2, 3], [4, 5, 6]]"]), "[[6, 9, 12], [13, 17, 21]]");
    assert_eq!(result(&[a, "transpose([[1, 2, 3], [4, 5, 6]])"]), "[[1, 4], [2, 5], [3, 6]]");
    assert_eq!(result(&[a, "A[1, 2] + A[2, 2]"]), "4");
    assert_eq!(result(&["v = [3, 4] m", "norm(v)"]), "5 m");
    assert_eq!(result(&["cross([1, 0, 0], [0, 1, 0])"]), "[0, 0, 1]");
    assert_eq!(result(&["dot([1, 2, 3], [4, 5, 6])"]), "32");
    assert_eq!(result(&["identity(2)*3"]), "[[3, 0], [0, 3]]");
    // Units follow the algebra: two springs, K u = F → u = [6.667, 3.333] m; det K in N²/m².
    let k = ["K = [[2, -1], [-1, 2]] kN/m", "F = [10, 0] kN"];
    assert_eq!(result(&[k[0], k[1], "u = lsolve(K, F)"]), "[6.667, 3.333] m");
    assert_eq!(result(&[k[0], "det(K)"]), "3×10⁶ N²/m²");
    let o = run(&[k[0], k[1], "u = lsolve(K, F)", "u_2 = u[2]"]);
    assert_eq!(o.regions[3].result, "3.333 m");
    assert!(o.regions[0].result_tex.contains("\\begin{bmatrix}") && o.regions[0].latex.contains("\\begin{bmatrix}"));
    // What it says when it cannot.
    for (lines, msg) in [
        (&[a, "A + [1, 2]"][..], "These have different shapes (2×2 and 2×1)."),
        (&["[[1, 2], [3]]"][..], "Every row of a matrix needs the same number of values."),
        (&[a, "A[3, 1]"][..], "Out of range: this has 2 rows and 2 columns."),
        (&["[1 m, 2 s]"][..], "Every entry needs the same units"),
        (&["inv([[1, 2], [2, 4]])"][..], "This matrix has no inverse (it is singular)."),
        (&[a, "A*[[1, 2, 3]]"][..], "A 2×2 times a 1×3"),
        (&[a, "A/A"][..], "To divide by a matrix, multiply by its inverse"),
        (&["det([[1, 2, 3], [4, 5, 6]])"][..], "det needs a square matrix."),
        (&[a, "A[1]"][..], "A matrix entry needs a row and a column"),
        (&["[[20 °C, 30 °C]]"][..], "A matrix cannot hold absolute temperatures"),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}

#[test]
fn matrices_review() {
    // Limits: an outer product of a long range is refused rather than filling memory; units of det stay in range.
    assert!(error(&["v = 1, 2 .. 100000", "W = v*transpose(v)"]).starts_with("A 100000×100000 result: a matrix holds at most 10,000 values."));
    assert!(error(&["det(identity(50)*1 W)"]).starts_with("The determinant of this 50 × 50 matrix"));
    assert_eq!(result(&["det(identity(5)*1 kN/m)"]), "1×10¹⁵ kg⁵/s¹⁰");
    assert!(run(&["det(identity(5)*1 kN/m)"]).regions[0].result_tex.contains("s^{10}"), "two-digit powers draw as one superscript");
    // Singular means singular, judged row by row: det 0; a badly scaled but regular system solves.
    assert_eq!(result(&["det([[1, 2, 3], [4, 5, 6], [7, 8, 9]])"]), "0");
    assert_eq!(result(&["lsolve([[1, 1], [1e-14, 2e-14]], [2, 3e-14])"]), "[1, 1]");
    assert_eq!(result(&["det([[2, -1], [-1, 2]] kN/m)"]), "3×10⁶ N²/m²");
    // A number is a 1 × 1 matrix; norm does not overflow.
    assert_eq!(result(&["det([[2]] m)"]), "2 m");
    assert_eq!(result(&["lsolve(2 m, 4 m^2)"]), "2 m");
    assert_eq!(result(&["norm([1e200, 1e200])"]), "141.4×10¹⁹⁸");
    for (lines, msg) in [
        (&["solve [[1, 2], [3, 4]]*[x, 1] = [3, 4] for x in -10 .. 10"][..], "solve needs an equation between single values"),
        (&["M = [[1, 2], [3, 4]] K", "M + 20 °C"][..], "A matrix cannot hold absolute temperatures"),
        (&["T = 20 °C, 30 °C .. 40 °C", "norm(T)"][..], "A matrix cannot hold absolute temperatures"),
        (&["T = 20 °C", "T*identity(2)"][..], "A matrix cannot hold absolute temperatures"),
        (&["x = 3", "x[1]"][..], "Only a vector or a matrix has entries."),
        (&["[1, 2][1 + 1j]"][..], "An index is a whole number from 1."),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}

#[test]
fn programs() {
    // A function as a program: laminar 64/Re, else the Colebrook equation by fixed-point iteration. At Re = 10⁵ and
    // k/D = 0.001 the friction factor is 0.0222 (Moody chart); at Re = 1000 it is 64/1000 = 0.064.
    let fd = "f_D(Re, k) =\n  if Re < 2300\n    return 64/Re\n  end\n  f = 0.02\n  for i in 1 .. 50\n    f = (-2*log(k/3.7 + 2.51/(Re*sqrt(f))))^-2\n  end\n  return f";
    assert_eq!(result(&[fd, "f_D(1000, 0.001)"]), "0.064");
    assert_eq!(result(&[fd, "f_D(1e5, 0.001)"]), "0.02217");
    // Run once: every name it sets is defined below, with units; the loop's own name is not.
    let p = "total = 0 kN\nF = [10, 20, 30] kN\nfor x in F\n  total = total + x\nend\nn = 0\nwhile 2^n < 1000\n  n = n + 1\nend";
    assert_eq!(result(&[p]), "total = 60 kN, F = [10, 20, 30] kN, n = 10");
    assert_eq!(result(&[p, "total*2 + n*1 kN"]), "130 kN");
    assert!(error(&[p, "x"]).contains("not defined") || result(&[p, "x"]).starts_with("ERR"), "the loop name stays local");
    assert_eq!(result(&["L = 6 m\nif L > 5 m\n  c = 1\nelse if L > 2 m\n  c = 2\nelse\n  c = 3\nend"]), "L = 6 m, c = 1");
    assert_eq!(result(&["L = 3 m\nif L > 5 m\n  c = 1\nelse if L > 2 m\n  c = 2\nelse\n  c = 3\nend"]), "L = 3 m, c = 2");
    assert_eq!(result(&["s = 0\nfor i in 1, 3 .. 9\n  s = s + i\nend"]), "s = 25");
    assert_eq!(result(&["s = 0\nfor i in 1 .. 10\n  if i > 4\n    break\n  end\n  s = s + i\nend"]), "s = 10");
    assert_eq!(result(&["h(n) =\n  if n <= 1\n    return 1\n  end\n  return n*h(n - 1)", "h(5)"]), "120");
    // Names that read as units (m, s) are the program's own names.
    assert_eq!(result(&["m = 2\ns = m*3 kg"]), "m = 2, s = 6 kg");
    // Drawn as maths, keywords in bold, one line each.
    let o = run(&[fd]);
    assert!(o.regions[0].latex.starts_with("\\begin{array}{l} f_{D}\\left(\\mathrm{Re}, k\\right) =") && o.regions[0].latex.contains("\\textbf{for}"), "{}", o.regions[0].latex);
    // What it says, and where.
    for (lines, msg, at) in [
        (&["x = 1\nwhile x > 0\n  x = x + 1\nend"][..], "The loops went round more than 100,000 times", (6, 17)),
        (&["if 1\n  y = 2"][..], "This block needs an end below it.", (0, 4)),
        (&["a = 1\nreturn a"][..], "return is for a program that is a function", (6, 14)),
        (&["z = 5 m\nq = z + 2 s"][..], "Cannot add a length and a time", (12, 19)),
        (&["a = 1\nend"][..], "This end has no if, for or while to close.", (6, 9)),
        (&["a = 1\nelse"][..], "This else has no if above it.", (6, 10)),
        (&["a = 1\nbreak"][..], "break is for leaving a for or while loop.", (6, 11)),
        (&["a = 1\nplot a for a in 0 .. 1"][..], "A program line sets a name", (6, 28)),
    ] {
        let o = run(lines);
        let e = o.regions.last().unwrap().error.clone().unwrap_or_else(|| panic!("{lines:?}"));
        assert!(e.msg.starts_with(msg) && (e.from, e.to) == at, "{lines:?}: {} {:?}", e.msg, (e.from, e.to));
    }
    assert!(error(&["g(a) =\n  b = a*2", "g(3)"]).starts_with("g ended without return"));
    assert!(error(&["g(a) =\n  return a + 1 m", "g(3)"]).starts_with("In g: Cannot add"));
    assert!(error(&["sqrt(a) =\n  return a"]).starts_with("sqrt is a built-in function"));
}

#[test]
fn programs_review() {
    // The rounds of every program on a line count together, so a plot or solve that calls a slow function stops
    // quickly and says so, rather than running for minutes or drawing half a curve.
    let f = "f(x) =\n  i = 0\n  while i < 50000\n    i = i + 1\n  end\n  return i + x";
    let t = std::time::Instant::now();
    assert!(error(&[f, "plot f(x) for x in 0 .. 100"]).starts_with("The programs on this line went round more than 1,000,000 times"));
    assert!(error(&[f, "solve f(x) = 50010 for x in 0 .. 100"]).contains("went round more than 1,000,000 times"));
    assert!(error(&["f(n) =\n  i = 0\n  while i < 99000\n    i = i + 1\n  end\n  if n <= 0\n    return 0\n  end\n  return f(n - 1)", "z = f(39)"]).contains("1,000,000"));
    assert!(t.elapsed().as_secs_f64() < if cfg!(debug_assertions) { 60.0 } else { 6.0 }, "{:?}", t.elapsed());
    assert_eq!(result(&[f, "z = f(3)"]), "50\u{2009}000", "a single call is well within the budget");
    // A line f(x) = … below a program function replaces it.
    assert_eq!(result(&["f(x) =\n  return x + 1", "f(x) = x^2", "z = f(3)"]), "9");
    // A function's names are its own: a sheet expression or a primed name of the same spelling stays as it was.
    assert_eq!(result(&["q = expand((x+1)^2)", "g(x) =\n  q = x\n  return q", "z = g(3)", "factor(q)"]), "(x + 1)²");
    assert_eq!(result(&["x' = 7", "f(a) =\n  x' = a\n  return x'", "z = f(1)", "w = x'"]), "7");
    assert_eq!(result(&["x' = 3\ny = x' + 1"]), "x' = 3, y = 4");
    // A program that fails part way changes nothing on the sheet.
    assert!(error(&["b = 10", "a = 1\nb = b + 1\nc = qq", "y = a + b"]).starts_with("a has an error on its own line."));
    assert_eq!(result(&["b = 10", "a = 1\nb = b + 1\nc = qq", "y2 = b"]), "10");
    // A name set only on a path that did not run says so.
    assert!(error(&["a = 1\nif a > 5\n  b = 1\nend", "z = b"]).starts_with("b is set by the program above only on a path it did not take"));
    // Tabs and non-breaking spaces after keywords; error positions in characters.
    assert_eq!(result(&["s = 0\nfor\tk in\t1 .. 3\n  s = s + k\nend"]), "s = 6");
    let o = run(&["a = 1\nif a > 5\n  b = 1\nelse\u{a0}if σ < qq\n  b = 2\nend"]);
    let e = o.regions[0].error.clone().unwrap();
    assert!(e.msg.starts_with("σ is not defined") && (e.from, e.to) == (31, 32), "{e:?}");
}

#[test]
fn iapws_if97_verification_tables() {
    use crate::fluids;
    // The computer-program verification values printed in IAPWS R7-97(2012): tables 5 (region 1), 15 (region 2),
    // 33 (region 3, from density), 35 and 36 (saturation), 42 (region 5). v in m³/kg, h in kJ/kg, s and cp in kJ/(kg·K).
    let rel = |a: f64, b: f64| ((a - b) / b).abs() < 6e-9;
    for (t, p, v, h, s, cp, w) in [
        (300.0, 3e6, 0.100215168e-2, 0.115331273e3, 0.392294792, 0.417301218e1, 0.150773921e4),
        (300.0, 80e6, 0.971180894e-3, 0.184142828e3, 0.368563852, 0.401008987e1, 0.163469054e4),
        (500.0, 3e6, 0.120241800e-2, 0.975542239e3, 0.258041912e1, 0.465580682e1, 0.124071337e4),
        (300.0, 0.0035e6, 0.394913866e2, 0.254991145e4, 0.852238967e1, 0.191300162e1, 0.427920172e3),
        (700.0, 0.0035e6, 0.923015898e2, 0.333568375e4, 0.101749996e2, 0.208141274e1, 0.644289068e3),
        (700.0, 30e6, 0.542946619e-2, 0.263149474e4, 0.517540298e1, 0.103505092e2, 0.480386523e3),
        (1500.0, 0.5e6, 0.138455090e1, 0.521976855e4, 0.965408875e1, 0.261609445e1, 0.917068690e3),
        (1500.0, 30e6, 0.230761299e-1, 0.516723514e4, 0.772970133e1, 0.272724317e1, 0.928548002e3),
        (2000.0, 30e6, 0.311385219e-1, 0.657122604e4, 0.853640523e1, 0.288569882e1, 0.106736948e4),
    ] {
        let st = fluids::tp(t, p).unwrap();
        assert!(rel(1.0 / st.rho, v) && rel(st.h / 1e3, h) && rel(st.s / 1e3, s) && rel(st.cp / 1e3, cp) && rel(st.w, w), "{t} K {p} Pa: {st:?}");
    }
    // Region 3 from (T, p) must land on the tabled density (table 33 gives p from ρ = 500 and 200 kg/m³).
    for (rho, t, p, h, s) in [(500.0, 650.0, 0.255837018e2, 0.186343019e4, 0.405427273e1), (200.0, 650.0, 0.222930643e2, 0.237512401e4, 0.485438792e1), (500.0, 750.0, 0.783095639e2, 0.225868845e4, 0.446971906e1)] {
        let st = fluids::tp(t, p * 1e6).unwrap();
        assert!(((st.rho - rho) / rho).abs() < 1e-7 && ((st.h / 1e3 - h) / h).abs() < 1e-7 && ((st.s / 1e3 - s) / s).abs() < 1e-7, "{st:?}");
    }
    for (t, p) in [(300.0, 0.353658941e-2), (500.0, 0.263889776e1), (600.0, 0.123443146e2)] { assert!(rel(fluids::psat(t).unwrap() / 1e6, p)); }
    for (p, t) in [(0.1, 0.372755919e3), (1.0, 0.453035632e3), (10.0, 0.584149488e3)] { assert!(rel(fluids::tsat(p * 1e6).unwrap(), t)); }
    // Viscosity (IAPWS R12-08, table 4, µPa·s, at the tabled densities).
    for (t, rho, mu) in [(298.15, 998.0, 889.735100), (298.15, 1200.0, 1437.649467), (373.15, 1000.0, 307.883622), (433.15, 1.0, 14.538324), (433.15, 1000.0, 217.685358), (873.15, 1.0, 32.619287), (873.15, 100.0, 35.802262), (873.15, 600.0, 77.430195), (1173.15, 1.0, 44.217245), (1173.15, 100.0, 47.640433), (1173.15, 400.0, 64.154608)] {
        assert!((fluids::mu(rho, t) * 1e6 - mu).abs() < 5e-7, "{t} {rho}: {}", fluids::mu(rho, t) * 1e6);
    }
    // Out of range says so; (p, h) and (p, s) give back the temperature.
    assert!(fluids::tp(250.0, 1e5).is_err() && fluids::tp(1200.0, 60e6).is_err() && fluids::psat(700.0).is_err());
    for (t, p) in [(300.0, 1e5), (450.0, 1e6), (700.0, 30e6), (650.0, 25e6), (1500.0, 1e6)] {
        let st = fluids::tp(t, p).unwrap();
        assert!((fluids::p_and(p, st.h, 0).unwrap().t - t).abs() < 1e-7 && (fluids::p_and(p, st.s, 1).unwrap().t - t).abs() < 1e-7);
    }
}

#[test]
fn water_and_gas_properties_on_a_sheet() {
    // Steam tables: 40 bar, 400 °C: h = 3214 kJ/kg, s = 6.771 kJ/(kg·K); 1 atm boils at 99.97 °C; water at 20 °C,
    // 1 bar: ρ = 998.2 kg/m³, μ = 1.002 mPa·s, k = 0.598 W/(m·K), Pr = 7.01, σ = 72.74 mN/m.
    assert_eq!(result(&["water_h(400 °C, 40 bar)"]), "3214 kJ/kg");
    assert_eq!(result(&["water_s(40 bar, 400 °C)"]), "6.771 kJ/(kg·K)", "either order");
    assert_eq!(result(&["water_Tsat(1 atm)"]), "99.97 °C");
    assert_eq!(result(&["water_rho(20 °C, 1 bar)"]), "998.2 kg/m³");
    assert_eq!(result(&["water_k(20 °C, 1 bar)"]), "0.598 W/(m·K)");
    assert_eq!(result(&["water_Pr(20 °C, 1 bar)"]), "7.009");
    assert_eq!(result(&["water_sigma(20 °C)"]), "0.07274 N/m");
    // Wet steam from a pressure and an enthalpy; a vector of temperatures gives a vector (100 °C at 1 bar is steam).
    assert_eq!(result(&["water_x(0.1 bar, 2144 kJ/kg)"]), "0.8161");
    assert_eq!(result(&["T = 20 °C, 40 °C .. 100 °C", "water_rho(T, 1 bar)"]), "[998.2, 992.2, 983.2, 971.8, 0.5896] kg/m³");
    // Gases (ideal gas; air 20 °C, 1 atm: 1.204 kg/m³; γ = 1.40; Pr = 0.706).
    assert_eq!(result(&["gas_rho(air, 20 °C, 1 atm)"]), "1.204 kg/m³");
    assert_eq!(result(&["gas_gamma(air, 300 K)"]), "1.4");
    assert_eq!(result(&["gas_Pr(air, 300 K)"]), "0.7063");
    assert_eq!(result(&["gas_M(CH4)"]), "0.01604 kg/mol");
    assert!(run(&["gas_cp(CO2, 500 K)"]).regions[0].latex.contains("c_{p,\\mathrm{CO_2}}"));
    for (lines, msg) in [
        (&["water_h(180 °C, 10)"][..], "A value without a unit is read as a vapour fraction x"),
        (&["water_cp(0.1 bar, 0.5)"][..], "cp is not defined for wet steam"),
        (&["water_h(1 m, 2 s)"][..], "water_h takes two of: a temperature, a pressure"),
        (&["water_h(-10 °C, 1 bar)"][..], "Water and steam properties are given from 0 °C"),
        (&["gas_cp(steam, 300 K)"][..], "gas_cp needs a gas first: air, N2, O2, Ar, CO2, H2, He, CH4."),
        (&["gas_mu(CH4, 800 K)"][..], "Methane: viscosity and conductivity are given from 200 K to 625 K"),
        (&["gas_rho(air, 300 K)"][..], "gas_rho takes the gas, a temperature and a pressure."),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}

#[test]
fn properties_review() {
    // Within millikelvins of the critical point the liquid and vapour branches meet at about ρc = 322 kg/m³.
    assert_eq!(result(&["water_T(220.638 bar, 2000 kJ/kg)"]), "373.9 °C");
    assert_eq!(result(&["water_rho(373.945 °C, 0.5)"]), "322.1 kg/m³");
    assert_eq!(result(&["water_h(220.639 bar, 0)"]), "2084 kJ/kg");
    // The ends of the range give themselves back.
    assert_eq!(result(&["water_T(10 bar, water_h(2000 °C, 10 bar))"]), "2000 °C");
    assert_eq!(result(&["water_T(10 bar, water_s(0 °C, 10 bar))"]), "0 °C");
    // Nitrogen and air below 300 K from coefficients valid there: cp(N₂, 200 K) = 1.040 kJ/(kg·K).
    assert_eq!(result(&["gas_cp(N2, 200 K)"]), "1.04 kJ/(kg·K)");
    // A sheet's own gas_flow(q) is drawn and worked out as any function; the gas is drawn as the library writes it.
    let o = run(&["gas_flow(q) = 2*q", "y = gas_flow(3)", "gas_s(co2, 300 K, 1 bar)"]);
    assert!(o.regions[1].result == "6" && o.regions[1].latex.contains("\\left(3\\right)") && o.regions[2].latex.contains("s_{\\mathrm{CO_2}}"), "{}", o.regions[1].latex);
    for (lines, msg) in [
        (&["water_x(500 Pa, 50 °C)"][..], "No vapour fraction below 611.2 Pa"),
        (&["water_Tsat(611.2 Pa)"][..], "Saturation is from 611.213 Pa"),
        (&["gas_rho(air, -300 °C, 1 bar)"][..], "An ideal gas needs an absolute temperature and a pressure above zero."),
        (&["water_h(1 bar, 0.5 rad)"][..], "water_h takes two of"),
        (&["water_cp(373.946 °C, 220.64 bar)"][..], "cp goes to infinity at the critical point"),
        (&["gas_mu(CH4, 100 K)"][..], "Methane: viscosity and conductivity are given from 200 K to 625 K (−73 °C"),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}

#[test]
fn data_into_tables() {
    let parse = |t: &str| crate::tabin::parse(t, false);
    let t = |v: &serde_json::Value| (v["cols"].as_array().unwrap().iter().map(|c| format!("{}|{}", c["name"].as_str().unwrap(), c["unit"].as_str().unwrap())).collect::<Vec<_>>(), v["rows"].clone(), v["notes"].as_array().unwrap().len());
    // Copied from a spreadsheet: tabs, a heading with units in brackets (m3 is m^3).
    let (c, r, n) = t(&parse("Q (m3/h)\tH [m]\n0\t50.3\n40\t46.5\n80\t37.4\n").unwrap());
    assert_eq!(c, ["Q|m^3/h", "H|m"]);
    assert_eq!(r, serde_json::json!([["0", "50.3"], ["40", "46.5"], ["80", "37.4"]]));
    assert_eq!(n, 0);
    // A European CSV: semicolons and a decimal comma; a second row of units; quotes; a byte-order mark.
    let (c, r, _) = t(&parse("\u{feff}\"Flow rate\";Head\r\nm3/h;m\r\n1,5;20,25\r\n2;\"19,5\"\r\n").unwrap());
    assert_eq!(c, ["Flow_rate|m^3/h", "Head|m"]);
    assert_eq!(r, serde_json::json!([["1.5", "20.25"], ["2", "19.5"]]));
    // A plain CSV with commas, names made valid and kept apart, an unknown unit noted, a short row padded.
    let (c, r, n) = t(&parse("T (degC),T (degC),3rd col (furlongs)\n1,2,3\n4,5\n").unwrap());
    assert_eq!(c, ["T|degC", "T_2|degC", "c3rd_col|"]);
    assert_eq!(r[1], serde_json::json!(["4", "5", ""]));
    assert_eq!(n, 1);
    // Numbers only: columns c1, c2.
    let (c, _, _) = t(&parse("1\t2\n3\t4").unwrap());
    assert_eq!(c, ["c1|", "c2|"]);
    assert!(parse("\n  \n").is_err());
    let big: String = (0..2500).map(|i| format!("{i}\n")).collect();
    assert_eq!(parse(&big).unwrap()["rows"].as_array().unwrap().len(), 2000);
    // A parsed table on a sheet: its columns are vectors with their units.
    let v = parse("Q (m3/h)\tH [m]\n0\t50.3\n40\t46.5\n80\t37.4\n120\t20.9\n160\t-1\n").unwrap();
    let mut s = sheet(&["fit H_f(q) = a + b*q + c*q^2 to Q, H", "H_f(100 m^3/h)"]);
    s.regions.insert(0, Region { id: 99, kind: Kind::Table, text: "Pump curve".into(), data: serde_json::json!({ "cols": v["cols"], "rows": v["rows"] }), ..Default::default() });
    let o = eval::run(&s);
    assert!(o.regions.iter().all(|r| r.error.is_none()), "{:?}", o.regions.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    assert_eq!(o.regions[1].result, "a = 50.23 m, b = -0.009071 m/(m³/h), c = -0.001946 m/(m³/h)²; R² = 0.9999");
}

#[test]
fn curve_fitting() {
    // A name's SI value after the whole sheet is worked out.
    let val = |lines: &[&str]| -> f64 {
        let (name, rest) = lines.split_last().unwrap();
        let s = sheet(rest);
        let (o, env) = eval::run_env(&s, &eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: false });
        assert!(o.regions.iter().all(|r| r.error.is_none()), "{rest:?}");
        env.vars.get(&eval::canon(name)).map(|v| v.x()).unwrap_or(f64::NAN)
    };
    // Least squares against numpy.polyfit (a, b, c) and scipy curve_fit (A, τ; k, n).
    let q = ["Q_d = [0, 40, 80, 120, 160] m^3/h", "H_d = [50.3, 46.5, 37.4, 20.9, -1.0] m", "fit H(Q) = a + b*Q + c*Q^2 to Q_d, H_d"];
    assert_eq!(result(&q), "a = 50.23 m, b = -0.009071 m/(m³/h), c = -0.001946 m/(m³/h)²; R² = 0.9999");
    assert!((val(&[q[0], q[1], q[2], "a"]) - 50.231_428_571_428_6).abs() < 1e-9);
    assert!((val(&[q[0], q[1], q[2], "c"]) / -0.001_946_428_571_428_6 * 3600f64.powi(-2) - 1.0).abs() < 1e-9);
    assert_eq!(result(&[q[0], q[1], q[2], "H(100 m^3/h)"]), "29.86 m", "the fit defines its function");
    let t = ["t_d = [0, 1, 2, 3, 4, 5] s", "y_d = [10, 6.2, 3.5, 2.3, 1.3, 0.9] mm"];
    let e = [t[0], t[1], "fit y(t) = A*exp(-t/tau) to t_d, y_d for A = 5 mm, tau = 1 s"];
    assert_eq!(result(&e), "A = 10.03 mm, τ = 1.995 s; R² = 0.9990");
    assert!((val(&[e[0], e[1], e[2], "A"]) / 0.010_026_959_247_719_6 - 1.0).abs() < 1e-7 && (val(&[e[0], e[1], e[2], "tau"]) / 1.994_649_891_514_43 - 1.0).abs() < 1e-7);
    let p = ["x = [10, 20, 40, 80]", "y = [1.1, 4.1, 15.8, 62.5]", "fit f(z) = k*z^n to x, y for k = 0.01, n = 2"];
    assert!((val(&[p[0], p[1], p[2], "k"]) / 0.010_716_427_912_555 - 1.0).abs() < 1e-6 && (val(&[p[0], p[1], p[2], "n"]) / 1.978_769_928_007_445 - 1.0).abs() < 1e-7);
    // Listed parameters, a known term (no parameter) subtracted, a straight line through exact points.
    assert_eq!(result(&["L = 2 m", "x = [0, 1, 2] s", "y = [2, 4, 6] m", "fit s(t) = L + v*t to x, y for v"]), "v = 2 m/s; R² = 1.0000; L as defined", "a name the fit holds is said");
    assert_eq!(result(&["x = [1, 2, 4] m", "y = [3, 2, 1.5] N", "fit F(r) = a + b/r to x, y"]), "a = 1 N, b = 2 N·m; R² = 1.0000");
    for (lines, msg) in [
        (&["x = [1, 2, 3]", "y = [1, 2]", "fit f(z) = a + b*z to x, y"][..], "The data have 3 x values and 2 y values"),
        (&["x = [1, 2]", "y = [1, 2]", "fit f(z) = a + b*z + c*z^2 to x, y"][..], "3 parameters need at least 3 data points"),
        (&["x = [1, 1, 1]", "y = [1, 2, 3]", "fit f(z) = a + b*z to x, y"][..], "The data do not fix every parameter"),
        (&["x = [1, 2, 3]", "y = [1, 2, 3]", "fit f(z) = a*exp(b*z) to x, y"][..], "This model is not linear in its parameters"),
        (&["x = [1, 2, 3]", "y = [1, 2, 3]", "a = 1", "fit f(z) = a*z to x, y"][..], "Every name in this model is defined on the sheet already"),
        (&["x = [1, 2, 3] m", "y = [1, 2, 3] s", "fit f(z) = a + b*z to x, 3"][..], "The data are two vectors"),
        (&["x = [1, 2, 3] m", "y = [1, 2, 3] s", "fit f(z) = a + a*z to x, y"][..], "a multiplies terms in different units"),
        (&["x = [1, 2, 3] m", "y = [1, 2, 3] s", "fit f(z) = a + b*z to x, y"][..], ""),
        (&["x = [1, 2, 3]", "y = [1, 2, 3]", "fit f(z) = a + b*z"][..], "Give the data after the model"),
        (&["points([1, 2], [3, 4])"][..], "points(xs, ys) marks measured values on a plot"),
        (&["x = [1, 2] m", "y = [3, 4] s", "plot 2*q, points(x, y) for q in 0 .. 3"][..], "The points' x values are a length, the plot's x is a pure number."),
    ] { let got = error(lines); assert!(if msg.is_empty() { got.is_empty() } else { got.starts_with(msg) }, "{lines:?}: {got}"); }
}

#[test]
fn fitting_review() {
    let x = ["xs = [0, 1, 2, 3]", "ys = [1.1, 2.9, 5.1, 7]", "zs = [0, 1, 4, 9]"];
    // A fitted function keeps its own fitted values when a later fit reuses the names.
    let o = run(&[x[0], x[1], x[2], "fit f(z) = a + b*z to xs, ys", "y_1 = f(2)", "fit g(z) = a + b*z^2 to xs, zs for a, b", "y_2 = f(2)"]);
    assert!(o.regions[4].result == o.regions[6].result && o.regions[4].result == "5.02", "{} {}", o.regions[4].result, o.regions[6].result);
    // An absolute temperature stays absolute: T = −30 °C + 5 K/kW·P.
    let t = ["P_d = [0, 4, 8, 12] kW", "T_d = [-30, -10, 10, 30] °C", "fit T(P) = a + b*P to P_d, T_d"];
    assert_eq!(result(&t), "a = -30 °C, b = 5 K/kW; R² = 1.0000");
    assert_eq!(result(&[t[0], t[1], t[2], "T(16 kW)"]), "50 °C");
    // Exact data: round-off shown as 0; all y the same: R² is not defined unless the fit is exact.
    assert_eq!(result(&["Q_d = [0, 40, 80] m^3/h", "H_d = [50, 46.8, 37.2] m", "fit H(Q) = a + b*Q + c*Q^2 to Q_d, H_d"]), "a = 50 m, b = 0 m/(m³/h), c = -0.002 m/(m³/h)²; R² = 1.0000");
    assert_eq!(result(&["xs = [1, 2, 3, 4]", "ys = [5, 5, 5, 5]", "fit f(z) = b*z to xs, ys"]), "b = 1.667; R² = —");
    for (lines, msg) in [
        (&[x[0], x[1], "fit f(z) = p*q*z to xs, ys for p = 0, q = 0"][..], "p does not change the model at its starting guess"),
        (&[x[0], x[1], "fit f(z) = s*z to xs, ys for s = 1, w = 2"][..], "w is listed but is not in the model."),
        (&[x[0], x[1], "fit f(z) = a + b*z to xs, ys for a, a"][..], "a is listed twice."),
        (&[x[0], x[1], "fit f(z) = a + b*z to xs, ys for a, z"][..], "z is the model's variable, not a parameter."),
        (&[x[0], "ys = [1, 2, 0/0, 4]", "fit f(z) = a + b*z to xs, ys"][..], ""),
        (&["xs = 1, 2 .. 6000", "ys = xs*2", "fit f(z) = a*exp(b*z/6000) to xs, ys for a = 1, b = 1"][..], "A fit from starting guesses takes at most 5,000 points."),
        (&[x[0], x[1], "fit f(z) = 2*(a + b*z) to xs, ys"][..], "This model is not linear in its parameters as written: multiply out brackets"),
        (&[x[0], x[1], "fit f(z) = a + b*z + 1 m to xs, ys"][..], "Cannot add"),
    ] { let got = error(lines); assert!(if msg.is_empty() { !got.is_empty() } else { got.starts_with(msg) }, "{lines:?}: {got}"); }
    // A blank cell in a table column is said, not taken for dependent data.
    let mut s = sheet(&["fit f(z) = a + b*z to X, Y"]);
    s.regions.insert(0, Region { id: 99, kind: Kind::Table, data: serde_json::json!({ "cols": [{ "name": "X" }, { "name": "Y" }], "rows": [["1", "2"], ["2", ""], ["3", "6"]] }), ..Default::default() });
    let e = eval::run(&s).regions[1].error.clone().map(|e| e.msg).unwrap_or_default();
    assert!(e.starts_with("Value 2 of the y data is empty or not a number."), "{e}");
}

#[test]
fn reading_data_review() {
    let rows = |t: &str, pasted: bool| { let v = crate::tabin::parse(t, pasted).unwrap(); (v["cols"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect::<Vec<_>>(), v["rows"].clone(), v["notes"].as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect::<Vec<_>>()) };
    // One column copied with decimal commas (no tabs): one column, not two.
    assert_eq!(rows("1,5\n2,5\n3,5", true).1, serde_json::json!([["1.5"], ["2.5"], ["3.5"]]));
    // A title above the table is left out; 1,2 in a comma file is two cells.
    let (c, r, n) = rows("Pump test 2026\nQ,H\n1,2\n3,4", false);
    assert!(c == ["Q", "H"] && r == serde_json::json!([["1", "2"], ["3", "4"]]) && n[0].starts_with("Left out 1 line"), "{c:?} {r} {n:?}");
    // Thousands with commas in tab-separated text; thousands with points or spaces before a decimal comma.
    let (_, r, n) = rows("1,234\t2\n5,678\t3", true);
    assert!(r == serde_json::json!([["1234", "2"], ["5678", "3"]]) && n.iter().any(|x| x.contains("thousands")), "{r}");
    assert_eq!(rows("Q;H\n1.234,5;2\n1 234,5;3", false).1, serde_json::json!([["1234.5", "2"], ["1234.5", "3"]]));
    // Values with units are data, not a heading; quoted line breaks; old line ends; a heading alone.
    assert_eq!(rows("12 m;3 s\n4 m;5 s", false).1, serde_json::json!([["12 m", "3 s"], ["4 m", "5 s"]]));
    assert_eq!(rows("a,b\n\"multi\nline\",2\n3,4", false).1, serde_json::json!([["multi\nline", "2"], ["3", "4"]]));
    assert_eq!(rows("Q,H\r1,2\r3,4", false).1, serde_json::json!([["1", "2"], ["3", "4"]]));
    let (c, r, _) = rows("Q (m3/h);H [m]", false);
    assert!(c == ["Q", "H"] && r == serde_json::json!([["", ""]]), "{c:?} {r}");
}

#[test]
fn differential_equations() {
    // Against exact solutions: decay x = e^(−t/τ); a spring y = A cos ωt, y' = −Aω sin ωt (ω = √(k/m) = 10 rad/s);
    // cooling T = T_a + (T₀ − T_a) e^(−t/τ); a = e^t and b = e^t − 1 together (b' = a).
    let close = |lines: &[&str], want: f64| {
        let (o, env) = eval::run_env(&sheet(lines), &eval::Fmt { digits: 4, eng: false, us: false, polar: false, exact: false });
        assert!(o.regions.iter().all(|r| r.error.is_none()), "{lines:?}: {:?}", o.regions.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
        let got = env.vars.get("got").map(|v| v.x()).unwrap();
        assert!(((got - want) / want).abs() < 1e-7, "{lines:?}: {got} against {want}");
    };
    close(&["tau = 2 s", "ode x' = -x/tau for t in 0 s .. 10 s from x = 1 m", "got = x(3.7 s)"], (-3.7f64 / 2.0).exp());
    let spring = ["k = 100 N/m", "m_1 = 1 kg", "ode y'' = -k/m_1*y for t in 0 .. 1 s from y = 1 cm, y' = 0"];
    close(&[spring[0], spring[1], spring[2], "got = y(0.73 s)"], 0.01 * (10.0f64 * 0.73).cos());
    close(&[spring[0], spring[1], spring[2], "got = y'(0.05 s)"], -0.1 * (0.5f64).sin());
    close(&["T_a = 20 °C", "ode T' = (T_a - T)/(600 s) for t in 0 .. 1 h from T = 80 °C", "got = T(10 min)"], 293.15 + 60.0 * (-1.0f64).exp());
    close(&["ode a' = a/(1 s), b' = a for t in 0 .. 3 s from a = 1 m, b = 0 m*s", "got = b(3 s)"], 3f64.exp() - 1.0);
    // What the line shows: the values at the end, and a plot of the states.
    let o = run(&["T_a = 20 °C", "ode T' = (T_a - T)/(600 s) for t in 0 .. 1 h from T = 80 °C", "T(10 min)"]);
    assert_eq!(o.regions[1].result, "at t = 1 h: T = 20.15 °C");
    assert_eq!(o.regions[2].result, "42.07 °C", "an absolute temperature stays absolute");
    assert!(o.regions[1].plot.as_ref().is_some_and(|p| p.series.len() == 1 && p.yunit == "°C" && p.xs.len() == 121));
    assert_eq!(result(&[spring[0], spring[1], spring[2]]), "at t = 1 s: y = -8.391 mm, y' = 0.0544 m/s");
    for (lines, msg) in [
        (&["ode z' = z^2 for t in 0 .. 2 s from z = 1 Hz"][..], "The solution changes too fast to follow"),
        (&["ode q' = 3 for t in 0 .. 1 s from q = 1 m"][..], "q' must be in a length per a time"),
        (&["ode q'' = -q/(1 s^2) for t in 0 .. 1 s from q = 1 m"][..], "Give a starting value for q'"),
        (&["ode u' = -u/(1 s) for t in 0 .. 1 s from u = 1 m, w = 2"][..], "w is not one of these equations' states: they are u."),
        (&["ode u = -u for t in 0 .. 1 s from u = 1 m"][..], "Write each equation for a derivative"),
        (&["ode u' = -u/(1 s) for t in 1 s .. 0 s from u = 1 m"][..], "The range must go from a smaller to a larger value."),
        (&["ode u' = -u/(1 s) for t in 0 .. 1 s"][..], "Give the starting values after the range"),
        (&["ode u'' = -u/(1 s^2) for t in 0 .. 1 s from u = 1 m, u' = 1 m"][..], "u' is the rate of u, so it is in a length per a time"),
        (&["ode x' = -x/(2 s) for t in 0 s .. 10 s from x = 1 m", "x(20 s)"][..], "x is solved from 0 s to 10 s only."),
        (&["ode x' = -x/(2 s) for t in 0 s .. 10 s from x = 1 m", "x(2 m)"][..], "x takes a time, not a length."),
    ] { assert!(error(lines).starts_with(msg), "{lines:?}: {}", error(lines)); }
}
