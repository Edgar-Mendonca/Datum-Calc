//! The property functions of a sheet (0.2.6): water_…(two of T, p, h, s, x) and gas_…(gas, T[, p]), with units.
//! The physics is in `fluids`; this file reads the arguments by their units and gives results with theirs.
use crate::eval::Val;
use crate::fluids::{self, State};
use crate::parse::{Error, Span};
use crate::units::{Dim, NONE, TEMP};

type R<T> = Result<T, Error>;
/// A gas property from its arguments' SI values.
type Prop<'a> = dyn Fn(&[f64]) -> Result<f64, String> + 'a;
fn fail<T>(msg: impl Into<String>, at: Span) -> R<T> { Err(Error { msg: msg.into(), at }) }

const fn d(l: i8, m: i8, t: i8, k: i8, n: i8) -> Dim { [l, m, t, k, 0, n, 0, 0] }
const PRESS: Dim = d(-1, 1, -2, 0, 0);
const SPEC_E: Dim = d(2, 0, -2, 0, 0);
const SPEC_S: Dim = d(2, 0, -2, -1, 0);
const DENS: Dim = d(-3, 1, 0, 0, 0);
const SPEC_V: Dim = d(3, -1, 0, 0, 0);
const SPEED: Dim = d(1, 0, -1, 0, 0);
const VISC: Dim = d(-1, 1, -1, 0, 0);
const KVISC: Dim = d(2, 0, -1, 0, 0);
const COND: Dim = d(1, 1, -3, -1, 0);
const TENSION: Dim = d(0, 1, -2, 0, 0);
const MOLAR: Dim = d(0, 1, 0, 0, -1);

/// The arguments value by value: a vector argument gives a vector of results (a table of properties).
fn each(f: &str, a: &[Val], at: Span, mut g: impl FnMut(&[f64]) -> Result<f64, String>, dim: Dim, abs: bool) -> R<Val> {
    if a.iter().any(|v| v.cx()) { return fail(format!("{f} needs real values, not complex ones."), at); }
    if a.iter().any(|v| v.mat()) { return fail(format!("{f} takes single values or vectors, not a matrix."), at); }
    let n = a.iter().filter(|v| v.vec).map(|v| v.v.len()).max();
    if let Some(n) = n { if a.iter().any(|v| v.vec && v.v.len() != n) { return fail(format!("{f}: the vectors need the same length."), at); } }
    let mut out = vec![];
    for k in 0..n.unwrap_or(1) {
        let xs: Vec<f64> = a.iter().map(|v| if v.vec { v.v[k] } else { v.x() }).collect();
        let y = g(&xs).map_err(|m| Error { msg: m, at })?;
        if !y.is_finite() { return fail(format!("{f} has no value there."), at); }
        out.push(y);
    }
    Ok(Val { v: out, dim, abs, vec: n.is_some(), ..Default::default() })
}

/// What an argument is, by its units.
#[derive(Clone, Copy, PartialEq, Debug)]
enum By { T, P, H, S, X }

fn by(v: &Val) -> Option<By> {
    let s = |d: &Dim| v.dim[..7] == d[..7];
    if s(&TEMP) { Some(By::T) } else if s(&PRESS) { Some(By::P) } else if s(&SPEC_E) { Some(By::H) } else if s(&SPEC_S) { Some(By::S) } else if v.dim == NONE { Some(By::X) } else { None }
}

const PAIRS: &str = "two of: a temperature, a pressure, an enthalpy (kJ/kg), an entropy (kJ/(kg·K)) or a vapour fraction x, as (T, p), (p, h), (p, s), (T, x) or (p, x)";

/// water_…: the state from two values, recognised by their units in either order.
pub fn water(f: &str, prop: &str, a: &[Val], at: Span) -> R<Val> {
    let one = |what: &str| if a.len() == 1 { Ok(()) } else { fail(format!("{f} takes one value: {what}."), at) };
    match prop {
        "psat" => { one("a temperature")?; want(f, &a[0], By::T, at)?; return each(f, a, at, |x| fluids::psat(x[0]), PRESS, false); }
        "Tsat" => { one("a pressure")?; want(f, &a[0], By::P, at)?; return each(f, a, at, |x| fluids::tsat(x[0]), TEMP, true); }
        "sigma" => { one("a temperature")?; want(f, &a[0], By::T, at)?; return each(f, a, at, |x| fluids::sigma(x[0]), TENSION, false); }
        _ => {}
    }
    if a.len() != 2 { return fail(format!("{f} takes {PAIRS}."), at); }
    let (b0, b1) = (by(&a[0]), by(&a[1]));
    let (Some(b0), Some(b1)) = (b0, b1) else { return fail(format!("{f} takes {PAIRS}."), at) };
    // In the order T, P, H, S, X, whatever order they were written in.
    let (first, second, swap) = if (b0 as u8) <= (b1 as u8) { (b0, b1, false) } else { (b1, b0, true) };
    let state = move |x: &[f64]| -> Result<State, String> {
        let (u, v) = if swap { (x[1], x[0]) } else { (x[0], x[1]) };
        if second == By::X && !(0.0..=1.0).contains(&v) { return Err("A value without a unit is read as a vapour fraction x, from 0 to 1; for a pressure or a temperature, give its unit (10 bar, 180 °C).".into()); }
        match (first, second) {
            (By::T, By::P) => fluids::tp(u, v),
            (By::P, By::H) => fluids::p_and(u, v, 0),
            (By::P, By::S) => fluids::p_and(u, v, 1),
            (By::T, By::X) => fluids::tx(u, v),
            (By::P, By::X) => fluids::px(u, v),
            _ => Err(format!("{f} takes {PAIRS}.")),
        }
    };
    let wet = |st: &State, what: &str| -> Result<(), String> {
        if st.x.is_none() && !(st.cp.is_finite() && st.cp < 1e7) { return Err(format!("{what} goes to infinity at the critical point (373.946 °C, 220.64 bar): this state is too near it.")); }
        if st.x.is_some() { Err(format!("{what} is not defined for wet steam (a mix of liquid and vapour): give a state outside the saturation dome.")) } else { Ok(()) }
    };
    let (dim, abs) = match prop {
        "T" => (TEMP, true), "p" => (PRESS, false), "rho" => (DENS, false), "v" => (SPEC_V, false), "h" | "u" => (SPEC_E, false),
        "s" | "cp" | "cv" => (SPEC_S, false), "w" => (SPEED, false), "x" | "Pr" => (NONE, false), "mu" => (VISC, false), "nu" => (KVISC, false), "k" => (COND, false),
        _ => return fail(format!("{f} is not a water property."), at),
    };
    each(f, a, at, |x| {
        let st = state(x)?;
        Ok(match prop {
            "T" => st.t, "p" => st.p, "rho" => st.rho, "v" => 1.0 / st.rho, "h" => st.h, "u" => st.u, "s" => st.s,
            "cp" => { wet(&st, "cp")?; st.cp } "cv" => { wet(&st, "cv")?; st.cv } "w" => { wet(&st, "The speed of sound")?; st.w }
            "x" => match st.x {
                Some(x) => x,
                // Outside the dome: the thermodynamic quality (h − h′)/(h″ − h′), below 0 for liquid, above 1 for superheated steam.
                None if st.p < 611.213 => return Err("No vapour fraction below 611.2 Pa (the triple point): there is no liquid at that pressure.".into()),
                None if st.p < 22.064e6 => { let (l, v) = fluids::sat(fluids::tsat(st.p)?)?; (st.h - l.h) / (v.h - l.h) }
                None => return Err("Above the critical pressure (220.64 bar) there is no vapour fraction.".into()),
            },
            _ => {
                wet(&st, if prop == "k" { "The conductivity" } else if prop == "Pr" { "The Prandtl number" } else { "The viscosity" })?;
                if st.t > 1173.15 { return Err("Viscosity and conductivity of water are given up to 900 °C.".into()); }
                let mu = fluids::mu(st.rho, st.t);
                match prop { "mu" => mu, "nu" => mu / st.rho, "k" => fluids::k(&st), _ => mu * st.cp / fluids::k(&st) }
            }
        })
    }, dim, abs)
}

/// An absolute temperature and a pressure, both above zero.
fn positive(x: &[f64]) -> Result<(), String> {
    if x[0] > 0.0 && x[1] > 0.0 { Ok(()) } else { Err("An ideal gas needs an absolute temperature and a pressure above zero.".into()) }
}

fn want(f: &str, v: &Val, b: By, at: Span) -> R<()> {
    if by(v) == Some(b) { return Ok(()); }
    fail(format!("{f} needs {}.", match b { By::T => "a temperature", By::P => "a pressure", _ => "a value" }), at)
}

/// gas_…(gas, T[, p]): the gas is a name (air, N2, O2, Ar, CO2, H2, He, CH4), then a temperature and, for density,
/// kinematic viscosity and entropy, a pressure (either order).
pub fn gas(f: &str, prop: &str, g: &fluids::Gas, a: &[Val], at: Span) -> R<Val> {
    let need = match prop { "M" | "R" => 0, "rho" | "nu" | "s" => 2, "cp" | "cv" | "gamma" | "h" | "mu" | "k" | "Pr" => 1, _ => return fail(format!("{f} is not a gas property."), at) };
    if a.len() != need {
        return fail(format!("{f} takes the gas{}.", match need { 0 => String::new(), 1 => ", then a temperature".into(), _ => ", a temperature and a pressure".into() }), at);
    }
    let mut a = a.to_vec();
    if need == 2 && by(&a[0]) == Some(By::P) { a.swap(0, 1); }
    if need >= 1 { want(f, &a[0], By::T, at)?; }
    if need == 2 { want(f, &a[1], By::P, at)?; }
    let r = g.r();
    let (dim, val): (Dim, Box<Prop>) = match prop {
        "M" => (MOLAR, Box::new(|_| Ok(g.m))),
        "R" => (SPEC_S, Box::new(move |_| Ok(r))),
        "cp" => (SPEC_S, Box::new(|x| g.cp(x[0]))),
        "cv" => (SPEC_S, Box::new(move |x| Ok(g.cp(x[0])? - r))),
        "gamma" => (NONE, Box::new(move |x| { let cp = g.cp(x[0])?; Ok(cp / (cp - r)) })),
        "h" => (SPEC_E, Box::new(|x| g.h(x[0]))),
        "s" => (SPEC_S, Box::new(|x| g.s(x[0], x[1]))),
        "rho" => (DENS, Box::new(move |x| { positive(x)?; Ok(x[1] / (r * x[0])) })),
        "mu" => (VISC, Box::new(|x| g.mu(x[0]))),
        "nu" => (KVISC, Box::new(move |x| { positive(x)?; Ok(g.mu(x[0])? * r * x[0] / x[1]) })),
        "k" => (COND, Box::new(|x| g.k(x[0]))),
        _ => (NONE, Box::new(|x| Ok(g.mu(x[0])? * g.cp(x[0])? / g.k(x[0])?))),
    };
    if need == 0 { return Ok(Val::num(val(&[]).map_err(|m| Error { msg: m, at })?, dim)); }
    each(f, &a, at, |x| val(x), dim, false)
}
