//! MiG-29 (9.12/9.13) aerodynamic and engine data, in the axes and sign
//! conventions of aero.rs. No wind-tunnel tables of the Fulcrum are public,
//! so the longitudinal model is built from the published figures:
//!
//! - Geometry (Mikoyan): 38.06 m^2 of wing, 11.36 m span, 42 degrees of
//!   leading-edge sweep. The 3.73 m mean aerodynamic chord is worked out
//!   from that planform with a taper ratio of about 0.26.
//! - MiG-29A practical aerodynamics manual, as quoted from its charts:
//!   zero-lift drag Cx0 = 0.022, lift Cy = 1.43 at 24 degrees AoA, critical
//!   AoA 26 to 28 degrees, an Oswald factor near 0.87 at small lift.
//! - Its turn charts, which fix the induced drag at speed: 6.6 g sustained at
//!   Mach 0.9 and 5,000 m with 1,500 kg of fuel, and at 1,000 m and 800 to
//!   1,000 km/h in full afterburner a 6 g turn still gaining about 20 km/h a
//!   second.
//! - Top speeds, which fix the wave drag against the engines: 1,500 km/h
//!   (Mach 1.26) at sea level and 2,450 km/h (Mach 2.3) at altitude.
//! - Engines: two Klimov RD-33s of 5,040 kgf dry and 8,300 kgf in
//!   afterburner, behind variable-ramp inlets.
//!
//! The wing-body blend gives some 40 % of the lift from the fuselage and the
//! leading-edge root extensions, whose vortices hold the lift past 25
//! degrees; past about 12 degrees the leading-edge suction fades and drag
//! climbs toward the normal force tilted back with the AoA. The jet is
//! slightly stable in pitch subsonic, unlike the F-16. Lateral-directional
//! data and rate damping are the F-16's (aero.rs) on the MiG's own span and
//! chord.
use crate::aero::{curve, lateral, lerp1, Coeffs, Flow, CD_BRAKE, DAMP, GROUND_EFFECT, XCG_REF};
use crate::math::{clamp, mix, smoothstep};
use crate::structure;

pub const WING_AREA: f64 = 38.06;
pub const SPAN: f64 = 11.36;
pub const CHORD: f64 = 3.73;

/// Sea-level static thrust of both RD-33s, newtons.
pub const MIL_THRUST: f64 = 2.0 * 5040.0 * 9.80665;
pub const MAX_THRUST: f64 = 2.0 * 8300.0 * 9.80665;

/// Lift coefficient at low Mach with the slats out, AoA -10 to 45 degrees
/// in 5 degree steps: 0.059 a degree, 1.43 at 24, the peak at the 26 to 28
/// degrees critical and a gentle fall as the root-extension vortices burst.
const CL: [f64; 12] = [-0.56, -0.27, 0.02, 0.31, 0.61, 0.91, 1.20, 1.47, 1.56, 1.50, 1.42, 1.38];

/// Lift slope against its low-speed value: compressibility raises it toward
/// Mach 0.9, and it falls away supersonic.
const CL_MACH: [(f64, f64); 8] = [(0.0, 1.0), (0.6, 1.08), (0.9, 1.18), (1.0, 1.15), (1.2, 0.95), (1.6, 0.75), (2.0, 0.62), (2.5, 0.52)];

/// Zero-lift drag against Mach: Cx0 = 0.022 subsonic, the transonic rise,
/// and wave drag that lets the RD-33s reach Mach 1.26 at sea level and 2.3 high.
const CD0: [(f64, f64); 14] = [
    (0.0, 0.022), (0.6, 0.022), (0.8, 0.0232), (0.9, 0.026), (0.95, 0.031), (1.0, 0.037), (1.05, 0.041),
    (1.1, 0.0425), (1.2, 0.0425), (1.3, 0.042), (1.6, 0.041), (2.0, 0.040), (2.3, 0.039), (2.6, 0.039),
];

/// Induced-drag factor of the attached-flow polar Cd = Cd0 + K Cl^2. The
/// turn at 900 km/h fixes 0.108 subsonic, which is the manual's Oswald factor
/// of 0.87 on the 3.39 aspect ratio; the sustained 6.6 g at Mach 0.9 fixes the
/// transonic rise, and supersonic it tends to the no-suction 1/Cl_alpha.
const K: [(f64, f64); 9] = [(0.0, 0.108), (0.6, 0.108), (0.75, 0.110), (0.9, 0.15), (1.0, 0.22), (1.2, 0.29), (1.5, 0.36), (2.0, 0.46), (2.5, 0.55)];

/// Static margin, fraction of MAC: slightly stable subsonic, the
/// aerodynamic centre moving aft through Mach 1.
const MARGIN: [(f64, f64); 4] = [(0.0, 0.03), (0.8, 0.04), (1.1, 0.15), (2.0, 0.17)];

/// All-moving stabilator: pitching moment and lift per degree (trailing edge
/// down positive), from its 7.05 m^2 on a tail volume of about 0.3, and its
/// effectiveness over Mach.
const CM_EL: f64 = -0.0135;
const CZ_EL: f64 = -0.008;
const EL_MACH: [(f64, f64); 4] = [(0.0, 1.0), (0.9, 1.05), (1.2, 0.85), (2.0, 0.6)];

pub fn coefficients(f: &Flow) -> Coeffs {
    let a = clamp(f.alpha, -10.0, 45.0);
    let b = clamp(f.beta, -30.0, 30.0);
    let el = clamp(f.el, -25.0, 25.0);
    let d: [f64; 9] = std::array::from_fn(|i| lerp1(&DAMP[i], -10.0, 5.0, a));
    let cq = CHORD / (2.0 * f.speed.max(30.0));
    // The elastic wing sheds lift outboard and moves its centre.
    let (e_lift, shift, _) = structure::effectiveness(f.qbar, f.mach);

    // Wind-axis lift and drag, then resolved into body axes.
    let lift = lerp1(&CL, -10.0, 5.0, a) * curve(&CL_MACH, f.mach) * (1.0 - (b / 57.3).powi(2)) * e_lift
        + 0.5 * curve(&GROUND_EFFECT, f.h_over_b);
    let cd0 = curve(&CD0, f.mach) * (1.0 + 0.25 * f.brake) + CD_BRAKE * f.brake;
    let attached = curve(&K, f.mach) * lift * lift;
    let separated = (lift * a.to_radians().tan()).abs();
    let drag = cd0 + mix(attached, separated, smoothstep(12.0, 28.0, a.abs()));
    let (sa, ca) = (a.to_radians().sin(), a.to_radians().cos());
    let wing_z = -lift * ca - drag * sa;

    let stab = curve(&EL_MACH, f.mach) * (1.0 - 0.4 * smoothstep(25.0, 45.0, a.abs()));
    let cx = -drag * ca + lift * sa + cq * d[0] * f.q;
    let cz = wing_z + CZ_EL * el * stab + cq * d[3] * f.q;
    let arm = curve(&MARGIN, f.mach) + XCG_REF - f.xcg - shift;
    let cm = wing_z * arm + CM_EL * el * stab + cq * d[6] * f.q - 0.02 * f.brake
        // Beyond the data the jet pitches out of a deep stall like a flat plate.
        - 0.4 * smoothstep(50.0, 80.0, f.alpha.abs()) * f.alpha.signum();
    let (cy, cl, cn) = lateral(f, SPAN);
    Coeffs { cx, cy, cz, cl, cm, cn }
}

/// Lift coefficient available at the 25 degree AoA limit.
pub fn cl_max() -> f64 {
    lerp1(&CL, -10.0, 5.0, 25.0)
}

/// Total-pressure recovery of the variable-ramp inlets against the
/// MIL-E-5007 recovery the engine tables assume: the ramps hold it up to
/// their design Mach, past which the shocks spill.
pub fn inlet_factor(mach: f64) -> f64 {
    curve(&[(2.2, 1.0), (2.5, 0.85), (3.0, 0.6)], mach)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aero::thrust;
    use crate::sim::{atmosphere_at, Type, G0};

    fn flow(alpha: f64, mach: f64, sound: f64) -> Flow {
        Flow { alpha, speed: mach * sound, mach, xcg: XCG_REF, h_over_b: 10.0, kind: Type::Mig29, ..Default::default() }
    }

    /// Wind-axis lift and drag coefficients.
    fn wind(c: &Coeffs, alpha: f64) -> (f64, f64) {
        let (sa, ca) = (alpha.to_radians().sin(), alpha.to_radians().cos());
        (-c.cz * ca + c.cx * sa, -c.cx * ca - c.cz * sa)
    }

    /// Lift, drag and full-afterburner thrust (N) at an AoA, Mach and altitude.
    fn forces(alpha: f64, mach: f64, alt: f64) -> (f64, f64, f64) {
        let atm = atmosphere_at(alt);
        let qs = 0.5 * atm.rho * (mach * atm.sound).powi(2) * WING_AREA;
        let (cl, cd) = wind(&coefficients(&flow(alpha, mach, atm.sound)), alpha);
        (qs * cl, qs * cd, thrust(Type::Mig29, 100.0, mach, alt))
    }

    /// AoA where lift plus the thrust's share reaches `n` g at `mass`.
    fn alpha_for(n: f64, mass: f64, mach: f64, alt: f64) -> f64 {
        let (mut lo, mut hi) = (-5.0, 25.0);
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            let (l, _, t) = forces(mid, mach, alt);
            if l + t * mid.to_radians().sin() < n * mass * G0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Acceleration along the path in a turn at `n` g, m/s^2.
    fn excess(n: f64, mass: f64, mach: f64, alt: f64) -> f64 {
        let a = alpha_for(n, mass, mach, alt);
        let (_, d, t) = forces(a, mach, alt);
        (t * a.to_radians().cos() - d) / mass
    }

    #[test]
    fn lift_matches_the_manual() {
        let atm = atmosphere_at(1000.0);
        let (cl, _) = wind(&coefficients(&flow(24.0, 0.3, atm.sound)), 24.0);
        assert!((cl - 1.43).abs() < 0.06, "Cy at 24 deg {cl:.2}");
        let (_, cd) = wind(&coefficients(&flow(0.0, 0.5, atm.sound)), 0.0);
        assert!((cd - 0.022).abs() < 0.003, "Cx0 {cd:.4}");
    }

    #[test]
    fn turns_match_the_manual_charts() {
        // 6.6 g sustained at Mach 0.9 and 5,000 m with 1,500 kg of fuel.
        let mass = 11200.0 + 1500.0;
        let (mut lo, mut hi) = (1.0, 12.0);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if excess(mid, mass, 0.9, 5000.0) > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        assert!(lo > 6.1 && lo < 7.2, "sustained {lo:.2} g");
        // 6 g at 1,000 m and 900 km/h, full afterburner: about 20 km/h a second.
        let mach = 250.0 / atmosphere_at(1000.0).sound;
        let gain = excess(6.0, 13000.0, mach, 1000.0) * 3.6;
        assert!(gain > 13.0 && gain < 27.0, "{gain:.1} km/h a second");
    }

    #[test]
    fn top_speeds_match_the_published_figures() {
        let top = |alt: f64, mass: f64| {
            let (mut lo, mut hi) = (0.8, 3.0);
            for _ in 0..40 {
                let mid = 0.5 * (lo + hi);
                if excess(1.0, mass, mid, alt) > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            lo
        };
        let sl = top(0.0, 13000.0);
        assert!(sl > 1.15 && sl < 1.32, "sea level Mach {sl:.2}");
        let high = top(11000.0, 13000.0);
        assert!(high > 2.15 && high < 2.45, "11 km Mach {high:.2}");
    }

    #[test]
    fn stable_in_pitch_with_the_expected_control_signs() {
        let s = atmosphere_at(3000.0).sound;
        let cm = |alpha: f64, el: f64| coefficients(&Flow { el, ..flow(alpha, 0.6, s) }).cm;
        assert!(cm(8.0, 0.0) < cm(4.0, 0.0), "nose drops as AoA grows");
        assert!(cm(4.0, 10.0) < cm(4.0, 0.0), "TE-down stabilator pitches nose down");
        assert!(cm(24.0, -25.0) > 0.0, "full aft stick holds the limit AoA");
    }
}
