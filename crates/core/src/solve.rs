//! Root finding: Brent's method in a range, after a scan for a sign change.

/// The x in [a, b] where f(x) = 0. If f has the same sign at both ends, the range is scanned in
/// 200 steps for a change of sign first. Points where f cannot be worked out are skipped.
pub fn root(f: &mut dyn FnMut(f64) -> f64, a: f64, b: f64) -> Result<f64, String> {
    if !(a.is_finite() && b.is_finite()) || a == b { return Err("The range is empty".into()); }
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    let n = 200;
    let mut prev: Option<(f64, f64)> = None;
    let mut bracket = None;
    let mut best: Option<(f64, f64)> = None;
    for i in 0..=n {
        let x = lo + (hi - lo) * i as f64 / n as f64;
        let y = f(x);
        if !y.is_finite() { continue; }
        if y == 0.0 { return Ok(x); }
        if best.is_none_or(|b| y.abs() < b.1.abs()) { best = Some((x, y)); }
        if let Some((px, py)) = prev { if py.signum() != y.signum() { bracket = Some((px, x, py, y)); break; } }
        prev = Some((x, y));
    }
    let Some((mut a, mut b, mut fa, mut fb)) = bracket else {
        return Err(if best.is_none() { "The equation cannot be worked out anywhere".into() } else { "No root".into() });
    };
    // Brent's method (Brent 1973, "Algorithms for Minimization without Derivatives", ch. 4).
    if fa.abs() < fb.abs() { std::mem::swap(&mut a, &mut b); std::mem::swap(&mut fa, &mut fb); }
    let (mut c, mut fc, mut d, mut mflag) = (a, fa, a, true);
    for _ in 0..200 {
        if fb == 0.0 || (b - a).abs() <= 1e-14 * b.abs().max(1e-300) { return Ok(b); }
        let mut s = if fa != fc && fb != fc {
            a * fb * fc / ((fa - fb) * (fa - fc)) + b * fa * fc / ((fb - fa) * (fb - fc)) + c * fa * fb / ((fc - fa) * (fc - fb))
        } else { b - fb * (b - a) / (fb - fa) };
        let q = (3.0 * a + b) / 4.0;
        let out = !((s > q.min(b)) && (s < q.max(b)));
        if out || (mflag && (s - b).abs() >= (b - c).abs() / 2.0) || (!mflag && (s - b).abs() >= (c - d).abs() / 2.0)
            || (mflag && (b - c).abs() < 1e-15) || (!mflag && (c - d).abs() < 1e-15) {
            s = (a + b) / 2.0;
            mflag = true;
        } else { mflag = false; }
        let mut fs = f(s);
        if !fs.is_finite() { s = (a + b) / 2.0; fs = f(s); if !fs.is_finite() { return Err("The equation cannot be worked out near the root".into()); } }
        d = c;
        c = b;
        fc = fb;
        if fa.signum() != fs.signum() { b = s; fb = fs; } else { a = s; fa = fs; }
        if fa.abs() < fb.abs() { std::mem::swap(&mut a, &mut b); std::mem::swap(&mut fa, &mut fb); }
    }
    Ok(b)
}
