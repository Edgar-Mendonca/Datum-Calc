//! The expression grammar: a lexer and a small Pratt parser with precise error positions.
//!
//! A region's text is one statement:
//! - `Q = 120 m^3/h` (assignment), `f(x) = 3*x^2 + 2` (user function),
//! - `x = 0, 0.1 .. 10` or `x = 0 .. 10` (range: a vector),
//! - `check sigma <= sigma_allow` (check), `solve f(x) = 0 for x in 0 .. 5` (root in a range),
//! - `plot H_pump(Q), H_sys(Q) for Q in 0 .. 200 m^3/h` (plot), or an expression to show (`P/Q =`).
//!
//! Units follow a number directly (`120 m^3/h`, `2.5 kN*m`, `1.2 W/(m^2*K)`). After the first unit,
//! `*` and `/` continue the unit only when the next word is a unit that is not one of the sheet's names,
//! so `2 m*t` reads as metre-tonnes but `2 m*t` with a variable `t` reads as 2 m times t.
use crate::units::{self, UnitExpr};

/// Character positions [from, to) in the region's text.
pub type Span = (usize, usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Error { pub msg: String, pub at: Span }
fn err<T>(msg: impl Into<String>, at: Span) -> Result<T, Error> { Err(Error { msg: msg.into(), at }) }

#[derive(Debug, Clone, PartialEq)]
pub enum Tok { Num(f64), Id(String), Op(&'static str) }

/// Splits text into tokens with their character positions.
pub fn lex(s: &str) -> Result<Vec<(Tok, Span)>, Error> {
    let c: Vec<char> = s.chars().collect();
    let (mut i, mut out) = (0, vec![]);
    let word = |ch: char| ch.is_alphabetic() || ch == '°' || ch == 'µ' || ch == 'Ω' || ch == 'Δ';
    while i < c.len() {
        let ch = c[i];
        let start = i;
        if ch.is_whitespace() { i += 1; continue; }
        if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            while i < c.len() && (c[i].is_ascii_digit() || (c[i] == '.' && c.get(i + 1) != Some(&'.'))) { i += 1; }
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                let mut j = i + 1;
                if j < c.len() && (c[j] == '+' || c[j] == '-') { j += 1; }
                if j < c.len() && c[j].is_ascii_digit() { while j < c.len() && c[j].is_ascii_digit() { j += 1; } i = j; }
            }
            let t: String = c[start..i].iter().collect();
            let v = t.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| Error { msg: format!("“{t}” is not a number this size can hold (up to about 10³⁰⁸)."), at: (start, i) })?;
            out.push((Tok::Num(v), (start, i)));
            continue;
        }
        if word(ch) {
            while i < c.len() && (word(c[i]) || c[i].is_ascii_digit() || c[i] == '_' || c[i] == '\'') { i += 1; }
            out.push((Tok::Id(c[start..i].iter().collect()), (start, i)));
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        let op: &'static str = match two.as_str() {
            "<=" => "<=", ">=" => ">=", "==" => "==", "!=" => "!=", ".." => "..", "**" => "^", "/_" => "∠",
            _ => match ch {
                '+' => "+", '-' | '−' => "-", '*' | '·' | '×' | '⋅' => "*", '/' | '÷' => "/", '^' => "^", '(' => "(", ')' => ")", '[' => "[", ']' => "]", ',' => ",", '=' => "=",
                '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '¹' | '⁰' => {
                    // A superscript digit is a power: m⁴ is m^4 (labels such as kg/s⁴ can be typed back).
                    out.push((Tok::Op("^"), (start, i + 1)));
                    out.push((Tok::Num("⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|d| d == ch).unwrap_or(0) as f64), (start, i + 1)));
                    i += 1;
                    continue;
                }
                '<' => "<", '>' => ">", '≤' => "<=", '≥' => ">=", '≠' => "!=", '²' => "²", '³' => "³", '%' => "%", ';' => ",", '…' => "..", '∠' => "∠",
                _ => return err(format!("“{ch}” cannot be used here."), (i, i + 1)),
            },
        };
        i += if op.len() == 2 && two == op || two == "**" || two == "/_" { 2 } else { 1 };
        if op == "%" { out.push((Tok::Id("%".into()), (start, i))); } else { out.push((Tok::Op(op), (start, i))); }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
pub enum E {
    /// A number with an optional unit.
    Num(f64, Option<UnitExpr>),
    Var(String),
    Neg(Box<Node>),
    /// + - * / ^ and comparisons.
    Bin(&'static str, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    /// Brackets the user typed (kept so the maths shows them).
    Group(Box<Node>),
    /// A vector `[1, 2, 3]` (one list) or a matrix `[[1, 2], [3, 4]]` (a list of rows).
    Mat(bool, Vec<Vec<Node>>),
    /// An entry of a vector or matrix, counted from 1: `v[2]`, `M[1, 2]`.
    Index(Box<Node>, Vec<Node>),
    /// An imaginary number typed as `4j` or `4i` (with its unit, `4j Ω`): the letter and the number.
    Im(char, Box<Node>),
    /// Brackets followed by a unit: `(3 + 4j) Ω`.
    Unit(Box<Node>, UnitExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node { pub e: E, pub at: Span }

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Assign { name: String, at: Span, e: Node },
    Func { name: String, params: Vec<String>, e: Node },
    Range { name: String, at: Span, start: Node, next: Option<Node>, end: Node },
    Check { e: Node },
    Solve { lhs: Node, rhs: Node, var: String, lo: Node, hi: Node },
    /// Several equations for as many unknowns, each from a starting guess (which gives its unit):
    /// `solve x + y = 3, x - y = 1 for x = 0, y = 0`.
    SolveSys { eqs: Vec<(Node, Node)>, vars: Vec<(String, Node)> },
    /// A model fitted to data by least squares (0.2.7): `fit H(Q) = a + b*Q + c*Q^2 to Q_d, H_d`, its parameters
    /// found by name or listed after `for`, with starting guesses when the model is not linear in them.
    Fit { name: String, var: String, model: Node, xs: Node, ys: Node, params: Vec<(String, Option<Node>)> },
    /// Differential equations (0.2.8): `ode x'' = -k/m*x for t in 0 s .. 10 s from x = 1 cm, x' = 0 m/s`, each
    /// equation for the highest derivative of a state, and a starting value for every state and lower derivative.
    Ode { eqs: Vec<(String, Node)>, var: String, lo: Node, hi: Node, inits: Vec<(String, Node)> },
    /// A unit of the sheet's own (0.2.9): `unit bbl = 158.987 L`, for the lines below.
    UnitDef { name: String, at: Span, e: Node },
    /// `log` after the range: a logarithmic x axis (a frequency sweep for a Bode plot).
    Plot { ys: Vec<Node>, var: String, lo: Node, hi: Node, log: bool },
    Show { e: Node },
    Empty,
}

/// Parses unit expressions over a token list (shared by numbers in expressions and `units::parse`).
pub struct UnitParser<'a> { pub toks: &'a [(Tok, Span)], pub i: usize, pub known: &'a [String] }

impl UnitParser<'_> {
    fn peek(&self, k: usize) -> Option<Tok> { self.toks.get(self.i + k).map(|t| t.0.clone()) }
    /// One unit (or bracketed unit expression) with an optional integer power.
    fn atom(&mut self, first: bool) -> Option<UnitExpr> {
        let save = self.i;
        let mut u = match self.peek(0)? {
            Tok::Id(s) => {
                if !first && self.known.contains(&s) { return None; }
                let (factor, dim, offset) = units::find(&s)?;
                self.i += 1;
                UnitExpr { factor, dim, offset, text: s }
            }
            Tok::Op("(") => {
                self.i += 1;
                let Some(inner) = self.expr_inner(true) else { self.i = save; return None };
                if self.peek(0) != Some(Tok::Op(")")) { self.i = save; return None; }
                self.i += 1;
                UnitExpr { text: format!("({})", inner.text), ..inner }
            }
            _ => return None,
        };
        let pw = match (self.peek(0), self.peek(1), self.peek(2)) {
            (Some(Tok::Op("^")), Some(Tok::Num(n)), _) if n.fract() == 0.0 && n.abs() <= 60.0 => { self.i += 2; Some(n as i8) }
            (Some(Tok::Op("^")), Some(Tok::Op("-")), Some(Tok::Num(n))) if n.fract() == 0.0 && n.abs() <= 60.0 => { self.i += 3; Some(-(n as i8)) }
            (Some(Tok::Op("²")), _, _) => { self.i += 1; Some(2) }
            (Some(Tok::Op("³")), _, _) => { self.i += 1; Some(3) }
            _ => None,
        };
        if let Some(n) = pw {
            if u.offset != 0.0 { self.i = save; return None; }
            u = UnitExpr { factor: u.factor.powi(n as i32), dim: units::pow(&u.dim, n), offset: 0.0, text: format!("{}^{n}", u.text) };
        }
        Some(u)
    }
    fn expr_inner(&mut self, first: bool) -> Option<UnitExpr> {
        let mut u = self.atom(first)?;
        loop {
            let op = match self.peek(0) { Some(Tok::Op("*")) => "*", Some(Tok::Op("/")) => "/", _ => break };
            let save = self.i;
            self.i += 1;
            let Some(v) = self.atom(false) else { self.i = save; break };
            if u.offset != 0.0 || v.offset != 0.0 { self.i = save; break; }
            u = if op == "*" { UnitExpr { factor: u.factor * v.factor, dim: units::mul(&u.dim, &v.dim), offset: 0.0, text: format!("{}*{}", u.text, v.text) } }
                else { UnitExpr { factor: u.factor / v.factor, dim: units::div(&u.dim, &v.dim), offset: 0.0, text: format!("{}/{}", u.text, v.text) } };
        }
        Some(u)
    }
    pub fn expr(&mut self) -> Option<UnitExpr> { self.expr_inner(true) }
}

struct P<'a> { t: Vec<(Tok, Span)>, i: usize, known: &'a [String], end: usize, depth: u32 }

impl P<'_> {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.i).map(|t| &t.0) }
    fn at(&self) -> Span { self.t.get(self.i).map(|t| t.1).unwrap_or((self.end, self.end)) }
    fn is(&self, op: &str) -> bool { matches!(self.peek(), Some(Tok::Op(o)) if *o == op) }
    fn word(&self, w: &str) -> bool { matches!(self.peek(), Some(Tok::Id(s)) if s == w) }
    fn eat(&mut self, op: &str) -> Result<Span, Error> {
        if self.is(op) { let a = self.at(); self.i += 1; Ok(a) } else {
            let got = self.t.get(self.i).map(|t| match &t.0 { Tok::Num(n) => n.to_string(), Tok::Id(s) => s.clone(), Tok::Op(o) => o.to_string() });
            err(match got { Some(g) => format!("Expected “{op}” before “{g}”."), None => format!("Expected “{op}” at the end.") }, self.at())
        }
    }
    fn name(&mut self) -> Result<(String, Span), Error> {
        match self.t.get(self.i) { Some((Tok::Id(s), a)) => { let r = (s.clone(), *a); self.i += 1; Ok(r) } _ => err("Expected a name here.", self.at()) }
    }

    /// Brackets, minus signs and powers inside each other are limited, so no line can exhaust the stack.
    fn expr(&mut self, min: u8) -> Result<Node, Error> {
        if self.depth >= MAX_NEST { return err(format!("Too many brackets or signs inside each other (more than {MAX_NEST})."), self.at()); }
        self.depth += 1;
        let r = self.expr_in(min);
        self.depth -= 1;
        r
    }

    fn expr_in(&mut self, min: u8) -> Result<Node, Error> {
        let mut lhs = self.prefix()?;
        loop {
            let Some(Tok::Op(op)) = self.peek().cloned() else {
                if let Some(t) = self.peek() {
                    let log_scale = matches!(t, Tok::Id(w) if w == "log") && !matches!(self.t.get(self.i + 1), Some((Tok::Op("("), _)));
                    if !log_scale && (matches!(t, Tok::Num(_)) || matches!(t, Tok::Id(w) if w != "for" && w != "in" && w != "to" && w != "from")) {
                        let a = self.at();
                        let what = match t { Tok::Id(s) if lhs_is_num(&lhs) => format!("“{s}” is not a unit. Put * before it to multiply."), Tok::Id(s) => format!("Put an operator (such as *) before “{s}”."), _ => "Put an operator (such as *) before this number.".into() };
                        return err(what, a);
                    }
                }
                break;
            };
            let (bp, right) = match op { "<" | "<=" | ">" | ">=" | "==" | "!=" => (1, 2), "+" | "-" => (3, 4), "*" | "/" => (5, 6), "∠" => (6, 7), "^" => (8, 7), "²" | "³" => (9, 0), "[" => (10, 0), _ => break };
            if bp < min { break; }
            self.i += 1;
            // An entry: v[2], M[1, 2].
            if op == "[" {
                let mut idx = vec![self.expr(1)?];
                while self.is(",") { self.i += 1; idx.push(self.expr(1)?); }
                let close = self.eat("]")?;
                let at = (lhs.at.0, close.1);
                lhs = Node { e: E::Index(Box::new(lhs), idx), at };
                continue;
            }
            if op == "²" || op == "³" {
                let at = (lhs.at.0, self.t[self.i - 1].1 .1);
                lhs = Node { e: E::Bin("^", Box::new(lhs), Box::new(Node { e: E::Num(if op == "²" { 2.0 } else { 3.0 }, None), at })), at };
                continue;
            }
            let rhs = self.expr(right)?;
            let at = (lhs.at.0, rhs.at.1);
            lhs = Node { e: E::Bin(op, Box::new(lhs), Box::new(rhs)), at };
        }
        Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Node, Error> {
        let a = self.at();
        match self.t.get(self.i).cloned() {
            Some((Tok::Num(v), at)) => {
                self.i += 1;
                // 4j and 4i (the letter right after the number) are imaginary: 3 + 4j Ω.
                if let Some((Tok::Id(w), wat)) = self.t.get(self.i).cloned() {
                    if (w == "j" || w == "i") && wat.0 == at.1 {
                        self.i += 1;
                        let mut up = UnitParser { toks: &self.t, i: self.i, known: self.known };
                        let u = up.expr();
                        let end = if u.is_some() { self.t[up.i - 1].1 .1 } else { wat.1 };
                        self.i = up.i;
                        let num = Node { e: E::Num(v, u), at: (at.0, end) };
                        return Ok(Node { e: E::Im(if w == "j" { 'j' } else { 'i' }, Box::new(num)), at: (at.0, end) });
                    }
                }
                let mut up = UnitParser { toks: &self.t, i: self.i, known: self.known };
                let u = up.expr();
                let end = if u.is_some() { self.t[up.i - 1].1 .1 } else { at.1 };
                self.i = up.i;
                Ok(Node { e: E::Num(v, u), at: (at.0, end) })
            }
            Some((Tok::Id(s), at)) => {
                self.i += 1;
                if self.is("(") {
                    self.i += 1;
                    let mut args = vec![];
                    if !self.is(")") {
                        loop { args.push(self.expr(1)?); if self.is(",") { self.i += 1; } else { break; } }
                    }
                    let close = self.eat(")")?;
                    return Ok(Node { e: E::Call(s, args), at: (at.0, close.1) });
                }
                Ok(Node { e: E::Var(s), at })
            }
            Some((Tok::Op("-"), at)) => {
                self.i += 1;
                // −40 °C is a temperature below zero, not minus (40 °C).
                if let Some((Tok::Num(_), _)) = self.t.get(self.i) {
                    let save = self.i;
                    if let Node { e: E::Num(v, Some(u)), at: nat } = self.prefix()? { if u.offset != 0.0 { return Ok(Node { e: E::Num(-v, Some(u)), at: (at.0, nat.1) }); } }
                    self.i = save;
                }
                let e = self.expr(7)?;
                let at = (at.0, e.at.1);
                Ok(Node { e: E::Neg(Box::new(e)), at })
            }
            Some((Tok::Op("+"), _)) => { self.i += 1; self.expr(7) }
            Some((Tok::Op("("), at)) => {
                self.i += 1;
                let e = self.expr(1)?;
                let close = self.eat(")")?;
                let g = Node { e: E::Group(Box::new(e)), at: (at.0, close.1) };
                // A unit after the brackets applies to all of it, (3 + 4j) Ω, unless it is one of the sheet's names.
                let mut up = UnitParser { toks: &self.t, i: self.i, known: self.known };
                match up.expr_inner(false) {
                    Some(u) if u.offset == 0.0 => { let end = self.t[up.i - 1].1 .1; self.i = up.i; Ok(Node { e: E::Unit(Box::new(g), u), at: (at.0, end) }) }
                    _ => Ok(g),
                }
            }
            Some((Tok::Op("["), at)) => {
                // [1, 2, 3] is a vector; [[1, 2], [3, 4]] a matrix, a list of rows. A unit after it applies to all.
                self.i += 1;
                let matrix = self.is("[");
                let mut rows: Vec<Vec<Node>> = vec![];
                let list = |p: &mut Self| -> Result<Vec<Node>, Error> {
                    let mut v = vec![];
                    if p.is("]") { return Ok(v); }
                    loop { v.push(p.expr(1)?); if p.is(",") { p.i += 1; } else { break; } }
                    Ok(v)
                };
                if matrix {
                    loop {
                        self.eat("[")?;
                        rows.push(list(self)?);
                        self.eat("]")?;
                        if self.is(",") { self.i += 1; } else { break; }
                    }
                } else { rows.push(list(self)?); }
                let close = self.eat("]")?;
                if rows.iter().all(|r| r.is_empty()) { return err("A vector or matrix needs at least one value.", (at.0, close.1)); }
                if matrix && rows.iter().any(|r| r.len() != rows[0].len()) { return err("Every row of a matrix needs the same number of values.", (at.0, close.1)); }
                let m = Node { e: E::Mat(matrix, rows), at: (at.0, close.1) };
                let mut up = UnitParser { toks: &self.t, i: self.i, known: self.known };
                match up.expr_inner(false) {
                    Some(u) if u.offset == 0.0 => { let end = self.t[up.i - 1].1 .1; self.i = up.i; Ok(Node { e: E::Unit(Box::new(m), u), at: (at.0, end) }) }
                    // Temperatures after a vector of plain numbers, [20, 30] °C: each number takes the unit, as typed after each.
                    Some(u) if !matrix => {
                        let E::Mat(_, rows) = &m.e else { return Ok(m) };
                        let num = |n: &Node| match &n.e { E::Num(x, None) => Some(*x), E::Neg(a) => if let E::Num(x, None) = a.e { Some(-x) } else { None }, _ => None };
                        let Some(xs): Option<Vec<f64>> = rows[0].iter().map(num).collect() else { return Ok(m) };
                        let end = self.t[up.i - 1].1 .1;
                        self.i = up.i;
                        Ok(Node { e: E::Mat(false, vec![xs.iter().zip(&rows[0]).map(|(x, n)| Node { e: E::Num(*x, Some(u.clone())), at: n.at }).collect()]), at: (at.0, end) })
                    }
                    _ => Ok(m),
                }
            }
            Some((Tok::Op(o), _)) => err(format!("“{o}” cannot start a value here."), a),
            None => err("Something is missing at the end.", a),
        }
    }

    fn done(&self) -> Result<(), Error> {
        if self.i < self.t.len() { return err(match &self.t[self.i].0 { Tok::Op(")") => "This bracket has no opening “(”.".to_string(), _ => "Unexpected text here.".to_string() }, (self.at().0, self.end)); }
        Ok(())
    }

    /// `for x in lo .. hi` after solve and plot.
    fn for_in(&mut self) -> Result<(String, Node, Node), Error> {
        if !self.word("for") { return err("Write “for x in a .. b” to say what to vary and over which range.", self.at()); }
        self.i += 1;
        let (var, _) = self.name()?;
        if !self.word("in") { return err("Write “in” before the range, as in “for x in 0 .. 10”.", self.at()); }
        self.i += 1;
        let lo = self.expr(1)?;
        self.eat("..")?;
        let hi = self.expr(1)?;
        Ok((var, lo, hi))
    }
}

fn lhs_is_num(n: &Node) -> bool { matches!(n.e, E::Num(..)) }

const MAX_NEST: u32 = 60;
const MAX_TOKENS: usize = 1000;
/// The deepest expression tree that is worked out (a chain of 1 + 1 + … grows one level per sign).
const MAX_DEPTH: usize = 400;

/// The depth of an expression tree, found without recursion.
pub fn depth(n: &Node) -> usize {
    let mut stack = vec![(n, 1usize)];
    let mut max = 0;
    while let Some((n, d)) = stack.pop() {
        max = max.max(d);
        match &n.e {
            E::Neg(a) | E::Group(a) | E::Im(_, a) | E::Unit(a, _) => stack.push((a, d + 1)),
            E::Bin(_, a, b) => { stack.push((a, d + 1)); stack.push((b, d + 1)); }
            E::Call(_, args) => stack.extend(args.iter().map(|a| (a, d + 1))),
            E::Mat(_, rows) => stack.extend(rows.iter().flatten().map(|a| (a, d + 1))),
            E::Index(a, idx) => { stack.push((a, d + 1)); stack.extend(idx.iter().map(|x| (x, d + 1))); }
            _ => {}
        }
    }
    max
}

/// Names that are never read as a unit after `*` or `/`: the built-in constants (g is gravity, not grams).
const NOT_UNITS: [&str; 4] = ["g", "e", "R", "c_0"];

/// Parses one region's text. `known` are the names defined above it (they stop unit continuation).
pub fn statement(s: &str, known: &[String]) -> Result<Stmt, Error> {
    let s2 = s.trim_end();
    let s2 = s2.strip_suffix('=').unwrap_or(s2);
    let t = lex(s2)?;
    if t.is_empty() { return Ok(Stmt::Empty); }
    // A function's parameters and the variable after "for" are names too, though defined on this line.
    let mut names: Vec<String> = known.to_vec();
    names.extend(NOT_UNITS.iter().map(|x| x.to_string()));
    if let (Some(Tok::Id(_)), Some(Tok::Op("("))) = (t.first().map(|x| &x.0), t.get(1).map(|x| &x.0)) {
        if is_def(&t) { names.extend(t.iter().skip(2).take_while(|x| x.0 != Tok::Op(")")).filter_map(|x| if let Tok::Id(n) = &x.0 { Some(n.clone()) } else { None })); }
    }
    // The unknowns after "for" (for x in …, or for x = guess, y = guess) are names on this line.
    if let Some(k) = t.iter().position(|x| x.0 == Tok::Id("for".into())) {
        if let Some((Tok::Id(n), _)) = t.get(k + 1) { names.push(n.clone()); }
        for j in k + 1..t.len() { if let (Tok::Id(n), Some((Tok::Op("="), _))) = (&t[j].0, t.get(j + 1)) { names.push(n.clone()); } }
    }
    // Words in a unit after a number or a bracket (1 s, 0.5 m^3/h, 10 W/(m^2*K), (2 + 3) mm/s) stay units: the
    // same reader that reads them later marks them; the other words are names.
    let unit_at = {
        let mut u = vec![false; t.len()];
        let base = names.clone();
        let mut j = 1;
        while j < t.len() {
            if matches!(t[j - 1].0, Tok::Num(_) | Tok::Op(")" | "]")) && matches!(t[j].0, Tok::Id(_) | Tok::Op("(")) {
                let mut up = UnitParser { toks: &t, i: j, known: &base };
                if up.expr_inner(false).is_some() && up.i > j { u[j..up.i].iter_mut().for_each(|x| *x = true); j = up.i; continue; }
            }
            j += 1;
        }
        u
    };
    if matches!(t.first().map(|x| &x.0), Some(Tok::Id(w)) if w == "ode") {
        let to = t.iter().position(|x| x.0 == Tok::Id("for".into())).unwrap_or(t.len());
        for (j, x) in t.iter().enumerate().take(to).skip(1) { if let Tok::Id(n) = &x.0 { if t.get(j + 1).map(|y| &y.0) != Some(&Tok::Op("(")) && !unit_at[j] { names.push(n.clone()); } } }
    }
    // On a fit line the names in the model (a, h, t would otherwise read as units), its variable and the parameters
    // after "for" are names; the data and the guesses keep their units.
    if matches!(t.first().map(|x| &x.0), Some(Tok::Id(w)) if w == "fit") {
        let to = t.iter().position(|x| x.0 == Tok::Id("to".into())).unwrap_or(t.len());
        for (j, x) in t.iter().enumerate().take(to).skip(2) { if let Tok::Id(n) = &x.0 { if t.get(j + 1).map(|y| &y.0) != Some(&Tok::Op("(")) && !unit_at[j] { names.push(n.clone()); } } }
        if let Some(k) = t.iter().position(|x| x.0 == Tok::Id("for".into())) {
            for j in k + 1..t.len() { if let (Tok::Id(n), Some(prev)) = (&t[j].0, t.get(j - 1)) { if matches!(&prev.0, Tok::Op(",")) || j == k + 1 { names.push(n.clone()); } } }
        }
    }
    if t.len() > MAX_TOKENS { return err(format!("This line is too long (more than {MAX_TOKENS} symbols): split it over several lines."), (0, s2.chars().count())); }
    let mut p = P { end: s2.chars().count(), t, i: 0, known: &names, depth: 0 };
    let st = match (&p.t[0].0, p.t.get(1).map(|x| &x.0)) {
        (Tok::Id(w), Some(_)) if w == "check" => { p.i = 1; Stmt::Check { e: p.expr(0)? } }
        (Tok::Id(w), Some(_)) if w == "solve" => {
            p.i = 1;
            let mut eqs = vec![];
            loop {
                let lhs = p.expr(2)?;
                p.eat("=")?;
                let rhs = p.expr(2)?;
                eqs.push((lhs, rhs));
                if p.is(",") { p.i += 1; } else { break; }
            }
            // "for x = guess, …": from starting guesses (several equations); "for x in a .. b": in a range (one).
            if p.word("for") && matches!(p.t.get(p.i + 2).map(|x| &x.0), Some(Tok::Op("="))) {
                p.i += 1;
                let mut vars = vec![];
                loop {
                    let (n, _) = p.name()?;
                    p.eat("=")?;
                    vars.push((n, p.expr(2)?));
                    if p.is(",") { p.i += 1; } else { break; }
                }
                Stmt::SolveSys { eqs, vars }
            } else {
                if eqs.len() > 1 { return err("Several equations need a starting guess for each unknown: for x = 1 m, y = 2 m.", p.at()); }
                let (lhs, rhs) = eqs.pop().ok_or_else(|| Error { msg: "Write an equation to solve.".into(), at: p.at() })?;
                let (var, lo, hi) = p.for_in()?;
                Stmt::Solve { lhs, rhs, var, lo, hi }
            }
        }
        (Tok::Id(w), Some(Tok::Id(_))) if w == "unit" && matches!(p.t.get(2).map(|x| &x.0), Some(Tok::Op("="))) => {
            let (name, at) = (match &p.t[1].0 { Tok::Id(n) => n.clone(), _ => unreachable!() }, p.t[1].1);
            p.i = 3;
            Stmt::UnitDef { name, at, e: p.expr(1)? }
        }
        (Tok::Id(w), Some(_)) if w == "ode" => {
            p.i = 1;
            let mut eqs = vec![];
            loop {
                let (n, at) = p.name()?;
                if !n.ends_with('\'') { return err("Write each equation for a derivative: x' = … (or x'' = … for a second derivative).", at); }
                p.eat("=")?;
                eqs.push((n, p.expr(1)?));
                if p.is(",") { p.i += 1; } else { break; }
            }
            let (var, lo, hi) = p.for_in()?;
            if !p.word("from") { return err("Give the starting values after the range: … from x = 1 m, x' = 0 m/s.", p.at()); }
            p.i += 1;
            let mut inits = vec![];
            loop {
                let (n, _) = p.name()?;
                p.eat("=")?;
                inits.push((n, p.expr(1)?));
                if p.is(",") { p.i += 1; } else { break; }
            }
            Stmt::Ode { eqs, var, lo, hi, inits }
        }
        (Tok::Id(w), Some(_)) if w == "fit" => {
            p.i = 1;
            let (name, _) = p.name()?;
            p.eat("(")?;
            let (var, _) = p.name()?;
            p.eat(")")?;
            p.eat("=")?;
            let model = p.expr(1)?;
            if !p.word("to") { return err("Give the data after the model: fit H(Q) = a + b*Q to Q_d, H_d.", p.at()); }
            p.i += 1;
            let xs = p.expr(1)?;
            p.eat(",")?;
            let ys = p.expr(1)?;
            let mut params = vec![];
            if p.word("for") {
                p.i += 1;
                loop {
                    let (n, _) = p.name()?;
                    let g = if p.is("=") { p.i += 1; Some(p.expr(2)?) } else { None };
                    params.push((n, g));
                    if p.is(",") { p.i += 1; } else { break; }
                }
            }
            Stmt::Fit { name, var, model, xs, ys, params }
        }
        (Tok::Id(w), Some(_)) if w == "plot" => {
            p.i = 1;
            let mut ys = vec![p.expr(1)?];
            while p.is(",") { p.i += 1; ys.push(p.expr(1)?); }
            let (var, lo, hi) = p.for_in()?;
            let log = p.word("log");
            if log { p.i += 1; }
            Stmt::Plot { ys, var, lo, hi, log }
        }
        (Tok::Id(name), Some(Tok::Op("("))) if p.t.iter().any(|x| x.0 == Tok::Op("=")) && is_def(&p.t) => {
            let name = name.clone();
            p.i = 2;
            let mut params = vec![];
            while !p.is(")") { params.push(p.name()?.0); if p.is(",") { p.i += 1; } else { break; } }
            p.eat(")")?;
            p.eat("=")?;
            Stmt::Func { name, params, e: p.expr(1)? }
        }
        (Tok::Id(name), Some(Tok::Op("="))) => {
            let (name, at) = (name.clone(), p.t[0].1);
            p.i = 2;
            let first = p.expr(1)?;
            if p.is(",") || p.is("..") {
                let next = if p.is(",") { p.i += 1; Some(p.expr(1)?) } else { None };
                p.eat("..")?;
                Stmt::Range { name, at, start: first, next, end: p.expr(1)? }
            } else { Stmt::Assign { name, at, e: first } }
        }
        _ => Stmt::Show { e: p.expr(1)? },
    };
    p.done()?;
    let nodes: Vec<&Node> = match &st {
        Stmt::Assign { e, .. } | Stmt::Func { e, .. } | Stmt::Check { e } | Stmt::Show { e } => vec![e],
        Stmt::Range { start, next, end, .. } => [Some(start), next.as_ref(), Some(end)].into_iter().flatten().collect(),
        Stmt::Solve { lhs, rhs, lo, hi, .. } => vec![lhs, rhs, lo, hi],
        Stmt::SolveSys { eqs, vars } => eqs.iter().flat_map(|(a, b)| [a, b]).chain(vars.iter().map(|v| &v.1)).collect(),
        Stmt::Plot { ys, lo, hi, .. } => ys.iter().chain([lo, hi]).collect(),
        Stmt::UnitDef { e, .. } => vec![e],
        Stmt::Ode { eqs, lo, hi, inits, .. } => eqs.iter().map(|e| &e.1).chain([lo, hi]).chain(inits.iter().map(|i| &i.1)).collect(),
        Stmt::Fit { model, xs, ys, params, .. } => [model, xs, ys].into_iter().chain(params.iter().filter_map(|p| p.1.as_ref())).collect(),
        Stmt::Empty => vec![],
    };
    if nodes.iter().any(|n| depth(n) > MAX_DEPTH) { return err("This line is too long to work out: split it over several lines.", (0, p.end)); }
    Ok(st)
}

/// `f(x, y) = …` defines a function when the brackets hold plain names.
fn is_def(t: &[(Tok, Span)]) -> bool {
    let mut i = 2;
    loop {
        match t.get(i).map(|x| &x.0) { Some(Tok::Id(_)) => i += 1, Some(Tok::Op(")")) => break, _ => return false }
        match t.get(i).map(|x| &x.0) { Some(Tok::Op(",")) => i += 1, Some(Tok::Op(")")) => break, _ => return false }
    }
    t.get(i + 1).map(|x| &x.0) == Some(&Tok::Op("="))
}
