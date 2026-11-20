//! Word equations (0.2.10): an expression as Office Math (OMML), the way latex.rs writes it as LaTeX, so an
//! exported calc sheet has equations Word can edit.
use crate::eval::canon;
use crate::parse::{Node, Stmt, E};

/// Text for XML.
pub fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }

/// A run of maths (italic for one letter, as Word sets variables).
fn r(t: &str) -> String { format!("<m:r><m:t xml:space=\"preserve\">{}</m:t></m:r>", esc(t)) }
/// A run set upright as normal text: words, units, function names (normal text also keeps a unit such as bar from
/// being read as a command by other editors).
pub fn rp(t: &str) -> String { format!("<m:r><m:rPr><m:nor/><m:sty m:val=\"p\"/></m:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>", esc(t)) }
fn sub(b: &str, s: &str) -> String { format!("<m:sSub><m:e>{b}</m:e><m:sub>{s}</m:sub></m:sSub>") }
fn sup(b: &str, s: &str) -> String { format!("<m:sSup><m:e>{b}</m:e><m:sup>{s}</m:sup></m:sSup>") }
fn frac(a: &str, b: &str) -> String { format!("<m:f><m:num>{a}</m:num><m:den>{b}</m:den></m:f>") }
fn paren(x: &str) -> String { format!("<m:d><m:e>{x}</m:e></m:d>") }
fn delim(o: &str, c: &str, x: &str) -> String { format!("<m:d><m:dPr><m:begChr m:val=\"{o}\"/><m:endChr m:val=\"{c}\"/></m:dPr><m:e>{x}</m:e></m:d>") }
fn func(name: &str, arg: &str) -> String { format!("<m:func><m:fName>{}</m:fName><m:e>{}</m:e></m:func>", rp(name), paren(arg)) }

/// A name as written on the sheet: ρ_steel → ρ with steel below it, x' → x′; words upright.
pub fn name(n: &str) -> String {
    let core = n.trim_end_matches('\'');
    let primes = "′".repeat(n.len() - core.len());
    let c = canon(if core.is_empty() { n } else { core });
    let mut parts = c.split('_');
    let word = |w: &str| if w.chars().count() == 1 { r(w) } else { rp(w) };
    let base = word(parts.next().unwrap_or(""));
    let subs: Vec<String> = parts.filter(|p| !p.is_empty()).map(word).collect();
    let b = if subs.is_empty() { base } else { sub(&base, &subs.join(&rp(","))) };
    if primes.is_empty() { b } else { format!("{b}{}", r(&primes)) }
}

/// A unit label (kN·m, m³/h) upright after a thin space.
pub fn unit(label: &str) -> String { rp(&format!("\u{2009}{label}")) }

fn num(x: f64) -> String {
    if x != 0.0 && !(1e-3..1e6).contains(&x.abs()) {
        let s = format!("{x:e}");
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        format!("{}{}", r(&format!("{m}×")), sup(&r("10"), &r(&e.replace('-', "−"))))
    } else { r(&format!("{x}").replace('-', "−")) }
}

fn prec(n: &Node) -> u8 {
    match &n.e { E::Bin(op, ..) => match *op { "<" | "<=" | ">" | ">=" | "==" | "!=" => 1, "+" | "-" => 3, "*" => 5, "/" => 9, _ => 7 }, E::Neg(_) => 4, _ => 9 }
}

/// An expression without its outer brackets (inside a fraction they are not written).
fn strip(n: &Node) -> String { match &n.e { E::Group(x) => node(x), _ => node(n) } }

/// An expression in OMML.
pub fn node(n: &Node) -> String {
    match &n.e {
        E::Num(x, u) => format!("{}{}", num(*x), u.as_ref().map(|u| unit(&crate::units::pretty(&u.text))).unwrap_or_default()),
        E::Im(c, e) => match &e.e {
            E::Num(x, u) => format!("{}{}{}", num(*x), rp(&c.to_string()), u.as_ref().map(|u| unit(&crate::units::pretty(&u.text))).unwrap_or_default()),
            _ => format!("{}{}", node(e), rp(&c.to_string())),
        },
        E::Unit(e, u) => format!("{}{}", node(e), unit(&crate::units::pretty(&u.text))),
        E::Mat(matrix, rows) => {
            let mr = |cells: &[Node]| format!("<m:mr>{}</m:mr>", cells.iter().map(|x| format!("<m:e>{}</m:e>", strip(x))).collect::<String>());
            let body = if *matrix { rows.iter().map(|r| mr(r)).collect::<String>() } else { rows[0].iter().map(|x| mr(std::slice::from_ref(x))).collect::<String>() };
            delim("[", "]", &format!("<m:m>{body}</m:m>"))
        }
        E::Index(a, idx) => sub(&node(a), &idx.iter().map(strip).collect::<Vec<_>>().join(&rp(","))),
        E::Var(v) => name(v),
        E::Group(e) => paren(&node(e)),
        E::Neg(e) => format!("{}{}", r("−"), if prec(e) < 5 { paren(&node(e)) } else { node(e) }),
        E::Bin(op, a, b) => {
            let wrap = |x: &Node, p: u8| if prec(x) < p && !matches!(x.e, E::Group(_)) { paren(&node(x)) } else { node(x) };
            match *op {
                "/" => frac(&strip(a), &strip(b)),
                "^" => sup(&if matches!(a.e, E::Var(_) | E::Num(_, None) | E::Group(_) | E::Call(..)) { node(a) } else { paren(&node(a)) }, &strip(b)),
                "*" => format!("{}{}{}", wrap(a, 5), r("·"), wrap(b, 5)),
                "+" => format!("{}{}{}", wrap(a, 3), r("+"), wrap(b, 3)),
                "-" => format!("{}{}{}", wrap(a, 3), r("−"), if prec(b) <= 3 && !matches!(b.e, E::Group(_)) { paren(&node(b)) } else { node(b) }),
                "∠" => format!("{}{}{}", wrap(a, 7), r("∠"), wrap(b, 7)),
                o => format!("{}{}{}", node(a), r(match o { "<" => "<", "<=" => "≤", ">" => ">", ">=" => "≥", "==" => "=", _ => "≠" }), node(b)),
            }
        }
        E::Call(f, args) => {
            let a: Vec<String> = args.iter().map(strip).collect();
            let list = a.join(&r(", "));
            match f.as_str() {
                "sqrt" if a.len() == 1 => format!("<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/><m:e>{}</m:e></m:rad>", a[0]),
                "cbrt" if a.len() == 1 => format!("<m:rad><m:deg>{}</m:deg><m:e>{}</m:e></m:rad>", r("3"), a[0]),
                "abs" if a.len() == 1 => delim("|", "|", &a[0]),
                "exp" if a.len() == 1 => sup(&r("e"), &a[0]),
                "log" if a.len() == 1 => format!("<m:func><m:fName>{}</m:fName><m:e>{}</m:e></m:func>", sub(&rp("log"), &r("10")), paren(&a[0])),
                f if (f.starts_with("water_") || f.starts_with("gas_")) && crate::eval::builtin_name(f) => fluid(f, args, &a),
                f if f.chars().count() > 1 && (crate::eval::builtin_name(f) || matches!(f, "sin" | "cos" | "tan" | "ln")) => func(f, &list),
                f => format!("{}{}", name(f), paren(&list)),
            }
        }
    }
}

/// A property function as a symbol with the fluid below, as on the sheet: h_water(T, p), c_p,CO₂(T).
fn fluid(f: &str, args: &[Node], a: &[String]) -> String {
    let (kind, prop) = f.split_once('_').unwrap_or(("", f));
    let (sym, inner) = match prop {
        "rho" => ("ρ", ""), "mu" => ("μ", ""), "nu" => ("ν", ""), "sigma" => ("σ", ""), "gamma" => ("γ", ""), "cp" => ("c", "p"), "cv" => ("c", "v"),
        "psat" => ("p", "sat"), "Tsat" => ("T", "sat"), "Pr" => ("Pr", ""), p => (p, ""),
    };
    let (who, rest) = if kind == "gas" {
        let g = match args.first().map(|x| &x.e) { Some(E::Var(g)) => crate::fluids::gas(g).map(|g| g.id.to_string()).unwrap_or_else(|| g.clone()), _ => String::new() };
        (g.chars().map(|c| match c { '2' => '₂', '4' => '₄', c => c }).collect::<String>(), &a[1.min(a.len())..])
    } else { ("water".to_string(), a) };
    let base = if sym.chars().count() == 1 { r(sym) } else { rp(sym) };
    let low = if inner.is_empty() { rp(&who) } else { format!("{}{}", rp(inner), rp(&format!(",{who}"))) };
    let s = sub(&base, &low);
    if rest.is_empty() { s } else { format!("{s}{}", paren(&rest.join(&r(", ")))) }
}

/// A line as an equation, when it is one that reads as an equation (an assignment, a function, a check, a value);
/// None for the others (solve, plot, fit, ode …), which an export writes as their text.
pub fn stmt(st: &Stmt) -> Option<String> {
    Some(match st {
        Stmt::Assign { name: n, e, .. } => format!("{}{}{}", name(n), r("="), node(e)),
        Stmt::Func { name: n, params, e } => format!("{}{}{}{}", name(n), paren(&params.iter().map(|p| name(p)).collect::<Vec<_>>().join(&r(", "))), r("="), node(e)),
        Stmt::Range { name: n, start, next, end, .. } => format!("{}{}{}{}{}{}", name(n), r("="), node(start), next.as_ref().map(|x| format!("{}{}", r(", "), node(x))).unwrap_or_default(), r(" … "), node(end)),
        Stmt::Show { e } | Stmt::Check { e } => node(e),
        _ => return None,
    })
}
