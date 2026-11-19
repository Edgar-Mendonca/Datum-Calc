//! Evaluation: the sheet top to bottom with a symbol table. Each region gets its own result or its
//! own error; an error never stops the rest of the sheet.
use crate::latex;
use crate::parse::{self, Error, Node, Span, Stmt, E};
use crate::sheet::{Kind, Region, Sheet};
use crate::units::{self, Dim, NONE, TEMP};
use serde::Serialize;
use std::collections::HashMap;

/// A value: one number or a vector, in SI base units, with its dimension. `abs` marks an absolute
/// temperature (given in °C or °F); `logic` marks the result of a comparison. `im` holds the imaginary
/// parts of a complex value (empty when it is real); `nm` marks a force times a length, shown in N·m;
/// `pi` is the power of π in a plain number (2π: 1, 1/(2π): −1); `w` marks a frequency made with it (ω = 2πf), shown in rad/s.
/// Neither changes the dimension, so nothing worked out from ω (ωτ, ωL/R) is taken for an angle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Val { pub v: Vec<f64>, pub dim: Dim, pub abs: bool, pub vec: bool, pub logic: bool, pub im: Vec<f64>, pub nm: bool, pub pi: i8, pub w: bool, pub rows: usize }

impl Val {
    pub fn num(x: f64, dim: Dim) -> Self { Self { v: vec![x], dim, ..Default::default() } }
    pub fn plain(x: f64) -> Self { Self::num(x, NONE) }
    fn vector(v: Vec<f64>, dim: Dim) -> Self { Self { v, dim, vec: true, ..Default::default() } }
    /// The first value (a scalar's value); NaN for an empty vector, never a panic.
    pub fn x(&self) -> f64 { self.v.first().copied().unwrap_or(f64::NAN) }
    pub fn cx(&self) -> bool { !self.im.is_empty() }
    /// A matrix keeps its entries row by row in `v`, with `rows` rows (0 for a number or a vector).
    pub fn mat(&self) -> bool { self.rows > 0 }
    /// (rows, columns): a vector is a column, a number is 1 × 1.
    pub fn shape(&self) -> (usize, usize) { if self.rows > 0 { (self.rows, self.v.len() / self.rows) } else if self.vec { (self.v.len(), 1) } else { (1, 1) } }
    /// Value k as (real, imaginary); a scalar gives its one value for every k.
    fn at(&self, k: usize) -> (f64, f64) { let i = if self.vec { k } else { 0 }; (self.v.get(i).copied().unwrap_or(f64::NAN), self.im.get(i).copied().unwrap_or(0.0)) }
    /// A complex value whose imaginary parts are negligible is real (j² = −1, not −1 + 0j).
    /// A real part that is round-off beside its imaginary part is zero (polar(1, 90°) is 1j).
    fn tidy(mut self) -> Self {
        if self.im.iter().zip(&self.v).all(|(b, a)| b.abs() <= 1e-13 * a.abs() || *b == 0.0) { self.im.clear(); }
        for (a, b) in self.v.iter_mut().zip(&self.im) { if a.abs() <= 1e-13 * b.abs() { *a = 0.0; } }
        self
    }
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
        "π" => Val { pi: 1, ..Val::plain(std::f64::consts::PI) },
        "e" => Val::plain(std::f64::consts::E),
        "g" => Val::num(9.806_65, [1, 0, -2, 0, 0, 0, 0, 0]),
        "R" => Val::num(8.314_462_618, [2, 1, -2, -1, 0, -1, 0, 0]),
        "σ_SB" => Val::num(5.670_374_419e-8, [0, 1, -3, -4, 0, 0, 0, 0]),
        "N_A" => Val::num(6.022_140_76e23, [0, 0, 0, 0, 0, -1, 0, 0]),
        "k_B" => Val::num(1.380_649e-23, [2, 1, -2, -1, 0, 0, 0, 0]),
        "c_0" => Val::num(299_792_458.0, [1, 0, -1, 0, 0, 0, 0, 0]),
        "g_c" => Val::plain(1.0),
        "i" | "j" => Val { im: vec![1.0], ..Val::plain(0.0) },
        _ => return None,
    })
}
pub const CONSTANTS: [(&str, &str); 9] = [("π", "pi, 3.14159…"), ("j", "the imaginary unit √−1 (also i); 4j is 4 times j"), ("e", "Euler's number, 2.71828…"), ("g", "standard gravity, 9.80665 m/s²"), ("R", "molar gas constant, 8.314462618 J/(mol·K)"),
    ("σ_SB", "Stefan–Boltzmann constant, 5.670374419×10⁻⁸ W/(m²·K⁴)"), ("N_A", "Avogadro constant, 6.02214076×10²³ /mol"), ("k_B", "Boltzmann constant, 1.380649×10⁻²³ J/K"), ("c_0", "speed of light, 299792458 m/s")];

/// Built-in functions: name, arguments, what it does (for help and the window's menus).
pub const FUNCTIONS: [(&str, &str); 47] = [
    ("sqrt(x)", "square root"), ("cbrt(x)", "cube root"), ("abs(x)", "absolute value"), ("exp(x)", "e to the power x"), ("ln(x)", "natural logarithm"), ("log(x)", "logarithm base 10"),
    ("sin(x)", "sine (x in rad, or with deg)"), ("cos(x)", "cosine"), ("tan(x)", "tangent"), ("asin(x)", "inverse sine (an angle)"), ("acos(x)", "inverse cosine"), ("atan(x)", "inverse tangent"),
    ("atan2(y, x)", "angle of the point (x, y)"), ("sinh(x)", "hyperbolic sine"), ("cosh(x)", "hyperbolic cosine"), ("tanh(x)", "hyperbolic tangent"),
    ("min(a, b, …)", "smallest value"), ("max(a, b, …)", "largest value"), ("round(x, n)", "x rounded to n decimals"), ("floor(x)", "round down"), ("ceil(x)", "round up"),
    ("if(test, a, b)", "a when the test holds, otherwise b"), ("sum(v)", "sum of a vector"), ("mean(v)", "mean of a vector"), ("len(v)", "number of values"),
    ("interp(x, xs, ys)", "straight-line interpolation in a table"), ("hypot(a, b)", "√(a² + b²)"),
    ("re(z)", "real part of a complex value"), ("im(z)", "imaginary part"), ("arg(z)", "angle of a complex value"), ("conj(z)", "complex conjugate"),
    ("polar(r, θ)", "the complex value r∠θ; also written r∠θ (or r/_θ)"), ("par(a, b, …)", "values in parallel: 1/(1/a + 1/b + …), for impedances"),
    ("db(x)", "decibels of a ratio: 20·log₁₀|x|"),
    ("simplify(e)", "an expression in lowest terms: like terms collected, fractions cancelled"), ("expand(e)", "an expression multiplied out: (x + 1)² → x² + 2x + 1"),
    ("factor(e)", "an expression as a product: x² − 1 → (x − 1)(x + 1)"),
    ("transpose(M)", "rows and columns swapped"), ("inv(M)", "inverse of a square matrix (also M^-1)"), ("det(M)", "determinant of a square matrix"),
    ("lsolve(A, b)", "x with A x = b, a linear system"), ("identity(n)", "the n × n identity matrix"), ("dot(a, b)", "dot product of two vectors"),
    ("cross(a, b)", "cross product of two 3-vectors"), ("norm(v)", "length of a vector (Frobenius norm of a matrix)"), ("rows(M)", "number of rows"), ("cols(M)", "number of columns"),
];

pub struct Env {
    pub vars: HashMap<String, Val>,
    pub funcs: HashMap<String, (Vec<String>, Node)>,
    /// The unit each name is shown in on its own line, so substituted values read the same.
    pub shown: HashMap<String, String>,
    /// Names that hold an exact expression from simplify, expand or factor (y = expand((x+1)^2)).
    pub exprs: HashMap<String, crate::sym::RatF>,
    /// Functions written as programs (0.2.5).
    pub progs: HashMap<String, std::sync::Arc<crate::prog::Prog>>,
    /// Times round the loops of program functions on the current line (a plot calls them once per point).
    steps: u64,
    /// Names that are defined further down the sheet, or whose own line has an error.
    later: HashMap<String, bool>,
    depth: u32,
}

impl Env {
    pub fn new() -> Self { Self { vars: HashMap::new(), funcs: HashMap::new(), shown: HashMap::new(), exprs: HashMap::new(), progs: HashMap::new(), steps: 0, later: HashMap::new(), depth: 0 } }

    /// What a name is for exact maths: a plain number (as a fraction), a kept expression, or a symbol.
    fn look(&self, v: &str) -> crate::sym::Look {
        use crate::sym::Look;
        let k = canon(v);
        if let Some(r) = self.exprs.get(&k) { return Look::Expr(r.clone()); }
        // A name the sheet would refuse on a numeric line is refused here too, rather than taken for a symbol.
        if !self.vars.contains_key(&k) {
            if self.funcs.contains_key(&k) || self.progs.contains_key(&k) || FUNCTIONS.iter().any(|f| f.0.split('(').next() == Some(k.as_str())) { return Look::Refuse(format!("{k} is a function; exact maths works on names and numbers.")); }
            match self.later.get(&k) {
                Some(true) => return Look::Refuse(format!("{k} has an error on its own line.")),
                Some(false) => return Look::Refuse(format!("{k} is defined further down: move its line above this one.")),
                None => {}
            }
        }
        let val = self.vars.get(&k).cloned().or_else(|| if matches!(k.as_str(), "π" | "e") { None } else { constant(&k) });
        match val {
            None => Look::Symbol,
            Some(x) if x.cx() => Look::Refuse(format!("{k} is complex; exact maths works on real numbers.")),
            Some(x) if x.vec => Look::Refuse(format!("{k} is a vector; exact maths works on single values.")),
            Some(x) if !units::is_none(&x.dim) => Look::Refuse(format!("{k} has units; exact maths works on plain numbers and symbols.")),
            Some(x) => crate::sym::fraction(x.x(), 1_000_000).map(Look::Value).unwrap_or_else(|| Look::Refuse(format!("{k} = {} is not a fraction exact maths can keep.", fmt_plain(x.x(), 6)))),
        }
    }

    fn var(&self, name: &str, at: Span) -> R<Val> {
        let k = canon(name);
        if let Some(v) = self.vars.get(&k) { return Ok(v.clone()); }
        if self.exprs.contains_key(&k) { return fail(format!("{k} is an expression, not a value: use it inside simplify, expand or factor, such as factor({k})."), at); }
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
                // A torque typed as a force times a length (50 N*m, 2 kN·m) keeps that reading in later lines; 50 J stays energy.
                Some(u) => Val { v: vec![x * u.factor + u.offset], dim: u.dim, abs: u.offset != 0.0, nm: units::same(&u.dim, &ENERGY) && u.text.contains('*'), ..Default::default() },
            }),
            E::Im(_, e) => { let v = self.eval(e)?; if v.abs { return fail("A temperature cannot be imaginary.", n.at); } Ok(Val { im: v.v.clone(), v: vec![0.0; v.v.len()], ..v }.tidy()) }
            E::Mat(matrix, rows) => {
                // Every entry a single real value with the same units; a matrix is kept row by row.
                let (mut v, mut dim): (Vec<f64>, Option<Dim>) = (vec![], None);
                if rows.len() * rows[0].len() > 10_000 { return fail("A matrix holds at most 10,000 values.", n.at); }
                for x in rows.iter().flatten() {
                    let e = self.eval(x)?;
                    real(&e, "A matrix", x.at)?;
                    if e.vec { return fail("An entry of a matrix is one value.", x.at); }
                    if e.abs { return fail(ABS_MAT, x.at); }
                    match dim { Some(d) if !units::same(&d, &e.dim) => return fail(format!("Every entry needs the same units: this is {}, the first is {}.", dim_name(&e.dim), dim_name(&d)), x.at), None => dim = Some(e.dim), _ => {} }
                    v.push(e.x());
                }
                Ok(Val { v, dim: dim.unwrap_or(NONE), vec: true, rows: if *matrix { rows.len() } else { 0 }, ..Default::default() })
            }
            E::Index(a, idx) => {
                let base = self.eval(a)?;
                if !self.eval(a)?.vec { return fail("Only a vector or a matrix has entries.", n.at); }
                let ix = idx.iter().map(|x| { let v = self.eval(x)?; if !units::is_none(&v.dim) || v.cx() || v.vec || v.x().fract() != 0.0 || v.x() < 1.0 { return fail("An index is a whole number from 1.", x.at); } Ok(v.x() as usize) }).collect::<R<Vec<usize>>>()?;
                let (r, c) = base.shape();
                let k = match (base.mat(), ix.as_slice()) {
                    (true, [i, j]) if *i <= r && *j <= c => (i - 1) * c + j - 1,
                    (false, [i]) if base.vec && *i <= r => i - 1,
                    (true, [_, _]) | (false, [_]) => return fail(format!("Out of range: this has {r} row{}{}.", if r == 1 { "" } else { "s" }, if base.mat() { format!(" and {c} column{}", if c == 1 { "" } else { "s" }) } else { String::new() }), n.at),
                    (true, _) => return fail("A matrix entry needs a row and a column: M[1, 2].", n.at),
                    _ => return fail(if base.vec { "A vector entry needs one index: v[2]." } else { "Only a vector or a matrix has entries." }, n.at),
                };
                let (x, y) = base.at(k);
                Ok(Val { v: vec![x], im: if base.cx() { vec![y] } else { vec![] }, dim: base.dim, abs: base.abs, ..Default::default() })
            }
            E::Unit(e, u) => {
                let v = self.eval(e)?;
                // (3 + 4j) Ω gives a plain number its unit; it is not a conversion of a value that has one.
                if !units::is_none(&v.dim) { return fail(format!("A unit after brackets needs a plain number inside, not {}; to show a result in {}, choose it in the result's unit menu.", dim_name(&v.dim), units::pretty(&u.text)), n.at); }
                bin("*", v, Val::num(u.factor, u.dim), n.at, n.at)
            }
            E::Var(s) => self.var(s, n.at),
            E::Group(e) => self.eval(e),
            E::Neg(e) => { let mut v = self.eval(e)?; v.v.iter_mut().chain(v.im.iter_mut()).for_each(|x| *x = -*x); v.abs = false; v.logic = false; Ok(v) }
            E::Bin(op, a, b) => { let (x, y) = (self.eval(a)?, self.eval(b)?); bin(op, x, y, n.at, b.at) }
            E::Call(f, args) => self.call(f, args, n.at),
        }
    }

    fn call(&mut self, f: &str, args: &[Node], at: Span) -> R<Val> {
        let k = canon(f);
        if let Some(p) = self.progs.get(&k).cloned() {
            let params = &p.head.as_ref().expect("a function program").1;
            if params.len() != args.len() { return fail(format!("{f} takes {} value{}.", params.len(), if params.len() == 1 { "" } else { "s" }), at); }
            if self.depth > 40 { return fail(format!("{f} calls itself too deeply."), at); }
            let vals = args.iter().map(|a| self.eval(a)).collect::<R<Vec<_>>>()?;
            // Its parameters and the names it sets are its own: put back afterwards.
            let mut keys: Vec<String> = params.iter().chain(&p.sets).map(|n| canon(n)).collect();
            keys.dedup();
            let saved: Vec<(String, Option<Val>)> = keys.iter().map(|k| (k.clone(), self.vars.get(k).cloned())).collect();
            for (n, v) in params.iter().zip(vals) { self.vars.insert(canon(n), v); }
            self.depth += 1;
            let mut steps = 0;
            let r = crate::prog::exec(self, &p.body, &mut steps);
            self.depth -= 1;
            for (k, v) in saved { match v { Some(v) => { self.vars.insert(k, v); } None => { self.vars.remove(&k); } } }
            self.steps += u64::from(steps) + 1;
            if self.steps > 1_000_000 { return fail(format!("{f} ran more than 1,000,000 steps on this line: check its loops."), at); }
            return match r {
                Ok(crate::prog::Flow::Return(v)) => Ok(v),
                Ok(_) => fail(format!("{f} ended without return: give it a return line."), at),
                Err(e) => fail(if e.msg.starts_with("In ") { e.msg } else { format!("In {f}: {}", e.msg) }, at),
            };
        }
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
        if crate::sym::OPS.contains(&f) { return fail(format!("{f} gives an expression, not a value: write it on its own line, such as {f}((x + 1)^2) or y = {f}(…)."), at); }
        if f == "if" {
            want(3)?;
            let c = self.eval(&args[0])?;
            real(&c, "if", at)?;
            if !c.vec { return if c.x() != 0.0 { self.eval(&args[1]) } else { self.eval(&args[2]) }; }
            let (a, b) = (self.eval(&args[1])?, self.eval(&args[2])?);
            if !units::same(&a.dim, &b.dim) { return fail("Both choices of if must have the same units.", at); }
            if a.cx() || b.cx() { return fail("if over a vector needs real choices.", at); }
            let pick = |i: usize, v: &Val| if v.vec { v.v.get(i).copied().unwrap_or(f64::NAN) } else { v.x() };
            return Ok(Val { v: c.v.iter().enumerate().map(|(i, t)| if *t != 0.0 { pick(i, &a) } else { pick(i, &b) }).collect(), vec: true, ..a });
        }
        let vals = args.iter().map(|a| self.eval(a)).collect::<R<Vec<_>>>()?;
        builtin(f, vals, at)
    }
}

impl Default for Env { fn default() -> Self { Self::new() } }

/// Checks, comparisons, ranges, solve, plot and tables work on real values only.
pub fn real(v: &Val, what: &str, at: Span) -> R<()> {
    if v.cx() { return fail(format!("{what} needs a real value, not a complex one: use abs, re, im or arg."), at); }
    Ok(())
}

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
    named.iter().find(|x| units::same(&x.1, d)).map(|x| x.0.to_string()).unwrap_or_else(|| format!("a value in {}", units::label(d)))
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
    let r = bin_raw(op, a, b, at, rat)?;
    if r.v.iter().chain(&r.im).any(|x| !x.is_finite()) { return fail("The result is too large to hold (beyond about 10³⁰⁸) or not a number.", at); }
    if r.dim.iter().any(|e| e.unsigned_abs() > 60) { return fail("Units with powers above 60 cannot be worked out.", at); }
    Ok(r.tidy())
}

const FORCE: Dim = [1, 1, -2, 0, 0, 0, 0, 0];
const ENERGY: Dim = [2, 1, -2, 0, 0, 0, 0, 0];

/// A force (or a load per length) times a length reads as a moment in N·m rather than J; a plain
/// factor keeps that reading (F·d/2), and so does adding two moments.
fn moment(op: &str, a: &Val, b: &Val, dim: &Dim) -> bool {
    let force = |v: &Val| units::same(&v.dim, &FORCE) || units::same(&v.dim, &[0, 1, -2, 0, 0, 0, 0, 0]);
    match op {
        "+" | "-" => a.nm || b.nm,
        "*" => units::same(dim, &ENERGY) && dim[7] == 0 && (force(a) || force(b) || (a.nm && units::is_none(&b.dim)) || (b.nm && units::is_none(&a.dim))),
        "/" => a.nm && units::is_none(&b.dim) && dim[7] == 0,
        _ => false,
    }
}

/// A matrix from its shape and entries: n × 1 is a vector, 1 × 1 a number.
const ABS_MAT: &str = "A matrix cannot hold absolute temperatures; use differences (K).";

fn mat_val(r: usize, c: usize, v: Vec<f64>, dim: Dim) -> Val {
    match (r, c) { (1, 1) => Val::num(v[0], dim), (_, 1) => Val { v, dim, vec: true, ..Default::default() }, _ => Val { v, dim, vec: true, rows: r, ..Default::default() } }
}

/// The matrix product of (m × k) and (k × n), entries row by row.
fn matmul(a: &[f64], (m, k): (usize, usize), b: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![0.0; m * n];
    for i in 0..m { for t in 0..k { let x = a[i * k + t]; if x != 0.0 { for j in 0..n { out[i * n + j] += x * b[t * n + j]; } } } }
    out
}

/// The rows of a square matrix, for the LU factorisation.
fn rows_of(a: &[f64], n: usize) -> Vec<Vec<f64>> { (0..n).map(|i| a[i * n..(i + 1) * n].to_vec()).collect() }

/// The inverse of a square matrix from one LU factorisation; None when singular.
fn inverse(a: &[f64], n: usize) -> Option<Vec<f64>> {
    let lu = crate::solve::Lu::new(rows_of(a, n))?;
    let mut out = vec![0.0; n * n];
    for j in 0..n {
        let col = lu.solve(&(0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect::<Vec<_>>())?;
        for i in 0..n { out[i * n + j] = col[i]; }
    }
    Some(out)
}

/// Matrix functions work on differences, not absolute temperatures.
fn no_abs(a: &[Val], at: Span) -> R<()> { if a.iter().any(|v| v.abs) { fail(ABS_MAT, at) } else { Ok(()) } }

/// Arithmetic with a matrix (+ − × ÷ and powers): entry by entry with a number or a matrix of the same shape; × of two is the
/// matrix product (rows of the first by columns of the second); a square matrix to a whole power, −1 its inverse.
fn mat_bin(op: &str, a: &Val, b: &Val, at: Span, rat: Span) -> R<Val> {
    if a.cx() || b.cx() { return fail("Matrices of complex values are not supported.", at); }
    if a.abs || b.abs { return fail(ABS_MAT, at); }
    let (sa, sb) = (a.shape(), b.shape());
    let scalar = |v: &Val| !v.vec;
    let each = |f: &dyn Fn(f64, f64) -> f64, dim: Dim| -> R<Val> {
        let (r, c) = if scalar(a) { sb } else { sa };
        if !scalar(a) && !scalar(b) && sa != sb { return fail(format!("These have different shapes ({}×{} and {}×{}).", sa.0, sa.1, sb.0, sb.1), at); }
        let v = (0..r * c).map(|k| f(if scalar(a) { a.x() } else { a.v[k] }, if scalar(b) { b.x() } else { b.v[k] })).collect();
        Ok(mat_val(r, c, v, dim))
    };
    match op {
        "+" | "-" => { same_or(a, b, if op == "+" { "add" } else { "subtract" }, at)?; each(&|x, y| if op == "+" { x + y } else { x - y }, a.dim) }
        "*" if scalar(a) || scalar(b) => each(&|x, y| x * y, units::mul(&a.dim, &b.dim)),
        "*" => {
            if sa.1 != sb.0 { return fail(format!("A {}×{} times a {}×{}: the columns of the first must match the rows of the second.", sa.0, sa.1, sb.0, sb.1), at); }
            if sa.0 * sb.1 > 10_000 { return fail(format!("A {}×{} result: a matrix holds at most 10,000 values.", sa.0, sb.1), at); }
            Ok(mat_val(sa.0, sb.1, matmul(&a.v, sa, &b.v, sb.1), units::mul(&a.dim, &b.dim)))
        }
        "/" if scalar(b) => { if b.x() == 0.0 { return fail("Division by zero.", rat); } each(&|x, y| x / y, units::div(&a.dim, &b.dim)) }
        "/" => fail("To divide by a matrix, multiply by its inverse: A*inv(B).", rat),
        "^" => {
            if !scalar(b) || !units::is_none(&b.dim) || b.x().fract() != 0.0 || b.x().abs() > 64.0 { return fail("A matrix power is a whole number up to 64 (−1 for the inverse).", rat); }
            if sa.0 != sa.1 { return fail("Only a square matrix has powers.", at); }
            let n = sa.0;
            let mut base = a.v.clone();
            if b.x() < 0.0 { base = inverse(&a.v, n).ok_or_else(|| Error { msg: "This matrix has no inverse (it is singular).".into(), at })?; }
            let mut r: Vec<f64> = (0..n * n).map(|k| if k / n == k % n { 1.0 } else { 0.0 }).collect();
            for _ in 0..b.x().abs() as usize { r = matmul(&r, (n, n), &base, n); }
            Ok(mat_val(n, n, r, pow_dim(&a.dim, b.x(), at)?))
        }
        _ => { same_or(a, b, "compare", at)?; let mut v = each(&|x, y| { let t = |x: f64, y: f64| (x - y).abs() <= 1e-12 * x.abs().max(y.abs()); (match op { "<" => x < y, "<=" => x <= y || t(x, y), ">" => x > y, ">=" => x >= y || t(x, y), "==" => t(x, y), _ => !t(x, y) }) as u8 as f64 }, NONE)?; v.logic = true; Ok(v) }
    }
}

/// The values of a range `a .. b` or `a, next .. b` (a range line, or a program's for loop).
pub(crate) fn range_vals(env: &mut Env, start: &Node, next: Option<&Node>, end: &Node) -> R<Val> {
    let (a, b) = (env.eval(start)?, env.eval(end)?);
    real(&a, "A range", start.at)?;
    real(&b, "A range", end.at)?;
    if a.vec || b.vec { return fail("A range goes between single values.", (start.at.0, end.at.1)); }
    // A bare 0 at the start takes the unit of the rest: L = 0, 1 m .. 3 m.
    let a = if a.x() == 0.0 && units::is_none(&a.dim) { Val { dim: b.dim, ..a } } else { a };
    let st = match next { Some(n) => { let c = env.eval(n)?; real(&c, "A range", n.at)?; same_or(&a, &c, "use", n.at)?; bin("-", c, a.clone(), n.at, n.at)? } None => Val::num(1.0, a.dim) };
    let st = if units::is_none(&st.dim) && !units::is_none(&b.dim) && next.is_none() { return fail("Give the step with its unit: x = 0, 10 m .. 100 m.", end.at) } else { st };
    same_or(&a, &b, "use", end.at)?;
    if st.x() == 0.0 || (b.x() - a.x()) / st.x() < 0.0 { return fail("The step must go from the first value towards the last.", end.at); }
    let count = ((b.x() - a.x()) / st.x() + 1e-9).floor() + 1.0;
    if !(1.0..=100_000.0).contains(&count) { return fail("A range holds at most 100,000 values.", end.at); }
    let n = count as usize;
    Ok(Val { abs: a.abs, ..Val::vector((0..n).map(|i| a.x() + st.x() * i as f64).collect(), a.dim) })
}

/// The units of a value to a power: whole exponents only (x^2, x^0.5 of an area); the angle tag is soft.
fn pow_dim(d: &Dim, p: f64, at: Span) -> R<Dim> {
    let mut dim = NONE;
    for (i, (o, x)) in dim.iter_mut().zip(d).enumerate() {
        let e = *x as f64 * p;
        if i == 7 && (e - e.round()).abs() > 1e-9 { *o = 0; continue; }
        if (e - e.round()).abs() > 1e-9 { return fail(format!("{} to the power {} has no units.", dim_name(d), fmt_plain(p, 4)), at); }
        *o = e.round() as i8;
    }
    Ok(dim)
}

/// Complex arithmetic, value by value: (re, im) pairs.
fn cmul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0) }
fn cdiv(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { let d = b.0 * b.0 + b.1 * b.1; ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d) }
fn cexp(a: (f64, f64)) -> (f64, f64) { let r = a.0.exp(); (r * a.1.cos(), r * a.1.sin()) }
fn cln(a: (f64, f64)) -> (f64, f64) { (a.0.hypot(a.1).ln(), a.1.atan2(a.0)) }
fn csin(a: (f64, f64)) -> (f64, f64) { (a.0.sin() * a.1.cosh(), a.0.cos() * a.1.sinh()) }
fn ccos(a: (f64, f64)) -> (f64, f64) { (a.0.cos() * a.1.cosh(), -a.0.sin() * a.1.sinh()) }
fn csinh(a: (f64, f64)) -> (f64, f64) { (a.0.sinh() * a.1.cos(), a.0.cosh() * a.1.sin()) }
fn ccosh(a: (f64, f64)) -> (f64, f64) { (a.0.cosh() * a.1.cos(), a.0.sinh() * a.1.sin()) }
fn csqrt(a: (f64, f64)) -> (f64, f64) { let r = a.0.hypot(a.1); (((r + a.0) / 2.0).sqrt(), ((r - a.0) / 2.0).sqrt().copysign(a.1)) }

/// The number of values an operation on a and b gives (vectors must match).
fn count(a: &Val, b: &Val, at: Span) -> R<usize> {
    match (a.vec, b.vec) {
        (false, false) => Ok(1),
        (true, false) => Ok(a.v.len()),
        (false, true) => Ok(b.v.len()),
        (true, true) if a.v.len() == b.v.len() => Ok(a.v.len()),
        _ => fail(format!("These vectors have different lengths ({} and {}).", a.v.len(), b.v.len()), at),
    }
}

fn bin_complex(op: &str, a: Val, b: Val, at: Span, rat: Span) -> R<Val> {
    let n = count(&a, &b, at)?;
    let dim = match op {
        "+" | "-" => { same_or(&a, &b, if op == "+" { "add" } else { "subtract" }, at)?; let mut d = a.dim; d[7] = ang(&a.dim, &b.dim); d }
        "*" => units::mul(&a.dim, &b.dim),
        "/" => { if !b.vec && b.x() == 0.0 && b.at(0).1 == 0.0 { return fail("Division by zero.", rat); } units::div(&a.dim, &b.dim) }
        "^" => {
            if !units::is_none(&b.dim) || b.vec { return fail("A power must be a plain number.", rat); }
            if b.cx() && !units::is_none(&a.dim) { return fail("A complex power needs a plain number below it.", at); }
            if b.cx() { NONE } else { pow_dim(&a.dim, b.x(), at)? }
        }
        "<" | "<=" | ">" | ">=" | "==" | "!=" => return fail("Complex values cannot be compared: compare abs(z), re(z) or im(z).", at),
        o => return fail(format!("“{o}” cannot be used here."), at),
    };
    let (mut re, mut im) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for k in 0..n {
        let (x, y) = (a.at(k), b.at(k));
        let r = match op {
            "+" => (x.0 + y.0, x.1 + y.1),
            "-" => (x.0 - y.0, x.1 - y.1),
            "*" => cmul(x, y),
            "/" => cdiv(x, y),
            // 0 to a power is 0 only when the power's real part is positive.
            _ if x == (0.0, 0.0) => if y.0 > 0.0 { (0.0, 0.0) } else { (f64::NAN, 0.0) },
            _ => cexp(cmul(y, cln(x))),
        };
        re.push(r.0);
        im.push(r.1);
    }
    Ok(Val { v: re, im, dim, vec: a.vec || b.vec, nm: moment(op, &a, &b, &dim), ..Default::default() })
}

/// 2π times a frequency in cycles (Hz), or 2π over a period, is an angular frequency, shown in rad/s; ω/(2π) is
/// back in Hz. A plain number made with π keeps its mark through plain factors (2*pi, pi/30), and ω keeps
/// its mark through plain factors and sums of angular frequencies.
fn angular(op: &str, a: &Val, b: &Val, mut r: Val) -> Val {
    let plain = |v: &Val| units::is_none(&v.dim) && v.dim[7] == 0;
    let freq = |v: &Val| units::same(&v.dim, &[0, 0, -1, 0, 0, 0, 0, 0]) && v.dim[7] == 0;
    // A factor 2π (π to the power +1) turns cycles into radians; 1/(2π) (power −1) turns them back.
    let up = |p: &Val| plain(p) && p.pi > 0;
    let down = |p: &Val| plain(p) && p.pi < 0;
    match op {
        "*" | "/" if plain(a) && plain(b) => r.pi = if op == "*" { a.pi.saturating_add(b.pi) } else { a.pi.saturating_sub(b.pi) },
        "*" if (up(a) && freq(b) && !b.w) || (up(b) && freq(a) && !a.w) => r.w = true,
        "*" if (down(a) && freq(b) && b.w) || (down(b) && freq(a) && a.w) => r.w = false,
        "/" if up(a) && units::same(&b.dim, &[0, 0, 1, 0, 0, 0, 0, 0]) && b.dim[7] == 0 => r.w = true,
        "/" if up(b) && freq(a) && a.w => r.w = false,
        "/" if down(b) && freq(a) && !a.w => r.w = true,
        "*" | "/" => r.w = freq(&r) && ((a.w && plain(b)) || (op == "*" && b.w && plain(a))),
        "+" | "-" => r.w = freq(&r) && (a.w || b.w),
        _ => {}
    }
    r
}

fn bin_raw(op: &str, a: Val, b: Val, at: Span, rat: Span) -> R<Val> {
    let r = bin_raw2(op, &a, &b, at, rat)?;
    Ok(angular(op, &a, &b, r))
}

fn bin_raw2(op: &str, a: &Val, b: &Val, at: Span, rat: Span) -> R<Val> {
    let (a, b) = (a.clone(), b.clone());
    // 230 V∠-30° is polar(230 V, -30°).
    // A bare number after ∠ is in degrees, as phasors are written (230∠-30); an angle keeps its own unit.
    if op == "∠" {
        let b = if units::is_none(&b.dim) && b.dim[7] == 0 && !b.cx() { Val { v: b.v.iter().map(|x| x.to_radians()).collect(), dim: units::ANGLE, ..b } } else { b };
        return builtin("polar", vec![a, b], at);
    }
    if a.mat() || b.mat() { return mat_bin(op, &a, &b, at, rat); }
    if a.cx() || b.cx() { return bin_complex(op, a, b, at, rat); }
    let vec = a.vec || b.vec;
    let mk = |v: Vec<f64>, dim: Dim, abs: bool| Val { v, dim, abs, vec, nm: moment(op, &a, &b, &dim), ..Default::default() };
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
            Ok(mk(a.v.iter().map(|x| x.powf(p)).collect(), pow_dim(&a.dim, p, at)?, false))
        }
        "<" | "<=" | ">" | ">=" | "==" | "!=" => {
            same_or(&a, &b, "compare", at)?;
            let tol = |x: f64, y: f64| (x - y).abs() <= 1e-12 * x.abs().max(y.abs());
            let v = zip(&a, &b, at, |x, y| (match op { "<" => x < y, "<=" => x <= y || tol(x, y), ">" => x > y, ">=" => x >= y || tol(x, y), "==" => tol(x, y), _ => !tol(x, y) }) as u8 as f64)?;
            Ok(Val { v, dim: NONE, vec, logic: true, ..Default::default() })
        }
        o => fail(format!("“{o}” cannot be used here."), at),
    }
}

fn builtin(f: &str, a: Vec<Val>, at: Span) -> R<Val> {
    let n = a.len();
    let want = |m: usize| -> R<()> { if n == m { Ok(()) } else { fail(format!("{f} takes {m} value{}.", if m == 1 { "" } else { "s" }), at) } };
    let map = |v: &Val, dim: Dim, g: &dyn Fn(f64) -> f64| Val { v: v.v.iter().map(|x| g(*x)).collect(), dim, vec: v.vec, rows: v.rows, ..Default::default() };
    // A complex function, value by value; the result is complex.
    let cmap = |v: &Val, dim: Dim, g: &dyn Fn((f64, f64)) -> (f64, f64)| { let (re, im) = (0..v.v.len()).map(|k| g(v.at(k))).unzip(); Val { v: re, im, dim, vec: v.vec, ..Default::default() } };
    let plain = |v: &Val| -> R<()> { if units::is_none(&v.dim) { Ok(()) } else { Err(Error { msg: format!("{f} needs a plain number, not {}.", dim_name(&v.dim)), at }) } };
    let fail = |m: String| -> R<Val> { Err(Error { msg: m, at }) };
    if a.iter().any(|v| v.cx()) && !matches!(f, "abs" | "sqrt" | "exp" | "ln" | "re" | "im" | "arg" | "conj" | "sum" | "mean" | "len" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "db" | "par") {
        return fail(format!("{f} needs real values: use abs, re, im or arg of a complex value."));
    }
    let r = match f {
        "sqrt" | "cbrt" => {
            want(1)?;
            let k = if f == "sqrt" { 2 } else { 3 };
            // The angle tag is soft: it is dropped when it does not divide evenly (∛ of N·m/rad is fine).
            if a[0].dim[..7].iter().any(|e| e % k != 0) { return fail(format!("{f} of {} has no units.", dim_name(&a[0].dim))); }
            let dim = std::array::from_fn(|i| if i == 7 && a[0].dim[7] % k != 0 { 0 } else { a[0].dim[i] / k });
            if a[0].cx() { cmap(&a[0], dim, &csqrt) } else {
                if f == "sqrt" && a[0].v.iter().any(|x| *x < 0.0) { return fail("sqrt of a negative number.".into()); }
                map(&a[0], dim, &|x| if k == 2 { x.sqrt() } else { x.cbrt() })
            }
        }
        "abs" => { want(1)?; Val { v: (0..a[0].v.len()).map(|k| { let (x, y) = a[0].at(k); x.hypot(y) }).collect(), im: vec![], abs: a[0].abs && !a[0].cx(), ..a[0].clone() } }
        "re" | "im" | "conj" => {
            want(1)?;
            let z = &a[0];
            match f {
                "re" => Val { im: vec![], abs: z.abs && !z.cx(), ..z.clone() },
                "im" => Val { v: (0..z.v.len()).map(|k| z.at(k).1).collect(), im: vec![], abs: false, ..z.clone() },
                _ => Val { im: z.im.iter().map(|y| -y).collect(), ..z.clone() },
            }
        }
        "db" => {
            want(1)?;
            plain(&a[0])?;
            let v: Vec<f64> = (0..a[0].v.len()).map(|k| { let (x, y) = a[0].at(k); 20.0 * x.hypot(y).log10() }).collect();
            if v.iter().any(|x| !x.is_finite()) { return fail("db of zero.".into()); }
            Val { v, vec: a[0].vec, ..Default::default() }
        }
        "par" => {
            if n < 2 { return fail("par needs two or more values: par(Z1, Z2).".into()); }
            for x in &a[1..] { same_or(&a[0], x, "combine", at)?; }
            let n2 = a.iter().try_fold(1usize, |m, x| if x.vec && m > 1 && x.v.len() != m { Err(()) } else { Ok(if x.vec { x.v.len() } else { m }) }).map_err(|_| Error { msg: "These vectors have different lengths.".into(), at })?;
            let mut out = (vec![], vec![]);
            for k in 0..n2 {
                let mut y = (0.0, 0.0);
                for x in &a { let z = x.at(k); if z == (0.0, 0.0) { return fail("par of a zero value (a short circuit).".into()); } let r = cdiv((1.0, 0.0), z); y = (y.0 + r.0, y.1 + r.1); }
                let z = cdiv((1.0, 0.0), y);
                out.0.push(z.0);
                out.1.push(z.1);
            }
            Val { v: out.0, im: if a.iter().any(|x| x.cx()) { out.1 } else { vec![] }, dim: a[0].dim, vec: a.iter().any(|x| x.vec), ..Default::default() }
        }
        "arg" => { want(1)?; Val { v: (0..a[0].v.len()).map(|k| { let (x, y) = a[0].at(k); y.atan2(x) }).collect(), dim: units::ANGLE, vec: a[0].vec, ..Default::default() } }
        "polar" => {
            want(2)?;
            if !units::is_none(&a[1].dim) { return fail(format!("polar needs an angle second, not {}.", dim_name(&a[1].dim))); }
            let n = count(&a[0], &a[1], at)?;
            let (re, im) = (0..n).map(|k| { let (r, t) = (a[0].at(k).0, a[1].at(k).0); (r * t.cos(), r * t.sin()) }).unzip();
            Val { v: re, im, dim: a[0].dim, vec: a[0].vec || a[1].vec, ..Default::default() }
        }
        "exp" | "ln" | "log" | "log10" | "sinh" | "cosh" | "tanh" => {
            want(1)?;
            plain(&a[0])?;
            if a[0].cx() {
                if f == "ln" && (0..a[0].v.len()).any(|k| a[0].at(k) == (0.0, 0.0)) { return fail("ln of zero.".into()); }
                if f.starts_with("log") { return fail(format!("{f} needs real values: use ln, or db for a ratio in decibels.")); }
                cmap(&a[0], NONE, &|z| match f { "exp" => cexp(z), "ln" => cln(z), "sinh" => csinh(z), "cosh" => ccosh(z), _ => cdiv(csinh(z), ccosh(z)) })
            } else {
            if (f == "ln" || f.starts_with("log")) && a[0].v.iter().any(|x| *x <= 0.0) { return fail(format!("{f} of zero or a negative number.")); }
            map(&a[0], NONE, &|x| match f { "exp" => x.exp(), "ln" => x.ln(), "sinh" => x.sinh(), "cosh" => x.cosh(), "tanh" => x.tanh(), _ => x.log10() })
            }
        }
        "sin" | "cos" | "tan" => {
            want(1)?;
            if !units::is_none(&a[0].dim) { return fail(format!("{f} needs an angle or a plain number, not {}.", dim_name(&a[0].dim))); }
            if a[0].cx() { cmap(&a[0], NONE, &|z| match f { "sin" => csin(z), "cos" => ccos(z), _ => cdiv(csin(z), ccos(z)) }) }
            else { map(&a[0], NONE, &|x| match f { "sin" => x.sin(), "cos" => x.cos(), _ => x.tan() }) }
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
            Val { v, dim: if f == "atan2" { units::ANGLE } else { a[0].dim }, vec: a[0].vec || a[1].vec, ..Default::default() }
        }
        "min" | "max" => {
            if n == 0 { return fail(format!("{f} needs at least one value.")); }
            for x in &a[1..] { same_or(&a[0], x, "compare", at)?; }
            let all: Vec<f64> = a.iter().flat_map(|x| x.v.iter().copied()).collect();
            let m = if f == "min" { all.iter().copied().fold(f64::INFINITY, f64::min) } else { all.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
            Val { v: vec![m], dim: a[0].dim, abs: a.iter().all(|x| x.abs), nm: a[0].nm, ..Default::default() }
        }
        "round" | "floor" | "ceil" => {
            if n == 0 || n > 2 { return fail(format!("{f} takes one or two values.")); }
            let d = if n == 2 { plain(&a[1])?; a[1].x().round() as i32 } else { 0 };
            let s = 10f64.powi(d);
            Val { v: a[0].v.iter().map(|x| match f { "round" => (x * s).round() / s, "floor" => (x * s).floor() / s, _ => (x * s).ceil() / s }).collect(), ..a[0].clone() }
        }
        "sum" | "mean" => {
            want(1)?;
            let k = if f == "sum" { 1.0 } else { a[0].v.len() as f64 };
            let im = if a[0].cx() { vec![a[0].im.iter().sum::<f64>() / k] } else { vec![] };
            Val { v: vec![a[0].v.iter().sum::<f64>() / k], im, dim: a[0].dim, abs: f == "mean" && a[0].abs, nm: a[0].nm, ..Default::default() }
        }
        "len" => { want(1)?; Val::plain(a[0].v.len() as f64) }
        "rows" | "cols" => { want(1)?; let (r, c) = a[0].shape(); Val::plain(if f == "rows" { r } else { c } as f64) }
        "transpose" => {
            want(1)?;
            no_abs(&a, at)?;
            let (r, c) = a[0].shape();
            if !a[0].vec { return fail("transpose needs a vector or a matrix.".into()); }
            let v: Vec<f64> = (0..r * c).map(|k| a[0].v[(k % r) * c + k / r]).collect();
            // A vector (a column) becomes a row: a 1 × n matrix.
            if c == 1 { Val { v, dim: a[0].dim, vec: true, rows: 1, ..Default::default() } } else { mat_val(c, r, v, a[0].dim) }
        }
        "inv" | "det" => {
            want(1)?;
            no_abs(&a, at)?;
            let (r, c) = if a[0].vec { a[0].shape() } else { (1, 1) };
            if (a[0].vec && !a[0].mat()) || r != c { return fail(format!("{f} needs a square matrix.")); }
            if f == "inv" {
                let v = inverse(&a[0].v, r).ok_or_else(|| Error { msg: "This matrix has no inverse (it is singular).".into(), at })?;
                mat_val(r, r, v, units::pow(&a[0].dim, -1))
            } else {
                // Its units to the power n: refused beyond the powers units can hold, as for x^n.
                if a[0].dim.iter().take(7).any(|p| (*p as i32 * r as i32).abs() > 60) { return fail(format!("The determinant of this {r} × {r} matrix would raise its units to the power {r}, beyond what units can hold.")); }
                let d = crate::solve::Lu::new(rows_of(&a[0].v, r)).map(|l| l.det()).unwrap_or(0.0);
                Val::num(d, pow_dim(&a[0].dim, r as f64, at)?)
            }
        }
        "lsolve" => {
            want(2)?;
            let ((r, c), (rb, cb)) = (a[0].shape(), a[1].shape());
            no_abs(&a, at)?;
            let ((r, c), (rb, cb)) = if a[0].vec { ((r, c), (rb, cb)) } else { ((1, 1), (if a[1].vec { rb } else { 1 }, cb)) };
            if (a[0].vec && !a[0].mat()) || r != c { return fail("lsolve(A, b) needs a square matrix A.".into()); }
            if (r > 1 && !a[1].vec) || cb != 1 || rb != r { return fail(format!("lsolve(A, b) needs a vector b of {r} values.")); }
            let x = crate::solve::linear(rows_of(&a[0].v, r), a[1].v.clone()).ok_or_else(|| Error { msg: "The system has no single solution (the matrix is singular).".into(), at })?;
            mat_val(r, 1, x, units::div(&a[1].dim, &a[0].dim))
        }
        "identity" => {
            want(1)?;
            plain(&a[0])?;
            let n = a[0].x();
            if n.fract() != 0.0 || !(1.0..=100.0).contains(&n) { return fail("identity(n) needs a whole number n from 1 to 100.".into()); }
            let n = n as usize;
            mat_val(n, n, (0..n * n).map(|k| if k / n == k % n { 1.0 } else { 0.0 }).collect(), NONE)
        }
        "dot" | "cross" => {
            want(2)?;
            no_abs(&a, at)?;
            if a[0].mat() || a[1].mat() || !a[0].vec || !a[1].vec || a[0].v.len() != a[1].v.len() { return fail(format!("{f} needs two vectors of the same length.")); }
            let dim = units::mul(&a[0].dim, &a[1].dim);
            if f == "dot" { Val::num(a[0].v.iter().zip(&a[1].v).map(|(x, y)| x * y).sum(), dim) } else {
                if a[0].v.len() != 3 { return fail("cross needs two vectors of 3 values.".into()); }
                let (x, y) = (&a[0].v, &a[1].v);
                Val { v: vec![x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]], dim, vec: true, ..Default::default() }
            }
        }
        "norm" => {
            // Scaled by the largest entry first, so 1e200 and 1e-200 do not overflow or vanish.
            want(1)?;
            no_abs(&a, at)?;
            let m = a[0].v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
            Val::num(if m == 0.0 { 0.0 } else { m * a[0].v.iter().map(|x| (x / m).powi(2)).sum::<f64>().sqrt() }, a[0].dim)
        }
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
            Val { v: x.v.iter().map(|q| one(*q)).collect(), dim: ys.dim, abs: ys.abs, vec: x.vec, ..Default::default() }
        }
        _ if crate::sym::OPS.contains(&f) => return fail(format!("{f} gives an expression, not a value: write it on its own line, such as {f}((x + 1)^2) or y = {f}(…).")),
        _ => return fail(format!("{f} is not a function. Define it first, such as {f}(x) = 2*x.")),
    };
    if r.v.iter().chain(&r.im).any(|x| !x.is_finite()) { return fail(format!("{f} gave a value that is not a finite number.")); }
    Ok(r.tidy())
}

// ---------- the sheet ----------

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct CheckOut { pub pass: bool, pub ratio: Option<f64>, pub lhs: String, pub rhs: String, pub op: String }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Series { pub label: String, pub ys: Vec<f64> }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct PlotOut { pub xs: Vec<f64>, pub xlabel: String, pub xunit: String, pub series: Vec<Series>, pub yunit: String, pub logx: bool }

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
    /// A material region worked out: its datasheet.
    pub material: Option<MatOut>,
    /// Names a table defines (one per column), for the Variables panel.
    pub defs: Vec<VarOut>,
    /// Units this result can be shown in (same dimension), for the unit menu.
    pub units: Vec<String>,
    /// The description after `#` on the line.
    pub note: String,
    /// The equation again with each name replaced by its value (when the sheet shows numbers).
    pub subst_tex: String,
}

/// A material region as shown: the material, which end of the ranges is used, and a row per property.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct MatOut { pub title: String, pub family: String, pub des: String, pub notes: String, pub pick: String, pub draft: bool, pub rows: Vec<MatRow>, pub sources: Vec<String> }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct MatRow { pub tex: String, pub what: String, pub value: String, pub unit: String, pub range: String, pub source: String }

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VarOut { pub name: String, pub tex: String, pub value: String, pub unit: String, pub id: u32 }

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct SheetOut { pub regions: Vec<RegionOut>, pub vars: Vec<VarOut>, pub checks: usize, pub failed: usize, pub errors: usize }

/// How results are written: significant figures, engineering notation, US units, complex values as r∠θ.
pub struct Fmt { pub digits: u8, pub eng: bool, pub us: bool, pub polar: bool, pub exact: bool }

/// A number with `digits` significant figures, as text and as LaTeX. Very large or small numbers
/// (or every number with `eng`) use engineering notation: a power of ten in steps of three.
pub fn fmt_num(x: f64, digits: u8, eng: bool) -> (String, String) {
    if !x.is_finite() { return ("—".into(), "\\text{—}".into()); }
    // Below 10⁻³⁰⁰ (subnormal) the powers of ten underflow; such values are zero for any engineering purpose.
    if x == 0.0 || x.abs() < 1e-300 { return ("0".into(), "0".into()); }
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

/// A number as shown on a sheet: with View › Fractions, a fraction as one (1/3) and a whole number in full;
/// otherwise `fmt_num`.
pub fn fmt_val(x: f64, f: &Fmt) -> (String, String) {
    if f.exact && x.abs() < 1e15 {
        if let Some(q) = crate::sym::fraction(x, 10_000) {
            if q.d != 1 { return crate::sym::q_text(q); }
            let t = fmt_plain(q.n as f64, 16);
            return (t.clone(), t.replace('\u{2009}', "\\,"));
        }
    }
    fmt_num(x, f.digits, f.eng)
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
    let big = (0..v.v.len()).map(|k| { let (x, y) = v.at(k); x.hypot(y) }).filter(|x| x.is_finite()).fold(0.0f64, f64::max);
    // A force times a length reads as a torque (N·m), as a power over a speed does (J/rad).
    // An angular frequency (ω = 2πf) reads in rad/s, and as cycles in Hz or rpm when one is chosen.
    let dim = if v.nm && v.dim[7] == 0 && want.trim().is_empty() { let mut d = v.dim; d[7] = -1; d } else { shown_dim(v) };
    let (k, off, label) = units::display(big, &dim, v.abs, want, f.us);
    let one = |(x, y): (f64, f64)| if v.cx() { fmt_cx((x - off) / k, y / k, f) } else { fmt_val((x - off) / k, f) };
    let (text, tex) = if v.mat() {
        // A matrix in brackets, up to 12 rows and columns shown (… for the rest).
        let (r, c) = v.shape();
        let (sr, sc) = (r.min(12), c.min(12));
        let cell = |i: usize, j: usize| one(v.at(i * c + j));
        let t = (0..sr).map(|i| format!("[{}{}]", (0..sc).map(|j| cell(i, j).0).collect::<Vec<_>>().join(", "), if sc < c { ", …" } else { "" })).collect::<Vec<_>>().join(", ");
        let x = (0..sr).map(|i| format!("{}{}", (0..sc).map(|j| cell(i, j).1).collect::<Vec<_>>().join(" & "), if sc < c { " & \\cdots" } else { "" })).collect::<Vec<_>>().join(" \\\\ ");
        (format!("[{t}{}]", if sr < r { ", …" } else { "" }), format!("\\begin{{bmatrix}} {x}{} \\end{{bmatrix}}", if sr < r { " \\\\ \\vdots" } else { "" }))
    } else if v.vec {
        let n = v.v.len();
        let pick: Vec<usize> = if n <= 6 { (0..n).collect() } else { vec![0, 1, 2, n - 1] };
        let parts: Vec<(String, String)> = pick.iter().map(|i| one(v.at(*i))).collect();
        let join = |i: usize| -> String {
            let p: Vec<&str> = parts.iter().map(|x| if i == 0 { x.0.as_str() } else { x.1.as_str() }).collect();
            if n <= 6 { p.join(", ") } else if i == 0 { format!("{}, {}, {}, … {}", p[0], p[1], p[2], p[3]) } else { format!("{}, {}, {}, \\ldots, {}", p[0], p[1], p[2], p[3]) }
        };
        (format!("[{}]", join(0)), format!("\\left[{}\\right]", join(1)))
    } else { one(v.at(0)) };
    let unit_tex = latex::unit(&label);
    // (3 + 4j) Ω: a complex value in rectangular form is bracketed before its unit.
    let rect = v.cx() && !v.vec && !f.polar && !label.is_empty() && text.contains(' ');
    let (bt, bx) = if rect { (format!("({text})"), format!("\\left({tex}\\right)")) } else { (text.clone(), tex) };
    // A fraction before a unit is bracketed in text: (2/3) m.
    let bt = if !rect && !v.vec && !label.is_empty() && label != "°" && text.contains('/') { format!("({bt})") } else { bt };
    let full_t = if label.is_empty() { text.clone() } else if label == "°" { format!("{bt}°") } else { format!("{bt} {label}") };
    let full_x = if label.is_empty() { bx } else if label == "°" { format!("{bx}{unit_tex}") } else { format!("{bx}\\,{unit_tex}") };
    (full_t, full_x, text, label)
}

/// The dimension a value is shown with: an angular frequency (ω = 2πf) as rad/s.
fn shown_dim(v: &Val) -> Dim { let mut d = v.dim; if v.w && d[7] == 0 { d[7] = 1; } d }

/// A complex number as text and LaTeX: 3 + 4j, or 5∠53.13° in polar form.
fn fmt_cx(x: f64, y: f64, f: &Fmt) -> (String, String) {
    if f.polar {
        let (r, t) = (fmt_num(x.hypot(y), f.digits, f.eng), fmt_num(y.atan2(x).to_degrees(), f.digits, false));
        return (format!("{}∠{}°", r.0, t.0), format!("{}\\angle{{{}}}^{{\\circ}}", r.1, t.1));
    }
    let (a, b) = (fmt_val(x, f), fmt_val(y.abs(), f));
    // A fraction before j is bracketed in text: (1/4)j, not 1/4j.
    let b = if b.0.contains('/') { (format!("({})", b.0), b.1) } else { b };
    let sign = if y < 0.0 { "−" } else { "+" };
    if b.0 == "0" { return a; }
    if a.0 == "0" { return (format!("{}{}j", if y < 0.0 { "-" } else { "" }, b.0), format!("{}{}\\,\\mathrm{{j}}", if y < 0.0 { "-" } else { "" }, b.1)); }
    (format!("{} {sign} {}j", a.0, b.0), format!("{} {} {}\\,\\mathrm{{j}}", a.1, if y < 0.0 { "-" } else { "+" }, b.1))
}

/// Units the window offers for a result: every unit of the same kind, plus the automatic choices.
fn unit_choices(dim: &Dim, abs: bool, nm: bool) -> Vec<String> {
    if units::is_none(dim) { return if dim[7] != 0 { vec!["°".into(), "rad".into()] } else { vec![] }; }
    const ALIAS: [&str; 19] = ["um", "us", "uF", "l", "ml", "lbm", "Btu", "degR", "degC", "degF", "deltaC", "deltaF", "delta°C", "delta°F", "tonne", "hr", "knot", "ohm", "kohm"];
    let mut v: Vec<String> = vec![];
    if abs && units::same(dim, &TEMP) { v.push("°C".into()); v.push("°F".into()); }
    v.extend(units::UNITS.iter().filter(|u| units::same(&u.dim, dim) && !ALIAS.contains(&u.sym)).map(|u| u.sym.to_string()));
    if units::same(dim, &[0, 0, -1, 0, 0, 0, 0, 0]) && !v.iter().any(|u| u == "rad/s") { v.push("rad/s".into()); }
    // Torque and moments share the units of energy.
    if units::same(dim, &ENERGY) {
        let t = ["N·m", "kN·m", "N·mm", "MN·m", "lbf·ft", "lbf·in", "kip·ft"].map(String::from);
        v = if nm || dim[7] < 0 { [t.to_vec(), v].concat() } else { [v, t.to_vec()].concat() };
    }
    let dim = &if nm && dim[7] == 0 { let mut d = *dim; d[7] = -1; d } else { *dim };
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
    let f = Fmt { digits: s.digits, eng: false, us: s.units == "US", polar: s.polar, exact: s.exact };
    run_with(s, &f)
}

pub fn run_with(s: &Sheet, f: &Fmt) -> SheetOut { run_env(s, f).0 }

/// The sheet worked out, with the final symbol table (the packs' tests read values from it). It runs on
/// its own thread with a large stack: the window's thread has 1 MB on Windows, and user functions nest.
pub fn run_env(s: &Sheet, f: &Fmt) -> (SheetOut, Env) {
    let (s2, f2) = (s.clone(), Fmt { digits: f.digits, eng: f.eng, us: f.us, polar: f.polar, exact: f.exact });
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || run_here(&s2, &f2)).ok()
        .and_then(|h| h.join().ok())
        .unwrap_or_else(|| (SheetOut { errors: 1, ..Default::default() }, Env::new()))
}

fn run_here(s: &Sheet, f: &Fmt) -> (SheetOut, Env) {
    let mut env = Env::new();
    // Names defined anywhere, so a name used too early gets a helpful message.
    for r in &s.regions {
        if r.kind == Kind::Material {
            let d: crate::materials::MatData = serde_json::from_value(r.data.clone()).unwrap_or_default();
            if let Some(m) = d.mat.as_ref().or_else(|| crate::materials::find(&d.id)) { for k in crate::materials::KEYS.iter().filter(|k| m.props.contains_key(k.0)) { env.later.insert(canon(&format!("{}_{}", k.1, d.name)), false); } }
        } else if r.kind == Kind::Table {
            for c in serde_json::from_value::<TableData>(r.data.clone()).unwrap_or_default().cols { if !c.name.trim().is_empty() { env.later.insert(canon(c.name.trim()), false); } }
        } else { for n in region_names(r) { env.later.insert(canon(&n), false); } }
    }
    let mut out = SheetOut::default();
    let mut known: Vec<String> = vec![];
    for r in &s.regions {
        let o = region(&mut env, r, f, &mut known, s.subst);
        if let Some(c) = &o.check { out.checks += 1; if !c.pass { out.failed += 1; } }
        if o.error.is_some() {
            out.errors += 1;
            if !o.name.is_empty() { env.later.insert(canon(&o.name), true); }
            if r.kind != Kind::Table && r.kind != Kind::Material { for n in region_names(r) { env.later.insert(canon(&n), true); } }
        }
        for v in &o.defs { out.vars.retain(|x| x.name != v.name); out.vars.push(v.clone()); }
        if let Some(t) = &o.table { for c in t.cols.iter().filter(|c| !c.error.is_empty() && !c.name.is_empty()) { env.later.insert(canon(&c.name), true); } }
        if o.error.is_none() && !o.name.is_empty() && !o.unit.is_empty() { env.shown.insert(canon(&o.name), o.unit.clone()); }
        for d in &o.defs { env.shown.insert(d.name.clone(), d.unit.clone()); }
        if o.error.is_none() && !o.name.is_empty() && !o.value.is_empty() {
            out.vars.retain(|v| v.name != o.name);
            out.vars.push(VarOut { name: o.name.clone(), tex: latex::name(&o.name), value: o.value.clone(), unit: o.unit.clone(), id: r.id });
        }
        out.regions.push(o);
    }
    (out, env)
}

/// Every name a region of text defines: a program's function or the names it sets, else its line's.
fn region_names(r: &Region) -> Vec<String> {
    if r.kind != Kind::Program { return defined_names(split_note(&r.text).0); }
    match crate::prog::names(&r.text) { (Some((n, _)), ..) => vec![n], (None, sets, loops) => sets.into_iter().filter(|n| !loops.contains(n)).collect() }
}

/// A program region: a function for the lines below, or lines run once whose names are defined below.
fn program(env: &mut Env, r: &Region, f: &Fmt, known: &mut Vec<String>, o: &mut RegionOut) {
    if r.text.trim().is_empty() { return; }
    let p = match crate::prog::parse(&r.text, known) {
        Ok(p) => p,
        Err(e) => { o.error = Some(ErrOut { msg: e.msg, from: e.at.0, to: e.at.1 }); o.latex = latex::escape_text(r.text.lines().next().unwrap_or("")); return; }
    };
    o.latex = p.tex.clone();
    if let Some((name, _)) = p.head.clone() {
        let k = canon(&name);
        if FUNCTIONS.iter().any(|x| x.0.split('(').next() == Some(k.as_str())) { o.error = Some(ErrOut { msg: format!("{name} is a built-in function: choose another name."), from: 0, to: name.chars().count() }); return; }
        o.name = k.clone();
        env.funcs.remove(&k);
        env.progs.insert(k, std::sync::Arc::new(p));
        known.push(name.clone());
        return;
    }
    let loops: Vec<(String, Option<Val>)> = p.loops.iter().map(|n| (canon(n), env.vars.get(&canon(n)).cloned())).collect();
    let mut steps = 0;
    let res = crate::prog::exec(env, &p.body, &mut steps);
    for (k, v) in loops { match v { Some(v) => { env.vars.insert(k, v); } None => { env.vars.remove(&k); } } }
    match res {
        Err(e) => { o.error = Some(ErrOut { msg: e.msg, from: e.at.0, to: e.at.1 }); }
        Ok(_) => {
            let (mut texts, mut texes) = (vec![], vec![]);
            for name in p.sets.iter().filter(|n| !p.loops.contains(n)) {
                let k = canon(name);
                let Some(v) = env.vars.get(&k).cloned() else { continue };
                let (t, tx, value, unit) = show_val(&v, "", f);
                texts.push(format!("{k} = {t}"));
                texes.push(format!("{} = {tx}", latex::name(name)));
                o.defs.push(VarOut { name: k.clone(), tex: latex::name(name), value, unit: unit.clone(), id: r.id });
                if !unit.is_empty() { env.shown.insert(k.clone(), unit); }
                known.push(name.clone());
            }
            o.result = texts.join(", ");
            o.result_tex = if texes.is_empty() { String::new() } else { format!("\\Rightarrow\\; {}", texes.join(",\\; ")) };
        }
    }
}

/// Every name a line defines: one, or the unknowns of `solve … for x = g, y = g`.
fn defined_names(t: &str) -> Vec<String> {
    let Ok(toks) = parse::lex(t) else { return vec![] };
    if matches!(toks.first().map(|x| &x.0), Some(parse::Tok::Id(w)) if w == "solve") {
        if let Some(k) = toks.iter().position(|x| x.0 == parse::Tok::Id("for".into())) {
            let sys: Vec<String> = (k + 1..toks.len()).filter_map(|j| match (&toks[j].0, toks.get(j + 1).map(|x| &x.0)) { (parse::Tok::Id(n), Some(parse::Tok::Op("="))) => Some(n.clone()), _ => None }).collect();
            if !sys.is_empty() { return sys; }
        }
    }
    defined_name(t).into_iter().collect()
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

/// A complex number written out, such as 3 + 4j, -2j or (8 + 6j) Ω: an input, so its value is not repeated.
fn complex_literal(n: &Node) -> bool {
    fn lit(n: &Node, im: &mut bool) -> bool {
        match &n.e {
            E::Num(..) => true,
            E::Im(..) => { *im = true; true }
            E::Neg(x) | E::Group(x) | E::Unit(x, _) => lit(x, im),
            E::Bin("+" | "-", a, b) => lit(a, im) && lit(b, im),
            _ => false,
        }
    }
    let mut im = false;
    lit(n, &mut im) && im
}

/// `\\mathrm{j}` for j or i when the sheet has no name of that spelling.
fn imag_unit(env: &Env, v: &str) -> Option<String> {
    matches!(v, "i" | "j").then_some(()).filter(|_| !env.vars.contains_key(v)).map(|_| format!("\\mathrm{{{v}}}"))
}

/// The expression with each name replaced by its value in the unit it is shown in; π and e stay symbols.
fn substituted(env: &Env, e: &Node, f: &Fmt) -> String {
    let has_name = |n: &Node| { let mut st = vec![n]; while let Some(x) = st.pop() { match &x.e { E::Var(v) if !matches!(canon(v).as_str(), "π" | "e") && !(matches!(v.as_str(), "i" | "j") && !env.vars.contains_key(v.as_str())) => return true, E::Neg(a) | E::Group(a) => st.push(a), E::Bin(_, a, b) => { st.push(a); st.push(b); } E::Call(_, a) => st.extend(a.iter()), _ => {} } } false };
    if !has_name(e) { return String::new(); }
    let sub = |name: &str| -> Option<String> {
        let k = canon(name);
        if k == "π" || k == "e" { return None; }
        if let Some(t) = imag_unit(env, name) { return Some(t); }
        let v = env.vars.get(&k).cloned().or_else(|| constant(&k).filter(|v| !v.cx())).filter(|v| !v.vec && !v.logic)?;
        // An angle is put in as radians: e^(0.3·2.840 rad), not e^(0.3·162.7°), which reads as 48.8 in the exponent.
        let want = if units::is_none(&v.dim) && v.dim[7] != 0 { "rad" } else { env.shown.get(&k).map(|u| u.as_str()).unwrap_or("") };
        let t = show_val(&v, want, f).1;
        // A complex value inside a product is bracketed: (3 + 4j) · 2.
        Some(if v.cx() && !t.starts_with("\\left(") { format!("\\left({t}\\right)") } else { t })
    };
    latex::node_with(e, &sub)
}

fn region(env: &mut Env, r: &Region, f: &Fmt, known: &mut Vec<String>, subst: bool) -> RegionOut {
    let mut o = RegionOut { id: r.id, ..Default::default() };
    env.steps = 0;
    if r.kind == Kind::Table { table(env, r, f, known, &mut o); return o; }
    if r.kind == Kind::Material { material(env, r, f, known, &mut o); return o; }
    if r.kind == Kind::Program { program(env, r, f, known, &mut o); return o; }
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
    // j and i, when they are the imaginary unit rather than a name of the sheet, are drawn upright like a unit.
    let bound: Vec<&str> = match &st { Stmt::Func { params, .. } => params.iter().map(|p| p.as_str()).collect(), Stmt::Solve { var, .. } | Stmt::Plot { var, .. } => vec![var.as_str()], Stmt::SolveSys { vars, .. } => vars.iter().map(|v| v.0.as_str()).collect(), _ => vec![] };
    o.latex = latex::stmt_with(&st, &|v| imag_unit(env, v).filter(|_| !bound.contains(&v)));
    // simplify, expand and factor give an exact expression, not a value.
    let sym_call = |e: &Node| match &e.e { E::Call(fname, args) if crate::sym::OPS.contains(&fname.as_str()) && args.len() == 1 => Some((fname.clone(), args[0].clone())), _ => None };
    let symbolic = match &st { Stmt::Show { e } => sym_call(e).map(|c| (None, c, e.at)), Stmt::Assign { name, e, .. } => sym_call(e).map(|c| (Some(name.clone()), c, e.at)), _ => None };
    if let Some((name, (op, arg), at)) = symbolic {
        let nm = |v: &str| latex::name(v);
        let res = crate::sym::from_node(&arg, &|v| env.look(v)).and_then(|r| {
            let (t, x) = if op == "factor" {
                let (mut n, mut d) = (crate::sym::factor(&r.num)?, crate::sym::factor(&r.den)?);
                // One whole-number constant on each side: 3/(2(x + 1)(x − 1)), not (3/2)/((x + 1)(x − 1)).
                if r.den.as_const() != Some(crate::sym::Q::ONE) { let q = crate::sym::Q::new(n.c.n * d.c.d, n.c.d * d.c.n).map_err(|e| e.to_string())?; (n.c, d.c) = (crate::sym::Q::int(q.n), crate::sym::Q::int(q.d)); }
                let (nt, nx) = crate::sym::factored_text(&n, &nm);
                if r.den.as_const() == Some(crate::sym::Q::ONE) { (nt, nx) } else { let (dt, dx) = crate::sym::factored_text(&d, &nm); (format!("{}/{}", crate::sym::bracket(nt, crate::sym::factored_term(&n)), crate::sym::bracket(dt, crate::sym::factored_atom(&d))), format!("\\frac{{{nx}}}{{{dx}}}")) }
            } else { crate::sym::ratf_text(&r, &nm) };
            Ok((r, t, x))
        });
        match res {
            Ok((r, t, x)) => {
                o.result = t.clone();
                o.value = t;
                o.result_tex = x;
                if let Some(n) = name {
                    o.name = canon(&n);
                    env.vars.remove(&canon(&n));
                    env.exprs.insert(canon(&n), r);
                    known.push(n);
                }
            }
            Err(m) => { o.error = Some(ErrOut { msg: m, from: at.0, to: at.1 }); if let Some(n) = name { o.name = canon(&n); } }
        }
        return o;
    }
    let res: R<()> = (|| {
        match &st {
            Stmt::Empty => {}
            Stmt::Assign { name, e, .. } => {
                o.name = canon(name);
                let v = env.eval(e)?;
                o.input = matches!(e.e, E::Num(..)) || matches!(&e.e, E::Neg(x) if matches!(x.e, E::Num(..))) || complex_literal(e);
                if subst && !o.input { o.subst_tex = substituted(env, e, f); }
                // An input keeps the unit it was typed in (8356 cm⁴ stays cm⁴), unless it belongs to the other unit system.
                let typed = match &e.e { E::Num(_, Some(u)) | E::Unit(_, u) => Some(u.text.clone()), E::Im(_, x) => match &x.e { E::Num(_, Some(u)) => Some(u.text.clone()), _ => None }, E::Neg(x) => match &x.e { E::Num(_, Some(u)) => Some(u.text.clone()), _ => None }, _ => None };
                match typed.filter(|t| r.unit.trim().is_empty() && (units::system(t).is_none_or(|us| us == f.us) || !units::has_ladder(&v.dim, f.us))) {
                    Some(t) => set_result(&mut o, &v, &Region { unit: t, ..r.clone() }, f),
                    None => set_result(&mut o, &v, r, f),
                }
                env.exprs.remove(&canon(name)); env.vars.insert(canon(name), v);
                known.push(name.clone());
            }
            Stmt::Func { name, params, e } => {
                o.name = canon(name);
                env.exprs.remove(&canon(name));
                env.funcs.insert(canon(name), (params.clone(), e.clone()));
                known.push(name.clone());
            }
            Stmt::Range { name, start, next, end, .. } => {
                o.name = canon(name);
                let v = range_vals(env, start, next.as_ref(), end)?;
                set_result(&mut o, &v, r, f);
                env.exprs.remove(&canon(name)); env.vars.insert(canon(name), v);
                known.push(name.clone());
            }
            Stmt::Check { e } => {
                let E::Bin(op, a, b) = &e.e else { return fail("A check compares two values: check a <= b.", e.at) };
                if !matches!(*op, "<" | "<=" | ">" | ">=" | "==" | "!=") { return fail("A check compares two values with <, <=, >, >= or ==.", e.at); }
                let (x, y) = (env.eval(a)?, env.eval(b)?);
                real(&x, "A check", a.at)?;
                real(&y, "A check", b.at)?;
                let t = bin(op, x.clone(), y.clone(), e.at, b.at)?;
                if subst { o.subst_tex = substituted(env, e, f); }
                if t.vec { return fail("A check needs single values, not vectors (use max or min).", e.at); }
                let pass = t.x() != 0.0;
                // A utilisation of absolute temperatures (in kelvin) means nothing: 30 °C against 60 °C is not 91 %.
                let ratio = match *op { _ if x.abs || y.abs => None, "<" | "<=" if y.x() != 0.0 => Some(x.x() / y.x()), ">" | ">=" if x.x() != 0.0 => Some(y.x() / x.x()), _ => None };
                // The left side reads in the check's own unit, else in the unit its name is shown in (3 %, not 0.03).
                let want = if r.unit.trim().is_empty() { match &a.e { E::Var(n) => env.shown.get(&canon(n)).cloned().unwrap_or_default(), _ => String::new() } } else { r.unit.clone() };
                let (lt, _, _, lu) = show_val(&x, &want, f);
                let (rt, _, _, _) = show_val(&y, &lu, f);
                o.check = Some(CheckOut { pass, ratio, lhs: lt, rhs: rt, op: op.to_string() });
                let sym = match *op { "<=" => "≤", ">=" => "≥", "!=" => "≠", "==" => "=", o => o };
                o.result = format!("{} ({} {sym} {})", if pass { "PASS" } else { "FAIL" }, o.check.as_ref().unwrap().lhs, o.check.as_ref().unwrap().rhs);
            }
            Stmt::Solve { lhs, rhs, var, lo, hi } => {
                o.name = canon(var);
                let (a, b) = (env.eval(lo)?, env.eval(hi)?);
                real(&a, "solve", lo.at)?;
                real(&b, "solve", hi.at)?;
                same_or(&a, &b, "use", hi.at)?;
                let k = canon(var);
                let saved = env.vars.get(&k).cloned();
                let mut first_err: Option<Error> = None;
                let mut fx = |x: f64| -> f64 {
                    env.vars.insert(k.clone(), Val { v: vec![x], ..a.clone() });
                    match env.eval(lhs).and_then(|l| { let rr = env.eval(rhs)?; real(&l, "solve", lhs.at)?; real(&rr, "solve", rhs.at)?; if l.vec || rr.vec { return fail("solve needs an equation between single values; for a vector or matrix equation use lsolve, or solve its entries together.", (lhs.at.0, rhs.at.1)); } same_or(&l, &rr, "compare", rhs.at)?; Ok(l.x() - rr.x()) }) {
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
                env.exprs.remove(&k);
                env.vars.insert(k, v);
                known.push(var.clone());
            }
            Stmt::SolveSys { eqs, vars } => {
                let n = vars.len();
                if eqs.len() != n { return fail(format!("{} equation{} for {} unknown{}: give as many equations as unknowns.", eqs.len(), if eqs.len() == 1 { "" } else { "s" }, n, if n == 1 { "" } else { "s" }), (eqs[0].0.at.0, vars[n - 1].1.at.1)); }
                if n > 20 { return fail("At most 20 equations can be solved together.", vars[20].1.at); }
                // The guesses give each unknown its starting value and its unit.
                let mut g = vec![];
                for (_, e) in vars { let v = env.eval(e)?; real(&v, "A starting guess", e.at)?; if v.vec { return fail("A starting guess is one value.", e.at); } g.push(v); }
                let keys: Vec<String> = vars.iter().map(|v| canon(&v.0)).collect();
                let saved: Vec<Option<Val>> = keys.iter().map(|k| env.vars.get(k).cloned()).collect();
                let restore = |env: &mut Env| for (k, s) in keys.iter().zip(&saved) { match s { Some(v) => { env.vars.insert(k.clone(), v.clone()); } None => { env.vars.remove(k); } } };
                // Each equation as left − right. It is measured against the size of its terms (the larger side, or
                // how strongly it moves with the unknowns), so equations in different units weigh alike and an
                // equation of small quantities is not held to one SI unit.
                let residual = |env: &mut Env, x: &[f64]| -> R<(Vec<f64>, Vec<f64>)> {
                    for (i, k) in keys.iter().enumerate() { env.vars.insert(k.clone(), Val { v: vec![x[i]], ..g[i].clone() }); }
                    let (mut d, mut size) = (vec![0.0; n], vec![0.0; n]);
                    for (i, (a, b)) in eqs.iter().enumerate() {
                        let (l, rr) = (env.eval(a)?, env.eval(b)?);
                        real(&l, "solve", a.at)?;
                        real(&rr, "solve", b.at)?;
                        if l.vec || rr.vec { return fail("Each equation is between single values.", (a.at.0, b.at.1)); }
                        same_or(&l, &rr, "compare", (a.at.0, b.at.1))?;
                        d[i] = l.x() - rr.x();
                        size[i] = l.x().abs().max(rr.x().abs());
                    }
                    Ok((d, size))
                };
                let nf = || Error { msg: "No solution found from these guesses: try others closer to the answer.".into(), at: (eqs[0].0.at.0, vars[n - 1].1.at.1) };
                let rel = |d: &[f64], size: &[f64], term: &[f64]| d.iter().enumerate().fold(0.0f64, |m, (i, v)| {
                    let s = size[i].max(term[i]);
                    m.max(if s > 0.0 && s.is_finite() { v.abs() / s } else { v.abs() })
                });
                let mut x: Vec<f64> = g.iter().map(|v| v.x()).collect();
                let res: R<Vec<f64>> = (|| {
                    let (mut d, mut size) = residual(env, &x)?;
                    let mut term = vec![0.0; n];
                    for step in 0..200 {
                        // The Jacobian by forward differences, each unknown relative to its size (so MPa beside mm²
                        // or a strain stays well conditioned); a step of √ε, larger if that moves nothing.
                        let cs: Vec<f64> = (0..n).map(|j| if x[j] != 0.0 { x[j].abs() } else if g[j].x() != 0.0 { g[j].x().abs() } else { 1.0 }).collect();
                        let mut jac = vec![vec![0.0; n]; n];
                        for j in 0..n {
                            for h in [1.5e-8, 1e-5, 1e-2].map(|e| e * cs[j]) {
                                let mut xh = x.clone();
                                xh[j] += h;
                                let (dh, _) = residual(env, &xh)?;
                                for i in 0..n { jac[i][j] = (dh[i] - d[i]) / h * cs[j]; }
                                if (0..n).any(|i| jac[i][j] != 0.0) { break; }
                            }
                        }
                        for i in 0..n { term[i] = jac[i].iter().map(|v| v.abs()).sum(); }
                        let m = rel(&d, &size, &term);
                        if m < 1e-12 { return Ok(x.clone()); }
                        // Each row divided by its size, then a Newton step, halved until it does better.
                        let s: Vec<f64> = (0..n).map(|i| { let s = size[i].max(term[i]); if s > 0.0 && s.is_finite() { s } else { 1.0 } }).collect();
                        let a = jac.iter().zip(&s).map(|(r, s)| r.iter().map(|v| v / s).collect()).collect();
                        // Singular at the guess: the equations do not fix the unknowns. Later on the way: no solution there.
                        let Some(dy) = crate::solve::linear(a, d.iter().zip(&s).map(|(v, s)| -v / s).collect()) else {
                            return Err(if step == 0 { Error { msg: "The equations do not fix every unknown here (they are not independent at this point): check that each unknown appears, or try other guesses.".into(), at: (eqs[0].0.at.0, eqs[n - 1].1.at.1) } } else { nf() });
                        };
                        let mut lam = 1.0;
                        loop {
                            let xn: Vec<f64> = x.iter().zip(dy.iter().zip(&cs)).map(|(a, (d, c))| a + lam * d * c).collect();
                            match residual(env, &xn) {
                                Ok((dn, sn)) if dn.iter().all(|v| v.is_finite()) && (rel(&dn, &sn, &term) < m || lam < 1e-4) => {
                                    let small = dy.iter().all(|v| (lam * v).abs() <= 1e-14);
                                    (x, d, size) = (xn, dn, sn);
                                    if small && rel(&d, &size, &term) < 1e-9 { return Ok(x.clone()); }
                                    break;
                                }
                                _ if lam < 1e-4 => return Err(nf()),
                                _ => lam /= 2.0,
                            }
                        }
                    }
                    if rel(&d, &size, &term) < 1e-9 { Ok(x.clone()) } else { Err(nf()) }
                })();
                let x = match res { Ok(x) => x, Err(e) => { restore(env); return Err(e); } };
                let (mut texts, mut texes) = (vec![], vec![]);
                for (i, (name, _)) in vars.iter().enumerate() {
                    let v = Val { v: vec![x[i]], ..g[i].clone() };
                    let (t, tx, value, unit) = show_val(&v, "", f);
                    texts.push(format!("{} = {t}", canon(name)));
                    texes.push(format!("{} = {tx}", latex::name(name)));
                    o.defs.push(VarOut { name: keys[i].clone(), tex: latex::name(name), value, unit: unit.clone(), id: r.id });
                    if !unit.is_empty() { env.shown.insert(keys[i].clone(), unit); }
                    env.exprs.remove(&keys[i]);
                    env.vars.insert(keys[i].clone(), v);
                    known.push(name.clone());
                }
                o.name = keys[0].clone();
                o.result = texts.join(", ");
                o.result_tex = format!("\\Rightarrow\\; {}", texes.join(",\\; "));
            }
            Stmt::Plot { ys, var, lo, hi, log } => {
                let (a, b) = (env.eval(lo)?, env.eval(hi)?);
                real(&a, "A plot range", lo.at)?;
                real(&b, "A plot range", hi.at)?;
                same_or(&a, &b, "use", hi.at)?;
                if !(b.x() - a.x()).is_finite() || a.x() == b.x() { return fail("The plot range must be two different finite values.", (lo.at.0, hi.at.1)); }
                let k = canon(var);
                let saved = env.vars.get(&k).cloned();
                let n = 121;
                if *log && (a.x() <= 0.0 || b.x() <= 0.0 || a.abs) { return fail("A log scale needs a range above zero, such as 10 Hz .. 100 kHz.", (lo.at.0, hi.at.1)); }
                // A log scale spaces the points evenly in decades: as many per decade over 10 Hz .. 100 kHz as 0 .. 10 gets on a line.
                let xs: Vec<f64> = (0..n).map(|i| { let t = i as f64 / (n - 1) as f64; if *log { a.x() * (b.x() / a.x()).powf(t) } else { a.x() + (b.x() - a.x()) * t } }).collect();
                let mut p = PlotOut { logx: *log, ..Default::default() };
                let (_, _, _, xu) = show_val(&a, "", f);
                let (xk, xo, _) = units::display(a.x(), &shown_dim(&a), a.abs, &xu, f.us);
                p.xs = xs.iter().map(|x| (x - xo) / xk).collect();
                p.xlabel = show(var);
                p.xunit = xu;
                let mut ydim: Option<(Dim, bool)> = None;
                let mut res = Ok(());
                // Each curve's angle tag as shown, so f (cycles) and ω (radians) on one plot share the axis' reading.
                let mut tags: Vec<i8> = vec![];
                for y in ys {
                    let mut vals = vec![];
                    let mut tag = 0;
                    for x in &xs {
                        env.vars.insert(k.clone(), Val { v: vec![*x], ..a.clone() });
                        match env.eval(y) {
                            Ok(v) if v.cx() => { res = fail("A plot shows real values: plot abs(z), db(z) or arg(z) (a Bode plot: add log after the range).", y.at); break; }
                            Ok(v) => {
                                if let Some((d, _)) = &ydim { if !units::same(d, &v.dim) { res = fail("Every curve on a plot must have the same units.", y.at); break; } } else { ydim = Some((shown_dim(&v), v.abs)); }
                                tag = shown_dim(&v)[7];
                                vals.push(if v.vec { f64::NAN } else { v.x() });
                            }
                            Err(_) => vals.push(f64::NAN),
                        }
                    }
                    if vals.iter().all(|v| v.is_nan()) { res = env.eval(y).map(|_| ()); if res.is_ok() { res = fail("This curve has no values in the range.", y.at); } }
                    p.series.push(Series { label: latex::plain(y), ys: vals });
                    tags.push(tag);
                    if res.is_err() { break; }
                }
                match saved { Some(v) => { env.vars.insert(k, v); } None => { env.vars.remove(&k); } }
                res?;
                if let Some((d, abs)) = ydim {
                    if units::same(&d, &[0, 0, -1, 0, 0, 0, 0, 0]) {
                        for (s, t) in p.series.iter_mut().zip(&tags) { if *t != d[7] { let c = (2.0 * std::f64::consts::PI).powi((d[7] - t) as i32); s.ys.iter_mut().for_each(|v| *v *= c); } }
                    }
                    let big = p.series.iter().flat_map(|s| s.ys.iter()).copied().filter(|v| v.is_finite()).fold(0.0f64, |m, v| m.max(v.abs()));
                    let (yk, yo, yl) = units::display(big, &d, abs, &r.unit, f.us);
                    for s in &mut p.series { for v in &mut s.ys { *v = (*v - yo) / yk; } }
                    p.yunit = yl;
                }
                o.plot = Some(p);
            }
            Stmt::Show { e } => {
                let v = env.eval(e)?;
                if subst { o.subst_tex = substituted(env, e, f); }
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
    o.units = unit_choices(&v.dim, v.abs, v.nm);
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
                .map(|v| if v.vec || n == 0 { v } else { let (x, y) = v.at(0); Val { v: vec![x; n], im: if v.cx() { vec![y; n] } else { vec![] }, vec: true, ..v } })
        } else {
            let (mut vals, mut ims) = (Vec::with_capacity(n), Vec::with_capacity(n));
            let mut dim: Option<units::Dim> = None;
            let mut res = Ok(());
            let mut abs = false;
            for (i, row) in d.rows.iter().take(n).enumerate() {
                let cell = row.get(j).map(|x| x.trim()).unwrap_or("");
                if cell.is_empty() { vals.push(f64::NAN); ims.push(0.0); continue; }
                let v = match parse::statement(cell, known) { Ok(Stmt::Show { e }) => env.eval(&e).map_err(|e| e.msg), Ok(_) => Err("a cell holds a value".into()), Err(e) => Err(e.msg) };
                let v = match (v, &unit) {
                    (Ok(v), Some(u)) if v.cx() && u.offset != 0.0 => Err(format!("{} cannot be complex.", dim_name(&u.dim))),
                    (Ok(v), Some(u)) if units::is_none(&v.dim) && !units::is_none(&u.dim) => { let (x, y) = v.at(0); Ok(Val { v: vec![x * u.factor + u.offset], im: if v.cx() { vec![y * u.factor] } else { vec![] }, dim: u.dim, abs: u.offset != 0.0, ..Default::default() }) }
                    (v, _) => v,
                };
                match v {
                    Ok(v) if v.vec => { res = Err(format!("Row {}: a cell holds one value.", i + 1)); break; }
                    Ok(v) => {
                        if let Some(d0) = &dim { if !units::same(d0, &v.dim) { res = Err(format!("Row {}: {} does not match the rest of the column.", i + 1, dim_name(&v.dim))); break; } } else { dim = Some(v.dim); abs = v.abs; }
                        vals.push(v.x());
                        ims.push(v.at(0).1);
                    }
                    Err(m) => { res = Err(format!("Row {}: {m}", i + 1)); break; }
                }
            }
            // A column with a complex cell is complex (an impedance per frequency); otherwise real.
            let im = if ims.iter().any(|y| *y != 0.0) { ims } else { vec![] };
            res.map(|_| Val { v: vals, im, dim: dim.or(unit.as_ref().map(|u| u.dim)).unwrap_or(units::NONE), abs, vec: true, ..Default::default() })
        };
        match val {
            Ok(v) => {
                if v.v.len() != n && n > 0 { co.error = format!("{} values for {} rows.", v.v.len(), n); }
                let big = (0..v.v.len()).map(|i| { let (x, y) = v.at(i); x.hypot(y) }).filter(|x| x.is_finite()).fold(0.0f64, f64::max);
                let (k, off, label) = units::display(big, &shown_dim(&v), v.abs, &c.unit, f.us);
                co.unit = label;
                co.cells = (0..n).map(|i| v.v.get(i).filter(|x| x.is_finite()).map(|x| if v.cx() { fmt_cx((x - off) / k, v.at(i).1 / k, f).0 } else { fmt_val((x - off) / k, f).0 }).unwrap_or_default()).collect();
                if !name.is_empty() && co.error.is_empty() {
                    let (_, _, value, unit) = show_val(&v, &c.unit, f);
                    o.defs.push(VarOut { name: canon(name), tex: latex::name(name), value, unit, id: r.id });
                    env.exprs.remove(&canon(name)); env.vars.insert(canon(name), v);
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

/// A material region: each property the material has becomes a name (ρ_steel, σ_y,steel) with its unit, from
/// the values kept in the sheet (or the library, for a sheet written by hand), at the end of the range chosen.
fn material(env: &mut Env, r: &Region, f: &Fmt, known: &mut Vec<String>, o: &mut RegionOut) {
    use crate::materials::{self as mats, KEYS};
    let d: mats::MatData = serde_json::from_value(r.data.clone()).unwrap_or_default();
    let Some(m) = d.mat.clone().or_else(|| mats::find(&d.id).cloned()) else {
        o.error = Some(ErrOut { msg: if d.id.is_empty() { "Choose a material from the list.".into() } else { format!("“{}” is not in this version's materials.", d.id) }, from: 0, to: 0 });
        return;
    };
    let mut mo = MatOut { title: m.name.clone(), family: m.family.clone(), des: m.des.clone(), notes: m.notes.clone(), pick: if d.pick.is_empty() { "typical".into() } else { d.pick.clone() }, draft: m.status == "draft", ..Default::default() };
    if !mats::valid_name(&d.name) {
        o.material = Some(mo);
        o.error = Some(ErrOut { msg: "Give the material a short name of letters and digits, such as steel: its properties become ρ_steel, E_steel …".into(), from: 0, to: 0 });
        return;
    }
    // Names another material (or line) above already defined are replaced from here: say so, as a second
    // material given the same short name would otherwise change the first one's values silently.
    let reused = KEYS.iter().filter(|k| m.props.contains_key(k.0)).any(|k| env.vars.contains_key(&canon(&format!("{}_{}", k.1, d.name))));
    if reused { o.warn = format!("Names ending _{} are already defined above; this material's values replace them from here. Give it another name to keep both.", d.name); }
    for (key, base, what, unit) in KEYS {
        let Some((lo, hi, src)) = m.props.get(key) else { continue };
        if !lo.is_finite() || !hi.is_finite() { continue; }
        let u = if unit.is_empty() { None } else { units::parse(unit).ok() };
        let val = |x: f64| match &u { Some(u) => Val { v: vec![x * u.factor + u.offset], dim: u.dim, abs: u.offset != 0.0, ..Default::default() }, None => Val::plain(x) };
        let v = val(mats::pick(*lo, *hi, &d.pick));
        // Shown in the unit the library keeps (US units on a US sheet).
        // % is the same in both systems: elongation reads 21 %, not 0.21.
        let want = if unit.is_empty() || (f.us && unit != "%") { String::new() } else { units::pretty(unit) };
        let (_, _, value, shown) = show_val(&v, &want, f);
        let (_, _, a, _) = show_val(&val(*lo), &shown, f);
        let (_, _, b, _) = show_val(&val(*hi), &shown, f);
        let raw = format!("{base}_{}", d.name);
        let k = canon(&raw);
        mo.rows.push(MatRow { tex: latex::name(&raw), what: what.to_string(), value: value.clone(), unit: if key == "hv" { "HV".into() } else { shown.clone() }, range: if a == b { String::new() } else { format!("{a} – {b}") }, source: d.sources.get(src).cloned().unwrap_or_else(|| mats::source_title(src)) });
        o.defs.push(VarOut { name: k.clone(), tex: latex::name(&raw), value, unit: shown.clone(), id: r.id });
        env.shown.insert(k.clone(), shown);
        env.exprs.remove(&k);
        env.vars.insert(k, v);
        known.push(raw);
    }
    let mut srcs: Vec<String> = mo.rows.iter().map(|x| x.source.clone()).collect();
    srcs.sort();
    srcs.dedup();
    mo.sources = srcs;
    o.material = Some(mo);
}

/// The code of a line and its description after `#`.
pub fn split_note(t: &str) -> (&str, &str) {
    match t.split_once('#') { Some((c, n)) => (c, n.trim()), None => (t, "") }
}
