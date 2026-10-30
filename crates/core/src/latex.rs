//! LaTeX from the expression tree, so the maths shown is exactly what is computed: fractions for `/`,
//! roots, powers, subscripts after `_`, Greek letters by name, units in roman type.
use crate::eval::{canon, greek_name};
use crate::parse::{Node, Stmt, E};

/// A name: `sigma_allow` → σ_{allow}; one letter is italic, longer words upright.
pub fn name(n: &str) -> String {
    let c = canon(n);
    let mut parts = c.split('_');
    let base = parts.next().unwrap_or("");
    let subs: Vec<String> = parts.filter(|p| !p.is_empty()).map(word).collect();
    let b = word(base);
    if subs.is_empty() { b } else { format!("{b}_{{{}}}", subs.join(",")) }
}

fn word(w: &str) -> String {
    if let Some(g) = greek_name(w) { return format!("\\{g}"); }
    if w == "π" { return "\\pi".into(); }
    let esc = escape(w);
    if w.chars().count() == 1 { esc } else { format!("\\mathrm{{{esc}}}") }
}

fn escape(s: &str) -> String {
    s.chars().map(|c| match c { '\\' => "\\backslash ".into(), '{' | '}' | '_' | '#' | '$' | '%' | '&' => format!("\\{c}"), '^' => "\\hat{}".into(), '~' => "\\sim ".into(), '\'' => "'".into(), c => c.to_string() }).collect()
}

/// Plain text in the maths font (for lines that could not be read).
pub fn escape_text(s: &str) -> String { format!("\\text{{{}}}", s.replace('\\', "/").replace(['{', '}', '$', '#', '%', '&', '_', '^', '~'], " ")) }

/// A unit label (`kN·m`, `m³/h`, `W/(m²·K)`, `°C`) in roman type.
pub fn unit(label: &str) -> String {
    if label.is_empty() { return String::new(); }
    let mut s = String::new();
    for c in label.chars() {
        match c {
            '·' => s.push_str("\\cdot "),
            '°' => s.push_str("{^\\circ}"),
            'Ω' => s.push_str("\\Omega "),
            'µ' => s.push_str("\\mu "),
            'Δ' => s.push_str("\\Delta "),
            '%' => s.push_str("\\%"),
            '⁻' => s.push_str("^{-"),
            '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '⁰' => {
                let d = "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|x| x == c).unwrap_or(0);
                if s.ends_with("^{-") { s.push_str(&format!("{d}}}")); } else { s.push_str(&format!("^{{{d}}}")); }
            }
            '_' => s.push_str("\\_"),
            c => s.push(c),
        }
    }
    format!("\\mathrm{{{s}}}")
}

fn unit_of(u: &crate::units::UnitExpr) -> String { unit(&crate::units::pretty(&u.text)) }

const FUNCS: [&str; 26] = ["sin", "cos", "tan", "sinh", "cosh", "tanh", "ln", "log", "exp", "min", "max", "asin", "acos", "atan", "atan2", "abs", "round", "floor", "ceil", "sum", "mean", "len", "re", "im", "arg", "conj"];

fn prec(n: &Node) -> u8 {
    match &n.e { E::Bin(op, ..) => match *op { "<" | "<=" | ">" | ">=" | "==" | "!=" => 1, "+" | "-" => 3, "*" => 5, "/" => 9, _ => 7 }, E::Neg(_) => 4, _ => 9 }
}

/// Replaces a name by its value for the substituted line (None keeps the name).
pub type Sub<'a> = dyn Fn(&str) -> Option<String> + 'a;

/// LaTeX of an expression.
pub fn node(n: &Node) -> String { nd(n, &|_| None) }

/// LaTeX of an expression with each name replaced by its value: ρ·g·Q → 998 kg/m³ · 9.807 m/s² · …
pub fn node_with(n: &Node, sub: &Sub) -> String { nd(n, sub) }

fn nd(n: &Node, s: &Sub) -> String {
    match &n.e {
        E::Num(x, u) => {
            let t = if *x != 0.0 && !(1e-3..1e6).contains(&x.abs()) { let s = format!("{x:e}"); let (m, e) = s.split_once('e').unwrap_or((&s, "0")); format!("{m} \\times 10^{{{e}}}") } else { format!("{x}") };
            match u { Some(u) => format!("{t}\\,{}", unit_of(u)), None => t }
        }
        E::Im(c, e) => match &e.e {
            E::Num(x, u) => format!("{}\\,\\mathrm{{{c}}}{}", nd(&Node { e: E::Num(*x, None), at: e.at }, s), u.as_ref().map(|u| format!("\\,{}", unit_of(u))).unwrap_or_default()),
            _ => format!("{}\\,\\mathrm{{{c}}}", nd(e, s)),
        },
        E::Unit(e, u) => format!("{}\\,{}", nd(e, s), unit_of(u)),
        E::Var(v) => s(v).map(|t| if t.starts_with('-') { format!("\\left({t}\\right)") } else { t }).unwrap_or_else(|| name(v)),
        E::Group(e) => format!("\\left({}\\right)", nd(e, s)),
        E::Neg(e) => if prec(e) < 5 { format!("-\\left({}\\right)", nd(e, s)) } else { format!("-{}", nd(e, s)) },
        E::Bin(op, a, b) => {
            let wrap = |x: &Node, p: u8| if prec(x) < p && !matches!(x.e, E::Group(_)) { format!("\\left({}\\right)", nd(x, s)) } else { nd(x, s) };
            match *op {
                "/" => format!("\\frac{{{}}}{{{}}}", strip(a, s), strip(b, s)),
                "^" => {
                    // A substituted value is bracketed under a power; a symbol put in its place (j) is not.
                    let subbed = matches!(&a.e, E::Var(v) if s(v).is_some_and(|t| !t.starts_with("\\mathrm{")));
                    let t = nd(a, s);
                    let base = if (!subbed && matches!(a.e, E::Var(_) | E::Num(_, None) | E::Group(_) | E::Call(..))) || t.starts_with("\\left(") { t } else { format!("\\left({t}\\right)") };
                    format!("{{{base}}}^{{{}}}", strip(b, s))
                }
                "*" => {
                    // A number times a name reads better without the dot only for 2πr-like terms; keep the dot otherwise.
                    format!("{} \\cdot {}", wrap(a, 5), wrap(b, 5))
                }
                "+" => format!("{} + {}", wrap(a, 3), wrap(b, 3)),
                "-" => format!("{} - {}", wrap(a, 3), if prec(b) <= 3 && !matches!(b.e, E::Group(_)) { format!("\\left({}\\right)", nd(b, s)) } else { nd(b, s) }),
                o => format!("{} {} {}", nd(a, s), match o { "<" => "<", "<=" => "\\le", ">" => ">", ">=" => "\\ge", "==" => "=", _ => "\\ne" }, nd(b, s)),
            }
        }
        E::Call(f, args) => {
            let a: Vec<String> = args.iter().map(|x| strip(x, s)).collect();
            match f.as_str() {
                "sqrt" if a.len() == 1 => format!("\\sqrt{{{}}}", a[0]),
                "cbrt" if a.len() == 1 => format!("\\sqrt[3]{{{}}}", a[0]),
                "abs" if a.len() == 1 => format!("\\left|{}\\right|", a[0]),
                "conj" if a.len() == 1 => format!("\\overline{{{}}}", a[0]),
                "polar" if a.len() == 2 => format!("{}\\angle {}", a[0], a[1]),
                "exp" if a.len() == 1 => format!("e^{{{}}}", a[0]),
                "if" if a.len() == 3 => format!("\\begin{{cases}} {} & \\text{{if }} {} \\\\ {} & \\text{{otherwise}} \\end{{cases}}", a[1], a[0], a[2]),
                "log" if a.len() == 1 => format!("\\log_{{10}}\\left({}\\right)", a[0]),
                f if FUNCS.contains(&f) => format!("\\{}\\left({}\\right)", if matches!(f, "atan2" | "round" | "floor" | "ceil" | "sum" | "mean" | "len" | "asin" | "acos" | "atan" | "abs" | "re" | "im" | "arg" | "conj") { format!("operatorname{{{f}}}") } else { f.to_string() }, a.join(", ")),
                f => format!("{}\\left({}\\right)", name(f), a.join(", ")),
            }
        }
    }
}

/// Without the outer brackets the user typed (a fraction or power already groups it).
fn strip(n: &Node, s: &Sub) -> String { match &n.e { E::Group(e) => nd(e, s), _ => nd(n, s) } }

/// LaTeX of a whole statement.
pub fn stmt(s: &Stmt) -> String { stmt_with(s, &|_| None) }

/// LaTeX of a whole statement with some names drawn differently (the imaginary unit j upright).
pub fn stmt_with(s: &Stmt, sub: &Sub) -> String {
    let node = |e: &Node| nd(e, sub);
    match s {
        Stmt::Assign { name: n, e, .. } => format!("{} = {}", name(n), node(e)),
        Stmt::Func { name: n, params, e } => format!("{}\\left({}\\right) = {}", name(n), params.iter().map(|p| name(p)).collect::<Vec<_>>().join(", "), node(e)),
        Stmt::Range { name: n, start, next, end, .. } => format!("{} = {}{} \\ldots {}", name(n), node(start), next.as_ref().map(|x| format!(", {}", node(x))).unwrap_or_default(), node(end)),
        Stmt::Check { e } => node(e),
        Stmt::Solve { lhs, rhs, var, lo, hi } => format!("{} = {} \\quad \\text{{for }} {} \\in \\left[{}, {}\\right]", node(lhs), node(rhs), name(var), node(lo), node(hi)),
        Stmt::Plot { ys, var, lo, hi } => format!("{} \\quad \\text{{for }} {} \\in \\left[{}, {}\\right]", ys.iter().map(&node).collect::<Vec<_>>().join(",\\; "), name(var), node(lo), node(hi)),
        Stmt::Show { e } => node(e),
        Stmt::Empty => String::new(),
    }
}

/// Plain text of an expression (plot legends): the typed form with Greek letters.
pub fn plain(n: &Node) -> String {
    match &n.e {
        E::Num(x, u) => match u { Some(u) => format!("{x} {}", crate::units::pretty(&u.text)), None => format!("{x}") },
        E::Im(c, e) => match &e.e { E::Num(x, Some(u)) => format!("{x}{c} {}", crate::units::pretty(&u.text)), E::Num(x, None) => format!("{x}{c}"), _ => plain(e) },
        E::Unit(e, u) => format!("{} {}", plain(e), crate::units::pretty(&u.text)),
        E::Var(v) => canon(v),
        E::Group(e) => format!("({})", plain(e)),
        E::Neg(e) => format!("−{}", plain(e)),
        E::Bin(op, a, b) => format!("{}{}{}", plain(a), match *op { "*" => "·", "-" => " − ", "+" => " + ", "/" => "/", "^" => "^", o => o }, plain(b)),
        E::Call(f, args) => format!("{}({})", canon(f), args.iter().map(plain).collect::<Vec<_>>().join(", ")),
    }
}
