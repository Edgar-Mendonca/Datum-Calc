//! Dynamic systems (0.2.11): a transfer function is a function of s the sheet defines, G(s) = K/(τ*s + 1), made of
//! sums, products, quotients and whole powers of s, other transfer functions of s and constants with units. From it:
//! poles and zeros, the steady gain, step and impulse responses in time, the step's overshoot, rise and settling
//! times, damping and natural frequency, and a loop's gain and phase margins with their crossover frequencies.
//! Units are checked throughout: every term of a sum must agree, so G carries the units of output over input.
use crate::eval::{canon, Env, Val};
use crate::parse::{Error, Node, Span, E};
use crate::units::{self, Dim, ANGLE, NONE};

type R<T> = Result<T, Error>;
fn fail<T>(msg: impl Into<String>, at: Span) -> R<T> { Err(Error { msg: msg.into(), at }) }

const TIME: Dim = [0, 0, 1, 0, 0, 0, 0, 0];
const FREQ: Dim = [0, 0, -1, 0, 0, 0, 0, 0];
/// The highest order a transfer function may have.
const MAX_ORDER: usize = 20;

/// The functions of this module: name, arguments, what it does (for help and the window's menus).
pub const FUNCTIONS: [(&str, &str); 15] = [
    ("poles(G)", "poles of a transfer function G(s), in rad/s (complex in pairs)"),
    ("zeros(G)", "zeros of a transfer function G(s), in rad/s"),
    ("dcgain(G)", "steady gain G(0): the output per unit of input once settled"),
    ("step(G, t)", "response at time t to a unit step in the input; step(G, t, u) for a step of size u"),
    ("impulse(G, t)", "response at time t to a unit impulse in the input"),
    ("overshoot(G)", "peak of the step response above its final value, as a fraction (choose % in the unit menu)"),
    ("risetime(G)", "time for the step response to go from 10 % to 90 % of its final value"),
    ("settling(G)", "time after which the step response stays within 2 % of its final value"),
    ("peaktime(G)", "time of the step response's highest value"),
    ("wn(G)", "natural frequency of the dominant poles (those nearest the imaginary axis)"),
    ("zeta(G)", "damping ratio of the dominant poles"),
    ("pm(L)", "phase margin of a loop gain L(s) (the loop with no units), at its gain crossover"),
    ("gm(L)", "gain margin of a loop gain L(s) as a ratio; db(gm(L)) in decibels"),
    ("wgc(L)", "gain crossover frequency: where |L(jω)| = 1"),
    ("wpc(L)", "phase crossover frequency: where L(jω) crosses −180°"),
];

pub fn is_fn(f: &str) -> bool { FUNCTIONS.iter().any(|x| x.0.split('(').next() == Some(f)) }

/// A polynomial in s: coefficients of s⁰, s¹, … in SI units, and the dimension every term has.
#[derive(Clone, Debug)]
struct Poly { c: Vec<f64>, d: Dim }

impl Poly {
    fn zero(&self) -> bool { self.c.iter().all(|x| *x == 0.0) }
    fn mul(&self, o: &Poly) -> Poly {
        let mut c = vec![0.0; self.c.len() + o.c.len() - 1];
        for (i, a) in self.c.iter().enumerate() { for (j, b) in o.c.iter().enumerate() { c[i + j] += a * b; } }
        Poly { c, d: units::mul(&self.d, &o.d) }
    }
    fn add(&self, o: &Poly, neg: bool, at: Span) -> R<Poly> {
        if !self.zero() && !o.zero() && !units::same(&self.d, &o.d) {
            return fail(format!("Terms with different units are added here ({} and {}): every term of a transfer function needs the same units.", units::label(&self.d), units::label(&o.d)), at);
        }
        let mut c = vec![0.0; self.c.len().max(o.c.len())];
        for (i, a) in self.c.iter().enumerate() { c[i] += a; }
        for (i, b) in o.c.iter().enumerate() { c[i] += if neg { -b } else { *b }; }
        Ok(Poly { c, d: if self.zero() { o.d } else { self.d } })
    }
}

/// A quotient of polynomials in s.
#[derive(Clone, Debug)]
struct Rat { n: Poly, d: Poly }

impl Rat {
    fn konst(x: f64, d: Dim) -> Rat { Rat { n: Poly { c: vec![x], d }, d: Poly { c: vec![1.0], d: NONE } } }
    fn mul(&self, o: &Rat) -> Rat { Rat { n: self.n.mul(&o.n), d: self.d.mul(&o.d) } }
    fn div(&self, o: &Rat, at: Span) -> R<Rat> {
        if o.n.zero() { return fail("This divides by zero.", at); }
        Ok(Rat { n: self.n.mul(&o.d), d: self.d.mul(&o.n) })
    }
    fn add(&self, o: &Rat, neg: bool, at: Span) -> R<Rat> { Ok(Rat { n: self.n.mul(&o.d).add(&o.n.mul(&self.d), neg, at)?, d: self.d.mul(&o.d) }) }
}

/// A transfer function ready to use: num/den with den monic, both in SI, and the units of G.
#[derive(Clone, Debug)]
pub struct Tf { num: Vec<f64>, den: Vec<f64>, dim: Dim, name: String }

fn mentions(n: &Node, p: &str) -> bool {
    match &n.e {
        E::Var(v) => canon(v) == p,
        E::Num(..) => false,
        E::Neg(a) | E::Group(a) | E::Unit(a, _) | E::Im(_, a) => mentions(a, p),
        E::Bin(_, a, b) => mentions(a, p) || mentions(b, p),
        E::Call(_, args) => args.iter().any(|a| mentions(a, p)),
        E::Mat(_, rows) => rows.iter().flatten().any(|a| mentions(a, p)),
        E::Index(a, ix) => mentions(a, p) || ix.iter().any(|a| mentions(a, p)),
    }
}

/// What reading a transfer function needs: its variable, the units s has, and whether any constant had units.
struct Ctx<'a> { p: &'a str, s: Dim, united: bool }

fn rat(env: &mut Env, n: &Node, cx: &mut Ctx, depth: usize) -> R<Rat> {
    let p = cx.p;
    let at = n.at;
    if !mentions(n, p) {
        let v = env.eval(n)?;
        if v.vec || v.mat() { return fail("A transfer function is made of single values, not vectors.", at); }
        if v.cx() { return fail("A transfer function has real coefficients: write j only in G(j*ω).", at); }
        if v.abs { return fail("A transfer function works with temperature differences, not temperatures in °C or °F.", at); }
        if !v.x().is_finite() { return fail("This value is not finite.", at); }
        cx.united |= !units::is_none(&v.dim);
        return Ok(Rat::konst(v.x(), v.dim));
    }
    match &n.e {
        E::Var(_) => Ok(Rat { n: Poly { c: vec![0.0, 1.0], d: cx.s }, d: Poly { c: vec![1.0], d: NONE } }),
        E::Group(a) => rat(env, a, cx, depth),
        E::Neg(a) => { let r = rat(env, a, cx, depth)?; Ok(Rat { n: Poly { c: r.n.c.iter().map(|x| -x).collect(), d: r.n.d }, d: r.d }) }
        E::Unit(a, u) => {
            if u.offset != 0.0 { return fail("A transfer function works with temperature differences, not temperatures in °C or °F.", at); }
            cx.united |= !units::is_none(&u.dim);
            Ok(rat(env, a, cx, depth)?.mul(&Rat::konst(u.factor, u.dim)))
        }
        E::Bin(op, a, b) => {
            let x = rat(env, a, cx, depth)?;
            match *op {
                "^" => {
                    if mentions(b, p) { return fail("A power of s must be a whole number, not a function of s.", b.at); }
                    let k = env.eval(b)?;
                    if !units::is_none(&k.dim) || k.vec || k.cx() || k.x().fract() != 0.0 || k.x().abs() > MAX_ORDER as f64 {
                        return fail(format!("Powers in a transfer function are whole numbers up to {MAX_ORDER}."), b.at);
                    }
                    let mut r = Rat::konst(1.0, NONE);
                    for _ in 0..k.x().abs() as usize { r = r.mul(&x); }
                    if k.x() < 0.0 { r = Rat::konst(1.0, NONE).div(&r, at)?; }
                    Ok(r)
                }
                "+" | "-" | "*" | "/" => {
                    let y = rat(env, b, cx, depth)?;
                    match *op { "+" => x.add(&y, false, at), "-" => x.add(&y, true, at), "*" => Ok(x.mul(&y)), _ => x.div(&y, b.at) }
                }
                _ => fail("A transfer function is made of +, −, ×, ÷ and whole powers of s.", at),
            }
        }
        // Another transfer function of the same s: C(s)*G(s), G(s)/(1 + G(s)).
        E::Call(f, args) if args.len() == 1 && matches!(&args[0].e, E::Var(v) if canon(v) == p) => {
            let Some((params, body)) = env.funcs.get(&canon(f)).cloned() else {
                return fail(format!("{f}(s) is not a transfer function the sheet defines: a transfer function is made of sums, products, quotients and whole powers of s (a delay e^(−θs) is not one)."), at);
            };
            if params.len() != 1 { return fail(format!("{f} takes {} values: a transfer function takes s alone.", params.len()), at); }
            if depth > 20 { return fail(format!("{f} calls itself too deeply."), at); }
            let q = canon(&params[0]);
            let mut inner = Ctx { p: &q, s: cx.s, united: cx.united };
            let r = rat(env, &body, &mut inner, depth + 1);
            cx.united = inner.united;
            r.map_err(|e| if e.at.0 < body.at.0 || e.at.1 > body.at.1 { e } else { Error { msg: format!("In {f}: {}", e.msg), at } })
        }
        E::Call(f, _) => fail(format!("{f}(…) of s does not make a transfer function: use sums, products, quotients and whole powers of s (a delay e^(−θs) is not one)."), at),
        _ => fail("A transfer function is made of +, −, ×, ÷ and whole powers of s.", at),
    }
}

/// The transfer function a call names: poles(G) with G(s) = … defined above.
fn tf(env: &mut Env, f: &str, arg: &Node) -> R<Tf> {
    let at = arg.at;
    let name = match &arg.e { E::Var(v) => canon(v), _ => return fail(format!("{f} takes the name of a transfer function defined on the sheet: G(s) = 10/(s + 2), then {f}(G)."), at) };
    let Some((params, body)) = env.funcs.get(&name).cloned() else {
        return fail(format!("{name} is not a transfer function: define it above as {name}(s) = …, then use {f}({name})."), at);
    };
    if params.len() != 1 { return fail(format!("{name} takes {} values: a transfer function takes s alone.", params.len()), at); }
    // s is in 1/s.
    let q = canon(&params[0]);
    let mut cx = Ctx { p: &q, s: FREQ, united: false };
    let r = match rat(env, &body, &mut cx, 0) {
        // A transfer function in plain numbers alone has none: s in 1/s and times in seconds, G a plain ratio.
        Err(e) if !cx.united && !e.msg.starts_with("Terms with different units") => Err(e),
        _ if !cx.united => { let mut plain = Ctx { p: &q, s: NONE, united: false }; rat(env, &body, &mut plain, 0) }
        r => r,
    };
    // Inside a function of s, s is the variable: kg/s there is kg over s, and 0.5 s is half a second.
    let r = r.map_err(|mut e| {
        if e.msg.starts_with("Terms with different units") { e.msg.push_str(&format!(" Inside {name}({q}), {q} is the variable: give values with units such as 3 kg/s their own lines above, then use their names.")); }
        e
    });
    let r = r.map_err(|e| if e.at.0 < body.at.0 || e.at.1 > body.at.1 { e } else { Error { msg: format!("In {name}: {}", e.msg), at } })?;
    if r.n.zero() { return fail(format!("{name} is zero for every s."), at); }
    // Units: every term of the numerator has r.n.d, of the denominator r.d.d; radians drop out (rad/s is 1/s here).
    let mut dim = units::div(&r.n.d, &r.d.d);
    dim[7] = 0;
    let (mut num, mut den) = (r.n.c, r.d.c);
    // Common factors of s cancel exactly (Kp + Ki/s over 1, and the like).
    while num.len() > 1 && den.len() > 1 && num[0] == 0.0 && den[0] == 0.0 { num.remove(0); den.remove(0); }
    let trim = |c: &mut Vec<f64>| while c.len() > 1 && c.last() == Some(&0.0) { c.pop(); };
    trim(&mut num);
    trim(&mut den);
    if den.len() - 1 > MAX_ORDER || num.len() - 1 > MAX_ORDER { return fail(format!("{name} is of order above {MAX_ORDER}."), at); }
    let lead = *den.last().unwrap();
    for x in num.iter_mut().chain(den.iter_mut()) { *x /= lead; }
    if num.iter().chain(&den).any(|x| !x.is_finite()) { return fail(format!("{name} has coefficients too large or too small to work with."), at); }
    // Factors common to both cancel: G/(1 + G) keeps only the closed loop's own poles. A root of the top that is also
    // one of the bottom (to round-off) is divided out of both, one at a time, a complex pair as a quadratic.
    while num.len() > 1 && den.len() > 1 {
        let size = |c: &[f64], z: C| c.iter().enumerate().map(|(k, x)| x.abs() * cabs(z).powi(k as i32)).sum::<f64>();
        let Some(z) = roots(&num).into_iter().find(|z| cabs(horner(&den, *z)) <= 1e-9 * size(&den, *z)) else { break };
        let d = if z.1 != 0.0 { vec![z.0 * z.0 + z.1 * z.1, -2.0 * z.0, 1.0] } else { vec![-z.0, 1.0] };
        if d.len() > num.len() || d.len() > den.len() { break; }
        num = divide(&num, &d);
        den = divide(&den, &d);
    }
    Ok(Tf { num, den, dim, name })
}

// Complex numbers as (re, im).
type C = (f64, f64);
fn cmul(a: C, b: C) -> C { (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0) }
fn cdiv(a: C, b: C) -> C { let d = b.0 * b.0 + b.1 * b.1; ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d) }
fn cabs(a: C) -> f64 { a.0.hypot(a.1) }
fn horner(c: &[f64], z: C) -> C { c.iter().rev().fold((0.0, 0.0), |acc, k| { let m = cmul(acc, z); (m.0 + k, m.1) }) }

/// The roots of a polynomial (coefficients of s⁰, s¹, …), by Aberth's method on a scaled copy, then polished.
fn roots(c: &[f64]) -> Vec<C> {
    let mut c: Vec<f64> = c.to_vec();
    while c.len() > 1 && c.last() == Some(&0.0) { c.pop(); }
    let mut out = vec![];
    while c.len() > 1 && c[0] == 0.0 { c.remove(0); out.push((0.0, 0.0)); }
    let n = c.len() - 1;
    if n == 0 { return out; }
    // Scale s = σz so the roots are near the unit circle.
    let sigma = (c[0].abs() / c[n].abs()).powf(1.0 / n as f64);
    let sigma = if sigma.is_finite() && sigma > 0.0 { sigma } else { 1.0 };
    let q: Vec<f64> = c.iter().enumerate().map(|(k, x)| x * sigma.powi(k as i32) / (c[n] * sigma.powi(n as i32))).collect();
    let dq: Vec<f64> = q.iter().enumerate().skip(1).map(|(k, x)| x * k as f64).collect();
    let mut z: Vec<C> = (0..n).map(|k| { let a = 2.0 * std::f64::consts::PI * k as f64 / n as f64 + 0.4; (a.cos(), a.sin()) }).collect();
    for _ in 0..500 {
        let mut moved = 0.0f64;
        for i in 0..n {
            let ratio = cdiv(horner(&q, z[i]), horner(&dq, z[i]));
            let s = (0..n).filter(|&j| j != i).fold((0.0, 0.0), |acc, j| { let d = (z[i].0 - z[j].0, z[i].1 - z[j].1); let r = cdiv((1.0, 0.0), d); (acc.0 + r.0, acc.1 + r.1) });
            let den = (1.0 - cmul(ratio, s).0, -cmul(ratio, s).1);
            let w = cdiv(ratio, den);
            if !w.0.is_finite() || !w.1.is_finite() { continue; }
            z[i] = (z[i].0 - w.0, z[i].1 - w.1);
            moved = moved.max(cabs(w) / cabs(z[i]).max(1e-300));
        }
        if moved < 1e-15 { break; }
    }
    let dc: Vec<f64> = c.iter().enumerate().skip(1).map(|(k, x)| x * k as f64).collect();
    let mut z: Vec<C> = z.into_iter().map(|r| {
        let mut s = (r.0 * sigma, r.1 * sigma);
        for _ in 0..3 {
            let w = cdiv(horner(&c, s), horner(&dc, s));
            if w.0.is_finite() && w.1.is_finite() && cabs(w) < 1e-6 * cabs(s).max(1e-300) { s = (s.0 - w.0, s.1 - w.1); }
        }
        s
    }).collect();
    // A k-fold root comes out as k roots close together, each off by about the k-th root of round-off; their mean is
    // accurate, so each cluster is put at its mean.
    let mut seen = vec![false; n];
    for i in 0..n {
        if seen[i] { continue; }
        let g: Vec<usize> = (i..n).filter(|&j| !seen[j] && cabs((z[j].0 - z[i].0, z[j].1 - z[i].1)) <= 1e-3 * cabs(z[i]).max(1e-300)).collect();
        let m = g.iter().fold((0.0, 0.0), |a, &j| (a.0 + z[j].0 / g.len() as f64, a.1 + z[j].1 / g.len() as f64));
        for j in g { seen[j] = true; z[j] = m; }
    }
    for s in z.iter_mut() {
        if s.1.abs() <= 1e-9 * cabs(*s) { s.1 = 0.0; }
        if s.0.abs() <= 1e-12 * cabs(*s) { s.0 = 0.0; }
    }
    // Complex roots in exact conjugate pairs, the upper one first; then by size.
    let mut used = vec![false; n];
    let mut items: Vec<Vec<C>> = vec![];
    for i in 0..n {
        if used[i] { continue; }
        used[i] = true;
        if z[i].1 == 0.0 { items.push(vec![z[i]]); continue; }
        let gap = |j: usize| cabs((z[j].0 - z[i].0, z[j].1 + z[i].1));
        match (0..n).filter(|&j| !used[j] && z[j].1 * z[i].1 < 0.0).min_by(|a, b| gap(*a).total_cmp(&gap(*b))) {
            Some(j) if gap(j) <= 1e-6 * cabs(z[i]) => {
                used[j] = true;
                let (re, im) = ((z[i].0 + z[j].0) / 2.0, (z[i].1.abs() + z[j].1.abs()) / 2.0);
                items.push(vec![(re, im), (re, -im)]);
            }
            _ => items.push(vec![z[i]]),
        }
    }
    // Sizes equal to round-off (a Butterworth filter's poles) are a tie, settled by the one nearest the imaginary
    // axis first, so the order is the same on every machine.
    let key = |z: C| { let m = cabs(z); if m == 0.0 { 0.0 } else { let e = 10f64.powi(m.log10().floor() as i32 - 8); (m / e).round() * e } };
    items.sort_by(|a, b| key(a[0]).total_cmp(&key(b[0])).then(b[0].0.total_cmp(&a[0].0)));
    out.extend(items.into_iter().flatten());
    out
}

/// The quotient of a by a monic divisor d (coefficients of s⁰, s¹, …); the remainder, round-off, is dropped.
fn divide(a: &[f64], d: &[f64]) -> Vec<f64> {
    let (n, m) = (a.len() - 1, d.len() - 1);
    let mut r = a.to_vec();
    let mut q = vec![0.0; n - m + 1];
    for k in (0..=n - m).rev() {
        let c = r[k + m];
        q[k] = c;
        for j in 0..=m { r[k + j] -= c * d[j]; }
    }
    q
}

/// Balances a matrix (Parlett and Reinsch, by powers of 2): D⁻¹AD with rows and columns of like size, so e^A keeps
/// its accuracy for a companion matrix with coefficients of very different sizes. Gives the scales D.
fn balance(a: &mut [f64], n: usize) -> Vec<f64> {
    let mut d = vec![1.0; n];
    for _ in 0..100 {
        let mut done = true;
        for i in 0..n {
            let (mut c, mut r) = (0.0, 0.0);
            for j in 0..n { if j != i { c += a[j * n + i].abs(); r += a[i * n + j].abs(); } }
            if c == 0.0 || r == 0.0 || !c.is_finite() || !r.is_finite() { continue; }
            let (s, mut f, mut cc) = (c + r, 1.0, c);
            while cc < r / 2.0 { f *= 2.0; cc *= 4.0; }
            while cc >= r * 2.0 { f /= 2.0; cc /= 4.0; }
            if c * f + r / f < 0.95 * s {
                done = false;
                d[i] *= f;
                for j in 0..n { a[i * n + j] /= f; a[j * n + i] *= f; }
            }
        }
        if done { break; }
    }
    d
}

/// e^M for an n × n matrix (row by row), by scaling and squaring with a Taylor series; None when it overflows.
fn expm(m: &[f64], n: usize) -> Option<Vec<f64>> {
    let norm = (0..n).map(|i| (0..n).map(|j| m[i * n + j].abs()).sum::<f64>()).fold(0.0, f64::max);
    if !norm.is_finite() { return None; }
    let sq = if norm > 0.5 { (norm / 0.5).log2().ceil() as i32 } else { 0 };
    // e^(norm) beyond the largest number there is: no use squaring 2,000 times to find out.
    if sq > 1100 { return None; }
    let k = 0.5f64.powi(sq);
    let a: Vec<f64> = m.iter().map(|x| x * k).collect();
    let mul = |x: &[f64], y: &[f64]| -> Vec<f64> { let mut r = vec![0.0; n * n]; for i in 0..n { for l in 0..n { let xv = x[i * n + l]; if xv != 0.0 { for j in 0..n { r[i * n + j] += xv * y[l * n + j]; } } } } r };
    let mut e: Vec<f64> = (0..n * n).map(|i| if i / n == i % n { 1.0 } else { 0.0 }).collect();
    let mut term = e.clone();
    for t in 1..=20 {
        term = mul(&term, &a).iter().map(|x| x / t as f64).collect();
        for (x, y) in e.iter_mut().zip(&term) { *x += y; }
    }
    for _ in 0..sq { e = mul(&e, &e); if e.iter().any(|x| !x.is_finite()) { return None; } }
    Some(e)
}

impl Tf {
    fn order(&self) -> usize { self.den.len() - 1 }
    /// The augmented matrix [[A, B], [0, 0]] of the controllable form, (n + 1) × (n + 1).
    fn aug(&self) -> Vec<f64> {
        let n = self.order();
        let m = n + 1;
        let mut a = vec![0.0; m * m];
        for i in 0..n.saturating_sub(1) { a[i * m + i + 1] = 1.0; }
        if n > 0 { for j in 0..n { a[(n - 1) * m + j] = -self.den[j]; } a[(n - 1) * m + n] = 1.0; }
        a
    }
    /// D and C of the controllable form: num = D·den + Σ c_k s^k.
    fn dc(&self) -> (f64, Vec<f64>) {
        let n = self.order();
        let d = if self.num.len() > n { self.num[n] } else { 0.0 };
        (d, (0..n).map(|k| self.num.get(k).copied().unwrap_or(0.0) - d * self.den[k]).collect())
    }
    fn proper(&self, what: &str, at: Span) -> R<()> {
        if self.num.len() > self.den.len() { return fail(format!("{} has more zeros than poles: its {what} is not finite (add poles, a filter for instance).", self.name), at); }
        Ok(())
    }
    /// e^(Mt) of the augmented matrix, worked out balanced; None when it overflows.
    fn exp_at(&self, t: f64) -> Option<Vec<f64>> {
        let m = self.order() + 1;
        let mut a: Vec<f64> = self.aug().iter().map(|x| x * t).collect();
        let d = balance(&mut a, m);
        let e = expm(&a, m)?;
        Some((0..m * m).map(|k| e[k] * d[k / m] / d[k % m]).collect())
    }
    /// The unit step response at time t (seconds).
    fn step(&self, t: f64) -> Option<f64> {
        let n = self.order();
        let (d, c) = self.dc();
        if n == 0 { return Some(d); }
        let e = self.exp_at(t)?;
        Some(d + (0..n).map(|i| c[i] * e[i * (n + 1) + n]).sum::<f64>()).filter(|y| y.is_finite())
    }
    fn impulse(&self, t: f64) -> Option<f64> {
        let n = self.order();
        let (_, c) = self.dc();
        let e = self.exp_at(t)?;
        Some((0..n).map(|i| c[i] * e[i * (n + 1) + n - 1]).sum::<f64>()).filter(|y| y.is_finite())
    }
    fn at(&self, s: C) -> C { cdiv(horner(&self.num, s), horner(&self.den, s)) }
    fn poles(&self) -> Vec<C> { roots(&self.den) }
    fn stable(&self, what: &str, at: Span) -> R<Vec<C>> {
        let p = self.poles();
        let big = p.iter().map(|z| cabs(*z)).fold(0.0, f64::max);
        if let Some(z) = p.iter().find(|z| z.0 >= -1e-9 * big.max(1e-300)) {
            let at_s = if z.0.abs() <= 1e-9 * big.max(1e-300) && z.1 == 0.0 { "at s = 0 (an integrator)".into() } else { format!("at {:.4} {} {:.4}j rad/s", z.0, if z.1 < 0.0 { "−" } else { "+" }, z.1.abs()) };
            return fail(format!("{} has a pole {at_s}: its step response does not settle, so it has no {what}.", self.name), at);
        }
        Ok(p)
    }
    /// The step response sampled to well past settling: times, values over the final value, and the final value.
    /// The horizon is 12 + 2n slowest time constants, doubled until the last tenth stays within 2 %; the samples only
    /// bracket the figures, which are then found on the exact response.
    fn step_samples(&self, at: Span, what: &str) -> R<(Vec<f64>, Vec<f64>, f64)> {
        self.proper("step response", at)?;
        let n = self.order();
        if n == 0 { return fail(format!("{} is a constant gain: its step response jumps at once, so it has no {what}.", self.name), at); }
        let p = self.stable(what, at)?;
        let fin = self.num[0] / self.den[0];
        if fin == 0.0 || !fin.is_finite() { return fail(format!("{}'s step response ends at zero, so it has no {what}.", self.name), at); }
        let slow = p.iter().map(|z| -z.0).fold(f64::INFINITY, f64::min);
        let fast = p.iter().map(|z| cabs(*z)).fold(0.0, f64::max);
        let budget = (30_000_000 / ((n + 1) * (n + 1))).max(4000);
        let mut horizon = (12.0 + 2.0 * n as f64) / slow;
        let (d, c) = self.dc();
        let bad = || fail(format!("{}'s step response cannot be worked out accurately: its poles are too far apart or too many.", self.name), at);
        for attempt in 0..6 {
            let steps = ((horizon * fast / 0.05).ceil() as usize).clamp(4000, budget);
            let dt = horizon / steps as f64;
            let Some(phi) = self.exp_at(dt) else { return bad() };
            let mut z = vec![0.0; n + 1];
            z[n] = 1.0;
            let (mut ts, mut ys) = (Vec::with_capacity(steps + 1), Vec::with_capacity(steps + 1));
            for k in 0..=steps {
                ts.push(k as f64 * dt);
                ys.push((d + (0..n).map(|i| c[i] * z[i]).sum::<f64>()) / fin);
                z = (0..=n).map(|i| (0..=n).map(|j| phi[i * (n + 1) + j] * z[j]).sum()).collect();
            }
            if ys.iter().any(|y| !y.is_finite() || y.abs() > 1e6) { return bad(); }
            if attempt == 5 || ys[ys.len() * 9 / 10..].iter().all(|y| (y - 1.0).abs() <= 0.02) { return Ok((ts, ys, fin)); }
            horizon *= 2.0;
        }
        unreachable!()
    }
    /// The dominant poles: those nearest the imaginary axis (not at 0).
    fn dominant(&self, at: Span) -> R<C> {
        let p: Vec<C> = self.poles().into_iter().filter(|z| cabs(*z) > 0.0).collect();
        p.into_iter().min_by(|a, b| a.0.abs().total_cmp(&b.0.abs()).then(b.1.total_cmp(&a.1))).map_or_else(|| fail(format!("{} has no poles away from 0.", self.name), at), Ok)
    }
    /// The frequency band to search: a hundred times beyond the poles and zeros, then widened until |G(jω)| is on
    /// the side of 1 its asymptote is at each end (a gain of 1000/(s + 1) crosses 1 near 1000 rad/s).
    fn band(&self) -> (f64, f64) {
        let mags: Vec<f64> = roots(&self.num).into_iter().chain(self.poles()).map(cabs).filter(|m| *m > 0.0).collect();
        let (mut lo, mut hi) = if mags.is_empty() { (1e-3, 1e3) } else { (mags.iter().cloned().fold(f64::INFINITY, f64::min) / 100.0, mags.iter().cloned().fold(0.0, f64::max) * 100.0) };
        let mag = |w: f64| cabs(self.at((0.0, w)));
        let (z0, p0) = (self.num.iter().take_while(|x| **x == 0.0).count(), self.den.iter().take_while(|x| **x == 0.0).count());
        let low_big = if p0 != z0 { p0 > z0 } else { (self.num[z0] / self.den[p0]).abs() > 1.0 };
        let high_big = self.num.len() == self.den.len() && self.num.last().unwrap().abs() > 1.0;
        for _ in 0..12 { if (mag(lo) > 1.0) == low_big { break; } lo /= 10.0; }
        for _ in 0..12 { if (mag(hi) > 1.0) == high_big { break; } hi *= 10.0; }
        (lo, hi)
    }
    /// Frequencies where g changes sign over the band, each found by bisection in log ω.
    fn crossings(&self, g: impl Fn(C) -> f64) -> Vec<f64> {
        let (lo, hi) = self.band();
        let n = 4000;
        let w = |k: f64| lo * (hi / lo).powf(k / n as f64);
        let val = |w: f64| g(self.at((0.0, w)));
        let mut out = vec![];
        let mut prev = val(w(0.0));
        for k in 1..=n {
            let cur = val(w(k as f64));
            if prev.is_finite() && cur.is_finite() && (prev < 0.0) != (cur < 0.0) {
                let (mut a, mut b) = ((k - 1) as f64, k as f64);
                let fa = prev;
                for _ in 0..60 { let m = (a + b) / 2.0; if (val(w(m)) < 0.0) == (fa < 0.0) { a = m; } else { b = m; } }
                out.push(w((a + b) / 2.0));
            }
            prev = cur;
        }
        out
    }
    fn loop_gain(&self, at: Span) -> R<()> {
        if !units::is_none(&self.dim) { return fail(format!("{} has units ({}): margins are for a loop gain with none, the controller times the plant.", self.name, units::label(&self.dim)), at); }
        self.proper("frequency response", at)
    }
    /// Gain crossover: (ω, phase margin), the smallest margin where there are several.
    fn gain_cross(&self, at: Span) -> R<(f64, f64)> {
        self.loop_gain(at)?;
        let ws = self.crossings(|l| cabs(l).ln());
        let wrap = |a: f64| { let x = (a + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI; if x <= -std::f64::consts::PI { x + 2.0 * std::f64::consts::PI } else { x } };
        ws.into_iter().map(|w| { let l = self.at((0.0, w)); (w, wrap(l.1.atan2(l.0) + std::f64::consts::PI)) }).min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or_else(|| fail(format!("|{}(jω)| never crosses 1, so it has no gain crossover or phase margin.", self.name), at), Ok)
    }
    /// Phase crossover: (ω, gain margin as a ratio), the smallest margin where there are several.
    fn phase_cross(&self, at: Span) -> R<(f64, f64)> {
        self.loop_gain(at)?;
        let ws = self.crossings(|l| l.1);
        ws.into_iter().filter_map(|w| { let l = self.at((0.0, w)); (l.0 < 0.0).then(|| (w, 1.0 / cabs(l))) }).min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or_else(|| fail(format!("{}(jω) never crosses −180°, so its gain margin is unlimited.", self.name), at), Ok)
    }
}

/// The first time the response reaches a level: bracketed by the samples, then bisected on the exact response.
fn first_cross(ts: &[f64], ys: &[f64], level: f64, y: &dyn Fn(f64) -> f64) -> Option<f64> {
    if ys[0] >= level { return Some(ts[0]); }
    let k = (1..ys.len()).find(|&k| ys[k] >= level)?;
    let (mut a, mut b) = (ts[k - 1], ts[k]);
    for _ in 0..60 { let m = (a + b) / 2.0; if y(m) >= level { b = m; } else { a = m; } }
    Some((a + b) / 2.0)
}

fn vals(v: Vec<C>, dim: Dim) -> Val {
    let cx = v.iter().any(|z| z.1 != 0.0);
    Val { v: v.iter().map(|z| z.0).collect(), im: if cx { v.iter().map(|z| z.1).collect() } else { vec![] }, dim, vec: true, w: units::same(&dim, &FREQ), ..Default::default() }
}

/// One of this module's functions, called with its arguments as written.
pub fn call(env: &mut Env, f: &str, args: &[Node], at: Span) -> R<Val> {
    let want = |m: &[usize]| -> R<()> {
        if m.contains(&args.len()) { Ok(()) } else { fail(format!("{f} takes {} value{}.", m.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" or "), if m == [1] { "" } else { "s" }), at) }
    };
    want(if f == "step" { &[2, 3] } else if f == "impulse" { &[2] } else { &[1] })?;
    // A transfer function may use these functions itself (H(s) = 1/(s + dcgain(G))), but not on itself without end.
    if env.depth > 40 { return fail(format!("{f} calls itself too deeply."), at); }
    env.depth += 1;
    let g = tf(env, f, &args[0]);
    env.depth -= 1;
    let g = g?;
    let scalar = |x: f64, dim: Dim| Val { w: units::same(&dim, &FREQ), ..Val::num(x, dim) };
    match f {
        "poles" | "zeros" => {
            let r = if f == "poles" { g.poles() } else { roots(&g.num) };
            if r.is_empty() { return fail(format!("{} has no {}.", g.name, if f == "poles" { "poles" } else { "finite zeros" }), at); }
            Ok(vals(r, FREQ))
        }
        "dcgain" => {
            if g.den[0] == 0.0 { return fail(format!("{} has a pole at s = 0 (an integrator): its gain grows without limit.", g.name), at); }
            Ok(Val::num(g.num[0] / g.den[0], g.dim))
        }
        "step" | "impulse" => {
            let t = env.eval(&args[1])?;
            if !units::same(&t.dim, &TIME) || t.cx() || t.mat() { return fail(format!("{f} needs a time, such as {f}({}, 2 s).", g.name), args[1].at); }
            g.proper(if f == "step" { "step response" } else { "impulse response" }, at)?;
            if f == "impulse" && g.num.len() == g.den.len() { return fail(format!("{} has as many zeros as poles: its impulse response starts with an impulse itself. Use step instead.", g.name), at); }
            if t.v.iter().any(|x| *x < 0.0) { return fail("The response starts at t = 0: give times of 0 s or more.", args[1].at); }
            if t.v.len() > 2000 { return fail(format!("{f} takes at most 2,000 times at once."), args[1].at); }
            let (k, kdim) = match args.get(2) {
                Some(a) => { let u = env.eval(a)?; if u.vec || u.cx() || u.abs { return fail("The size of the step is one real value (a difference for temperatures).", a.at); } (u.x(), u.dim) }
                None => (1.0, NONE),
            };
            let Some(y) = t.v.iter().map(|x| if f == "step" { g.step(*x) } else { g.impulse(*x) }.map(|y| k * y)).collect::<Option<Vec<f64>>>() else {
                return fail(format!("{}'s response grows too large to give by that time.", g.name), at);
            };
            let dim = units::mul(&g.dim, &kdim);
            Ok(Val { v: y, dim: if f == "step" { dim } else { units::div(&dim, &TIME) }, vec: t.vec, ..Default::default() })
        }
        "overshoot" | "risetime" | "settling" | "peaktime" => {
            let what = match f { "overshoot" => "overshoot", "risetime" => "rise time", "settling" => "settling time", _ => "peak time" };
            let (ts, ys, fin) = g.step_samples(at, what)?;
            let y = |t: f64| g.step(t).map_or(f64::NAN, |v| v / fin);
            match f {
                "overshoot" | "peaktime" => {
                    // The highest peaks of the samples, each found exactly by a golden-section search.
                    let last = ys.len() - 1;
                    let mut cand: Vec<usize> = (0..=last).filter(|&k| ys[k] > 1.0 && (k == 0 || ys[k] >= ys[k - 1]) && (k == last || ys[k] >= ys[k + 1])).collect();
                    cand.sort_by(|a, b| ys[*b].total_cmp(&ys[*a]));
                    cand.truncate(8);
                    let best = cand.into_iter().map(|k| {
                        let (mut a, mut b) = (ts[k.saturating_sub(1)], ts[(k + 1).min(last)]);
                        let r = (5f64.sqrt() - 1.0) / 2.0;
                        for _ in 0..80 { let (x1, x2) = (b - r * (b - a), a + r * (b - a)); if y(x1) >= y(x2) { b = x2; } else { a = x1; } }
                        let t = (a + b) / 2.0;
                        let (t, v) = if ys[k] > y(t) { (ts[k], ys[k]) } else { (t, y(t)) };
                        (t, v)
                    }).fold(None, |m: Option<(f64, f64)>, x| if m.is_none_or(|m| x.1 > m.1) { Some(x) } else { m });
                    match (f, best) {
                        ("overshoot", b) => Ok(Val::plain(b.map_or(0.0, |b| b.1 - 1.0))),
                        (_, Some(b)) if b.1 > 1.0 + 1e-9 => Ok(Val::num(b.0, TIME)),
                        _ => fail(format!("{}'s step response rises without overshoot: its highest value is the final one.", g.name), at),
                    }
                }
                "risetime" => match (first_cross(&ts, &ys, 0.1, &y), first_cross(&ts, &ys, 0.9, &y)) {
                    (Some(a), Some(b)) if b >= a => Ok(Val::num(b - a, TIME)),
                    _ => fail(format!("{}'s step response does not rise from 10 % to 90 % of its final value.", g.name), at),
                },
                _ => match ys.iter().rposition(|v| (v - 1.0).abs() > 0.02) {
                    None => Ok(Val::num(0.0, TIME)),
                    Some(k) if k + 1 < ys.len() => {
                        let (mut a, mut b) = (ts[k], ts[k + 1]);
                        for _ in 0..60 { let m = (a + b) / 2.0; if (y(m) - 1.0).abs() > 0.02 { a = m; } else { b = m; } }
                        Ok(Val::num((a + b) / 2.0, TIME))
                    }
                    Some(_) => fail(format!("{}'s step response does not settle within 2 %.", g.name), at),
                },
            }
        }
        "wn" => Ok(scalar(cabs(g.dominant(at)?), FREQ)),
        "zeta" => { let p = g.dominant(at)?; Ok(Val::plain(-p.0 / cabs(p))) }
        "pm" => Ok(Val::num(g.gain_cross(at)?.1, ANGLE)),
        "wgc" => Ok(scalar(g.gain_cross(at)?.0, FREQ)),
        "gm" => Ok(Val::plain(g.phase_cross(at)?.1)),
        "wpc" => Ok(scalar(g.phase_cross(at)?.0, FREQ)),
        _ => fail(format!("{f} is not a dynamic-systems function."), at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_and_expm() {
        // (s + 1)(s + 2)(s² + 2s + 5): −1, −2, −1 ± 2j.
        let r = roots(&[10.0, 19.0, 13.0, 5.0, 1.0][..]);
        let want = [(-1.0, 0.0), (-2.0, 0.0), (-1.0, 2.0), (-1.0, -2.0)];
        for w in want { assert!(r.iter().any(|z| cabs((z.0 - w.0, z.1 - w.1)) < 1e-10), "{r:?}"); }
        // e^(diag(1, 2)) and a rotation.
        let e = expm(&[0.0, 1.0, -1.0, 0.0], 2).unwrap();
        assert!((e[0] - 1f64.cos()).abs() < 1e-14 && (e[1] - 1f64.sin()).abs() < 1e-14);
        let e = expm(&[30.0, 0.0, 0.0, -30.0], 2).unwrap();
        assert!((e[0] / 30f64.exp() - 1.0).abs() < 1e-12 && (e[3] / (-30f64).exp() - 1.0).abs() < 1e-9);
    }
}
