//! Su-27SK base with two AL-31F engines; no thrust vectoring. Engineering polar fitted to published geometry and performance.
//! This is a game reconstruction, not the manufacturer's complete wind-tunnel
//! database. Sources, calibration targets and limitations are in docs/NEW-AIRFRAMES.md.
use crate::aero::{curve, lateral, lerp1, Coeffs, Flow, CD_BRAKE, DAMP, GROUND_EFFECT, XCG_REF};
use crate::math::{clamp, mix, smoothstep};
use crate::structure;
pub const WING_AREA: f64 = 62.0;
pub const SPAN: f64 = 14.7;
pub const CHORD: f64 = 4.646;
pub const MIL_THRUST: f64 = 2.0 * 7670.0 * 9.80665;
pub const MAX_THRUST: f64 = 2.0 * 12500.0 * 9.80665;
const CL: [f64; 12] = [
    -0.56, -0.27, 0.03, 0.34, 0.66, 0.97, 1.25, 1.49, 1.64, 1.63, 1.53, 1.42,
];
const CD0: [(f64, f64); 10] = [
    (0.0, 0.021),
    (0.6, 0.021),
    (0.85, 0.025),
    (1.0, 0.037),
    (1.1, 0.048),
    (1.3, 0.047),
    (1.6, 0.038),
    (1.8, 0.034),
    (2.3, 0.032),
    (2.7, 0.033),
];
const K: [(f64, f64); 5] = [
    (0.0, 0.105),
    (0.7, 0.11),
    (1.0, 0.19),
    (1.6, 0.35),
    (2.5, 0.50),
];
pub fn coefficients(f: &Flow) -> Coeffs {
    let a = clamp(f.alpha, -10.0, 45.0);
    let b = clamp(f.beta, -30.0, 30.0);
    let el = clamp(f.el, -25.0, 25.0);
    let d: [f64; 9] = std::array::from_fn(|i| lerp1(&DAMP[i], -10.0, 5.0, a));
    let cq = CHORD / (2.0 * f.speed.max(30.0));
    let (elastic, shift, _) = structure::effectiveness(f.qbar, f.mach);
    let compress = curve(
        &[
            (0.0, 1.0),
            (0.8, 1.12),
            (1.0, 1.15),
            (1.4, 0.82),
            (2.5, 0.55),
        ],
        f.mach,
    );
    let lift = lerp1(&CL, -10.0, 5.0, a) * compress * (1.0 - (b / 57.3).powi(2)) * elastic
        + 0.5 * curve(&GROUND_EFFECT, f.h_over_b);
    let parasite = curve(&CD0, f.mach) + CD_BRAKE * f.brake;
    let drag = parasite
        + mix(
            curve(&K, f.mach) * lift * lift,
            (lift * a.to_radians().tan()).abs(),
            smoothstep(17.0, 35.0, a.abs()),
        );
    let (sa, ca) = a.to_radians().sin_cos();
    let wing_z = -lift * ca - drag * sa;
    let stab = curve(&[(0.0, 1.0), (0.9, 1.05), (1.4, 0.8), (2.5, 0.55)], f.mach)
        * (1.0 - 0.35 * smoothstep(30.0, 50.0, a.abs()));
    let cx = -drag * ca + lift * sa + cq * d[0] * f.q;
    let cz = wing_z - 0.008 * el * stab + cq * d[3] * f.q;
    let arm = 0.025 + XCG_REF - f.xcg - shift + 0.10 * smoothstep(0.85, 1.3, f.mach);
    let cm = wing_z * arm - 0.017 * el * stab + cq * d[6] * f.q
        - 0.02 * f.brake
        - 0.4 * smoothstep(50.0, 80.0, f.alpha.abs()) * f.alpha.signum();
    let (cy, cl, cn) = lateral(f, SPAN);
    Coeffs {
        cx,
        cy,
        cz,
        cl: cl * 0.80,
        cm,
        cn: cn * 1.10,
    }
}
pub fn inlet_factor(mach: f64) -> f64 {
    curve(&[(2.2, 1.0), (2.5, 0.90), (2.8, 0.72), (3.2, 0.45)], mach)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{atmosphere_at, Type};
    #[test]
    fn polar_is_finite_with_stable_pitch_and_drag() {
        for alpha in [-10.0, 0.0, 5.0, 15.0, 25.0, 35.0, 60.0] {
            for mach in [0.2, 0.8, 1.0, 1.8, 2.3] {
                let f = Flow {
                    kind: Type::Su27,
                    alpha,
                    mach,
                    speed: mach * atmosphere_at(5000.0).sound,
                    xcg: XCG_REF,
                    h_over_b: 10.0,
                    ..Default::default()
                };
                let c = coefficients(&f);
                assert!([c.cx, c.cz, c.cm, c.cl, c.cn].iter().all(|n| n.is_finite()));
                let (sa, ca) = alpha.to_radians().sin_cos();
                assert!(-c.cx * ca - c.cz * sa > 0.0);
            }
        }
        let cm = |a| {
            coefficients(&Flow {
                kind: Type::Su27,
                alpha: a,
                speed: 220.0,
                xcg: XCG_REF,
                ..Default::default()
            })
            .cm
        };
        assert!(cm(10.0) < cm(5.0));
    }
}
