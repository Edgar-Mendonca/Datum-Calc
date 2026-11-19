//! Small programs in a region (0.2.5): one statement per line, `name = …`, `if` / `else if` / `else`,
//! `for x in a .. b` (or `a, next .. b`, or over a vector), `while`, `break` and `return`, each block closed by
//! `end`. With a first line `name(a, b) =` the program is a function that returns a value; otherwise it runs
//! once and every name it sets is defined for the lines below.
use crate::eval::{canon, real, Env, Val};
use crate::latex;
use crate::parse::{self, Error, Node, Span, Stmt, Tok};

type R<T> = Result<T, Error>;
fn fail<T>(msg: impl Into<String>, at: Span) -> R<T> { Err(Error { msg: msg.into(), at }) }

/// How many times the loops of one run may go round before the program is stopped.
pub const MAX_STEPS: u32 = 100_000;
/// How many rounds all the programs on one line may make together (a plot or solve calls a function many times).
pub const MAX_LINE: u64 = 1_000_000;
pub const LINE_MSG: &str = "The programs on this line went round more than 1,000,000 times in all: check their loops.";

/// One more round of a loop: within this run's limit and the line's.
fn tick(env: &mut Env, steps: &mut u32, at: Span) -> R<()> {
    *steps += 1;
    if *steps > MAX_STEPS { return fail("The loops went round more than 100,000 times: check that this one ends.", at); }
    if env.tick() > MAX_LINE { return fail(LINE_MSG, at); }
    Ok(())
}

pub enum Line {
    Set(String, Node),
    If(Vec<(Node, Vec<Line>)>, Vec<Line>),
    For(String, Each, Vec<Line>, Span),
    While(Node, Vec<Line>, Span),
    Break,
    Return(Node),
}

/// What a `for` runs over: a range like a range line (`a .. b`, `a, next .. b`), or each value of a vector.
pub enum Each { Range(Node, Option<Node>, Node), Vals(Node) }

/// A function program's name and parameters.
pub type Head = (String, Vec<String>);

pub struct Prog {
    /// `name(a, b) =` on the first line: the program is a function.
    pub head: Option<Head>,
    pub body: Vec<Line>,
    /// Every name the program sets, in order (loop names included), so a function can put them back.
    pub sets: Vec<String>,
    /// The names of its loops: local, put back after the program has run.
    pub loops: Vec<String>,
    pub tex: String,
}

/// The lines of the text with their start (in characters) and the code without a `#` note.
fn lines(t: &str) -> Vec<(usize, &str)> {
    let mut at = 0;
    t.split('\n').map(|l| { let s = at; at += l.chars().count() + 1; (s, crate::eval::split_note(l).0) }).collect()
}

/// A line padded on the left with spaces, so the positions in it are positions in the whole text.
fn padded(start: usize, s: &str) -> String { format!("{}{s}", " ".repeat(start)) }

/// `name(a, b) =` and nothing else.
fn header(s: &str) -> Option<Head> {
    let t = parse::lex(s).ok()?;
    let ids = |i: usize| match t.get(i).map(|x| &x.0) { Some(Tok::Id(n)) => Some(n.clone()), _ => None };
    let name = ids(0)?;
    if t.get(1)?.0 != Tok::Op("(") || t.last()?.0 != Tok::Op("=") || t[t.len() - 2].0 != Tok::Op(")") { return None; }
    let mut ps = vec![];
    let mut i = 2;
    while i < t.len() - 2 { ps.push(ids(i)?); i += 1; if i < t.len() - 2 { if t[i].0 != Tok::Op(",") { return None; } i += 1; } }
    Some((name, ps))
}

/// The first word of a line and the rest after it.
fn word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    let n = s.find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '\'' | '°' | 'µ' | 'Ω' | 'Δ'))).unwrap_or(s.len());
    (&s[..n], &s[n..])
}

/// The names a program sets (and its function name and parameters), read from its text without parsing it fully.
pub fn names(t: &str) -> (Option<Head>, Vec<String>, Vec<String>) {
    let ls = lines(t);
    let first = ls.iter().position(|l| !l.1.trim().is_empty());
    let head = first.and_then(|i| header(ls[i].1));
    let (mut sets, mut loops): (Vec<String>, Vec<String>) = (vec![], vec![]);
    for (k, (_, l)) in ls.iter().enumerate() {
        if head.is_some() && Some(k) == first { continue; }
        let (w, rest) = word(l);
        let n = if w == "for" { word(rest).0 } else if rest.trim_start().starts_with('=') && !rest.trim_start().starts_with("==") { w } else { "" };
        if n.is_empty() || matches!(n, "if" | "else" | "while" | "end" | "break" | "return") { continue; }
        if w == "for" && !loops.contains(&n.to_string()) { loops.push(n.to_string()); }
        if !sets.contains(&n.to_string()) { sets.push(n.to_string()); }
    }
    // A name set outside a loop header is the program's result, even if a loop also uses it.
    loops.retain(|n| !ls.iter().any(|(_, l)| { let (w, rest) = word(l); w == n && rest.trim_start().starts_with('=') && !rest.trim_start().starts_with("==") }));
    (head, sets, loops)
}

/// Reads a program. `known` holds the names defined above; the program's own names and parameters are added.
pub fn parse(t: &str, known: &[String]) -> R<Prog> {
    let (head, sets, loops) = names(t);
    let mut kn: Vec<String> = known.to_vec();
    kn.extend(sets.iter().cloned());
    if let Some((_, ps)) = &head { kn.extend(ps.iter().cloned()); }
    let ls = lines(t);
    let skip = if head.is_some() { ls.iter().position(|l| !l.1.trim().is_empty()) } else { None };
    if ls.len() > 300 { return fail("A program has at most 300 lines.", (0, t.chars().count())); }
    // Blocks being read: the lines so far, and how the block began.
    enum Open { Top, If(Vec<(Node, Vec<Line>)>, Option<Node>, bool), For(String, Each, Span), While(Node, Span) }
    let mut stack: Vec<(Open, Vec<Line>, Span)> = vec![(Open::Top, vec![], (0, 0))];
    let mut tex: Vec<String> = vec![];
    if let Some((n, ps)) = &head { tex.push(format!("{}\\left({}\\right) =", latex::name(n), ps.iter().map(|p| latex::name(p)).collect::<Vec<_>>().join(", "))); }
    let ind = |d: usize| "\\quad ".repeat(d + usize::from(head.is_some()));
    let kw = |w: &str| format!("\\textbf{{{w}}}\\ ");
    for (k, &(start, l)) in ls.iter().enumerate() {
        if Some(k) == skip || l.trim().is_empty() { continue; }
        let lead = l.chars().take_while(|c| c.is_whitespace()).count();
        let at: Span = (start + lead, start + l.trim_end().chars().count());
        let (w, rest) = word(l);
        // The expression after a keyword, at its place in the text.
        let after = |n: usize| -> R<Node> {
            let t = l.trim_start();
            let off = start + lead + t[..n].chars().count();
            match parse::statement(&padded(off, &t[n..]), &kn)? {
                Stmt::Show { e } => Ok(e),
                _ => fail(format!("After {w} comes a value or a condition."), at),
            }
        };
        let depth = stack.len() - 1;
        match w {
            "end" if rest.trim().is_empty() => {
                if stack.len() == 1 { return fail("This end has no if, for or while to close.", at); }
                let (open, body, _) = stack.pop().expect("open block");
                let line = match open {
                    Open::If(mut arms, cond, in_else) => if in_else { Line::If(arms, body) } else { arms.push((cond.expect("if"), body)); Line::If(arms, vec![]) },
                    Open::For(n, e, s) => Line::For(n, e, body, s),
                    Open::While(c, s) => Line::While(c, body, s),
                    Open::Top => unreachable!(),
                };
                stack.last_mut().expect("top").1.push(line);
                tex.push(format!("{}{}", ind(depth - 1), "\\textbf{end}"));
            }
            "if" | "while" => {
                let c = after(w.len())?;
                tex.push(format!("{}{}{}", ind(depth), kw(w), latex::node(&c)));
                stack.push((if w == "if" { Open::If(vec![], Some(c), false) } else { Open::While(c, at) }, vec![], at));
            }
            "else" => {
                let Some((Open::If(arms, cond, in_else), body, _)) = stack.last_mut() else { return fail("This else has no if above it.", at) };
                if *in_else { return fail("This if already has an else.", at); }
                arms.push((cond.take().expect("if"), std::mem::take(body)));
                let (w2, _) = word(rest);
                if w2 == "if" {
                    let n = l.trim_start().find("if").unwrap_or(4) + 2;
                    let c = after(n)?;
                    tex.push(format!("{}{}{}", ind(depth - 1), kw("else if"), latex::node(&c)));
                    *cond = Some(c);
                } else if rest.trim().is_empty() {
                    *in_else = true;
                    tex.push(format!("{}\\textbf{{else}}", ind(depth - 1)));
                } else { return fail("Write else on its own line, or else if with a condition.", at); }
            }
            "for" => {
                // for x in … reads as the line x = …: a range, or a vector to go through.
                let (n, r2) = word(rest);
                let r3 = r2.trim_start();
                if n.is_empty() || !r3.starts_with("in") || !r3[2..].starts_with(char::is_whitespace) { return fail("Write a loop as for x in 1 .. 10, or for x in v.", at); }
                let off = start + l.chars().count() - r3.chars().count();
                // x = … with the text after "in" at its own place.
                let at_rest = off + 2;
                let lhs = format!("{n} =");
                let each = match parse::statement(&padded(at_rest.saturating_sub(lhs.chars().count()), &format!("{lhs}{}", &r3[2..])), &kn)? {
                    Stmt::Range { start: a, next, end, .. } => Each::Range(a, next, end),
                    Stmt::Assign { e, .. } => Each::Vals(e),
                    _ => return fail("Write a loop as for x in 1 .. 10, or for x in v.", at),
                };
                let shown = match &each { Each::Range(a, nx, b) => format!("{}{}\\ldots {}", latex::node(a), nx.as_ref().map(|x| format!(", {}, ", latex::node(x))).unwrap_or_else(|| "\\,".into()), latex::node(b)), Each::Vals(v) => latex::node(v) };
                tex.push(format!("{}{}{} \\in {shown}", ind(depth), kw("for"), latex::name(n)));
                stack.push((Open::For(n.to_string(), each, at), vec![], at));
            }
            "break" if rest.trim().is_empty() => {
                if !stack.iter().any(|s| matches!(s.0, Open::For(..) | Open::While(..))) { return fail("break is for leaving a for or while loop.", at); }
                stack.last_mut().expect("top").1.push(Line::Break);
                tex.push(format!("{}\\textbf{{break}}", ind(depth)));
            }
            "return" => {
                if head.is_none() { return fail("return is for a program that is a function: start it with a line such as f(x) =", at); }
                let e = after(6)?;
                tex.push(format!("{}{}{}", ind(depth), kw("return"), latex::node(&e)));
                stack.last_mut().expect("top").1.push(Line::Return(e));
            }
            _ => match parse::statement(&padded(start, l), &kn)? {
                Stmt::Assign { name, e, .. } => {
                    tex.push(format!("{}{} \\leftarrow {}", ind(depth), latex::name(&name), latex::node(&e)));
                    stack.last_mut().expect("top").1.push(Line::Set(name, e));
                }
                _ => return fail("A program line sets a name (x = …), or is if, else, for, while, break, return or end.", at),
            },
        }
    }
    if stack.len() > 1 { let s = stack.last().expect("open").2; return fail("This block needs an end below it.", s); }
    let body = stack.pop().expect("top").1;
    if body.is_empty() { return fail("This program has no lines.", (0, t.chars().count())); }
    Ok(Prog { head, body, sets, loops, tex: format!("\\begin{{array}}{{l}} {} \\end{{array}}", tex.join(" \\\\ ")) })
}

/// How a run of lines ended.
pub enum Flow { Next, Break, Return(Val) }

/// A condition: one real value, true when not zero.
fn truth(env: &mut Env, c: &Node) -> R<bool> {
    let v = env.eval(c)?;
    real(&v, "A condition", c.at)?;
    if v.vec { return fail("A condition is one value: use max or min for a vector.", c.at); }
    Ok(v.x() != 0.0)
}

/// Runs lines; `steps` counts the times round the loops.
pub fn exec(env: &mut Env, lines: &[Line], steps: &mut u32) -> R<Flow> {
    for l in lines {
        let f = match l {
            Line::Set(n, e) => { let v = env.eval(e)?; env.exprs.remove(&canon(n)); env.vars.insert(canon(n), v); Flow::Next }
            Line::If(arms, other) => {
                let mut f = None;
                for (c, b) in arms { if truth(env, c)? { f = Some(exec(env, b, steps)?); break; } }
                match f { Some(f) => f, None => exec(env, other, steps)? }
            }
            Line::While(c, b, at) => {
                let mut f = Flow::Next;
                while truth(env, c)? {
                    tick(env, steps, *at)?;
                    match exec(env, b, steps)? { Flow::Break => break, Flow::Return(v) => { f = Flow::Return(v); break; } Flow::Next => {} }
                }
                f
            }
            Line::For(n, each, b, at) => {
                // The values one at a time: each entry with the units of the whole.
                let all = match each { Each::Range(a, nx, z) => crate::eval::range_vals(env, a, nx.as_ref(), z)?, Each::Vals(e) => env.eval(e)? };
                let count = if all.vec { all.v.len() } else { 1 };
                let mut f = Flow::Next;
                for i in 0..count {
                    let x = Val { v: vec![all.v[i]], im: all.im.get(i).map(|x| vec![*x]).unwrap_or_default(), dim: all.dim, abs: all.abs, nm: all.nm, ..Default::default() };
                    tick(env, steps, *at)?;
                    env.vars.insert(canon(n), x);
                    match exec(env, b, steps)? { Flow::Break => break, Flow::Return(v) => { f = Flow::Return(v); break; } Flow::Next => {} }
                }
                f
            }
            Line::Break => Flow::Break,
            Line::Return(e) => Flow::Return(env.eval(e)?),
        };
        if !matches!(f, Flow::Next) { return Ok(f); }
    }
    Ok(Flow::Next)
}
