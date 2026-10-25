//! F-16 aerodynamic and engine data, in aircraft body axes (x forward, y right,
//! z down). Surface signs follow the data: stabilator and flaperon trailing
//! edge down positive, rudder trailing edge left positive.
//!
//! Coefficients are the wind-tunnel data of NASA TP-1538 (Nguyen et al., 1979)
//! as tabulated by Stevens & Lewis, the same source behind the JSBSim F-16 that
//! FlightGear flies. Mach wave drag, the transonic aerodynamic-centre shift and
//! ground effect follow the FlightGear F-16 (jsb-aerodynamics-block15-subtle),
//! and thrust follows its F110-GE-129 tables. Engine power dynamics are the
//! Stevens & Lewis power-lag model.
//!
//! The MiG-29 has its own longitudinal data and engines (aero_mig29.rs);
//! `coefficients` and `thrust` hand it over by the flow's airframe type.
use crate::aero_mig29;
use crate::math::{clamp, smoothstep};
use crate::sim::Type;
use crate::structure;

pub const WING_AREA: f64 = 27.87; // 300 ft^2
pub const SPAN: f64 = 9.144; // 30 ft
pub const CHORD: f64 = 3.450; // 11.32 ft mean aerodynamic chord
/// Reference centre of gravity of the moment data, fraction of MAC.
pub const XCG_REF: f64 = 0.35;

pub const MIL_THRUST: f64 = 17000.0 * 4.448222;
pub const MAX_THRUST: f64 = 29000.0 * 4.448222;
/// Engine rotor angular momentum, kg m^2/s (160 slug ft^2/s).
pub const ENGINE_MOMENTUM: f64 = 216.9;

/// Linear interpolation in a uniformly spaced table, holding the end values.
fn bracket(n: usize, x0: f64, dx: f64, x: f64) -> (usize, usize, f64) {
    let s = clamp((x - x0) / dx, 0.0, (n - 1) as f64);
    let i = (s.floor() as usize).min(n - 2);
    (i, i + 1, s - i as f64)
}

pub(crate) fn lerp1(t: &[f64], x0: f64, dx: f64, x: f64) -> f64 {
    let (i, j, f) = bracket(t.len(), x0, dx, x);
    t[i] + (t[j] - t[i]) * f
}

/// Rows over a second variable (`y0`, `dy`), columns over alpha.
fn lerp2(t: &[[f64; 12]], y0: f64, dy: f64, y: f64, alpha: f64) -> f64 {
    let (i, j, f) = bracket(t.len(), y0, dy, y);
    let a = lerp1(&t[i], -10.0, 5.0, alpha);
    let b = lerp1(&t[j], -10.0, 5.0, alpha);
    a + (b - a) * f
}

/// Piecewise-linear table of (x, y) points, held at the ends.
pub fn curve(t: &[(f64, f64)], x: f64) -> f64 {
    if x <= t[0].0 {
        return t[0].1;
    }
    for k in 0..t.len() - 1 {
        if x <= t[k + 1].0 {
            let f = (x - t[k].0) / (t[k + 1].0 - t[k].0);
            return t[k].1 + (t[k + 1].1 - t[k].1) * f;
        }
    }
    t[t.len() - 1].1
}

// Alpha breakpoints -10 to 45 degrees in 5 degree steps throughout.

/// Axial force, rows: stabilator -24, -12, 0, 12, 24 deg.
const CX: [[f64; 12]; 5] = [
    [-0.099, -0.081, -0.081, -0.063, -0.025, 0.044, 0.097, 0.113, 0.145, 0.167, 0.174, 0.166],
    [-0.048, -0.038, -0.040, -0.021, 0.016, 0.083, 0.127, 0.137, 0.162, 0.177, 0.179, 0.167],
    [-0.022, -0.020, -0.021, -0.004, 0.032, 0.094, 0.128, 0.130, 0.154, 0.161, 0.155, 0.138],
    [-0.040, -0.038, -0.039, -0.025, 0.006, 0.062, 0.087, 0.085, 0.100, 0.110, 0.104, 0.091],
    [-0.083, -0.073, -0.076, -0.072, -0.046, 0.012, 0.024, 0.025, 0.043, 0.053, 0.047, 0.040],
];

/// Normal force at zero sideslip and stabilator.
const CZ: [f64; 12] = [0.770, 0.241, -0.100, -0.415, -0.731, -1.053, -1.355, -1.646, -1.917, -2.120, -2.248, -2.229];

/// Pitching moment about 0.35 MAC, rows: stabilator -24 to 24 deg.
const CM: [[f64; 12]; 5] = [
    [0.205, 0.168, 0.186, 0.196, 0.213, 0.251, 0.245, 0.238, 0.252, 0.231, 0.198, 0.192],
    [0.081, 0.077, 0.107, 0.110, 0.110, 0.141, 0.127, 0.119, 0.133, 0.108, 0.081, 0.093],
    [-0.046, -0.020, -0.009, -0.005, -0.006, 0.010, 0.006, -0.001, 0.014, 0.000, -0.013, 0.032],
    [-0.174, -0.145, -0.121, -0.127, -0.129, -0.102, -0.097, -0.113, -0.087, -0.084, -0.069, -0.006],
    [-0.259, -0.202, -0.184, -0.193, -0.199, -0.150, -0.160, -0.167, -0.104, -0.076, -0.041, -0.005],
];

/// Rolling moment, rows: |beta| 0 to 30 deg; odd in beta.
const CL: [[f64; 12]; 7] = [
    [0.0; 12],
    [-0.001, -0.004, -0.008, -0.012, -0.016, -0.022, -0.022, -0.021, -0.015, -0.008, -0.013, -0.015],
    [-0.003, -0.009, -0.017, -0.024, -0.030, -0.041, -0.045, -0.040, -0.016, -0.002, -0.010, -0.019],
    [-0.001, -0.010, -0.020, -0.030, -0.039, -0.054, -0.057, -0.054, -0.023, -0.006, -0.014, -0.027],
    [0.000, -0.010, -0.022, -0.034, -0.047, -0.060, -0.069, -0.067, -0.033, -0.036, -0.035, -0.035],
    [0.007, -0.010, -0.023, -0.034, -0.049, -0.063, -0.081, -0.079, -0.060, -0.058, -0.062, -0.059],
    [0.009, -0.011, -0.023, -0.037, -0.050, -0.068, -0.089, -0.088, -0.091, -0.076, -0.077, -0.076],
];

/// Yawing moment, rows: |beta| 0 to 30 deg; odd in beta.
const CN: [[f64; 12]; 7] = [
    [0.0; 12],
    [0.018, 0.019, 0.018, 0.019, 0.019, 0.018, 0.013, 0.007, 0.004, -0.014, -0.017, -0.033],
    [0.038, 0.042, 0.042, 0.042, 0.043, 0.039, 0.030, 0.017, 0.004, -0.035, -0.047, -0.057],
    [0.056, 0.057, 0.059, 0.058, 0.058, 0.053, 0.032, 0.012, 0.002, -0.046, -0.071, -0.073],
    [0.064, 0.077, 0.076, 0.074, 0.073, 0.057, 0.029, 0.007, 0.012, -0.034, -0.065, -0.041],
    [0.074, 0.086, 0.093, 0.089, 0.080, 0.062, 0.049, 0.022, 0.028, -0.012, -0.002, -0.013],
    [0.079, 0.090, 0.106, 0.106, 0.096, 0.080, 0.068, 0.030, 0.064, 0.015, 0.011, -0.001],
];

/// Rolling moment per 20 deg flaperon, rows: beta -30 to 30 deg.
const DLDA: [[f64; 12]; 7] = [
    [-0.041, -0.052, -0.053, -0.056, -0.050, -0.056, -0.082, -0.059, -0.042, -0.038, -0.027, -0.017],
    [-0.041, -0.053, -0.053, -0.053, -0.050, -0.051, -0.066, -0.043, -0.038, -0.027, -0.023, -0.016],
    [-0.042, -0.053, -0.052, -0.051, -0.049, -0.049, -0.043, -0.035, -0.026, -0.016, -0.018, -0.014],
    [-0.040, -0.052, -0.051, -0.052, -0.048, -0.048, -0.042, -0.037, -0.031, -0.026, -0.017, -0.012],
    [-0.043, -0.049, -0.048, -0.049, -0.043, -0.042, -0.042, -0.036, -0.025, -0.021, -0.016, -0.011],
    [-0.044, -0.048, -0.048, -0.047, -0.042, -0.041, -0.020, -0.028, -0.013, -0.014, -0.011, -0.010],
    [-0.043, -0.049, -0.047, -0.045, -0.042, -0.037, -0.003, -0.013, -0.010, -0.003, -0.007, -0.008],
];

/// Rolling moment per 30 deg rudder, rows: beta -30 to 30 deg.
const DLDR: [[f64; 12]; 7] = [
    [0.005, 0.017, 0.014, 0.010, -0.005, 0.009, 0.019, 0.005, -0.000, -0.005, -0.011, 0.008],
    [0.007, 0.016, 0.014, 0.014, 0.013, 0.009, 0.012, 0.005, 0.000, 0.004, 0.009, 0.007],
    [0.013, 0.013, 0.011, 0.012, 0.011, 0.009, 0.008, 0.005, -0.002, 0.005, 0.003, 0.005],
    [0.018, 0.015, 0.015, 0.014, 0.014, 0.014, 0.014, 0.015, 0.013, 0.011, 0.006, 0.001],
    [0.015, 0.014, 0.013, 0.013, 0.012, 0.011, 0.011, 0.010, 0.008, 0.008, 0.007, 0.003],
    [0.021, 0.011, 0.010, 0.011, 0.010, 0.009, 0.008, 0.010, 0.006, 0.005, 0.000, 0.001],
    [0.023, 0.010, 0.011, 0.011, 0.011, 0.010, 0.008, 0.010, 0.006, 0.014, 0.020, 0.000],
];

/// Yawing moment per 20 deg flaperon, rows: beta -30 to 30 deg.
const DNDA: [[f64; 12]; 7] = [
    [0.001, -0.027, -0.017, -0.013, -0.012, -0.016, 0.001, 0.017, 0.011, 0.017, 0.008, 0.016],
    [0.002, -0.014, -0.016, -0.016, -0.014, -0.019, -0.021, 0.002, 0.012, 0.016, 0.015, 0.011],
    [-0.006, -0.008, -0.006, -0.006, -0.005, -0.008, -0.005, 0.007, 0.004, 0.007, 0.006, 0.006],
    [-0.011, -0.011, -0.010, -0.009, -0.008, -0.006, 0.000, 0.004, 0.007, 0.010, 0.004, 0.010],
    [-0.015, -0.015, -0.014, -0.012, -0.011, -0.008, -0.002, 0.002, 0.006, 0.012, 0.011, 0.011],
    [-0.024, -0.010, -0.004, -0.002, -0.001, 0.003, 0.014, 0.006, -0.001, 0.004, 0.004, 0.006],
    [-0.022, 0.002, -0.003, -0.005, -0.003, -0.001, -0.009, -0.009, -0.001, 0.003, -0.002, 0.001],
];

/// Yawing moment per 30 deg rudder, rows: beta -30 to 30 deg.
const DNDR: [[f64; 12]; 7] = [
    [-0.018, -0.052, -0.052, -0.052, -0.054, -0.049, -0.059, -0.051, -0.030, -0.037, -0.026, -0.013],
    [-0.028, -0.051, -0.043, -0.046, -0.045, -0.049, -0.057, -0.052, -0.030, -0.033, -0.030, -0.008],
    [-0.037, -0.041, -0.038, -0.040, -0.040, -0.038, -0.037, -0.030, -0.027, -0.024, -0.019, -0.013],
    [-0.048, -0.045, -0.045, -0.045, -0.044, -0.045, -0.047, -0.048, -0.049, -0.045, -0.033, -0.016],
    [-0.043, -0.044, -0.041, -0.041, -0.040, -0.038, -0.034, -0.035, -0.035, -0.029, -0.022, -0.009],
    [-0.052, -0.034, -0.036, -0.036, -0.035, -0.028, -0.024, -0.023, -0.020, -0.016, -0.010, -0.014],
    [-0.062, -0.034, -0.027, -0.028, -0.027, -0.027, -0.023, -0.023, -0.019, -0.009, -0.025, -0.010],
];

/// Rate derivatives: CXq, CYr, CYp, CZq, Clr, Clp, Cmq, Cnr, Cnp.
pub(crate) const DAMP: [[f64; 12]; 9] = [
    [-0.267, -0.110, 0.308, 1.34, 2.08, 2.91, 2.76, 2.05, 1.50, 1.49, 1.83, 1.21],
    [0.882, 0.852, 0.876, 0.958, 0.962, 0.974, 0.819, 0.483, 0.590, 1.21, -0.493, -1.04],
    [-0.108, -0.108, -0.188, 0.110, 0.258, 0.226, 0.344, 0.362, 0.611, 0.529, 0.298, -0.227],
    [-8.80, -25.8, -28.9, -31.4, -31.2, -30.7, -27.7, -28.2, -29.0, -29.8, -38.3, -35.3],
    [-0.126, -0.026, 0.063, 0.113, 0.208, 0.230, 0.319, 0.437, 0.680, 0.100, 0.447, -0.330],
    [-0.360, -0.359, -0.443, -0.420, -0.383, -0.375, -0.329, -0.294, -0.230, -0.210, -0.120, -0.100],
    [-7.21, -0.540, -5.23, -5.26, -6.11, -6.64, -5.69, -6.00, -6.20, -6.40, -6.60, -6.00],
    [-0.380, -0.363, -0.378, -0.386, -0.370, -0.453, -0.550, -0.582, -0.595, -0.637, -1.02, -0.840],
    [0.061, 0.052, 0.052, -0.012, -0.013, -0.024, 0.050, 0.150, 0.130, 0.158, 0.240, 0.150],
];

/// FlightGear F-16 wave drag against Mach.
const CD_WAVE: [(f64, f64); 12] = [
    (0.0, 0.0), (0.8, 0.0), (0.9, 0.0115), (1.0082, 0.0351), (1.0664, 0.0303), (1.15, 0.0275),
    (1.2522, 0.0246), (1.3642, 0.0221), (1.5143, 0.0200), (1.6824, 0.0185), (1.8416, 0.0175), (2.0, 0.0170),
];

/// Lift increment in ground effect against height over span (FlightGear, halved there).
pub(crate) const GROUND_EFFECT: [(f64, f64); 11] = [
    (0.0, 0.229), (0.2, 0.124), (0.3, 0.116), (0.4, 0.1105), (0.6, 0.105), (0.8, 0.041),
    (1.0, 0.034), (1.2, 0.019), (1.4, 0.008), (1.6, 0.003), (1.8, 0.001),
];

/// Transonic aft shift of the aerodynamic centre, fraction of MAC (FlightGear estimate).
const AC_SHIFT: [(f64, f64); 3] = [(0.6, 0.0), (0.8, 0.05), (1.1, 0.19)];

/// Airbrake: drag at full 60 degree opening, and its wave-drag factor.
pub(crate) const CD_BRAKE: f64 = 0.075;

#[derive(Clone, Copy, Debug, Default)]
pub struct Flow {
    /// Angles in degrees, rates in rad/s about body x, y, z.
    pub alpha: f64,
    pub beta: f64,
    pub p: f64,
    pub q: f64,
    pub r: f64,
    pub speed: f64,
    pub mach: f64,
    /// Surface deflections in degrees.
    pub el: f64,
    pub ail: f64,
    pub rdr: f64,
    /// Airbrake opening 0..1.
    pub brake: f64,
    /// Centre of gravity, fraction of MAC.
    pub xcg: f64,
    /// Dynamic pressure (Pa) bending the wing (structure.rs); zero for a rigid jet.
    pub qbar: f64,
    /// Height of the wing above the ground over the span.
    pub h_over_b: f64,
    /// Airframe whose data the coefficients come from.
    pub kind: Type,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Coeffs {
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub cl: f64,
    pub cm: f64,
    pub cn: f64,
}

pub fn coefficients(f: &Flow) -> Coeffs {
    match f.kind {
        Type::Mig29 => return aero_mig29::coefficients(f),
        Type::F18 => return crate::aero_hornet::coefficients(f),
        Type::Su27 => return crate::aero_su27::coefficients(f),
        Type::F16 => {}
    }
    let a = clamp(f.alpha, -10.0, 45.0);
    let b = clamp(f.beta, -30.0, 30.0);
    let el = clamp(f.el, -25.0, 25.0);
    let d: [f64; 9] = std::array::from_fn(|i| lerp1(&DAMP[i], -10.0, 5.0, a));
    let v = f.speed.max(30.0);
    let cq = CHORD / (2.0 * v);
    // The elastic wing sheds lift outboard and the flaperons lose power.
    let (e_lift, shift, _) = structure::effectiveness(f.qbar, f.mach);

    let mut cx = lerp2(&CX, -24.0, 12.0, el, a) + cq * d[0] * f.q;
    let (cy, cl, cn) = lateral(f, SPAN);
    let mut cz = lerp1(&CZ, -10.0, 5.0, a) * (1.0 - (b / 57.3).powi(2)) * e_lift - 0.19 * (el / 25.0) + cq * d[3] * f.q;

    // Wind-axis drag and lift increments resolved into body axes.
    let (sa, ca) = (f.alpha.to_radians().sin(), f.alpha.to_radians().cos());
    let drag = curve(&CD_WAVE, f.mach) * (1.0 + 0.25 * f.brake) + CD_BRAKE * f.brake;
    let lift = 0.5 * curve(&GROUND_EFFECT, f.h_over_b);
    cx += -drag * ca + lift * sa;
    cz += -drag * sa - lift * ca;

    let arm = XCG_REF - f.xcg + curve(&AC_SHIFT, f.mach) - shift;
    let cm = lerp2(&CM, -24.0, 12.0, el, a) + cz * arm + cq * d[6] * f.q - 0.02 * f.brake
        // Beyond the data the jet pitches out of a deep stall like a flat plate.
        - 0.4 * smoothstep(50.0, 80.0, f.alpha.abs()) * f.alpha.signum();
    let cn = cn - cy * (XCG_REF - f.xcg) * CHORD / SPAN;
    Coeffs { cx, cy, cz, cl, cm, cn }
}

/// Side force, rolling and yawing moments from the F-16 tables, made
/// dimensionless on the span `span` for the rate terms. No lateral data of
/// the MiG-29 is public, so it flies these too, on its own span.
pub(crate) fn lateral(f: &Flow, span: f64) -> (f64, f64, f64) {
    let a = clamp(f.alpha, -10.0, 45.0);
    let b = clamp(f.beta, -30.0, 30.0);
    let d: [f64; 9] = std::array::from_fn(|i| lerp1(&DAMP[i], -10.0, 5.0, a));
    let bv = span / (2.0 * f.speed.max(30.0));
    let ail = f.ail * structure::effectiveness(f.qbar, f.mach).2;
    let cy = -0.02 * b + 0.021 * (ail / 20.0) + 0.086 * (f.rdr / 30.0) + bv * (d[1] * f.r + d[2] * f.p);
    let sb = b.signum();
    let cl = lerp2(&CL, 0.0, 5.0, b.abs(), a) * sb
        + lerp2(&DLDA, -30.0, 10.0, b, a) * (ail / 20.0)
        + lerp2(&DLDR, -30.0, 10.0, b, a) * (f.rdr / 30.0)
        + bv * (d[4] * f.r + d[5] * f.p);
    let cn = lerp2(&CN, 0.0, 5.0, b.abs(), a) * sb
        + lerp2(&DNDA, -30.0, 10.0, b, a) * (ail / 20.0)
        + lerp2(&DNDR, -30.0, 10.0, b, a) * (f.rdr / 30.0)
        + bv * (d[7] * f.r + d[8] * f.p);
    (cy, cl, cn)
}

/// Reference lift coefficient at the airframe's nominal AoA cap.
pub fn cl_max(kind: Type) -> f64 {
    if kind == Type::Mig29 {
        return aero_mig29::cl_max();
    }
    cl_at_alpha(kind, kind.alpha_limits().1)
}

/// Reference lift at an adaptive cap, for the available-g estimate. As with
/// cl_max, this is the rigid, steady, low-Mach wing without flap increments.
pub fn cl_at_alpha(kind: Type, alpha: f64) -> f64 {
    let f = Flow { kind, alpha, speed: 200.0, xcg: XCG_REF, h_over_b: 10.0, ..Default::default() };
    let c = coefficients(&f);
    let (sa, ca) = alpha.to_radians().sin_cos();
    -c.cz * ca + c.cx * sa
}

// ------------------------------------------------------------------ engine
// F110-GE-129 thrust fractions; rows Mach, columns altitude -10,000 to 60,000 ft.
const IDLE: [[f64; 8]; 6] = [
    [0.0430, 0.0488, 0.0528, 0.0694, 0.0899, 0.1183, 0.1467, 0.0],
    [0.0500, 0.0501, 0.0335, 0.0544, 0.0797, 0.1049, 0.1342, 0.0],
    [0.0040, 0.0047, 0.0020, 0.0272, 0.0595, 0.0891, 0.1203, 0.0],
    [-0.0804, -0.0804, -0.0560, -0.0237, 0.0276, 0.0718, 0.1073, 0.0],
    [-0.2129, -0.2129, -0.1498, -0.1025, 0.0474, 0.0868, 0.0900, 0.0],
    [-0.2839, -0.2839, -0.1104, -0.0469, -0.0270, 0.0552, 0.0800, 0.0],
];
const MIL: [[f64; 8]; 8] = [
    [1.2600, 1.0000, 0.7400, 0.5340, 0.3720, 0.2410, 0.1490, 0.0],
    [1.1514, 0.9184, 0.6853, 0.4975, 0.3491, 0.2271, 0.1406, 0.0],
    [1.1115, 0.8902, 0.6689, 0.4891, 0.3451, 0.2252, 0.1402, 0.0],
    [1.1218, 0.9033, 0.6848, 0.5053, 0.3590, 0.2356, 0.1463, 0.0],
    [1.1739, 0.9518, 0.7297, 0.5431, 0.3891, 0.2566, 0.1586, 0.0],
    [1.2546, 1.0264, 0.7982, 0.5966, 0.4353, 0.2887, 0.1787, 0.0],
    [1.3360, 1.1066, 0.8772, 0.6694, 0.4903, 0.3275, 0.2024, 0.0],
    [1.4075, 1.1832, 0.9589, 0.7461, 0.5545, 0.3744, 0.2322, 0.0],
];
const AUG: [[f64; 8]; 14] = [
    [1.1816, 1.0000, 0.8184, 0.6627, 0.5280, 0.3756, 0.2327, 0.0],
    [1.1308, 0.9599, 0.7890, 0.6406, 0.5116, 0.3645, 0.2258, 0.0],
    [1.1150, 0.9474, 0.7798, 0.6340, 0.5070, 0.3615, 0.2240, 0.0],
    [1.1284, 0.9589, 0.7894, 0.6420, 0.5134, 0.3661, 0.2268, 0.0],
    [1.1707, 0.9942, 0.8177, 0.6647, 0.5309, 0.3784, 0.2345, 0.0],
    [1.2411, 1.0529, 0.8648, 0.7017, 0.5596, 0.3983, 0.2467, 0.0],
    [1.3287, 1.1254, 0.9221, 0.7462, 0.5936, 0.4219, 0.2614, 0.0],
    [1.4365, 1.2149, 0.9933, 0.8021, 0.6360, 0.4509, 0.2794, 0.0],
    [1.5711, 1.3260, 1.0809, 0.8700, 0.6874, 0.4860, 0.3011, 0.0],
    [1.7301, 1.4579, 1.1857, 0.9512, 0.7495, 0.5289, 0.3277, 0.0],
    [1.8314, 1.5700, 1.3086, 1.0474, 0.8216, 0.5786, 0.3585, 0.0],
    [1.9700, 1.6900, 1.4100, 1.2400, 0.9100, 0.6359, 0.3940, 0.0],
    [2.0700, 1.8000, 1.5300, 1.3400, 1.0000, 0.7200, 0.4600, 0.0],
    [2.2000, 1.9200, 1.6400, 1.4400, 1.1000, 0.8000, 0.5200, 0.0],
];

fn engine_table(t: &[[f64; 8]], mach: f64, alt_ft: f64) -> f64 {
    let (i, j, f) = bracket(t.len(), 0.0, 0.2, mach);
    let a = lerp1(&t[i], -10000.0, 10000.0, alt_ft);
    let b = lerp1(&t[j], -10000.0, 10000.0, alt_ft);
    a + (b - a) * f
}

/// Total-pressure recovery of the F-16's fixed normal-shock inlet over the
/// MIL-E-5007 recovery engine tables assume: it caps the jet near Mach 2.
fn inlet_factor(mach: f64) -> f64 {
    if mach <= 1.0 {
        return 1.0;
    }
    let m2 = mach * mach;
    let shock = (2.4 * m2 / (0.4 * m2 + 2.0)).powf(3.5) * (2.4 / (2.8 * m2 - 0.4)).powf(2.5);
    shock / (1.0 - 0.075 * (mach - 1.0).powf(1.35))
}

/// Installed thrust in newtons for engine power 0..100 (50 = military) of
/// one F110 or the MiG-29's pair of RD-33s. Both are low-bypass afterburning
/// turbofans, so the RD-33s follow the F110's lapse with Mach and altitude
/// from their own sea-level ratings, behind their own inlets.
pub fn thrust(kind: Type, power: f64, mach: f64, altitude_m: f64) -> f64 {
    let (mil_sl, max_sl, inlet) = match kind {
        Type::F16 => (MIL_THRUST, MAX_THRUST, inlet_factor(mach)),
        Type::Mig29 => (aero_mig29::MIL_THRUST, aero_mig29::MAX_THRUST, aero_mig29::inlet_factor(mach)),
        Type::F18 => (crate::aero_hornet::MIL_THRUST,crate::aero_hornet::MAX_THRUST,crate::aero_hornet::inlet_factor(mach)),
        Type::Su27 => (crate::aero_su27::MIL_THRUST,crate::aero_su27::MAX_THRUST,crate::aero_su27::inlet_factor(mach)),
    };
    let ft = altitude_m / 0.3048;
    let mil = engine_table(&MIL, mach, ft) * mil_sl * inlet;
    if power < 50.0 {
        let idle = engine_table(&IDLE, mach, ft) * mil_sl;
        idle + (mil - idle) * power * 0.02
    } else {
        let max = engine_table(&AUG, mach, ft) * max_sl * inlet;
        mil + (max - mil) * (power - 50.0) * 0.02
    }
}

fn rtau(dp: f64) -> f64 {
    if dp <= 25.0 {
        1.0
    } else if dp >= 50.0 {
        0.1
    } else {
        1.9 - 0.036 * dp
    }
}

/// Rate of change of engine power towards the commanded power: the core
/// spools slowly across large steps, the afterburner lights in a fraction of a second.
pub fn power_rate(power: f64, command: f64) -> f64 {
    let (target, rate) = if command >= 50.0 {
        if power >= 50.0 {
            (command, 5.0)
        } else {
            (60.0, rtau(60.0 - power))
        }
    } else if power >= 50.0 {
        (40.0, 5.0)
    } else {
        (command, rtau(command - power))
    };
    rate * (target - power)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_interpolate_and_hold_their_ends() {
        assert_eq!(lerp1(&CZ, -10.0, 5.0, 0.0), -0.100);
        assert!((lerp1(&CZ, -10.0, 5.0, 2.5) - (-0.2575)).abs() < 1e-9);
        assert_eq!(lerp1(&CZ, -10.0, 5.0, 90.0), -2.229);
        assert_eq!(lerp2(&CM, -24.0, 12.0, 0.0, 10.0), -0.006);
        assert!(cl_max(Type::F16) > 1.4 && cl_max(Type::F16) < 1.7);
    }

    #[test]
    fn controls_have_the_expected_signs() {
        let base = Flow { alpha: 4.0, speed: 200.0, mach: 0.6, xcg: XCG_REF, h_over_b: 10.0, ..Default::default() };
        let c0 = coefficients(&base);
        assert!(coefficients(&Flow { el: 10.0, ..base }).cm < c0.cm, "TE-down stabilator pitches nose down");
        assert!(coefficients(&Flow { ail: 10.0, ..base }).cl < c0.cl, "positive flaperon rolls left");
        assert!(coefficients(&Flow { rdr: 10.0, ..base }).cn < c0.cn, "positive rudder yaws left");
        assert!(coefficients(&Flow { beta: 5.0, ..base }).cn > 0.0, "weathercock stable");
        assert!(coefficients(&Flow { p: 1.0, ..base }).cl < 0.0, "roll damping");
        assert!(coefficients(&Flow { brake: 1.0, ..base }).cx < c0.cx, "airbrake drag");
    }

    #[test]
    fn thrust_lapses_with_altitude_and_grows_with_afterburner() {
        assert!((thrust(Type::F16, 50.0, 0.0, 0.0) - MIL_THRUST).abs() < 1.0);
        assert!((thrust(Type::F16, 100.0, 0.0, 0.0) - MAX_THRUST).abs() < 1.0);
        assert!(thrust(Type::F16, 100.0, 0.9, 9000.0) < thrust(Type::F16, 100.0, 0.9, 1000.0));
        assert!(thrust(Type::F16, 0.0, 0.8, 0.0) < 0.0, "idle ram drag at speed");
    }

    #[test]
    fn power_spools_slowly_and_lights_afterburner_fast() {
        let mut p = 10.0;
        let mut t = 0.0;
        while p < 49.0 {
            p += power_rate(p, 50.0) * 0.01;
            t += 0.01;
        }
        assert!(t > 2.0 && t < 8.0, "idle to military in {t:.1} s");
        let mut t = 0.0;
        while p < 95.0 {
            p += power_rate(p, 100.0) * 0.01;
            t += 0.01;
        }
        assert!(t < 1.0, "afterburner in {t:.2} s");
    }
}
