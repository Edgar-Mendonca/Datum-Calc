//! Ordinary differential equations (0.2.8): the Dormand–Prince 5(4) method with step-size control, and a
//! solution that can be read at any time in its range (the method's own fourth-order dense output).

/// A solved system on a sheet: the path, each state's units and whether it is an absolute temperature, and the
/// units of the variable (time).
#[derive(Debug, Clone)]
pub struct Solved { pub path: Path, pub dims: Vec<crate::units::Dim>, pub abs: Vec<bool>, pub tdim: crate::units::Dim, pub tabs: bool }

/// A solution: the steps' times, the states and their derivatives at each step (states in SI), and for each step
/// the fifth coefficient of its dense output (the others follow from the values and derivatives at its ends).
#[derive(Debug, Clone, Default)]
pub struct Path { pub ts: Vec<f64>, pub ys: Vec<Vec<f64>>, pub fs: Vec<Vec<f64>>, pub dense: Vec<Vec<f64>> }

/// The equations: the derivatives of the states at a time.
pub type Rhs<'a, E> = dyn FnMut(f64, &[f64]) -> Result<Vec<f64>, E> + 'a;

/// How many steps a solution may take, and how much work (equations worked out × states), before it is called
/// too fast to follow (a stiff problem, or one that blows up).
pub const MAX_STEPS: usize = 50_000;
pub const MAX_WORK: usize = 3_000_000;

/// Why a solution stopped: at time t, with the equations' own error, or None when it was too fast to follow.
pub type Stop<E> = (f64, Option<E>);

/// Integrates y' = f(t, y) from t0 to t1 (t1 > t0). `scale` is each state's size, for its absolute tolerance;
/// the relative tolerance is 10⁻⁹. A step whose trial stages cannot be worked out (a square root of a value that
/// overshot below zero) is tried again shorter; only at the shortest step is the equations' error given back.
pub fn solve<E>(f: &mut Rhs<E>, t0: f64, t1: f64, y0: Vec<f64>, scale: &[f64]) -> Result<Path, Stop<E>> {
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
    // The dense output's fifth coefficient (Hairer, Nørsett and Wanner, Solving ODEs I, §II.6).
    const D: [f64; 7] = [-12715105075.0 / 11282082432.0, 0.0, 87487479700.0 / 32700410799.0, -10690763975.0 / 1880347072.0, 701980252875.0 / 199316789632.0, -1453857185.0 / 822651844.0, 69997945.0 / 29380423.0];
    let n = y0.len();
    let rtol = 1e-9;
    let atol: Vec<f64> = scale.iter().map(|s| 1e-12 * s.max(1e-300)).collect();
    let span = t1 - t0;
    let hmin = span * 1e-13;
    let mut path = Path::default();
    let (mut t, mut y) = (t0, y0);
    let mut work = 0usize;
    let mut k0 = f(t, &y).map_err(|e| (t, Some(e)))?;
    path.ts.push(t);
    path.ys.push(y.clone());
    path.fs.push(k0.clone());
    let mut h = span * 1e-4;
    let mut steps = 0;
    while t < t1 {
        steps += 1;
        work += 6 * n.max(1);
        if steps > MAX_STEPS || work > MAX_WORK { return Err((t, None)); }
        if t + h > t1 { h = t1 - t; }
        // The stages; one that cannot be worked out shortens the step.
        let mut k = vec![k0.clone()];
        let mut failed = None;
        for s in 1..7 {
            let ys: Vec<f64> = (0..n).map(|i| y[i] + h * (0..s).map(|j| A[s][j] * k[j][i]).sum::<f64>()).collect();
            match f(t + C[s] * h, &ys) { Ok(v) => k.push(v), Err(e) => { failed = Some(e); break; } }
        }
        if let Some(e) = failed {
            h *= 0.25;
            if h <= hmin { return Err((t, Some(e))); }
            continue;
        }
        let yn: Vec<f64> = (0..n).map(|i| y[i] + h * (0..6).map(|j| A[6][j] * k[j][i]).sum::<f64>()).collect();
        let err = ((0..n).map(|i| {
            let e = h * (0..7).map(|j| E[j] * k[j][i]).sum::<f64>();
            (e / (atol[i] + rtol * y[i].abs().max(yn[i].abs()))).powi(2)
        }).sum::<f64>() / n.max(1) as f64).sqrt();
        if !err.is_finite() || yn.iter().any(|v| !v.is_finite()) || k[6].iter().any(|v| !v.is_finite()) {
            h *= 0.25;
            if h <= hmin { return Err((t, None)); }
            continue;
        }
        if err <= 1.0 {
            path.dense.push((0..n).map(|i| h * (0..7).map(|j| D[j] * k[j][i]).sum::<f64>()).collect());
            t = if (t1 - (t + h)).abs() <= 1e-14 * span.abs() { t1 } else { t + h };
            y = yn;
            k0 = k[6].clone();
            path.ts.push(t);
            path.ys.push(y.clone());
            path.fs.push(k0.clone());
        }
        h *= (0.9 * err.max(1e-10).powf(-0.2)).clamp(0.2, 5.0);
        if h <= hmin { return Err((t, None)); }
    }
    Ok(path)
}

impl Path {
    /// The step around t and where t falls in it (0 at its start, 1 at its end).
    fn locate(&self, t: f64) -> (usize, f64, f64) {
        let k = match self.ts.binary_search_by(|x| x.total_cmp(&t)) { Ok(k) => k.max(1), Err(k) => k }.clamp(1, self.ts.len().max(2) - 1);
        let h = self.ts[k] - self.ts[k - 1];
        (k, if h > 0.0 { (t - self.ts[k - 1]) / h } else { 0.0 }, h)
    }

    /// State i at time t (inside the range), by the method's dense output.
    pub fn at(&self, i: usize, t: f64) -> f64 {
        if self.ts.len() < 2 { return self.ys[0][i]; }
        let (k, s, h) = self.locate(t);
        let (y0, y1) = (self.ys[k - 1][i], self.ys[k][i]);
        let r2 = y1 - y0;
        let r3 = h * self.fs[k - 1][i] - r2;
        let r4 = r2 - h * self.fs[k][i] - r3;
        let r5 = self.dense[k - 1][i];
        y0 + s * (r2 + (1.0 - s) * (r3 + s * (r4 + (1.0 - s) * r5)))
    }

    /// The rate of state i at time t: the derivative of the same dense output, the solver's own at each step.
    pub fn slope(&self, i: usize, t: f64) -> f64 {
        if self.ts.len() < 2 { return self.fs[0][i]; }
        let (k, s, h) = self.locate(t);
        let r2 = self.ys[k][i] - self.ys[k - 1][i];
        let r3 = h * self.fs[k - 1][i] - r2;
        let r4 = r2 - h * self.fs[k][i] - r3;
        let r5 = self.dense[k - 1][i];
        (r2 + (1.0 - 2.0 * s) * r3 + s * (2.0 - 3.0 * s) * r4 + 2.0 * s * (1.0 - s) * (1.0 - 2.0 * s) * r5) / h
    }
}
