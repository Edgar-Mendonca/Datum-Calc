//! Thermophysical properties (0.2.6), all in SI (K, Pa, kg/m³, J/kg, J/(kg·K)).
//!
//! Water and steam: IAPWS-IF97 (regions 1, 2, 3, 5 and the saturation line, region 4), viscosity by IAPWS 2008
//! (without the critical enhancement, which matters only very near the critical point), thermal conductivity by
//! IAPWS 2011 with its industrial critical enhancement, surface tension by IAPWS 2014. Valid from 0 °C to 800 °C up
//! to 1000 bar and from 800 °C to 2000 °C up to 500 bar.
//!
//! Gases, as ideal gases: heat capacity, enthalpy and entropy from the NASA Glenn 7-coefficient polynomials (McBride,
//! Zehe and Gordon, NASA/TP-2002-211556), 200 K to 3000 K; dilute-gas viscosity and thermal conductivity as fits
//! (within 0.2 %) to reference correlations, over each gas's own range.

const R: f64 = 461.526;
const TC: f64 = 647.096;
const PC: f64 = 22.064e6;
const RHOC: f64 = 322.0;

/// A state of water: everything else follows from these.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub t: f64,
    pub p: f64,
    pub rho: f64,
    pub h: f64,
    pub s: f64,
    pub u: f64,
    pub cp: f64,
    pub cv: f64,
    pub w: f64,
    /// (∂ρ/∂p) at constant T, for the critical enhancement of the conductivity.
    pub drdp: f64,
    /// The vapour fraction when the state is a mixture of liquid and vapour; None for a single phase.
    pub x: Option<f64>,
}

type Res<T> = Result<T, String>;

// IAPWS-IF97 coefficients (I, J, n): regions 1, 2 (residual), 3, and the ideal-gas part of region 2 (J, n).
const R1: [(i32, i32, f64); 34] = [
    (0, -2, 0.14632971213167), (0, -1, -0.84548187169114), (0, 0, -3.756360367204),
    (0, 1, 3.3855169168385), (0, 2, -0.95791963387872), (0, 3, 0.15772038513228),
    (0, 4, -0.016616417199501), (0, 5, 0.00081214629983568), (1, -9, 0.00028319080123804),
    (1, -7, -0.00060706301565874), (1, -1, -0.018990068218419), (1, 0, -0.032529748770505),
    (1, 1, -0.021841717175414), (1, 3, -5.283835796993e-05), (2, -3, -0.00047184321073267),
    (2, 0, -0.00030001780793026), (2, 1, 4.7661393906987e-05), (2, 3, -4.4141845330846e-06),
    (2, 17, -7.2694996297594e-16), (3, -4, -3.1679644845054e-05), (3, 0, -2.8270797985312e-06),
    (3, 6, -8.5205128120103e-10), (4, -5, -2.2425281908e-06), (4, -2, -6.5171222895601e-07),
    (4, 10, -1.4341729937924e-13), (5, -8, -4.0516996860117e-07), (8, -11, -1.2734301741641e-09),
    (8, -6, -1.7424871230634e-10), (21, -29, -6.8762131295531e-19), (23, -31, 1.4478307828521e-20),
    (29, -38, 2.6335781662795e-23), (30, -39, -1.1947622640071e-23), (31, -40, 1.8228094581404e-24),
    (32, -41, -9.3537087292458e-26),
];

const R2: [(i32, i32, f64); 43] = [
    (1, 0, -0.0017731742473213), (1, 1, -0.017834862292358), (1, 2, -0.045996013696365),
    (1, 3, -0.057581259083432), (1, 6, -0.05032527872793), (2, 1, -3.3032641670203e-05),
    (2, 2, -0.00018948987516315), (2, 4, -0.0039392777243355), (2, 7, -0.043797295650573),
    (2, 36, -2.6674547914087e-05), (3, 0, 2.0481737692309e-08), (3, 1, 4.3870667284435e-07),
    (3, 3, -3.227767723857e-05), (3, 6, -0.0015033924542148), (3, 35, -0.040668253562649),
    (4, 1, -7.8847309559367e-10), (4, 2, 1.2790717852285e-08), (4, 3, 4.8225372718507e-07),
    (5, 7, 2.2922076337661e-06), (6, 3, -1.6714766451061e-11), (6, 16, -0.0021171472321355),
    (6, 35, -23.895741934104), (7, 0, -5.905956432427e-18), (7, 11, -1.2621808899101e-06),
    (7, 25, -0.038946842435739), (8, 8, 1.1256211360459e-11), (8, 36, -8.2311340897998),
    (9, 13, 1.9809712802088e-08), (10, 4, 1.0406965210174e-19), (10, 10, -1.0234747095929e-13),
    (10, 14, -1.0018179379511e-09), (16, 29, -8.0882908646985e-11), (16, 50, 0.10693031879409),
    (18, 57, -0.33662250574171), (20, 20, 8.9185845355421e-25), (20, 35, 3.0629316876232e-13),
    (20, 48, -4.2002467698208e-06), (21, 21, -5.9056029685639e-26), (22, 53, 3.7826947613457e-06),
    (23, 39, -1.2768608934681e-15), (24, 26, 7.3087610595061e-29), (24, 40, 5.5414715350778e-17),
    (24, 58, -9.436970724121e-07),
];

const R3: [(i32, i32, f64); 39] = [
    (0, 0, -15.732845290239), (0, 1, 20.944396974307), (0, 2, -7.6867707878716),
    (0, 7, 2.6185947787954), (0, 10, -2.808078114862), (0, 12, 1.2053369696517),
    (0, 23, -0.0084566812812502), (1, 2, -1.2654315477714), (1, 6, -1.1524407806681),
    (1, 15, 0.88521043984318), (1, 17, -0.64207765181607), (2, 0, 0.38493460186671),
    (2, 2, -0.85214708824206), (2, 6, 4.8972281541877), (2, 7, -3.0502617256965),
    (2, 22, 0.039420536879154), (2, 26, 0.12558408424308), (3, 0, -0.2799932969871),
    (3, 2, 1.389979956946), (3, 4, -2.018991502357), (3, 16, -0.0082147637173963),
    (3, 26, -0.47596035734923), (4, 0, 0.0439840744735), (4, 2, -0.44476435428739),
    (4, 4, 0.90572070719733), (4, 26, 0.70522450087967), (5, 1, 0.10770512626332),
    (5, 3, -0.32913623258954), (5, 26, -0.50871062041158), (6, 0, -0.022175400873096),
    (6, 2, 0.094260751665092), (6, 26, 0.16436278447961), (7, 2, -0.013503372241348),
    (8, 26, -0.014834345352472), (9, 2, 0.00057922953628084), (9, 26, 0.0032308904703711),
    (10, 0, 8.0964802996215e-05), (10, 1, -0.00016557679795037), (11, 26, -4.4923899061815e-05),
];

const R2_0: [(i32, f64); 9] = [(0, -9.6927686500217), (1, 10.086655968018), (-5, -0.005608791128302), (-4, 0.071452738081455), (-3, -0.40710498223928), (-2, 1.4240819171444), (-1, -4.383951131945), (2, -0.28408632460772), (3, 0.021268463753307)];

const R5: [(i32, i32, f64); 6] = [(1, 1, 0.0015736404855259), (1, 2, 0.00090153761673944), (1, 3, -0.0050270077677648), (2, 3, 2.2440037409485e-06), (2, 9, -4.1163275453471e-06), (3, 7, 3.7919454822955e-08)];
const R5_0: [(i32, f64); 6] = [(0, -13.179983674201), (1, 6.8540841634434), (-3, -0.024805148933466), (-2, 0.36901534980333), (-1, -3.1161318213925), (2, -0.32961626538917)];
/// The saturation line (region 4).
const N4: [f64; 10] = [0.11670521452767e4, -0.72421316703206e6, -0.17073846940092e2, 0.12020824702470e5, -0.32325550322333e7, 0.14915108613530e2, -0.48232657361591e4, 0.40511340542057e6, -0.23855557567849, 0.65017534844798e3];
/// The boundary between regions 2 and 3.
const B23: [f64; 5] = [0.34805185628969e3, -0.11671859879975e1, 0.10192970039326e-2, 0.57254459862746e3, 0.1391883977870e2];

/// γ and its derivatives: γ, γπ, γππ, γτ, γττ, γπτ.
type G = [f64; 6];

fn gibbs1(pi: f64, tau: f64) -> G {
    let (a, b) = (7.1 - pi, tau - 1.222);
    let mut g = [0.0; 6];
    for &(i, j, n) in &R1 {
        let (fi, fj) = (i as f64, j as f64);
        let (ai, bj, ai1, bj1) = (a.powi(i), b.powi(j), a.powi(i - 1), b.powi(j - 1));
        g[0] += n * ai * bj;
        g[1] -= n * fi * ai1 * bj;
        g[2] += n * fi * (fi - 1.0) * a.powi(i - 2) * bj;
        g[3] += n * ai * fj * bj1;
        g[4] += n * ai * fj * (fj - 1.0) * b.powi(j - 2);
        g[5] -= n * fi * ai1 * fj * bj1;
    }
    g
}

/// Regions 2 and 5: an ideal-gas part and a residual part in π and τ − shift.
fn gibbs_gas(pi: f64, tau: f64, ideal: &[(i32, f64)], res: &[(i32, i32, f64)], shift: f64) -> G {
    let mut g = [pi.ln(), 1.0 / pi, -1.0 / (pi * pi), 0.0, 0.0, 0.0];
    for &(j, n) in ideal {
        let fj = j as f64;
        g[0] += n * tau.powi(j);
        g[3] += n * fj * tau.powi(j - 1);
        g[4] += n * fj * (fj - 1.0) * tau.powi(j - 2);
    }
    let b = tau - shift;
    for &(i, j, n) in res {
        let (fi, fj) = (i as f64, j as f64);
        let (pi_i, bj, pi1, bj1) = (pi.powi(i), b.powi(j), pi.powi(i - 1), b.powi(j - 1));
        g[0] += n * pi_i * bj;
        g[1] += n * fi * pi1 * bj;
        g[2] += n * fi * (fi - 1.0) * pi.powi(i - 2) * bj;
        g[3] += n * pi_i * fj * bj1;
        g[4] += n * pi_i * fj * (fj - 1.0) * b.powi(j - 2);
        g[5] += n * fi * pi1 * fj * bj1;
    }
    g
}

fn from_gibbs(t: f64, p: f64, pi: f64, tau: f64, g: G) -> State {
    let v = R * t * pi * g[1] / p;
    let d = g[1] - tau * g[5];
    let dvdp = v * g[2] * pi / (g[1] * p);
    State {
        t, p, rho: 1.0 / v, h: R * t * tau * g[3], s: R * (tau * g[3] - g[0]), u: R * t * (tau * g[3] - pi * g[1]),
        cp: -R * tau * tau * g[4], cv: R * (-tau * tau * g[4] + d * d / g[2]),
        w: (R * t * g[1] * g[1] / (d * d / (tau * tau * g[4]) - g[2])).sqrt(), drdp: -dvdp / (v * v), x: None,
    }
}

fn region1(t: f64, p: f64) -> State { let (pi, tau) = (p / 16.53e6, 1386.0 / t); from_gibbs(t, p, pi, tau, gibbs1(pi, tau)) }
fn region2(t: f64, p: f64) -> State { let (pi, tau) = (p / 1e6, 540.0 / t); from_gibbs(t, p, pi, tau, gibbs_gas(pi, tau, &R2_0, &R2, 0.5)) }
fn region5(t: f64, p: f64) -> State { let (pi, tau) = (p / 1e6, 1000.0 / t); from_gibbs(t, p, pi, tau, gibbs_gas(pi, tau, &R5_0, &R5, 0.0)) }

/// Region 3 from density and temperature (a Helmholtz equation).
fn region3(rho: f64, t: f64) -> State {
    let (d, tau) = (rho / RHOC, TC / t);
    let n1 = 1.0658070028513;
    let mut f = [n1 * d.ln(), n1 / d, -n1 / (d * d), 0.0, 0.0, 0.0];
    for &(i, j, n) in &R3 {
        let (fi, fj) = (i as f64, j as f64);
        let (di, tj, di1, tj1) = (d.powi(i), tau.powi(j), d.powi(i - 1), tau.powi(j - 1));
        f[0] += n * di * tj;
        f[1] += n * fi * di1 * tj;
        f[2] += n * fi * (fi - 1.0) * d.powi(i - 2) * tj;
        f[3] += n * di * fj * tj1;
        f[4] += n * di * fj * (fj - 1.0) * tau.powi(j - 2);
        f[5] += n * fi * di1 * fj * tj1;
    }
    let (a, b) = (d * f[1] - d * tau * f[5], 2.0 * d * f[1] + d * d * f[2]);
    State {
        t, p: rho * R * t * d * f[1], rho, h: R * t * (tau * f[3] + d * f[1]), s: R * (tau * f[3] - f[0]), u: R * t * tau * f[3],
        cp: R * (-tau * tau * f[4] + a * a / b), cv: -R * tau * tau * f[4], w: (R * t * (b - a * a / (tau * tau * f[4]))).sqrt(),
        drdp: 1.0 / (R * t * b), x: None,
    }
}

/// Saturated liquid and vapour densities (IAPWS auxiliary equations, about 0.1 %), to bracket region 3.
fn rho_sat_aux(t: f64) -> (f64, f64) {
    let th = (1.0 - t / TC).max(0.0).cbrt();
    let l = [(1.99274064, 1.0), (1.09965342, 2.0), (-0.510839303, 5.0), (-1.75493479, 16.0), (-45.5170352, 43.0), (-674694.45, 110.0)];
    let v = [(-2.0315024, 1.0), (-2.6830294, 2.0), (-5.38626492, 4.0), (-17.2991605, 9.0), (-44.7586581, 18.5), (-63.9201063, 35.5)];
    (RHOC * (1.0 + l.iter().map(|(a, e)| a * th.powf(*e)).sum::<f64>()), RHOC * v.iter().map(|(a, e)| a * th.powf(*e)).sum::<f64>().exp())
}

/// The density in region 3 at (T, p): on the liquid side, the vapour side, or above the critical temperature.
fn rho3(t: f64, p: f64, liquid: bool) -> Res<f64> {
    let f = |r: f64| region3(r, t).p - p;
    let (al, av) = rho_sat_aux(t);
    // Region 3 lies between about 100 and 770 kg/m³; its equation is not meant for much beyond that.
    let (mut lo, mut hi) = (40.0, [800.0, 850.0, 900.0].into_iter().find(|r| f(*r) > 0.0).unwrap_or(900.0));
    if t < TC {
        if liquid { lo = [0.998, 0.99, 0.97, 0.9].iter().map(|k| al * k).find(|r| *r > RHOC && f(*r) < 0.0).unwrap_or(RHOC); }
        else { hi = [1.002, 1.01, 1.03, 1.1].iter().map(|k| av * k).find(|r| *r < RHOC && f(*r) > 0.0).unwrap_or(RHOC); lo = 1.0; }
    }
    if !(f(lo) <= 0.0 && f(hi) >= 0.0) {
        if TC - t > 0.01 { return Err("The water property equations have no state there.".into()); }
        let rs: Vec<f64> = (0..=860).map(|i| 40.0 + i as f64).collect();
        let cross: Vec<usize> = (0..rs.len() - 1).filter(|&i| f(rs[i]).signum() != f(rs[i + 1]).signum()).collect();
        let k = if liquid { cross.last() } else { cross.first() }.ok_or("The water property equations have no state there.")?;
        (lo, hi) = (rs[*k], rs[k + 1]);
        if f(lo) > 0.0 { std::mem::swap(&mut lo, &mut hi); }
        // Bisection on the found bracket (lo has f < 0, hi has f > 0, in either order).
        for _ in 0..100 { let m = 0.5 * (lo + hi); if f(m) < 0.0 { lo = m } else { hi = m } }
        return Ok(0.5 * (lo + hi));
    }
    // Newton's method, kept inside the bracket (bisection when a step would leave it).
    let mut r = if liquid || t >= TC { hi } else { lo };
    for _ in 0..200 {
        let st = region3(r, t);
        let e = st.p - p;
        if e.abs() <= 1e-11 * p { return Ok(r); }
        if e > 0.0 { hi = r } else { lo = r }
        let n = r - e * st.drdp;
        r = if n > lo && n < hi && st.drdp > 0.0 { n } else { 0.5 * (lo + hi) };
        if hi - lo <= 1e-13 * hi { return Ok(r); }
    }
    Ok(r)
}

const RANGE: &str = "Water and steam properties are given from 0 °C to 800 °C up to 1000 bar, and from 800 °C to 2000 °C up to 500 bar.";

/// The saturation pressure at a temperature (region 4), 0 °C to the critical point (373.946 °C).
pub fn psat(t: f64) -> Res<f64> {
    if !(273.15..=TC).contains(&t) { return Err("Saturation is from 0 °C to the critical point, 373.946 °C.".into()); }
    let n = &N4;
    let th = t + n[8] / (t - n[9]);
    let (a, b, c) = (th * th + n[0] * th + n[1], n[2] * th * th + n[3] * th + n[4], n[5] * th * th + n[6] * th + n[7]);
    Ok((2.0 * c / (-b + (b * b - 4.0 * a * c).sqrt())).powi(4) * 1e6)
}

/// The saturation temperature at a pressure, 611.213 Pa to the critical point (220.64 bar).
pub fn tsat(p: f64) -> Res<f64> {
    if !(611.212..=PC).contains(&p) { return Err("Saturation is from 611.213 Pa (the triple point) to the critical pressure, 220.64 bar.".into()); }
    let n = &N4;
    let be = (p / 1e6).powf(0.25);
    let (e, f, g) = (be * be + n[2] * be + n[5], n[0] * be * be + n[3] * be + n[6], n[1] * be * be + n[4] * be + n[7]);
    let d = 2.0 * g / (-f - (f * f - 4.0 * e * g).sqrt());
    Ok((n[9] + d - ((n[9] + d).powi(2) - 4.0 * (n[8] + n[9] * d)).sqrt()) / 2.0)
}

fn p23(t: f64) -> f64 { (B23[0] + B23[1] * t + B23[2] * t * t) * 1e6 }

fn pmax(t: f64) -> f64 { if t > 1073.15 { 50e6 } else { 100e6 } }

/// Water at a temperature and pressure (a single phase: liquid at or above the saturation pressure).
pub fn tp(t: f64, p: f64) -> Res<State> {
    if !(t.is_finite() && p.is_finite()) || !(273.15..=2273.15).contains(&t) || p <= 0.0 || p > pmax(t) { return Err(RANGE.into()); }
    if t > 1073.15 { return Ok(region5(t, p)); }
    if t <= 623.15 { return Ok(if p >= psat(t)? { region1(t, p) } else { region2(t, p) }); }
    if p > p23(t) {
        let liquid = t < TC && p >= psat(t)?;
        let mut st = region3(rho3(t, p, liquid)?, t);
        st.p = p;
        return Ok(st);
    }
    Ok(region2(t, p))
}

/// Saturated liquid and saturated vapour at a temperature.
pub fn sat(t: f64) -> Res<(State, State)> {
    let p = psat(t)?;
    if t <= 623.15 { return Ok((region1(t, p), region2(t, p))); }
    let (mut l, mut v) = (region3(rho3(t, p, true)?, t), region3(rho3(t, p, false)?, t));
    l.p = p;
    v.p = p;
    Ok((l, v))
}

/// Liquid and vapour mixed: the vapour fraction x by mass.
fn mix(l: &State, v: &State, x: f64) -> State {
    let sp = (1.0 - x) / l.rho + x / v.rho;
    let m = |a: f64, b: f64| a + x * (b - a);
    State { t: l.t, p: l.p, rho: 1.0 / sp, h: m(l.h, v.h), s: m(l.s, v.s), u: m(l.u, v.u), cp: f64::NAN, cv: f64::NAN, w: f64::NAN, drdp: f64::NAN, x: Some(x) }
}

/// Wet steam at a temperature and vapour fraction.
pub fn tx(t: f64, x: f64) -> Res<State> {
    if !(0.0..=1.0).contains(&x) { return Err("A vapour fraction is from 0 (saturated liquid) to 1 (saturated vapour).".into()); }
    let (l, v) = sat(t)?;
    Ok(mix(&l, &v, x))
}

/// Wet steam at a pressure and vapour fraction.
pub fn px(p: f64, x: f64) -> Res<State> {
    let mut s = tx(tsat(p)?, x)?;
    s.p = p;
    Ok(s)
}

/// The state with a given pressure and enthalpy (`by` = 0) or entropy (`by` = 1): wet steam inside the dome.
pub fn p_and(p: f64, target: f64, by: usize) -> Res<State> {
    if !p.is_finite() || p <= 0.0 || p > 100e6 { return Err(RANGE.into()); }
    let val = |s: &State| if by == 0 { s.h } else { s.s };
    let tmax = if p > 50e6 { 1073.15 } else { 2273.15 };
    let (mut lo, mut hi) = (273.15, tmax);
    if (611.213..PC).contains(&p) {
        let ts = tsat(p)?;
        let (l, v) = sat(ts)?;
        let (a, b) = (val(&l), val(&v));
        if target >= a && target <= b {
            let mut st = mix(&l, &v, if b > a { (target - a) / (b - a) } else { 0.0 });
            st.p = p;
            return Ok(st);
        }
        if target < a { hi = ts } else { lo = ts }
    }
    // The temperature where the property matches, by Brent's method (it rises with temperature at a set pressure).
    let mut f = |t: f64| tp(t.clamp(273.15, tmax), p).map(|s| val(&s) - target).unwrap_or(f64::NAN);
    // A target at an end of the range (typed exactly at 2000 °C, say) is that end.
    for e in [lo, hi] { if f(e).abs() <= 1e-12 * target.abs().max(1.0) { return tp(e, p); } }
    let t = crate::solve::root(&mut f, lo, hi).map_err(|_| format!("No water state at this pressure has that {}: {}", if by == 0 { "enthalpy" } else { "entropy" }, RANGE))?;
    tp(t.clamp(273.15, tmax), p)
}

/// Dynamic viscosity (IAPWS 2008, without the critical enhancement), Pa·s.
pub fn mu(rho: f64, t: f64) -> f64 {
    let (tr, d) = (t / TC, rho / RHOC);
    let h0 = [1.67752, 2.20462, 0.6366564, -0.241605];
    let mu0 = 100.0 * tr.sqrt() / h0.iter().enumerate().map(|(i, h)| h / tr.powi(i as i32)).sum::<f64>();
    const H: [(i32, i32, f64); 21] = [(0, 0, 0.520094), (1, 0, 0.0850895), (2, 0, -1.08374), (3, 0, -0.289555), (0, 1, 0.222531), (1, 1, 0.999115), (2, 1, 1.88797), (3, 1, 1.26613), (5, 1, 0.120573), (0, 2, -0.281378), (1, 2, -0.906851), (2, 2, -0.772479), (3, 2, -0.489837), (4, 2, -0.25704), (0, 3, 0.161913), (1, 3, 0.257399), (0, 4, -0.0325372), (3, 4, 0.0698452), (4, 5, 0.00872102), (3, 6, -0.00435673), (5, 6, -0.000593264)];
    let mu1 = (d * H.iter().map(|&(i, j, h)| h * (1.0 / tr - 1.0).powi(i) * (d - 1.0).powi(j)).sum::<f64>()).exp();
    mu0 * mu1 * 1e-6
}

/// Thermal conductivity (IAPWS 2011, with its industrial critical enhancement), W/(m·K).
pub fn k(s: &State) -> f64 {
    let (tr, d) = (s.t / TC, s.rho / RHOC);
    let l0 = [2.443221e-3, 1.323095e-2, 6.770357e-3, -3.454586e-3, 4.096266e-4];
    let k0 = tr.sqrt() / l0.iter().enumerate().map(|(i, l)| l / tr.powi(i as i32)).sum::<f64>();
    const L: [[f64; 6]; 5] = [
        [1.60397357, -0.646013523, 0.111443906, 0.102997357, -0.0504123634, 0.00609859258],
        [2.33771842, -2.78843778, 1.53616167, -0.463045512, 0.0832827019, -0.00719201245],
        [2.19650529, -4.54580785, 3.55777244, -1.40944978, 0.275418278, -0.0205938816],
        [-1.21051378, 1.60812989, -0.621178141, 0.0716373224, 0.0, 0.0],
        [-2.720337, 4.57586331, -3.18369245, 1.1168348, -0.19268305, 0.012913842],
    ];
    let k1 = (d * L.iter().enumerate().map(|(i, r)| r.iter().enumerate().map(|(j, n)| n * (1.0 / tr - 1.0).powi(i as i32) * (d - 1.0).powi(j as i32)).sum::<f64>()).sum::<f64>()).exp();
    // The critical enhancement: (∂ρ/∂p)_T against its value at 1.5 T_c, from the industrial formulation's fits.
    let a: [f64; 6] = match d {
        d if d <= 0.310559006 => [6.53786807199516, -5.61149954923348, 3.39624167361325, -2.27492629730878, 10.2631854662709, 1.97815050331519],
        d if d <= 0.776397516 => [6.52717759281799, -6.30816983387575, 8.08379285492595, -9.82240510197603, 12.1358413791395, -5.54349664571295],
        d if d <= 1.242236025 => [5.35500529896124, -3.96415689925446, 8.91990208918795, -12.033872950579, 9.19494865194302, -2.16866274479712],
        d if d <= 1.863354037 => [1.55225959906681, 0.464621290821181, 8.93237374861479, -11.0321960061126, 6.1678099993336, -0.965458722086812],
        _ => [1.11999926419994, 0.595748562571649, 9.8895256507892, -10.325505114704, 4.66861294457414, -0.503243546373828],
    };
    let dref = 1.0 / a.iter().enumerate().map(|(i, a)| a * d.powi(i as i32)).sum::<f64>() * RHOC / PC;
    let dx = (d * (PC / RHOC * s.drdp - PC / RHOC * dref * 1.5 / tr)).max(0.0);
    let y = 0.13 * (dx / 0.06).powf(0.63 / 1.239) / 0.4;
    let kappa = s.cp / s.cv;
    let z = if y < 1.2e-7 { 0.0 } else { 2.0 / std::f64::consts::PI / y * (((1.0 - 1.0 / kappa) * y.atan() + y / kappa) - (1.0 - (-1.0 / (1.0 / y + y * y / 3.0 / (d * d))).exp())) };
    let k2 = 177.8514 * d * s.cp / 461.51805 * tr / (mu(s.rho, s.t) * 1e6) * z;
    (k0 * k1 + k2) * 1e-3
}

/// Surface tension of water against its vapour (IAPWS 2014), N/m, 0 °C to the critical point.
pub fn sigma(t: f64) -> Res<f64> {
    if !(273.15..=TC).contains(&t) { return Err("Surface tension is from 0 °C to the critical point, 373.946 °C.".into()); }
    let tau = 1.0 - t / TC;
    Ok(0.2358 * tau.powf(1.256) * (1.0 - 0.625 * tau))
}

/// An ideal gas: molar mass, NASA polynomials above and below 1000 K, and fits of ln μ and ln k in ln(T/300 K).
pub struct Gas { pub id: &'static str, pub name: &'static str, pub m: f64, pub t: (f64, f64), high: [f64; 7], low: [f64; 7], mu: [f64; 5], k: [f64; 5] }

const RU: f64 = 8.314462618;

/// A gas by its formula or name (air, N2, O2, Ar, CO2, H2, He, CH4), any case.
pub fn gas(id: &str) -> Option<&'static Gas> { GASES.iter().find(|g| g.id.eq_ignore_ascii_case(id)) }
pub fn gas_ids() -> Vec<&'static str> { GASES.iter().map(|g| g.id).collect() }

impl Gas {
    fn a(&self, t: f64) -> &[f64; 7] { if t > 1000.0 { &self.high } else { &self.low } }
    /// The specific gas constant, J/(kg·K).
    pub fn r(&self) -> f64 { RU / self.m }
    fn heat(&self, t: f64) -> Res<()> {
        if (200.0..=3000.0).contains(&t) { Ok(()) } else { Err(format!("{}: heat capacity, enthalpy and entropy are given from 200 K to 3000 K (−73 °C to 2727 °C).", self.name)) }
    }
    fn transport(&self, t: f64) -> Res<()> {
        if (self.t.0..=self.t.1).contains(&t) { Ok(()) } else { Err(format!("{}: viscosity and conductivity are given from {} K to {} K ({} °C to {} °C).", self.name, self.t.0, self.t.1, (self.t.0 - 273.15).round(), (self.t.1 - 273.15).round()).replace('-', "−")) }
    }
    pub fn cp(&self, t: f64) -> Res<f64> { self.heat(t)?; let a = self.a(t); Ok((a[0] + t * (a[1] + t * (a[2] + t * (a[3] + t * a[4])))) * self.r()) }
    /// Enthalpy above that at 25 °C, J/kg.
    pub fn h(&self, t: f64) -> Res<f64> {
        self.heat(t)?;
        let hh = |t: f64| { let a = self.a(t); t * (a[0] + t * (a[1] / 2.0 + t * (a[2] / 3.0 + t * (a[3] / 4.0 + t * a[4] / 5.0)))) + a[5] };
        Ok((hh(t) - hh(298.15)) * self.r())
    }
    /// Absolute entropy at a pressure (1 bar standard state), J/(kg·K).
    pub fn s(&self, t: f64, p: f64) -> Res<f64> {
        self.heat(t)?;
        if p <= 0.0 || !p.is_finite() { return Err("A gas entropy needs a pressure above zero.".into()); }
        let a = self.a(t);
        Ok((a[0] * t.ln() + t * (a[1] + t * (a[2] / 2.0 + t * (a[3] / 3.0 + t * a[4] / 4.0))) + a[6] - (p / 1e5).ln()) * self.r())
    }
    pub fn mu(&self, t: f64) -> Res<f64> { self.transport(t)?; Ok(fit(&self.mu, t)) }
    pub fn k(&self, t: f64) -> Res<f64> { self.transport(t)?; Ok(fit(&self.k, t)) }
}

fn fit(c: &[f64; 5], t: f64) -> f64 { let x = (t / 300.0).ln(); c.iter().rev().fold(0.0, |s, a| s * x + a).exp() }

// Gases: molar mass (kg/mol), the range of the viscosity and conductivity fits (K), NASA coefficients above and
// below 1000 K, and the fits of ln μ (Pa·s) and ln k (W/(m·K)) in powers of ln(T/300 K).
const GASES: [Gas; 8] = [
    Gas { id: "air", name: "Air", m: 0.02896546, t: (200.0, 1500.0), high: [3.097333676485, 0.0012292250386939801, -4.1459019899970003e-07, 6.5743205525256e-11, -3.873176110242e-15, -1000.7654335228001, 5.907704982883072], low: [3.573621910468, -0.0007210209253108801, 1.6672893139645202e-06, -1.2537432823679988e-10, -4.206774249256799e-13, -1064.7502881448, 3.694835528374072],
        mu: [-10.896497381069427, 0.7808917864567656, -0.07856867124713822, 0.008774763987953085, 0.0033099992559076454], k: [-3.636164132107852, 0.8467254027366861, -0.07371922737478229, 0.015155323688221794, 0.0019088635364366453] },
    Gas { id: "N2", name: "Nitrogen", m: 0.02801348, t: (200.0, 1500.0), high: [2.95257626, 0.00139690057, -4.92631691e-07, 7.86010367e-11, -4.60755321e-15, -923.948645, 5.87189252], low: [3.53100528, -0.000123660987, -5.02999437e-07, 2.43530612e-09, -1.40881235e-12, -1046.97628, 2.96747468],
        mu: [-10.931991981519722, 0.7741654359314618, -0.07739652186777962, 0.00934571533551679, 0.0033100008186225725], k: [-3.6521077753326088, 0.8353857119807802, -0.07834379185900625, 0.015510955856708452, 0.0023217726811238524] },
    Gas { id: "O2", name: "Oxygen", m: 0.0319988, t: (200.0, 1500.0), high: [3.66096083, 0.000656365523, -1.41149485e-07, 2.05797658e-11, -1.29913248e-15, -1215.97725, 3.41536184], low: [3.78245636, -0.00299673415, 9.847302e-06, -9.68129508e-09, 3.24372836e-12, -1063.94356, 3.65767573],
        mu: [-10.788729798706013, 0.8029247269164088, -0.081808101830929, 0.00695723661241094, 0.0033099989698703396], k: [-3.63285241464493, 0.8934286621954015, -0.07330807246003362, 0.013276717083967847, 0.0012722382463280378] },
    Gas { id: "Ar", name: "Argon", m: 0.039948, t: (200.0, 1500.0), high: [2.5, 0.0, 0.0, 0.0, 0.0, -745.375, 4.37967491], low: [2.5, 0.0, 0.0, 0.0, 0.0, -745.375, 4.37967491],
        mu: [-10.692084341189199, 0.8345603576888474, -0.08504781966001619, 0.004450510950775969, 0.0033100001071908925], k: [-4.028320755999433, 0.8372351196178257, -0.08862271891656678, 0.004760777663919134, 0.0033719413480133593] },
    Gas { id: "CO2", name: "Carbon dioxide", m: 0.0440098, t: (220.0, 1500.0), high: [4.63659493, 0.00274131991, -9.95828531e-07, 1.60373011e-10, -9.16103468e-15, -49024.9341, -1.93534855], low: [2.35677352, 0.00898459677, -7.12356269e-06, 2.45919022e-09, -1.43699548e-13, -48371.9697, 9.90105222],
        mu: [-11.10751867071644, 0.9549311445606458, -0.060752793619157454, -0.055263394729135856, 0.021284160396633656], k: [-4.090679956316922, 1.3773845448540558, -0.06796604114046197, -0.10276457708915987, 0.0293454506117183] },
    Gas { id: "H2", name: "Hydrogen", m: 0.00201588, t: (200.0, 1000.0), high: [2.93286579, 0.000826607967, -1.46402335e-07, 1.54100359e-11, -6.88804432e-16, -813.065597, -1.02432887], low: [2.34433112, 0.00798052075, -1.9478151e-05, 2.01572094e-08, -7.37611761e-12, -917.935173, 0.683010238],
        mu: [-11.625206572591912, 0.6920704986640737, -0.00040891697048577096, 0.00817199727982183, -0.002769809876318119], k: [-1.679025484840536, 0.7708089281294436, -0.1282181037142152, 0.10029686064417477, -0.006538870248530473] },
    Gas { id: "He", name: "Helium", m: 0.004002602, t: (200.0, 1500.0), high: [2.5, 0.0, 0.0, 0.0, 0.0, -745.375, 0.928724724], low: [2.5, 0.0, 0.0, 0.0, 0.0, -745.375, 0.928724724],
        mu: [-10.82347952439517, 0.6844987864339498, 0.014175614017506211, -0.0029146368076746272, 0.00022658733792651315], k: [-1.8585669601769557, 0.6914999247326152, 0.00564427023008564, -0.0015950324951917432, 0.0002220584488703231] },
    Gas { id: "CH4", name: "Methane", m: 0.0160428, t: (200.0, 625.0), high: [1.63552643, 0.0100842795, -3.36916254e-06, 5.34958667e-10, -3.15518833e-14, -10005.6455, 9.99313326], low: [5.14987613, -0.0136709788, 4.91800599e-05, -4.84743026e-08, 1.66693956e-11, -10246.6476, -4.64130376],
        mu: [-11.395821183398432, 0.8455629091104782, -0.08867007417884662, 0.02384755659613781, -0.005595439872800516], k: [-3.371664567279733, 1.2242777055856797, 0.2729232628172612, 0.056089295234311755, -0.2037729653122262] },
];
