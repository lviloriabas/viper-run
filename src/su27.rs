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
