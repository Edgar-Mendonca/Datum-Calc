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
            let v = t.parse::<f64>().map_err(|_| Error { msg: format!("“{t}” is not a number."), at: (start, i) })?;
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
            "<=" => "<=", ">=" => ">=", "==" => "==", "!=" => "!=", ".." => "..", "**" => "^",
            _ => match ch {
                '+' => "+", '-' | '−' => "-", '*' | '·' | '×' | '⋅' => "*", '/' | '÷' => "/", '^' => "^", '(' => "(", ')' => ")", ',' => ",", '=' => "=",
                '<' => "<", '>' => ">", '≤' => "<=", '≥' => ">=", '≠' => "!=", '²' => "²", '³' => "³", '%' => "%", ';' => ",", '…' => "..",
                _ => return err(format!("“{ch}” cannot be used here."), (i, i + 1)),
            },
        };
        i += if op.len() == 2 && two == op || two == "**" { 2 } else { 1 };
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
    Plot { ys: Vec<Node>, var: String, lo: Node, hi: Node },
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
                let u = units::lookup(&s)?;
                self.i += 1;
                UnitExpr { factor: u.factor, dim: u.dim, offset: u.offset, text: s }
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
            (Some(Tok::Op("^")), Some(Tok::Num(n)), _) if n.fract() == 0.0 => { self.i += 2; Some(n as i8) }
            (Some(Tok::Op("^")), Some(Tok::Op("-")), Some(Tok::Num(n))) if n.fract() == 0.0 => { self.i += 3; Some(-(n as i8)) }
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

struct P<'a> { t: Vec<(Tok, Span)>, i: usize, known: &'a [String], end: usize }

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

    fn expr(&mut self, min: u8) -> Result<Node, Error> {
        let mut lhs = self.prefix()?;
        loop {
            let Some(Tok::Op(op)) = self.peek().cloned() else {
                if let Some(t) = self.peek() {
                    if matches!(t, Tok::Num(_)) || matches!(t, Tok::Id(w) if w != "for" && w != "in") {
                        let a = self.at();
                        let what = match t { Tok::Id(s) if lhs_is_num(&lhs) => format!("“{s}” is not a unit. Put * before it to multiply."), Tok::Id(s) => format!("Put an operator (such as *) before “{s}”."), _ => "Put an operator (such as *) before this number.".into() };
                        return err(what, a);
                    }
                }
                break;
            };
            let (bp, right) = match op { "<" | "<=" | ">" | ">=" | "==" | "!=" => (1, 2), "+" | "-" => (3, 4), "*" | "/" => (5, 6), "^" => (8, 7), "²" | "³" => (9, 0), _ => break };
            if bp < min { break; }
            self.i += 1;
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
                Ok(Node { e: E::Group(Box::new(e)), at: (at.0, close.1) })
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

/// Parses one region's text. `known` are the names defined above it (they stop unit continuation).
pub fn statement(s: &str, known: &[String]) -> Result<Stmt, Error> {
    let s2 = s.trim_end();
    let s2 = s2.strip_suffix('=').unwrap_or(s2);
    let t = lex(s2)?;
    if t.is_empty() { return Ok(Stmt::Empty); }
    let mut p = P { end: s2.chars().count(), t, i: 0, known };
    let st = match (&p.t[0].0, p.t.get(1).map(|x| &x.0)) {
        (Tok::Id(w), Some(_)) if w == "check" => { p.i = 1; Stmt::Check { e: p.expr(0)? } }
        (Tok::Id(w), Some(_)) if w == "solve" => {
            p.i = 1;
            let lhs = p.expr(2)?;
            p.eat("=")?;
            let rhs = p.expr(2)?;
            let (var, lo, hi) = p.for_in()?;
            Stmt::Solve { lhs, rhs, var, lo, hi }
        }
        (Tok::Id(w), Some(_)) if w == "plot" => {
            p.i = 1;
            let mut ys = vec![p.expr(1)?];
            while p.is(",") { p.i += 1; ys.push(p.expr(1)?); }
            let (var, lo, hi) = p.for_in()?;
            Stmt::Plot { ys, var, lo, hi }
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
