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

const FUNCS: [&str; 22] = ["sin", "cos", "tan", "sinh", "cosh", "tanh", "ln", "log", "exp", "min", "max", "asin", "acos", "atan", "atan2", "abs", "round", "floor", "ceil", "sum", "mean", "len"];

fn prec(n: &Node) -> u8 {
    match &n.e { E::Bin(op, ..) => match *op { "<" | "<=" | ">" | ">=" | "==" | "!=" => 1, "+" | "-" => 3, "*" => 5, "/" => 9, _ => 7 }, E::Neg(_) => 4, _ => 9 }
}

/// LaTeX of an expression.
pub fn node(n: &Node) -> String {
    match &n.e {
        E::Num(x, u) => {
            let t = if *x != 0.0 && !(1e-3..1e6).contains(&x.abs()) { let s = format!("{x:e}"); let (m, e) = s.split_once('e').unwrap_or((&s, "0")); format!("{m} \\times 10^{{{e}}}") } else { format!("{x}") };
            match u { Some(u) => format!("{t}\\,{}", unit_of(u)), None => t }
        }
        E::Var(v) => name(v),
        E::Group(e) => format!("\\left({}\\right)", node(e)),
        E::Neg(e) => if prec(e) < 5 { format!("-\\left({}\\right)", node(e)) } else { format!("-{}", node(e)) },
        E::Bin(op, a, b) => {
            let wrap = |x: &Node, p: u8| if prec(x) < p && !matches!(x.e, E::Group(_)) { format!("\\left({}\\right)", node(x)) } else { node(x) };
            match *op {
                "/" => format!("\\frac{{{}}}{{{}}}", strip(a), strip(b)),
                "^" => {
                    let base = if matches!(a.e, E::Var(_) | E::Num(_, None) | E::Group(_) | E::Call(..)) { node(a) } else { format!("\\left({}\\right)", node(a)) };
                    format!("{{{base}}}^{{{}}}", strip(b))
                }
                "*" => {
                    // A number times a name reads better without the dot only for 2πr-like terms; keep the dot otherwise.
                    format!("{} \\cdot {}", wrap(a, 5), wrap(b, 5))
                }
                "+" => format!("{} + {}", wrap(a, 3), wrap(b, 3)),
                "-" => format!("{} - {}", wrap(a, 3), if prec(b) <= 3 && !matches!(b.e, E::Group(_)) { format!("\\left({}\\right)", node(b)) } else { node(b) }),
                o => format!("{} {} {}", node(a), match o { "<" => "<", "<=" => "\\le", ">" => ">", ">=" => "\\ge", "==" => "=", _ => "\\ne" }, node(b)),
            }
        }
        E::Call(f, args) => {
            let a: Vec<String> = args.iter().map(strip).collect();
            match f.as_str() {
                "sqrt" if a.len() == 1 => format!("\\sqrt{{{}}}", a[0]),
                "cbrt" if a.len() == 1 => format!("\\sqrt[3]{{{}}}", a[0]),
                "abs" if a.len() == 1 => format!("\\left|{}\\right|", a[0]),
                "exp" if a.len() == 1 => format!("e^{{{}}}", a[0]),
                "if" if a.len() == 3 => format!("\\begin{{cases}} {} & \\text{{if }} {} \\\\ {} & \\text{{otherwise}} \\end{{cases}}", a[1], a[0], a[2]),
                "log" if a.len() == 1 => format!("\\log_{{10}}\\left({}\\right)", a[0]),
                f if FUNCS.contains(&f) => format!("\\{}\\left({}\\right)", if matches!(f, "atan2" | "round" | "floor" | "ceil" | "sum" | "mean" | "len" | "asin" | "acos" | "atan" | "abs") { format!("operatorname{{{f}}}") } else { f.to_string() }, a.join(", ")),
                f => format!("{}\\left({}\\right)", name(f), a.join(", ")),
            }
        }
    }
}

/// Without the outer brackets the user typed (a fraction or power already groups it).
fn strip(n: &Node) -> String { match &n.e { E::Group(e) => node(e), _ => node(n) } }

/// LaTeX of a whole statement.
pub fn stmt(s: &Stmt) -> String {
    match s {
        Stmt::Assign { name: n, e, .. } => format!("{} = {}", name(n), node(e)),
        Stmt::Func { name: n, params, e } => format!("{}\\left({}\\right) = {}", name(n), params.iter().map(|p| name(p)).collect::<Vec<_>>().join(", "), node(e)),
        Stmt::Range { name: n, start, next, end, .. } => format!("{} = {}{} \\ldots {}", name(n), node(start), next.as_ref().map(|x| format!(", {}", node(x))).unwrap_or_default(), node(end)),
        Stmt::Check { e } => node(e),
        Stmt::Solve { lhs, rhs, var, lo, hi } => format!("{} = {} \\quad \\text{{for }} {} \\in \\left[{}, {}\\right]", node(lhs), node(rhs), name(var), node(lo), node(hi)),
        Stmt::Plot { ys, var, lo, hi } => format!("{} \\quad \\text{{for }} {} \\in \\left[{}, {}\\right]", ys.iter().map(node).collect::<Vec<_>>().join(",\\; "), name(var), node(lo), node(hi)),
        Stmt::Show { e } => node(e),
        Stmt::Empty => String::new(),
    }
}

/// Plain text of an expression (plot legends): the typed form with Greek letters.
pub fn plain(n: &Node) -> String {
    match &n.e {
        E::Num(x, u) => match u { Some(u) => format!("{x} {}", crate::units::pretty(&u.text)), None => format!("{x}") },
        E::Var(v) => canon(v),
        E::Group(e) => format!("({})", plain(e)),
        E::Neg(e) => format!("−{}", plain(e)),
        E::Bin(op, a, b) => format!("{}{}{}", plain(a), match *op { "*" => "·", "-" => " − ", "+" => " + ", "/" => "/", "^" => "^", o => o }, plain(b)),
        E::Call(f, args) => format!("{}({})", canon(f), args.iter().map(plain).collect::<Vec<_>>().join(", ")),
    }
}
