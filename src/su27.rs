//! Su-27SK base single-seat Flanker-B. Own long radome, broad blended wing,
//! separated AL-31F nacelles, ventral fins, IRST and central tail sting.
use crate::math::*;
pub const EYE: V3 = v3(0.0, 1.48, -4.60);
pub const NOZZLES: [V3; 2] = [v3(-1.4, -0.20, 8.85), v3(1.4, -0.20, 8.85)];
pub const STAB_PIVOT: V3 = v3(1.8, 0.15, 8.3);
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (v3(0.0, -0.4, -3.4), V3::X, -95.0 * DEG),
    (v3(-1.5, -0.3, 1.3), V3::Z, -90.0 * DEG),
    (v3(1.5, -0.3, 1.3), V3::Z, 90.0 * DEG),
];
/// z, half-width, centre height, upper/lower radius, section exponent.
/// The centre body spreads between the nacelles; the forward fuselage stays slim.
pub(crate) const FUSE: &[[f64; 6]] = &[
    [-10.20, 0.015, 0.16, 0.02, 0.02, 2.0],
    [-9.50, 0.24, 0.21, 0.24, 0.22, 2.0],
    [-8.50, 0.40, 0.29, 0.38, 0.34, 2.0],
    [-7.40, 0.51, 0.42, 0.50, 0.43, 2.1],
    [-6.20, 0.57, 0.52, 0.55, 0.46, 2.3],
    [-5.00, 0.63, 0.55, 0.53, 0.46, 2.4],
    [-3.80, 0.72, 0.54, 0.51, 0.47, 2.6],
    [-2.40, 1.04, 0.48, 0.49, 0.45, 3.0],
    [-0.50, 1.57, 0.40, 0.44, 0.39, 3.8],
    [1.80, 1.74, 0.38, 0.43, 0.35, 4.0],
    [4.00, 1.64, 0.38, 0.39, 0.29, 3.6],
    [6.00, 1.31, 0.37, 0.34, 0.24, 3.0],
    [7.80, 0.73, 0.39, 0.23, 0.20, 2.5],
    [9.40, 0.25, 0.42, 0.15, 0.13, 2.0],
    [10.40, 0.08, 0.42, 0.06, 0.06, 2.0],
];
pub fn build() -> crate::hornet::Airframe {
    crate::hornet::build_twin(true)
}
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    if x < 0.1 {
        return V3::ZERO;
    }
    let side = at.x.signum();
    let (from, to) = if x > 4.6 {
        (v3(4.8, 0.3, 0.55), v3(7.35, 0.2, 2.3))
    } else if x > 3.8 {
        (v3(4.15, -0.06, 1.0), v3(4.8, -0.2, 2.1))
    } else if x > 2.9 {
        (v3(3.5, -0.06, 0.52), v3(3.1, -0.25, 1.4))
    } else {
        (v3(2.3, -0.08, -0.28), v3(1.4, -1.03, -0.1))
    };
    let d = to - from;
    v3(side * d.x, d.y, d.z)
}
