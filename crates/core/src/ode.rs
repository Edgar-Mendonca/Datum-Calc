//! Ordinary differential equations (0.2.8): the Dormand–Prince 5(4) method with step-size control, and a
//! solution that can be read at any time in its range (cubic Hermite between the steps, from values and slopes).

/// A solved system on a sheet: the path, each state's units and whether it is an absolute temperature, and the
/// units of the variable (time).
#[derive(Debug, Clone)]
pub struct Solved { pub path: Path, pub dims: Vec<crate::units::Dim>, pub abs: Vec<bool>, pub tdim: crate::units::Dim, pub tabs: bool }

/// A solution: the steps' times, the states and their derivatives at each step (states in SI).
#[derive(Debug, Clone, Default)]
pub struct Path { pub ts: Vec<f64>, pub ys: Vec<Vec<f64>>, pub fs: Vec<Vec<f64>> }

/// The equations: the derivatives of the states at a time.
pub type Rhs<'a, E> = dyn FnMut(f64, &[f64]) -> Result<Vec<f64>, E> + 'a;

/// How many steps a solution may take before it is called too fast to follow (a stiff problem).
pub const MAX_STEPS: usize = 50_000;

/// Integrates y' = f(t, y) from t0 to t1 (t1 > t0). `scale` is each state's size, for its absolute tolerance;
/// the relative tolerance is 10⁻⁹. `Err(None)` means too many steps; `Err(Some(e))` is f's own error.
pub fn solve<E>(f: &mut Rhs<E>, t0: f64, t1: f64, y0: Vec<f64>, scale: &[f64]) -> Result<Path, Option<E>> {
    const C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
    const A: [[f64; 6]; 7] = [
        [0.0; 6],
        [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
        [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
        [19372.0 / 6561.0, -25360.0 / 2187.0, 64448.0 / 6561.0, -212.0 / 729.0, 0.0, 0.0],
        [9017.0 / 3168.0, -355.0 / 33.0, 46732.0 / 5247.0, 49.0 / 176.0, -5103.0 / 18656.0, 0.0],
        [35.0 / 384.0, 0.0, 500.0 / 1113.0, 125.0 / 192.0, -2187.0 / 6784.0, 11.0 / 84.0],
    ];
    // The difference between the fifth- and fourth-order weights: the local error estimate.
    const E: [f64; 7] = [71.0 / 57600.0, 0.0, -71.0 / 16695.0, 71.0 / 1920.0, -17253.0 / 339200.0, 22.0 / 525.0, -1.0 / 40.0];
    let n = y0.len();
    let rtol = 1e-9;
    let atol: Vec<f64> = scale.iter().map(|s| 1e-12 * s.max(1e-300)).collect();
    let span = t1 - t0;
    let mut path = Path::default();
    let (mut t, mut y) = (t0, y0);
    let mut k0 = f(t, &y).map_err(Some)?;
    path.ts.push(t);
    path.ys.push(y.clone());
    path.fs.push(k0.clone());
    let mut h = span * 1e-4;
    let mut steps = 0;
    while t < t1 {
        steps += 1;
        if steps > MAX_STEPS { return Err(None); }
        if t + h > t1 { h = t1 - t; }
        // The stages, the fifth-order step and the error estimate.
        let mut k = vec![k0.clone()];
        for s in 1..7 {
            let ys: Vec<f64> = (0..n).map(|i| y[i] + h * (0..s).map(|j| A[s][j] * k[j][i]).sum::<f64>()).collect();
            k.push(f(t + C[s] * h, &ys).map_err(Some)?);
        }
        let yn: Vec<f64> = (0..n).map(|i| y[i] + h * (0..6).map(|j| A[6][j] * k[j][i]).sum::<f64>()).collect();
        let err = ((0..n).map(|i| {
            let e = h * (0..7).map(|j| E[j] * k[j][i]).sum::<f64>();
            (e / (atol[i] + rtol * y[i].abs().max(yn[i].abs()))).powi(2)
        }).sum::<f64>() / n as f64).sqrt();
        if !err.is_finite() || yn.iter().any(|v| !v.is_finite()) {
            h *= 0.25;
            if h <= span * 1e-14 { return Err(None); }
            continue;
        }
        if err <= 1.0 {
            t = if (t1 - (t + h)).abs() <= 1e-14 * span.abs() { t1 } else { t + h };
            y = yn;
            k0 = k[6].clone();
            path.ts.push(t);
            path.ys.push(y.clone());
            path.fs.push(k0.clone());
        }
        h *= (0.9 * err.max(1e-10).powf(-0.2)).clamp(0.2, 5.0);
        if h <= span * 1e-14 { return Err(None); }
    }
    Ok(path)
}

impl Path {
    /// State i at time t (inside the range): cubic Hermite between the two steps around it.
    pub fn at(&self, i: usize, t: f64) -> f64 {
        let k = match self.ts.binary_search_by(|x| x.total_cmp(&t)) { Ok(k) => return self.ys[k][i], Err(k) => k.clamp(1, self.ts.len() - 1) };
        let (t0, t1) = (self.ts[k - 1], self.ts[k]);
        let h = t1 - t0;
        let s = (t - t0) / h;
        let (y0, y1, d0, d1) = (self.ys[k - 1][i], self.ys[k][i], self.fs[k - 1][i] * h, self.fs[k][i] * h);
        let (s2, s3) = (s * s, s * s * s);
        (2.0 * s3 - 3.0 * s2 + 1.0) * y0 + (s3 - 2.0 * s2 + s) * d0 + (-2.0 * s3 + 3.0 * s2) * y1 + (s3 - s2) * d1
    }

    /// The rate of state i at time t: the slope of the same cubic, which is the solver's own derivative at each step.
    pub fn slope(&self, i: usize, t: f64) -> f64 {
        let k = match self.ts.binary_search_by(|x| x.total_cmp(&t)) { Ok(k) => return self.fs[k][i], Err(k) => k.clamp(1, self.ts.len() - 1) };
        let (t0, t1) = (self.ts[k - 1], self.ts[k]);
        let h = t1 - t0;
        let s = (t - t0) / h;
        let (y0, y1, d0, d1) = (self.ys[k - 1][i], self.ys[k][i], self.fs[k - 1][i] * h, self.fs[k][i] * h);
        ((6.0 * s * s - 6.0 * s) * (y0 - y1) + (3.0 * s * s - 4.0 * s + 1.0) * d0 + (3.0 * s * s - 2.0 * s) * d1) / h
    }
}
