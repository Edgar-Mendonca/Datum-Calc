//! Exact algebra for `simplify`, `expand` and `factor`: polynomials and fractions of polynomials with
//! rational coefficients. Names that are not defined on the sheet are symbols (x, y, π); defined plain
//! numbers are put in. Anything else (units, functions, decimals that are not fractions) is refused with a
//! plain message, so a result is either exact or not given.
use crate::parse::{Node, E};
use std::collections::BTreeMap;

pub type R<T> = Result<T, String>;
const BIG: &str = "The numbers grow too large for exact maths.";

/// A rational number n/d in lowest terms, d > 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Q { pub n: i128, pub d: i128 }

fn gcd(a: i128, b: i128) -> i128 { let (mut a, mut b) = (a.abs(), b.abs()); while b != 0 { (a, b) = (b, a % b); } a }

impl Q {
    pub const ZERO: Q = Q { n: 0, d: 1 };
    pub const ONE: Q = Q { n: 1, d: 1 };
    pub fn int(n: i128) -> Q { Q { n, d: 1 } }
    pub fn new(n: i128, d: i128) -> R<Q> {
        if d == 0 { return Err("Division by zero.".into()); }
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Ok(Q { n: s * n / g, d: s * d / g })
    }
    fn add(self, o: Q) -> R<Q> {
        let n = self.n.checked_mul(o.d).and_then(|a| o.n.checked_mul(self.d).and_then(|b| a.checked_add(b))).ok_or(BIG)?;
        Q::new(n, self.d.checked_mul(o.d).ok_or(BIG)?)
    }
    fn mul(self, o: Q) -> R<Q> {
        let (g1, g2) = (gcd(self.n, o.d).max(1), gcd(o.n, self.d).max(1));
        Q::new((self.n / g1).checked_mul(o.n / g2).ok_or(BIG)?, (self.d / g2).checked_mul(o.d / g1).ok_or(BIG)?)
    }
    fn neg(self) -> Q { Q { n: -self.n, d: self.d } }
    fn inv(self) -> R<Q> { Q::new(self.d, self.n) }
    fn div(self, o: Q) -> R<Q> { self.mul(o.inv()?) }
    pub fn is_zero(self) -> bool { self.n == 0 }
    pub fn to_f64(self) -> f64 { self.n as f64 / self.d as f64 }
}

/// The fraction a decimal stands for, when it is one with a denominator up to `max_den` (to a few parts in
/// 10¹⁵, as the arithmetic of fractions in decimals leaves it): 0.30000000000000004 → 3/10, π → none.
pub fn fraction(x: f64, max_den: i128) -> Option<Q> {
    if !x.is_finite() || x.abs() > 1e15 { return None; }
    if x == x.trunc() { return Some(Q::int(x as i128)); }
    // Continued fraction convergents p/q of |x|.
    let (s, a) = (x.signum(), x.abs());
    let (mut p0, mut q0, mut p1, mut q1) = (0i128, 1i128, 1i128, 0i128);
    let mut r = a;
    for _ in 0..40 {
        let t = r.floor();
        if t > 1e15 { break; }
        let ti = t as i128;
        let (p2, q2) = (ti.checked_mul(p1)?.checked_add(p0)?, ti.checked_mul(q1)?.checked_add(q0)?);
        if q2 > max_den { break; }
        if ((p2 as f64 / q2 as f64) - a).abs() <= 4.0 * f64::EPSILON * a.max(1e-300) { return Q::new(s as i128 * p2, q2).ok(); }
        (p0, q0, p1, q1) = (p1, q1, p2, q2);
        let f = r - t;
        if f <= 0.0 { break; }
        r = 1.0 / f;
    }
    None
}

/// A product of symbols with whole powers, sorted by name: x²y = [("x", 2), ("y", 1)].
pub type Mono = Vec<(String, u32)>;

/// A polynomial: monomial → nonzero coefficient.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Poly(pub BTreeMap<Mono, Q>);

const MAX_TERMS: usize = 2000;

fn mono_mul(a: &Mono, b: &Mono) -> Mono {
    let mut m: BTreeMap<String, u32> = a.iter().cloned().collect();
    for (v, e) in b { *m.entry(v.clone()).or_insert(0) += e; }
    m.into_iter().collect()
}

impl Poly {
    pub fn constant(q: Q) -> Poly { let mut m = BTreeMap::new(); if !q.is_zero() { m.insert(vec![], q); } Poly(m) }
    pub fn symbol(s: &str) -> Poly { let mut m = BTreeMap::new(); m.insert(vec![(s.to_string(), 1)], Q::ONE); Poly(m) }
    pub fn is_zero(&self) -> bool { self.0.is_empty() }
    pub fn as_const(&self) -> Option<Q> { match self.0.len() { 0 => Some(Q::ZERO), 1 => self.0.get(&vec![]).copied(), _ => None } }
    fn add_term(&mut self, m: Mono, c: Q) -> R<()> {
        let e = self.0.entry(m.clone()).or_insert(Q::ZERO);
        *e = e.add(c)?;
        if e.is_zero() { self.0.remove(&m); }
        if self.0.len() > MAX_TERMS { return Err("The expression has too many terms for exact maths.".into()); }
        Ok(())
    }
    pub fn add(&self, o: &Poly) -> R<Poly> { let mut r = self.clone(); for (m, c) in &o.0 { r.add_term(m.clone(), *c)?; } Ok(r) }
    pub fn neg(&self) -> Poly { Poly(self.0.iter().map(|(m, c)| (m.clone(), c.neg())).collect()) }
    pub fn sub(&self, o: &Poly) -> R<Poly> { self.add(&o.neg()) }
    pub fn mul(&self, o: &Poly) -> R<Poly> {
        let mut r = Poly::default();
        for (m1, c1) in &self.0 { for (m2, c2) in &o.0 { r.add_term(mono_mul(m1, m2), c1.mul(*c2)?)?; } }
        Ok(r)
    }
    pub fn scale(&self, q: Q) -> R<Poly> { if q.is_zero() { return Ok(Poly::default()); } Ok(Poly(self.0.iter().map(|(m, c)| Ok((m.clone(), c.mul(q)?))).collect::<R<_>>()?)) }
    pub fn pow(&self, n: u32) -> R<Poly> { let mut r = Poly::constant(Q::ONE); for _ in 0..n { r = r.mul(self)?; } Ok(r) }
    /// The symbols used.
    pub fn vars(&self) -> Vec<String> { let mut v: Vec<String> = self.0.keys().flat_map(|m| m.iter().map(|x| x.0.clone())).collect(); v.sort(); v.dedup(); v }
    fn degree_in(&self, x: &str) -> u32 { self.0.keys().map(|m| m.iter().find(|t| t.0 == x).map(|t| t.1).unwrap_or(0)).max().unwrap_or(0) }
    /// Coefficients in one symbol, lowest power first (only for a polynomial in that symbol alone).
    fn coeffs(&self, x: &str) -> Vec<Q> {
        let mut c = vec![Q::ZERO; self.degree_in(x) as usize + 1];
        for (m, q) in &self.0 { let e = m.iter().find(|t| t.0 == x).map(|t| t.1).unwrap_or(0); c[e as usize] = *q; }
        c
    }
    fn from_coeffs(c: &[Q], x: &str) -> Poly {
        let mut r = BTreeMap::new();
        for (e, q) in c.iter().enumerate() { if !q.is_zero() { r.insert(if e == 0 { vec![] } else { vec![(x.to_string(), e as u32)] }, *q); } }
        Poly(r)
    }
    /// The term that leads when terms are ordered by total degree, then by name.
    fn lead(&self) -> Option<(&Mono, &Q)> { self.0.iter().max_by(|a, b| order(a.0, b.0)) }
}

fn total(m: &Mono) -> u32 { m.iter().map(|t| t.1).sum() }
/// Terms print highest total degree first, then by name: x² + xy + 2x + 1.
fn order(a: &Mono, b: &Mono) -> std::cmp::Ordering {
    total(a).cmp(&total(b)).then_with(|| {
        // x² before xy before y²: compare powers of the earliest symbol first.
        let all: Vec<&String> = { let mut v: Vec<&String> = a.iter().chain(b.iter()).map(|t| &t.0).collect(); v.sort(); v.dedup(); v };
        for s in all {
            let (ea, eb) = (a.iter().find(|t| &t.0 == s).map(|t| t.1).unwrap_or(0), b.iter().find(|t| &t.0 == s).map(|t| t.1).unwrap_or(0));
            if ea != eb { return ea.cmp(&eb); }
        }
        std::cmp::Ordering::Equal
    })
}

// ---------- one-variable polynomials: division and greatest common divisor ----------

fn trim(c: &mut Vec<Q>) { while c.len() > 1 && c.last().is_some_and(|q| q.is_zero()) { c.pop(); } }

/// (quotient, remainder) of a / b, coefficients lowest power first.
fn divmod(a: &[Q], b: &[Q]) -> R<(Vec<Q>, Vec<Q>)> {
    let mut r = a.to_vec();
    let mut b = b.to_vec();
    trim(&mut r);
    trim(&mut b);
    let lb = *b.last().ok_or(BIG)?;
    if lb.is_zero() { return Err("Division by zero.".into()); }
    if r.len() < b.len() { return Ok((vec![Q::ZERO], r)); }
    let mut q = vec![Q::ZERO; r.len() - b.len() + 1];
    while r.len() >= b.len() && !(r.len() == 1 && r[0].is_zero()) {
        let k = r.len() - b.len();
        let f = r.last().copied().ok_or(BIG)?.div(lb)?;
        q[k] = f;
        for (i, bi) in b.iter().enumerate() { r[k + i] = r[k + i].add(bi.mul(f)?.neg())?; }
        r.pop();
        if r.is_empty() { r.push(Q::ZERO); }
        trim(&mut r);
        if r.len() < b.len() { break; }
    }
    Ok((q, r))
}

fn monic(c: &[Q]) -> R<Vec<Q>> { let l = c.last().copied().unwrap_or(Q::ONE); c.iter().map(|q| q.div(l)).collect() }

fn gcd1(a: &[Q], b: &[Q]) -> R<Vec<Q>> {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    trim(&mut a);
    trim(&mut b);
    while !(b.len() == 1 && b[0].is_zero()) {
        let (_, r) = divmod(&a, &b)?;
        a = b;
        b = r;
    }
    monic(&a)
}

// ---------- fractions of polynomials ----------

/// num / den, kept in lowest terms where that can be done exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct RatF { pub num: Poly, pub den: Poly }

impl RatF {
    pub fn poly(p: Poly) -> RatF { RatF { num: p, den: Poly::constant(Q::ONE) } }
    pub fn add(&self, o: &RatF) -> R<RatF> { RatF { num: self.num.mul(&o.den)?.add(&o.num.mul(&self.den)?)?, den: self.den.mul(&o.den)? }.reduce() }
    pub fn sub(&self, o: &RatF) -> R<RatF> { self.add(&RatF { num: o.num.neg(), den: o.den.clone() }) }
    pub fn mul(&self, o: &RatF) -> R<RatF> { RatF { num: self.num.mul(&o.num)?, den: self.den.mul(&o.den)? }.reduce() }
    pub fn div(&self, o: &RatF) -> R<RatF> {
        if o.num.is_zero() { return Err("Division by zero.".into()); }
        RatF { num: self.num.mul(&o.den)?, den: self.den.mul(&o.num)? }.reduce()
    }
    pub fn pow(&self, n: i64) -> R<RatF> {
        if n.unsigned_abs() > 64 { return Err("A power above 64 is too large for exact maths.".into()); }
        let r = RatF { num: self.num.pow(n.unsigned_abs() as u32)?, den: self.den.pow(n.unsigned_abs() as u32)? };
        if n < 0 { if r.num.is_zero() { return Err("Division by zero.".into()); } RatF { num: r.den, den: r.num }.reduce() } else { Ok(r) }
    }
    /// Lowest terms: a constant denominator goes into the numerator; one symbol: divide by the common factor;
    /// several symbols: common symbols and an exact division where there is one. The denominator's leading
    /// coefficient is 1.
    pub fn reduce(self) -> R<RatF> {
        let RatF { mut num, mut den } = self;
        if num.is_zero() { return Ok(RatF::poly(Poly::default())); }
        if let Some(c) = den.as_const() { return Ok(RatF::poly(num.scale(c.inv()?)?)); }
        let vars = { let mut v = num.vars(); v.extend(den.vars()); v.sort(); v.dedup(); v };
        if vars.len() == 1 {
            let x = &vars[0];
            let g = gcd1(&num.coeffs(x), &den.coeffs(x))?;
            if g.len() > 1 {
                num = Poly::from_coeffs(&divmod(&num.coeffs(x), &g)?.0, x);
                den = Poly::from_coeffs(&divmod(&den.coeffs(x), &g)?.0, x);
            }
        } else {
            // Common symbols: x²y/(xy²) = x/y.
            let common: Mono = vars.iter().filter_map(|v| {
                let lo = |p: &Poly| p.0.keys().map(|m| m.iter().find(|t| &t.0 == v).map(|t| t.1).unwrap_or(0)).min().unwrap_or(0);
                let e = lo(&num).min(lo(&den));
                (e > 0).then(|| (v.clone(), e))
            }).collect();
            if !common.is_empty() {
                let cut = |p: &Poly| Poly(p.0.iter().map(|(m, c)| (m.iter().map(|(v, e)| (v.clone(), e - common.iter().find(|t| &t.0 == v).map(|t| t.1).unwrap_or(0))).filter(|t| t.1 > 0).collect(), *c)).collect());
                num = cut(&num);
                den = cut(&den);
            }
            if let Some(q) = divide_exact(&num, &den)? { return Ok(RatF::poly(q)); }
        }
        if let Some(c) = den.as_const() { return Ok(RatF::poly(num.scale(c.inv()?)?)); }
        let l = *den.lead().ok_or(BIG)?.1;
        Ok(RatF { num: num.scale(l.inv()?)?, den: den.scale(l.inv()?)? })
    }
}

/// a / b when b divides a exactly (several symbols), else None.
fn divide_exact(a: &Poly, b: &Poly) -> R<Option<Poly>> {
    let (bm, bc) = match b.lead() { Some((m, c)) => (m.clone(), *c), None => return Ok(None) };
    let mut r = a.clone();
    let mut q = Poly::default();
    for _ in 0..MAX_TERMS {
        let Some((rm, rc)) = r.lead().map(|(m, c)| (m.clone(), *c)) else { return Ok(Some(q)) };
        // The lead term of r must be a multiple of b's.
        let mut t: Mono = vec![];
        for (v, e) in &rm { let be = bm.iter().find(|x| &x.0 == v).map(|x| x.1).unwrap_or(0); if *e > be { t.push((v.clone(), e - be)); } }
        if bm.iter().any(|(v, e)| rm.iter().find(|x| &x.0 == v).map(|x| x.1).unwrap_or(0) < *e) { return Ok(None); }
        let f = rc.div(bc)?;
        let term = { let mut p = Poly::default(); p.add_term(t, f)?; p };
        q = q.add(&term)?;
        r = r.sub(&term.mul(b)?)?;
    }
    Ok(None)
}

// ---------- from the sheet's expressions ----------

/// What a name is on the sheet, for exact maths.
pub enum Look { Value(Q), Expr(RatF), Symbol, Refuse(String) }

pub const OPS: [&str; 3] = ["simplify", "expand", "factor"];

/// An expression as a fraction of polynomials. `look` says what each name is.
pub fn from_node(n: &Node, look: &dyn Fn(&str) -> Look) -> R<RatF> {
    let not = |what: &str| Err(format!("Exact maths works on numbers, names, + − × ÷ and whole powers, not {what}."));
    match &n.e {
        E::Num(x, None) => fraction(*x, 1_000_000).map(|q| RatF::poly(Poly::constant(q))).ok_or_else(|| format!("{x} is not a fraction exact maths can keep.")),
        E::Num(_, Some(_)) | E::Unit(..) => not("units"),
        E::Im(..) => not("complex numbers"),
        E::Var(v) => match look(v) {
            Look::Value(q) => Ok(RatF::poly(Poly::constant(q))),
            Look::Expr(r) => Ok(r),
            Look::Symbol => Ok(RatF::poly(Poly::symbol(&crate::eval::canon(v)))),
            Look::Refuse(m) => Err(m),
        },
        E::Group(a) => from_node(a, look),
        E::Neg(a) => { let r = from_node(a, look)?; Ok(RatF { num: r.num.neg(), den: r.den }) }
        E::Bin(op, a, b) => {
            let x = from_node(a, look)?;
            if *op == "^" {
                let p = from_node(b, look)?;
                let k = p.num.as_const().filter(|_| p.den.as_const() == Some(Q::ONE)).filter(|q| q.d == 1).ok_or("Exact maths needs a whole-number power.")?;
                return x.pow(k.n.clamp(-1000, 1000) as i64);
            }
            let y = from_node(b, look)?;
            match *op { "+" => x.add(&y), "-" => x.sub(&y), "*" => x.mul(&y), "/" => x.div(&y), o => not(&format!("“{o}”")) }
        }
        E::Call(f, args) if OPS.contains(&f.as_str()) && args.len() == 1 => from_node(&args[0], look),
        E::Call(f, _) => not(&format!("the function {f}")),
    }
}

// ---------- factor ----------

/// A polynomial as a constant, a product of symbols and factors with their powers.
pub struct Factored { pub c: Q, pub mono: Mono, pub parts: Vec<(Poly, u32)> }

trait FilterPair<A, B> { fn filter_pair(self, f: impl Fn(&A, &B) -> bool) -> (Option<A>, Option<B>); }
impl<A, B> FilterPair<A, B> for (Option<A>, Option<B>) {
    fn filter_pair(self, f: impl Fn(&A, &B) -> bool) -> (Option<A>, Option<B>) { match self { (Some(a), Some(b)) if f(&a, &b) => (Some(a), Some(b)), _ => (None, None) } }
}

fn divisors(n: i128) -> Option<Vec<i128>> {
    let n = n.abs();
    if n == 0 || n > 1_000_000_000_000 { return None; }
    let mut v = vec![];
    let mut i = 1i128;
    while i * i <= n { if n % i == 0 { v.push(i); if i * i != n { v.push(n / i); } } i += 1; }
    Some(v)
}

pub fn factor(p: &Poly) -> R<Factored> {
    if p.is_zero() { return Ok(Factored { c: Q::ZERO, mono: vec![], parts: vec![] }); }
    // Content: the rational that makes the coefficients coprime integers, with the leading sign.
    let den_lcm = p.0.values().try_fold(1i128, |l, q| l.checked_mul(q.d / gcd(l, q.d)).ok_or(BIG))?;
    let ints: Vec<i128> = p.0.values().map(|q| q.n * (den_lcm / q.d)).collect();
    let g = ints.iter().fold(0i128, |g, n| gcd(g, *n)).max(1);
    let sign = if p.lead().is_some_and(|l| l.1.n < 0) { -1 } else { 1 };
    let c = Q::new(sign * g, den_lcm)?;
    let mut rest = p.scale(c.inv()?)?;
    // Common symbols.
    let vars = rest.vars();
    let mono: Mono = vars.iter().filter_map(|v| { let e = rest.0.keys().map(|m| m.iter().find(|t| &t.0 == v).map(|t| t.1).unwrap_or(0)).min().unwrap_or(0); (e > 0).then(|| (v.clone(), e)) }).collect();
    if !mono.is_empty() { rest = Poly(rest.0.iter().map(|(m, q)| (m.iter().map(|(v, e)| (v.clone(), e - mono.iter().find(|t| &t.0 == v).map(|t| t.1).unwrap_or(0))).filter(|t| t.1 > 0).collect(), *q)).collect()); }
    let mut parts: Vec<(Poly, u32)> = vec![];
    let vars = rest.vars();
    if vars.len() == 1 && rest.degree_in(&vars[0]) <= 24 {
        // Rational roots p/q: p divides the constant, q the leading coefficient (both integers after the content).
        let x = &vars[0];
        let mut cs = rest.coeffs(x);
        let mut cf = Q::ONE;
        let (a0, an) = (cs[0], *cs.last().ok_or(BIG)?);
        // Numbers with very many divisors would mean millions of candidates: those keep their factor unsplit.
        if let (Some(ps), Some(qs)) = (divisors(a0.n).filter(|_| a0.d == 1), divisors(an.n).filter(|_| an.d == 1)).filter_pair(|p, q| p.len() * q.len() <= 4000) {
            let mut cands: Vec<Q> = vec![];
            for p in &ps { for q in &qs { for s in [1, -1] { let r = Q::new(s * p, *q)?; if !cands.contains(&r) { cands.push(r); } } } }
            cands.sort_by(|a, b| a.to_f64().partial_cmp(&b.to_f64()).unwrap_or(std::cmp::Ordering::Equal));
            for r in cands {
                let mut k = 0;
                // (q x − p) divides while the root is a root.
                let lin = vec![Q::int(-r.n), Q::int(r.d)];
                while cs.len() > 1 {
                    let (quo, rem) = divmod(&cs, &lin)?;
                    if !rem.iter().all(|q| q.is_zero()) { break; }
                    cs = quo;
                    k += 1;
                }
                if k > 0 { parts.push((Poly::from_coeffs(&lin, x), k)); }
            }
            // Dividing by (q x − p) leaves whole coefficients only up to a constant: keep it in front.
            let lead = *cs.last().ok_or(BIG)?;
            if cs.len() == 1 { cf = lead; cs = vec![Q::ONE]; }
        }
        let left = Poly::from_coeffs(&cs, x);
        if left.as_const().is_none() { parts.push((left, 1)); } else if let Some(k) = left.as_const() { cf = cf.mul(k)?; }
        return Ok(Factored { c: c.mul(cf)?, mono, parts });
    }
    if rest.as_const() != Some(Q::ONE) { parts.push((rest, 1)); }
    Ok(Factored { c, mono, parts })
}

// ---------- writing ----------

/// A rational as text and LaTeX.
pub fn q_text(q: Q) -> (String, String) {
    let s = if q.n < 0 { "-" } else { "" };
    if q.d == 1 { return (q.n.to_string(), q.n.to_string()); }
    (format!("{s}{}/{}", q.n.abs(), q.d), format!("{s}\\frac{{{}}}{{{}}}", q.n.abs(), q.d))
}

fn sup(n: u32) -> String { n.to_string().chars().map(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c.to_digit(10).unwrap_or(0) as usize).unwrap_or('⁰')).collect() }

fn mono_text(m: &Mono, name: &dyn Fn(&str) -> String) -> (String, String) {
    // π and e lead their term, as constants do (πr², not r²π).
    let mut m = m.clone();
    m.sort_by_key(|(v, _)| (!matches!(v.as_str(), "π" | "e"), v.clone()));
    // Names longer than one letter are joined with · in text, so x·y2 does not read as xy2.
    let sep = if m.iter().any(|(v, _)| v.chars().count() > 1) { "·" } else { "" };
    let t = m.iter().map(|(v, e)| if *e == 1 { crate::eval::canon(v) } else { format!("{}{}", crate::eval::canon(v), sup(*e)) }).collect::<Vec<_>>().join(sep);
    let x = m.iter().map(|(v, e)| if *e == 1 { name(v) } else { format!("{{{}}}^{{{e}}}", name(v)) }).collect::<Vec<_>>().join(" ");
    (t, x)
}

/// A polynomial as text and LaTeX, highest degree first: 3x² − 2xy + 1/2.
pub fn poly_text(p: &Poly, name: &dyn Fn(&str) -> String) -> (String, String) {
    if p.is_zero() { return ("0".into(), "0".into()); }
    let mut terms: Vec<(&Mono, &Q)> = p.0.iter().collect();
    terms.sort_by(|a, b| order(b.0, a.0));
    let (mut t, mut x) = (String::new(), String::new());
    for (i, (m, q)) in terms.iter().enumerate() {
        let neg = q.n < 0;
        let a = Q { n: q.n.abs(), d: q.d };
        let (mt, mx) = mono_text(m, name);
        let (ct, cx) = q_text(a);
        // A fractional coefficient is bracketed in text: (5/6)x, not 5/6x.
        let ct = if a.d != 1 && !m.is_empty() { format!("({ct})") } else { ct };
        let (body_t, body_x) = if m.is_empty() { (ct, cx) } else if a == Q::ONE { (mt, mx) } else { (format!("{ct}{mt}"), format!("{cx}{mx}")) };
        if i == 0 { t.push_str(if neg { "-" } else { "" }); x.push_str(if neg { "-" } else { "" }); }
        else { t.push_str(if neg { " − " } else { " + " }); x.push_str(if neg { " - " } else { " + " }); }
        t.push_str(&body_t);
        x.push_str(&body_x);
    }
    (t, x)
}

/// A fraction of polynomials: (x + 1)/(x − 1) as text, \frac{x + 1}{x - 1} as LaTeX.
pub fn ratf_text(r: &RatF, name: &dyn Fn(&str) -> String) -> (String, String) {
    let (nt, nx) = poly_text(&r.num, name);
    if r.den.as_const() == Some(Q::ONE) { return (nt, nx); }
    let (dt, dx) = poly_text(&r.den, name);
    let wrap = |s: String, p: &Poly| if p.0.len() > 1 { format!("({s})") } else { s };
    (format!("{}/{}", wrap(nt, &r.num), wrap(dt, &r.den)), format!("\\frac{{{nx}}}{{{dx}}}"))
}

/// A factored polynomial: 3xy(2x + 3), (x − 1)(2x + 1), 2(x + 1)².
pub fn factored_text(f: &Factored, name: &dyn Fn(&str) -> String) -> (String, String) {
    if f.c.is_zero() { return ("0".into(), "0".into()); }
    let (mut t, mut x) = (String::new(), String::new());
    let lone = f.mono.is_empty() && f.parts.is_empty();
    if f.c == Q::int(-1) && !lone { t.push('-'); x.push('-'); } else if f.c != Q::ONE || lone { let (a, b) = q_text(f.c); t.push_str(&a); x.push_str(&b); }
    let (mt, mx) = mono_text(&f.mono, name);
    t.push_str(&mt);
    if !mx.is_empty() { if !x.is_empty() && !x.ends_with('-') { x.push(' '); } x.push_str(&mx); }
    for (p, k) in &f.parts {
        let (pt, px) = poly_text(p, name);
        let single = p.0.len() == 1 || (f.parts.len() == 1 && f.mono.is_empty() && f.c == Q::ONE);
        let (pt, px) = if single && *k == 1 { (pt, px) } else { (format!("({pt})"), format!("\\left({px}\\right)")) };
        t.push_str(&if *k > 1 { format!("{pt}{}", sup(*k)) } else { pt });
        x.push_str(&if *k > 1 { format!("{px}^{{{k}}}") } else { px });
    }
    (t, x)
}
