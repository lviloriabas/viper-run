//! Su-27SK Flanker-B, adapted from the pinned FlightGear Su-27SK mesh.
//! The original wing, inlets, radome, canopy and tail topology is retained.
use crate::{hornet::Airframe, math::*, sim::Type};
pub const EYE: V3 = v3(0., 1.368786885, -5.767588036);
pub const NOZZLES: [V3; 2] = [v3(-1.27, -0.19, 8.08), v3(1.27, -0.19, 8.08)];
pub const STAB_PIVOT: V3 = v3(1.92787, -0.15905, 6.4637);
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (
        v3(0.0000000000, 0.1445901639, -3.3572075139),
        V3::X,
        95.0 * DEG,
    ),
    (
        v3(-1.5712131148, -0.0578360656, 2.3070696135),
        V3::X,
        92.0 * DEG,
    ),
    (
        v3(1.5712131148, -0.0578360656, 2.3070696135),
        V3::X,
        92.0 * DEG,
    ),
];
pub fn build() -> Airframe {
    crate::flightgear::build(Type::Su27)
}
pub(crate) fn brake_hinge() -> V3 {
    crate::flightgear::native(Type::Su27, v3(8.95, 1.176, 0.))
}
pub(crate) fn flap_hinge(side: f64) -> (V3, V3) {
    {
        let a = crate::flightgear::native(Type::Su27, v3(14.385, 0.2, -side * 2.15));
        let b = crate::flightgear::native(Type::Su27, v3(15.700, 0.2, -side * 5.25));
        ((a + b) * 0.5, ((b - a) * side).norm())
    }
}
pub(crate) fn rudder_hinge(side: f64) -> (V3, V3) {
    let a = crate::flightgear::native(Type::Su27, v3(17.3, 0.672, -side * 2.04));
    let b = crate::flightgear::native(Type::Su27, v3(17.9, 2.7, -side * 2.04));
    ((a + b) * 0.5, (b - a).norm())
}
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    let side = at.x.signum();
    let p = if x < 0.1 {
        v3(if at.z < 0. { 12.10 } else { 16.15 }, -0.55, 0.)
    } else if x > 4.6 {
        v3(15.40, 0.147, -side * 7.555)
    } else if x > 3.8 {
        v3(14.356, -0.188, -side * 4.849)
    } else if x > 2.9 {
        v3(13.70, -0.188, -side * 3.40)
    } else {
        v3(13.80, -1.30, -side * 1.22)
    };
    crate::flightgear::native(Type::Su27, p) - at
}
