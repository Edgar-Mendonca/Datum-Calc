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

/// An LU factorisation with partial pivoting of A with each row scaled to its largest entry, so a pivot is
/// judged beside its column whatever the sizes of the rows (rows 1 m apart from rows 1 µm apart).
pub struct Lu { m: Vec<Vec<f64>>, s: Vec<f64>, p: Vec<usize>, sign: f64 }

impl Lu {
    /// None when A is singular: a pivot negligible beside the largest entry of its column.
    pub fn new(mut a: Vec<Vec<f64>>) -> Option<Lu> {
        let n = a.len();
        if a.iter().any(|r| r.len() != n || r.iter().any(|v| !v.is_finite())) { return None; }
        let s: Vec<f64> = a.iter().map(|r| r.iter().fold(0.0f64, |m, v| m.max(v.abs()))).collect();
        if s.contains(&0.0) { return None; }
        for (r, k) in a.iter_mut().zip(&s) { r.iter_mut().for_each(|v| *v /= k); }
        let col: Vec<f64> = (0..n).map(|k| a.iter().fold(0.0f64, |m, r| m.max(r[k].abs()))).collect();
        let (mut p, mut sign): (Vec<usize>, f64) = ((0..n).collect(), 1.0);
        for k in 0..n {
            let q = (k..n).max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))?;
            if a[q][k].abs() <= 1e-13 * col[k] { return None; }
            if q != k { a.swap(k, q); p.swap(k, q); sign = -sign; }
            for i in k + 1..n {
                let f = a[i][k] / a[k][k];
                a[i][k] = f;
                if f == 0.0 { continue; }
                let (top, rest) = a.split_at_mut(i);
                for (x, y) in rest[0][k + 1..].iter_mut().zip(&top[k][k + 1..]) { *x -= f * y; }
            }
        }
        Some(Lu { m: a, s, p, sign })
    }

    /// x with A x = b; None if it is not finite.
    pub fn solve(&self, b: &[f64]) -> Option<Vec<f64>> {
        let n = self.m.len();
        let mut x: Vec<f64> = self.p.iter().map(|&i| b[i] / self.s[i]).collect();
        for i in 0..n { for j in 0..i { x[i] -= self.m[i][j] * x[j]; } }
        for i in (0..n).rev() { for j in i + 1..n { x[i] -= self.m[i][j] * x[j]; } x[i] /= self.m[i][i]; }
        x.iter().all(|v| v.is_finite()).then_some(x)
    }

    /// The determinant: the pivots, a sign per row swap, and the row scales put back.
    pub fn det(&self) -> f64 { (0..self.m.len()).fold(self.sign, |d, i| d * self.m[i][i] * self.s[self.p[i]]) }
}

/// x with A x = b; None when A is singular.
pub fn linear(a: Vec<Vec<f64>>, b: Vec<f64>) -> Option<Vec<f64>> {
    if a.len() != b.len() { return None; }
    Lu::new(a)?.solve(&b)
}
