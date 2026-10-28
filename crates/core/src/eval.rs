//! Evaluation: the sheet top to bottom with a symbol table. Each region gets its own result or its
//! own error; an error never stops the rest of the sheet.
use crate::latex;
use crate::parse::{self, Error, Node, Span, Stmt, E};
use crate::sheet::{Kind, Region, Sheet};
use crate::units::{self, Dim, NONE, TEMP};
use serde::Serialize;
use std::collections::HashMap;

/// A value: one number or a vector, in SI base units, with its dimension. `abs` marks an absolute
/// temperature (given in °C or °F); `logic` marks the result of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Val { pub v: Vec<f64>, pub dim: Dim, pub abs: bool, pub vec: bool, pub logic: bool }

impl Val {
    pub fn num(x: f64, dim: Dim) -> Self { Self { v: vec![x], dim, abs: false, vec: false, logic: false } }
    pub fn plain(x: f64) -> Self { Self::num(x, NONE) }
    fn vector(v: Vec<f64>, dim: Dim) -> Self { Self { v, dim, abs: false, vec: true, logic: false } }
    pub fn x(&self) -> f64 { self.v[0] }
}

type R<T> = Result<T, Error>;
fn fail<T>(msg: impl Into<String>, at: Span) -> R<T> { Err(Error { msg: msg.into(), at }) }

const GREEK: [(&str, &str); 34] = [
    ("alpha", "α"), ("beta", "β"), ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("zeta", "ζ"), ("eta", "η"), ("theta", "θ"), ("iota", "ι"), ("kappa", "κ"),
    ("lambda", "λ"), ("mu", "μ"), ("nu", "ν"), ("xi", "ξ"), ("pi", "π"), ("rho", "ρ"), ("sigma", "σ"), ("tau", "τ"), ("upsilon", "υ"), ("phi", "φ"), ("chi", "χ"),
    ("psi", "ψ"), ("omega", "ω"), ("Gamma", "Γ"), ("Delta", "Δ"), ("Theta", "Θ"), ("Lambda", "Λ"), ("Xi", "Ξ"), ("Pi", "Π"), ("Sigma", "Σ"), ("Phi", "Φ"),
    ("Psi", "Ψ"), ("Omega", "Ω"), ("varphi", "ϕ"),
];
pub fn greek(word: &str) -> Option<&'static str> { GREEK.iter().find(|g| g.0 == word).map(|g| g.1) }
pub fn greek_name(sym: &str) -> Option<&'static str> { GREEK.iter().find(|g| g.1 == sym).map(|g| g.0) }

/// One spelling per name: Greek words become letters, so `sigma_allow` and `σ_allow` are the same name.
pub fn canon(name: &str) -> String {
    name.split('_').map(|p| greek(p).unwrap_or(p)).collect::<Vec<_>>().join("_")
}

/// Built-in constants (a name defined on the sheet takes their place).
fn constant(name: &str) -> Option<Val> {
    Some(match name {
        "π" => Val::plain(std::f64::consts::PI),
        "e" => Val::plain(std::f64::consts::E),
        "g" => Val::num(9.806_65, [1, 0, -2, 0, 0, 0, 0, 0]),
        "R" => Val::num(8.314_462_618, [2, 1, -2, -1, 0, -1, 0, 0]),
        "σ_SB" => Val::num(5.670_374_419e-8, [0, 1, -3, -4, 0, 0, 0, 0]),
        "N_A" => Val::num(6.022_140_76e23, [0, 0, 0, 0, 0, -1, 0, 0]),
        "k_B" => Val::num(1.380_649e-23, [2, 1, -2, -1, 0, 0, 0, 0]),
        "c_0" => Val::num(299_792_458.0, [1, 0, -1, 0, 0, 0, 0, 0]),
        "g_c" => Val::plain(1.0),
        _ => return None,
    })
}
pub const CONSTANTS: [(&str, &str); 8] = [("π", "pi, 3.14159…"), ("e", "Euler's number, 2.71828…"), ("g", "standard gravity, 9.80665 m/s²"), ("R", "molar gas constant, 8.314462618 J/(mol·K)"),
    ("σ_SB", "Stefan–Boltzmann constant, 5.670374419×10⁻⁸ W/(m²·K⁴)"), ("N_A", "Avogadro constant, 6.02214076×10²³ /mol"), ("k_B", "Boltzmann constant, 1.380649×10⁻²³ J/K"), ("c_0", "speed of light, 299792458 m/s")];

/// Built-in functions: name, arguments, what it does (for help and the window's menus).
pub const FUNCTIONS: [(&str, &str); 27] = [
    ("sqrt(x)", "square root"), ("cbrt(x)", "cube root"), ("abs(x)", "absolute value"), ("exp(x)", "e to the power x"), ("ln(x)", "natural logarithm"), ("log(x)", "logarithm base 10"),
    ("sin(x)", "sine (x in rad, or with deg)"), ("cos(x)", "cosine"), ("tan(x)", "tangent"), ("asin(x)", "inverse sine (an angle)"), ("acos(x)", "inverse cosine"), ("atan(x)", "inverse tangent"),
    ("atan2(y, x)", "angle of the point (x, y)"), ("sinh(x)", "hyperbolic sine"), ("cosh(x)", "hyperbolic cosine"), ("tanh(x)", "hyperbolic tangent"),
    ("min(a, b, …)", "smallest value"), ("max(a, b, …)", "largest value"), ("round(x, n)", "x rounded to n decimals"), ("floor(x)", "round down"), ("ceil(x)", "round up"),
    ("if(test, a, b)", "a when the test holds, otherwise b"), ("sum(v)", "sum of a vector"), ("mean(v)", "mean of a vector"), ("len(v)", "number of values"),
    ("interp(x, xs, ys)", "straight-line interpolation in a table"), ("hypot(a, b)", "√(a² + b²)"),
];

pub struct Env {
    pub vars: HashMap<String, Val>,
    pub funcs: HashMap<String, (Vec<String>, Node)>,
    /// Names that are defined further down the sheet, or whose own line has an error.
    later: HashMap<String, bool>,
    depth: u32,
}

impl Env {
    pub fn new() -> Self { Self { vars: HashMap::new(), funcs: HashMap::new(), later: HashMap::new(), depth: 0 } }

    fn var(&self, name: &str, at: Span) -> R<Val> {
        let k = canon(name);
        if let Some(v) = self.vars.get(&k) { return Ok(v.clone()); }
        if let Some(v) = constant(&k) { return Ok(v); }
        match self.later.get(&k) {
            Some(true) => fail(format!("{} has an error on its own line.", show(&k)), at),
            Some(false) => fail(format!("{} is defined further down: move its line above this one.", show(&k)), at),
            None if units::lookup(name).is_some() => fail(format!("“{name}” is a unit: put a number before it (1 {name})."), at),
            None => fail(format!("{} is not defined. Define it above this line, such as {} = 1.", show(&k), name), at),
        }
    }

    pub fn eval(&mut self, n: &Node) -> R<Val> {
        match &n.e {
            E::Num(x, u) => Ok(match u {
                None => Val::plain(*x),
                Some(u) => Val { v: vec![x * u.factor + u.offset], dim: u.dim, abs: u.offset != 0.0, vec: false, logic: false },
            }),
            E::Var(s) => self.var(s, n.at),
            E::Group(e) => self.eval(e),
            E::Neg(e) => { let mut v = self.eval(e)?; v.v.iter_mut().for_each(|x| *x = -*x); v.abs = false; v.logic = false; Ok(v) }
            E::Bin(op, a, b) => { let (x, y) = (self.eval(a)?, self.eval(b)?); bin(op, x, y, n.at, b.at) }
            E::Call(f, args) => self.call(f, args, n.at),
        }
    }

    fn call(&mut self, f: &str, args: &[Node], at: Span) -> R<Val> {
        let k = canon(f);
        if let Some((params, body)) = self.funcs.get(&k).cloned() {
            if params.len() != args.len() { return fail(format!("{f} takes {} value{}.", params.len(), if params.len() == 1 { "" } else { "s" }), at); }
            if self.depth > 40 { return fail(format!("{f} calls itself too deeply."), at); }
            let vals = args.iter().map(|a| self.eval(a)).collect::<R<Vec<_>>>()?;
            let saved: Vec<(String, Option<Val>)> = params.iter().map(|p| (canon(p), self.vars.get(&canon(p)).cloned())).collect();
            for (p, v) in params.iter().zip(vals) { self.vars.insert(canon(p), v); }
            self.depth += 1;
            let r = self.eval(&body);
            self.depth -= 1;
            for (p, v) in saved { match v { Some(v) => { self.vars.insert(p, v); } None => { self.vars.remove(&p); } } }
            return r.map_err(|e| if e.at.0 < body.at.0 || e.at.1 > body.at.1 { e } else { Error { msg: format!("In {f}: {}", e.msg), at } });
        }
        let n = args.len();
        let want = |m: usize| -> R<()> { if n == m { Ok(()) } else { fail(format!("{f} takes {m} value{}.", if m == 1 { "" } else { "s" }), at) } };
        if f == "if" {
            want(3)?;
            let c = self.eval(&args[0])?;
            if !c.vec { return if c.x() != 0.0 { self.eval(&args[1]) } else { self.eval(&args[2]) }; }
            let (a, b) = (self.eval(&args[1])?, self.eval(&args[2])?);
            if !units::same(&a.dim, &b.dim) { return fail("Both choices of if must have the same units.", at); }
            let pick = |i: usize, v: &Val| if v.vec { v.v.get(i).copied().unwrap_or(f64::NAN) } else { v.x() };
            return Ok(Val { v: c.v.iter().enumerate().map(|(i, t)| if *t != 0.0 { pick(i, &a) } else { pick(i, &b) }).collect(), vec: true, ..a });
        }
        let vals = args.iter().map(|a| self.eval(a)).collect::<R<Vec<_>>>()?;
        builtin(f, vals, at)
    }
}

impl Default for Env { fn default() -> Self { Self::new() } }

fn same_or(a: &Val, b: &Val, what: &str, at: Span) -> R<()> {
    if units::same(&a.dim, &b.dim) { return Ok(()); }
    fail(format!("Cannot {what} {} and {}: the units do not match.", dim_name(&a.dim), dim_name(&b.dim)), at)
}

/// A plain word for a dimension, for messages: "a length", "a pressure", or its SI units.
pub fn dim_name(d: &Dim) -> String {
    let named: [(&str, Dim); 14] = [("a pure number", NONE), ("a length", [1, 0, 0, 0, 0, 0, 0, 0]), ("a mass", [0, 1, 0, 0, 0, 0, 0, 0]), ("a time", [0, 0, 1, 0, 0, 0, 0, 0]),
        ("a temperature", TEMP), ("an area", [2, 0, 0, 0, 0, 0, 0, 0]), ("a volume", [3, 0, 0, 0, 0, 0, 0, 0]), ("a force", [1, 1, -2, 0, 0, 0, 0, 0]),
        ("a pressure", [-1, 1, -2, 0, 0, 0, 0, 0]), ("an energy", [2, 1, -2, 0, 0, 0, 0, 0]), ("a power", [2, 1, -3, 0, 0, 0, 0, 0]), ("a speed", [1, 0, -1, 0, 0, 0, 0, 0]),
        ("a flow", [3, 0, -1, 0, 0, 0, 0, 0]), ("a current", [0, 0, 0, 0, 1, 0, 0, 0])];
    named.iter().find(|x| units::same(&x.1, d)).map(|x| x.0.to_string()).unwrap_or_else(|| format!("a value in {}", units::base_label(d)))
}

fn zip(a: &Val, b: &Val, at: Span, f: impl Fn(f64, f64) -> f64) -> R<Vec<f64>> {
    match (a.vec, b.vec) {
        (false, false) => Ok(vec![f(a.x(), b.x())]),
        (true, false) => Ok(a.v.iter().map(|x| f(*x, b.x())).collect()),
        (false, true) => Ok(b.v.iter().map(|y| f(a.x(), *y)).collect()),
        (true, true) if a.v.len() == b.v.len() => Ok(a.v.iter().zip(&b.v).map(|(x, y)| f(*x, *y)).collect()),
        _ => fail(format!("These vectors have different lengths ({} and {}).", a.v.len(), b.v.len()), at),
    }
}

fn ang(a: &Dim, b: &Dim) -> i8 { if a[7] != 0 { a[7] } else { b[7] } }

pub fn bin(op: &str, a: Val, b: Val, at: Span, rat: Span) -> R<Val> {
    let vec = a.vec || b.vec;
    let mk = |v: Vec<f64>, dim: Dim, abs: bool| Val { v, dim, abs, vec, logic: false };
    match op {
        "+" | "-" => {
            same_or(&a, &b, if op == "+" { "add" } else { "subtract" }, at)?;
            let mut dim = a.dim;
            dim[7] = ang(&a.dim, &b.dim);
            let abs = if op == "+" { a.abs || b.abs } else { a.abs && !b.abs };
            Ok(mk(zip(&a, &b, at, |x, y| if op == "+" { x + y } else { x - y })?, dim, abs))
        }
        "*" => {
            let abs = (a.abs && units::is_none(&b.dim)) || (b.abs && units::is_none(&a.dim));
            Ok(mk(zip(&a, &b, at, |x, y| x * y)?, units::mul(&a.dim, &b.dim), abs))
        }
        "/" => {
            if !b.vec && b.x() == 0.0 { return fail("Division by zero.", rat); }
            let abs = a.abs && units::is_none(&b.dim);
            Ok(mk(zip(&a, &b, at, |x, y| x / y)?, units::div(&a.dim, &b.dim), abs))
        }
        "^" => {
            if !units::is_none(&b.dim) || b.vec { return fail("A power must be a plain number.", rat); }
            let p = b.x();
            if units::is_none(&a.dim) && a.dim[7] == 0 { return Ok(mk(a.v.iter().map(|x| x.powf(p)).collect(), NONE, false)); }
            // With units, the power must give whole exponents (x^2, x^0.5 of an area, x^(1/3) of a volume).
            let mut dim = NONE;
            for (o, x) in dim.iter_mut().zip(a.dim) {
                let e = x as f64 * p;
                if (e - e.round()).abs() > 1e-9 { return fail(format!("{} to the power {} has no units.", dim_name(&a.dim), fmt_plain(p, 4)), at); }
                *o = e.round() as i8;
            }
            Ok(mk(a.v.iter().map(|x| x.powf(p)).collect(), dim, false))
        }
        "<" | "<=" | ">" | ">=" | "==" | "!=" => {
            same_or(&a, &b, "compare", at)?;
            let tol = |x: f64, y: f64| (x - y).abs() <= 1e-12 * x.abs().max(y.abs());
            let v = zip(&a, &b, at, |x, y| (match op { "<" => x < y, "<=" => x <= y || tol(x, y), ">" => x > y, ">=" => x >= y || tol(x, y), "==" => tol(x, y), _ => !tol(x, y) }) as u8 as f64)?;
            Ok(Val { v, dim: NONE, abs: false, vec, logic: true })
        }
        o => fail(format!("“{o}” cannot be used here."), at),
    }
}

fn builtin(f: &str, a: Vec<Val>, at: Span) -> R<Val> {
    let n = a.len();
    let want = |m: usize| -> R<()> { if n == m { Ok(()) } else { fail(format!("{f} takes {m} value{}.", if m == 1 { "" } else { "s" }), at) } };
    let map = |v: &Val, dim: Dim, g: &dyn Fn(f64) -> f64| Val { v: v.v.iter().map(|x| g(*x)).collect(), dim, abs: false, vec: v.vec, logic: false };
    let plain = |v: &Val| -> R<()> { if units::is_none(&v.dim) { Ok(()) } else { Err(Error { msg: format!("{f} needs a plain number, not {}.", dim_name(&v.dim)), at }) } };
    let fail = |m: String| -> R<Val> { Err(Error { msg: m, at }) };
    let r = match f {
        "sqrt" | "cbrt" => {
            want(1)?;
            let k = if f == "sqrt" { 2 } else { 3 };
            if a[0].dim.iter().any(|e| e % k != 0) { return fail(format!("{f} of {} has no units.", dim_name(&a[0].dim))); }
            if f == "sqrt" && a[0].v.iter().any(|x| *x < 0.0) { return fail("sqrt of a negative number.".into()); }
            map(&a[0], std::array::from_fn(|i| a[0].dim[i] / k), &|x| if k == 2 { x.sqrt() } else { x.cbrt() })
        }
        "abs" => { want(1)?; Val { v: a[0].v.iter().map(|x| x.abs()).collect(), ..a[0].clone() } }
        "exp" | "ln" | "log" | "log10" | "sinh" | "cosh" | "tanh" => {
            want(1)?;
            plain(&a[0])?;
            if (f == "ln" || f.starts_with("log")) && a[0].v.iter().any(|x| *x <= 0.0) { return fail(format!("{f} of zero or a negative number.")); }
            map(&a[0], NONE, &|x| match f { "exp" => x.exp(), "ln" => x.ln(), "sinh" => x.sinh(), "cosh" => x.cosh(), "tanh" => x.tanh(), _ => x.log10() })
        }
        "sin" | "cos" | "tan" => {
            want(1)?;
            if !units::is_none(&a[0].dim) { return fail(format!("{f} needs an angle or a plain number, not {}.", dim_name(&a[0].dim))); }
            map(&a[0], NONE, &|x| match f { "sin" => x.sin(), "cos" => x.cos(), _ => x.tan() })
        }
        "asin" | "acos" | "atan" => {
            want(1)?;
            if !units::is_none(&a[0].dim) { return fail(format!("{f} needs a plain number.")); }
            if f != "atan" && a[0].v.iter().any(|x| x.abs() > 1.0) { return fail(format!("{f} needs a value between −1 and 1.")); }
            map(&a[0], units::ANGLE, &|x| match f { "asin" => x.asin(), "acos" => x.acos(), _ => x.atan() })
        }
        "atan2" | "hypot" => {
            want(2)?;
            same_or(&a[0], &a[1], "combine", at)?;
            let v = zip(&a[0], &a[1], at, |y, x| if f == "atan2" { y.atan2(x) } else { y.hypot(x) })?;
            Val { v, dim: if f == "atan2" { units::ANGLE } else { a[0].dim }, abs: false, vec: a[0].vec || a[1].vec, logic: false }
        }
        "min" | "max" => {
            if n == 0 { return fail(format!("{f} needs at least one value.")); }
            for x in &a[1..] { same_or(&a[0], x, "compare", at)?; }
            let all: Vec<f64> = a.iter().flat_map(|x| x.v.iter().copied()).collect();
            let m = if f == "min" { all.iter().copied().fold(f64::INFINITY, f64::min) } else { all.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
            Val { v: vec![m], dim: a[0].dim, abs: a.iter().all(|x| x.abs), vec: false, logic: false }
        }
        "round" | "floor" | "ceil" => {
            if n == 0 || n > 2 { return fail(format!("{f} takes one or two values.")); }
            let d = if n == 2 { plain(&a[1])?; a[1].x().round() as i32 } else { 0 };
            let s = 10f64.powi(d);
            Val { v: a[0].v.iter().map(|x| match f { "round" => (x * s).round() / s, "floor" => (x * s).floor() / s, _ => (x * s).ceil() / s }).collect(), ..a[0].clone() }
        }
        "sum" | "mean" => {
            want(1)?;
            let s: f64 = a[0].v.iter().sum();
            Val { v: vec![if f == "sum" { s } else { s / a[0].v.len() as f64 }], dim: a[0].dim, abs: f == "mean" && a[0].abs, vec: false, logic: false }
        }
        "len" => { want(1)?; Val::plain(a[0].v.len() as f64) }
        "interp" => {
            want(3)?;
            let (x, xs, ys) = (&a[0], &a[1], &a[2]);
            same_or(x, xs, "look up", at)?;
            if xs.v.len() != ys.v.len() || xs.v.len() < 2 { return fail("interp needs two vectors of the same length (at least two points).".into()); }
            if xs.v.windows(2).any(|w| w[1] <= w[0]) { return fail("interp needs the xs in increasing order.".into()); }
            let one = |q: f64| {
                let k = xs.v.partition_point(|v| *v <= q).clamp(1, xs.v.len() - 1);
                let (x0, x1, y0, y1) = (xs.v[k - 1], xs.v[k], ys.v[k - 1], ys.v[k]);
                y0 + (y1 - y0) * (q - x0) / (x1 - x0)
            };
            Val { v: x.v.iter().map(|q| one(*q)).collect(), dim: ys.dim, abs: ys.abs, vec: x.vec, logic: false }
        }
        _ => return fail(format!("{f} is not a function. Define it first, such as {f}(x) = 2*x.")),
    };
    if r.v.iter().any(|x| !x.is_finite()) { return fail(format!("{f} gave a value that is not a finite number.")); }
    Ok(r)
}

// ---------- the sheet ----------

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct CheckOut { pub pass: bool, pub ratio: Option<f64>, pub lhs: String, pub rhs: String, pub op: String }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Series { pub label: String, pub ys: Vec<f64> }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct PlotOut { pub xs: Vec<f64>, pub xlabel: String, pub xunit: String, pub series: Vec<Series>, pub yunit: String }

/// A table region worked out: each column's heading (LaTeX), unit and cells as shown.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct TableOut { pub cols: Vec<ColOut>, pub rows: usize }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct ColOut { pub name: String, pub tex: String, pub unit: String, pub formula: String, pub cells: Vec<String>, pub error: String }

/// The data of a table region: columns with a name, a unit and either typed cells or a formula.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct TableData { pub cols: Vec<TableCol>, pub rows: Vec<Vec<String>> }

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct TableCol { pub name: String, pub unit: String, pub formula: String }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct ErrOut { pub msg: String, pub from: usize, pub to: usize }

/// What the window shows for one region.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct RegionOut {
    pub id: u32,
    /// The maths of what was typed.
    pub latex: String,
    /// "= 18.4 kW" as LaTeX, and as text for copying.
    pub result_tex: String,
    pub result: String,
    /// The value and unit alone (for copying a result).
    pub value: String,
    pub unit: String,
    /// The name defined (for the Variables panel and unit menus).
    pub name: String,
    /// The typed expression was a plain input (a number with a unit), so the result repeats it.
    pub input: bool,
    pub error: Option<ErrOut>,
    pub warn: String,
    pub check: Option<CheckOut>,
    pub plot: Option<PlotOut>,
    pub table: Option<TableOut>,
    /// Names a table defines (one per column), for the Variables panel.
    pub defs: Vec<VarOut>,
    /// Units this result can be shown in (same dimension), for the unit menu.
    pub units: Vec<String>,
    /// The description after `#` on the line.
    pub note: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VarOut { pub name: String, pub tex: String, pub value: String, pub unit: String, pub id: u32 }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct SheetOut { pub regions: Vec<RegionOut>, pub vars: Vec<VarOut>, pub checks: usize, pub failed: usize, pub errors: usize }

pub struct Fmt { pub digits: u8, pub eng: bool, pub us: bool }

/// A number with `digits` significant figures, as text and as LaTeX. Very large or small numbers
/// (or every number with `eng`) use engineering notation: a power of ten in steps of three.
pub fn fmt_num(x: f64, digits: u8, eng: bool) -> (String, String) {
    if !x.is_finite() { return ("—".into(), "\\text{—}".into()); }
    if x == 0.0 { return ("0".into(), "0".into()); }
    let a = x.abs();
    if eng || !(1e-3..1e6).contains(&a) {
        let mut e3 = (a.log10() / 3.0).floor() as i32 * 3;
        let mut m = x / 10f64.powi(e3);
        let mt = fmt_plain(m, digits);
        if mt.trim_start_matches('-').starts_with("1\u{2009}000") || mt.trim_start_matches('-').starts_with("1000") { e3 += 3; m /= 1000.0; }
        let mt = fmt_plain(m, digits);
        if e3 == 0 { return (mt.clone(), mt); }
        let sup: String = e3.to_string().chars().map(|c| match c { '-' => '⁻', '0' => '⁰', '1' => '¹', '2' => '²', '3' => '³', '4' => '⁴', '5' => '⁵', '6' => '⁶', '7' => '⁷', '8' => '⁸', _ => '⁹' }).collect();
        return (format!("{mt}×10{sup}"), format!("{mt} \\times 10^{{{e3}}}"));
    }
    let t = fmt_plain(x, digits);
    if t.replace('\u{2009}', "").parse::<f64>().is_ok_and(|r| r.abs() >= 1e6) { return fmt_num(x, digits, true); }
    (t.clone(), t.replace('\u{2009}', "\\,"))
}

/// Significant figures without exponent, trailing zeros removed; whole parts of five digits or more
/// are grouped in threes with a thin space (274 900).
pub fn fmt_plain(x: f64, digits: u8) -> String {
    if x == 0.0 || !x.is_finite() { return if x.is_finite() { "0".into() } else { "—".into() }; }
    let mag = x.abs().log10().floor() as i32;
    let dec = (digits as i32 - 1 - mag).clamp(0, 15) as usize;
    // Large numbers keep only their significant figures: 274 863 → 274 900 with 4.
    let x = if mag >= digits as i32 { let q = 10f64.powi(mag - digits as i32 + 1); (x / q).round() * q } else { x };
    let s = format!("{:.*}", dec, x);
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { return "0".into(); }
    let (sign, rest) = s.strip_prefix('-').map(|r| ("-", r)).unwrap_or(("", &s));
    let (int, frac) = rest.split_once('.').map(|(a, b)| (a, Some(b))).unwrap_or((rest, None));
    if int.len() < 5 { return s.clone(); }
    let mut g = String::new();
    for (i, ch) in int.chars().enumerate() { if i > 0 && (int.len() - i) % 3 == 0 { g.push('\u{2009}'); } g.push(ch); }
    format!("{sign}{g}{}", frac.map(|f| format!(".{f}")).unwrap_or_default())
}

/// A value in the unit it is shown in: (text, LaTeX, unit label).
pub fn show_val(v: &Val, want: &str, f: &Fmt) -> (String, String, String, String) {
    if v.logic {
        let t = if v.vec { format!("[{}]", v.v.iter().take(8).map(|x| if *x != 0.0 { "true" } else { "false" }).collect::<Vec<_>>().join(", ")) } else if v.x() != 0.0 { "true".into() } else { "false".into() };
        return (t.clone(), format!("\\text{{{t}}}"), String::new(), String::new());
    }
    let big = v.v.iter().copied().filter(|x| x.is_finite()).fold(0.0f64, |m, x| m.max(x.abs()));
    let (k, off, label) = units::display(big, &v.dim, v.abs, want, f.us);
    let conv = |x: f64| (x - off) / k;
    let (text, tex) = if v.vec {
        let n = v.v.len();
        let pick: Vec<f64> = if n <= 6 { v.v.clone() } else { vec![v.v[0], v.v[1], v.v[2], v.v[n - 1]] };
        let parts: Vec<(String, String)> = pick.iter().map(|x| fmt_num(conv(*x), f.digits, f.eng)).collect();
        let join = |i: usize| -> String {
            let p: Vec<&str> = parts.iter().map(|x| if i == 0 { x.0.as_str() } else { x.1.as_str() }).collect();
            if n <= 6 { p.join(", ") } else if i == 0 { format!("{}, {}, {}, … {}", p[0], p[1], p[2], p[3]) } else { format!("{}, {}, {}, \\ldots, {}", p[0], p[1], p[2], p[3]) }
        };
        (format!("[{}]", join(0)), format!("\\left[{}\\right]", join(1)))
    } else { fmt_num(conv(v.x()), f.digits, f.eng) };
    let unit_tex = latex::unit(&label);
    let full_t = if label.is_empty() { text.clone() } else { format!("{text} {label}") };
    let full_x = if label.is_empty() { tex } else { format!("{tex}\\,{unit_tex}") };
    (full_t, full_x, text, label)
}

/// Units the window offers for a result: every unit of the same kind, plus the automatic choices.
fn unit_choices(dim: &Dim, abs: bool) -> Vec<String> {
    if units::is_none(dim) { return if dim[7] != 0 { vec!["°".into(), "rad".into()] } else { vec![] }; }
    const ALIAS: [&str; 19] = ["um", "us", "uF", "l", "ml", "lbm", "Btu", "degR", "degC", "degF", "deltaC", "deltaF", "delta°C", "delta°F", "tonne", "hr", "knot", "ohm", "kohm"];
    let mut v: Vec<String> = vec![];
    if abs && units::same(dim, &TEMP) { v.push("°C".into()); v.push("°F".into()); }
    v.extend(units::UNITS.iter().filter(|u| units::same(&u.dim, dim) && !ALIAS.contains(&u.sym)).map(|u| u.sym.to_string()));
    for us in [false, true] {
        let (_, _, l) = units::display(1.0, dim, abs, "", us);
        if !l.is_empty() && !v.iter().any(|x| units::pretty(x) == l) { v.push(l); }
    }
    v
}

/// A name as shown in messages: σ_allow.
pub fn show(name: &str) -> String { name.to_string() }

/// Evaluates the whole sheet, top to bottom.
pub fn run(s: &Sheet) -> SheetOut {
    let f = Fmt { digits: s.digits, eng: false, us: s.units == "US" };
    run_with(s, &f)
}

pub fn run_with(s: &Sheet, f: &Fmt) -> SheetOut { run_env(s, f).0 }

/// The sheet worked out, with the final symbol table (the packs' tests read values from it).
pub fn run_env(s: &Sheet, f: &Fmt) -> (SheetOut, Env) {
    let mut env = Env::new();
    // Names defined anywhere, so a name used too early gets a helpful message.
    for r in &s.regions {
        if r.kind == Kind::Table {
            for c in serde_json::from_value::<TableData>(r.data.clone()).unwrap_or_default().cols { if !c.name.trim().is_empty() { env.later.insert(canon(c.name.trim()), false); } }
        } else if let Some(n) = defined_name(split_note(&r.text).0) { env.later.insert(canon(&n), false); }
    }
    let mut out = SheetOut::default();
    let mut known: Vec<String> = vec![];
    for r in &s.regions {
        let o = region(&mut env, r, f, &mut known);
        if let Some(c) = &o.check { out.checks += 1; if !c.pass { out.failed += 1; } }
        if o.error.is_some() { out.errors += 1; if !o.name.is_empty() { env.later.insert(canon(&o.name), true); } }
        for v in &o.defs { out.vars.retain(|x| x.name != v.name); out.vars.push(v.clone()); }
        if let Some(t) = &o.table { for c in t.cols.iter().filter(|c| !c.error.is_empty() && !c.name.is_empty()) { env.later.insert(canon(&c.name), true); } }
        if o.error.is_none() && !o.name.is_empty() && !o.value.is_empty() {
            out.vars.retain(|v| v.name != o.name);
            out.vars.push(VarOut { name: o.name.clone(), tex: latex::name(&o.name), value: o.value.clone(), unit: o.unit.clone(), id: r.id });
        }
        out.regions.push(o);
    }
    (out, env)
}

/// The name a line defines (`x = …`, `f(x) = …`, `solve … for x in …`), without parsing it fully.
fn defined_name(t: &str) -> Option<String> {
    let toks = parse::lex(t).ok()?;
    match (toks.first().map(|x| &x.0), toks.get(1).map(|x| &x.0)) {
        (Some(parse::Tok::Id(w)), _) if w == "solve" => toks.iter().position(|x| x.0 == parse::Tok::Id("for".into())).and_then(|i| match toks.get(i + 1) { Some((parse::Tok::Id(n), _)) => Some(n.clone()), _ => None }),
        (Some(parse::Tok::Id(n)), Some(parse::Tok::Op("=" | "("))) if n != "check" && n != "plot" => Some(n.clone()),
        _ => None,
    }
}

fn region(env: &mut Env, r: &Region, f: &Fmt, known: &mut Vec<String>) -> RegionOut {
    let mut o = RegionOut { id: r.id, ..Default::default() };
    if r.kind == Kind::Table { table(env, r, f, known, &mut o); return o; }
    if !matches!(r.kind, Kind::Math | Kind::Check | Kind::Solve | Kind::Plot) || r.text.trim().is_empty() { return o; }
    // Text after # describes the line: `Q = 120 m^3/h  # design flow`.
    let (code, note) = split_note(&r.text);
    o.note = note.into();
    let st = match parse::statement(code, known) {
        Ok(s) => s,
        Err(e) => {
            o.error = Some(ErrOut { msg: e.msg, from: e.at.0, to: e.at.1 });
            o.latex = latex::escape_text(code);
            if let Some(n) = defined_name(code) { o.name = canon(&n); }
            return o;
        }
    };
    if r.kind == Kind::Check && !matches!(st, Stmt::Check { .. } | Stmt::Empty) {
        o.latex = latex::stmt(&st);
        o.error = Some(ErrOut { msg: "A check starts with “check”, such as check sigma <= sigma_allow.".into(), from: 0, to: code.chars().count() });
        return o;
    }
    o.latex = latex::stmt(&st);
    let res: R<()> = (|| {
        match &st {
            Stmt::Empty => {}
            Stmt::Assign { name, e, .. } => {
                o.name = canon(name);
                let v = env.eval(e)?;
                o.input = matches!(e.e, E::Num(..)) || matches!(&e.e, E::Neg(x) if matches!(x.e, E::Num(..)));
                // An input keeps the unit it was typed in (8356 cm⁴ stays cm⁴), unless it belongs to the other unit system.
                let typed = match &e.e { E::Num(_, Some(u)) => Some(u.text.clone()), E::Neg(x) => match &x.e { E::Num(_, Some(u)) => Some(u.text.clone()), _ => None }, _ => None };
                match typed.filter(|t| r.unit.trim().is_empty() && units::system(t).is_none_or(|us| us == f.us)) {
                    Some(t) => set_result(&mut o, &v, &Region { unit: t, ..r.clone() }, f),
                    None => set_result(&mut o, &v, r, f),
                }
                env.vars.insert(canon(name), v);
                known.push(name.clone());
            }
            Stmt::Func { name, params, e } => {
                o.name = canon(name);
                env.funcs.insert(canon(name), (params.clone(), e.clone()));
                known.push(name.clone());
            }
            Stmt::Range { name, start, next, end, .. } => {
                o.name = canon(name);
                let (a, b) = (env.eval(start)?, env.eval(end)?);
                // A bare 0 at the start takes the unit of the rest: L = 0, 1 m .. 3 m.
                let a = if a.x() == 0.0 && units::is_none(&a.dim) { Val { dim: b.dim, ..a } } else { a };
                let st = match next { Some(n) => { let c = env.eval(n)?; same_or(&a, &c, "use", n.at)?; bin("-", c, a.clone(), n.at, n.at)? } None => Val::num(1.0, a.dim) };
                let st = if units::is_none(&st.dim) && !units::is_none(&b.dim) && next.is_none() { return fail("Give the step with its unit: x = 0, 10 m .. 100 m.", end.at) } else { st };
                same_or(&a, &b, "use", end.at)?;
                if st.x() == 0.0 || (b.x() - a.x()) / st.x() < 0.0 { return fail("The step must go from the first value towards the last.", end.at); }
                let n = ((b.x() - a.x()) / st.x() + 1e-9).floor() as usize + 1;
                if n > 100_000 { return fail("A range holds at most 100,000 values.", end.at); }
                let v = Val { abs: a.abs, ..Val::vector((0..n).map(|i| a.x() + st.x() * i as f64).collect(), a.dim) };
                set_result(&mut o, &v, r, f);
                env.vars.insert(canon(name), v);
                known.push(name.clone());
            }
            Stmt::Check { e } => {
                let E::Bin(op, a, b) = &e.e else { return fail("A check compares two values: check a <= b.", e.at) };
                if !matches!(*op, "<" | "<=" | ">" | ">=" | "==" | "!=") { return fail("A check compares two values with <, <=, >, >= or ==.", e.at); }
                let (x, y) = (env.eval(a)?, env.eval(b)?);
                let t = bin(op, x.clone(), y.clone(), e.at, b.at)?;
                if t.vec { return fail("A check needs single values, not vectors (use max or min).", e.at); }
                let pass = t.x() != 0.0;
                let ratio = match *op { "<" | "<=" if y.x() != 0.0 => Some(x.x() / y.x()), ">" | ">=" if x.x() != 0.0 => Some(y.x() / x.x()), _ => None };
                let (lt, _, _, lu) = show_val(&x, &r.unit, f);
                let (rt, _, _, _) = show_val(&y, &lu, f);
                o.check = Some(CheckOut { pass, ratio, lhs: lt, rhs: rt, op: op.to_string() });
                let sym = match *op { "<=" => "≤", ">=" => "≥", "!=" => "≠", "==" => "=", o => o };
                o.result = format!("{} ({} {sym} {})", if pass { "PASS" } else { "FAIL" }, o.check.as_ref().unwrap().lhs, o.check.as_ref().unwrap().rhs);
            }
            Stmt::Solve { lhs, rhs, var, lo, hi } => {
                o.name = canon(var);
                let (a, b) = (env.eval(lo)?, env.eval(hi)?);
                same_or(&a, &b, "use", hi.at)?;
                let k = canon(var);
                let saved = env.vars.get(&k).cloned();
                let mut first_err: Option<Error> = None;
                let mut fx = |x: f64| -> f64 {
                    env.vars.insert(k.clone(), Val { v: vec![x], ..a.clone() });
                    match env.eval(lhs).and_then(|l| { let rr = env.eval(rhs)?; same_or(&l, &rr, "compare", rhs.at)?; Ok(l.x() - rr.x()) }) {
                        Ok(v) => v,
                        Err(e) => { if first_err.is_none() { first_err = Some(e); } f64::NAN }
                    }
                };
                let root = crate::solve::root(&mut fx, a.x(), b.x());
                match saved { Some(v) => { env.vars.insert(k.clone(), v); } None => { env.vars.remove(&k); } }
                let x = match root {
                    Ok(x) => x,
                    Err(msg) => { if let Some(e) = first_err { return Err(e); } let (lt, ..) = show_val(&a, "", f); let (ht, ..) = show_val(&b, "", f); return fail(format!("{msg} between {lt} and {ht}."), (lo.at.0, hi.at.1)); }
                };
                let v = Val { v: vec![x], ..a.clone() };
                set_result(&mut o, &v, r, f);
                o.result_tex = format!("\\Rightarrow\\; {} = {}", latex::name(var), o.result_tex);
                env.vars.insert(k, v);
                known.push(var.clone());
            }
            Stmt::Plot { ys, var, lo, hi } => {
                let (a, b) = (env.eval(lo)?, env.eval(hi)?);
                same_or(&a, &b, "use", hi.at)?;
                let k = canon(var);
                let saved = env.vars.get(&k).cloned();
                let n = 121;
                let xs: Vec<f64> = (0..n).map(|i| a.x() + (b.x() - a.x()) * i as f64 / (n - 1) as f64).collect();
                let mut p = PlotOut::default();
                let (_, _, _, xu) = show_val(&a, "", f);
                let (xk, xo, _) = units::display(a.x(), &a.dim, a.abs, &xu, f.us);
                p.xs = xs.iter().map(|x| (x - xo) / xk).collect();
                p.xlabel = show(var);
                p.xunit = xu;
                let mut ydim: Option<(Dim, bool)> = None;
                let mut res = Ok(());
                for y in ys {
                    let mut vals = vec![];
                    for x in &xs {
                        env.vars.insert(k.clone(), Val { v: vec![*x], ..a.clone() });
                        match env.eval(y) {
                            Ok(v) => {
                                if let Some((d, _)) = &ydim { if !units::same(d, &v.dim) { res = fail("Every curve on a plot must have the same units.", y.at); break; } } else { ydim = Some((v.dim, v.abs)); }
                                vals.push(if v.vec { f64::NAN } else { v.x() });
                            }
                            Err(_) => vals.push(f64::NAN),
                        }
                    }
                    if vals.iter().all(|v| v.is_nan()) { res = env.eval(y).map(|_| ()); if res.is_ok() { res = fail("This curve has no values in the range.", y.at); } }
                    p.series.push(Series { label: latex::plain(y), ys: vals });
                    if res.is_err() { break; }
                }
                match saved { Some(v) => { env.vars.insert(k, v); } None => { env.vars.remove(&k); } }
                res?;
                if let Some((d, abs)) = ydim {
                    let big = p.series.iter().flat_map(|s| s.ys.iter()).copied().filter(|v| v.is_finite()).fold(0.0f64, |m, v| m.max(v.abs()));
                    let (yk, yo, yl) = units::display(big, &d, abs, &r.unit, f.us);
                    for s in &mut p.series { for v in &mut s.ys { *v = (*v - yo) / yk; } }
                    p.yunit = yl;
                }
                o.plot = Some(p);
            }
            Stmt::Show { e } => {
                let v = env.eval(e)?;
                set_result(&mut o, &v, r, f);
            }
        }
        Ok(())
    })();
    if let Err(e) = res {
        o.error = Some(ErrOut { msg: e.msg, from: e.at.0, to: e.at.1 });
        o.result.clear();
        o.result_tex.clear();
    }
    o
}

fn set_result(o: &mut RegionOut, v: &Val, r: &Region, f: &Fmt) {
    let (t, x, val, unit) = show_val(v, &r.unit, f);
    if !r.unit.trim().is_empty() && !v.logic {
        if let Ok(u) = units::parse(&r.unit) { if !units::same(&u.dim, &v.dim) { o.warn = format!("{} cannot show {}; shown in {}.", units::pretty(&r.unit), dim_name(&v.dim), if unit.is_empty() { "plain numbers".into() } else { unit.clone() }); } }
        else { o.warn = format!("“{}” is not a unit.", r.unit); }
    }
    o.result = t;
    o.result_tex = x;
    o.value = val;
    o.unit = unit;
    o.units = unit_choices(&v.dim, v.abs);
}

/// A table: typed columns become vectors (a cell without a unit takes the column's unit); formula
/// columns are worked out from the columns to their left and from names above, value by value.
fn table(env: &mut Env, r: &Region, f: &Fmt, known: &mut Vec<String>, o: &mut RegionOut) {
    let d: TableData = serde_json::from_value(r.data.clone()).unwrap_or_default();
    let n = d.rows.iter().rposition(|row| row.iter().any(|c| !c.trim().is_empty())).map(|i| i + 1).unwrap_or(0);
    let mut t = TableOut { cols: vec![], rows: n };
    let mut first_err: Option<String> = None;
    for (j, c) in d.cols.iter().enumerate() {
        let name = c.name.trim();
        let mut co = ColOut { name: name.into(), tex: if name.is_empty() { String::new() } else { latex::name(name) }, unit: String::new(), formula: c.formula.clone(), cells: vec![String::new(); n], error: String::new() };
        let unit = if c.unit.trim().is_empty() { None } else { match units::parse(c.unit.trim()) { Ok(u) => Some(u), Err(e) => { co.error = e; None } } };
        let val: Result<Val, String> = if !c.formula.trim().is_empty() {
            let src = c.formula.trim().trim_start_matches('=');
            parse::statement(src, known).map_err(|e| e.msg).and_then(|st| match st { Stmt::Show { e } => env.eval(&e).map_err(|e| e.msg), _ => Err("A formula column holds an expression, such as rho*g*Q*H.".into()) })
                .map(|v| if v.vec || n == 0 { v } else { Val { v: vec![v.x(); n], vec: true, ..v } })
        } else {
            let mut vals = Vec::with_capacity(n);
            let mut dim: Option<units::Dim> = None;
            let mut res = Ok(());
            let mut abs = false;
            for (i, row) in d.rows.iter().take(n).enumerate() {
                let cell = row.get(j).map(|x| x.trim()).unwrap_or("");
                if cell.is_empty() { vals.push(f64::NAN); continue; }
                let v = match parse::statement(cell, known) { Ok(Stmt::Show { e }) => env.eval(&e).map_err(|e| e.msg), Ok(_) => Err("a cell holds a value".into()), Err(e) => Err(e.msg) };
                let v = match (v, &unit) {
                    (Ok(v), Some(u)) if units::is_none(&v.dim) && !units::is_none(&u.dim) => Ok(Val { v: vec![v.x() * u.factor + u.offset], dim: u.dim, abs: u.offset != 0.0, vec: false, logic: false }),
                    (v, _) => v,
                };
                match v {
                    Ok(v) if v.vec => { res = Err(format!("Row {}: a cell holds one value.", i + 1)); break; }
                    Ok(v) => {
                        if let Some(d0) = &dim { if !units::same(d0, &v.dim) { res = Err(format!("Row {}: {} does not match the rest of the column.", i + 1, dim_name(&v.dim))); break; } } else { dim = Some(v.dim); abs = v.abs; }
                        vals.push(v.x());
                    }
                    Err(m) => { res = Err(format!("Row {}: {m}", i + 1)); break; }
                }
            }
            res.map(|_| Val { v: vals, dim: dim.or(unit.as_ref().map(|u| u.dim)).unwrap_or(units::NONE), abs, vec: true, logic: false })
        };
        match val {
            Ok(v) => {
                if v.v.len() != n && n > 0 { co.error = format!("{} values for {} rows.", v.v.len(), n); }
                let big = v.v.iter().copied().filter(|x| x.is_finite()).fold(0.0f64, |m, x| m.max(x.abs()));
                let (k, off, label) = units::display(big, &v.dim, v.abs, &c.unit, f.us);
                co.unit = label;
                co.cells = (0..n).map(|i| v.v.get(i).filter(|x| x.is_finite()).map(|x| fmt_num((x - off) / k, f.digits, f.eng).0).unwrap_or_default()).collect();
                if !name.is_empty() && co.error.is_empty() {
                    let (_, _, value, unit) = show_val(&v, &c.unit, f);
                    o.defs.push(VarOut { name: canon(name), tex: latex::name(name), value, unit, id: r.id });
                    env.vars.insert(canon(name), v);
                    known.push(name.into());
                }
            }
            Err(m) => co.error = m,
        }
        if !co.error.is_empty() && first_err.is_none() { first_err = Some(format!("{}: {}", if name.is_empty() { format!("Column {}", j + 1) } else { name.to_string() }, co.error)); }
        t.cols.push(co);
    }
    if let Some(m) = first_err { o.error = Some(ErrOut { msg: m, from: 0, to: 0 }); }
    o.table = Some(t);
}

/// The code of a line and its description after `#`.
pub fn split_note(t: &str) -> (&str, &str) {
    match t.split_once('#') { Some((c, n)) => (c, n.trim()), None => (t, "") }
}
