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

/// x with A x = b, by Gaussian elimination with partial pivoting; None when A is singular (a pivot is
/// negligible beside the largest entry of its column).
pub fn linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    if a.len() != n || a.iter().any(|r| r.len() != n) { return None; }
    let col: Vec<f64> = (0..n).map(|k| a.iter().fold(0.0f64, |m, r| m.max(r[k].abs()))).collect();
    if !col.iter().all(|c| c.is_finite() && *c > 0.0) { return None; }
    for k in 0..n {
        let p = (k..n).max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))?;
        if a[p][k].abs() <= 1e-13 * col[k] { return None; }
        a.swap(k, p);
        b.swap(k, p);
        for i in k + 1..n {
            let f = a[i][k] / a[k][k];
            if f == 0.0 { continue; }
            let (top, rest) = a.split_at_mut(i);
            for (x, y) in rest[0][k..].iter_mut().zip(&top[k][k..]) { *x -= f * y; }
            b[i] -= f * b[k];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() { x[i] = (b[i] - (i + 1..n).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i]; }
    x.iter().all(|v| v.is_finite()).then_some(x)
}
