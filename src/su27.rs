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
    let mut a=crate::flightgear::build(Type::Su27);
    a.body.round_shading(|p|p.z < -4.7 && p.x.abs()<0.9 || p.z>5.8 && p.y<0.7 && p.x.abs()<1.9);
    for side in [-1.0,1.0] {
        let p=v3(side*2.2,0.3464,0.14);let q=v3(side*7.15,0.3464,4.2485);
        let n=v3(-side*0.83,0.0,1.0);
        let(fixed,slat)=crate::articulation::extract(a.body,&[(n,n.dot(p)),(-V3::X*side,-2.2),(V3::X*side,7.15),(-V3::Y,-0.18),(V3::Y,0.60)]);
        a.body=fixed;a.controls.push(crate::articulation::Surface{mesh:slat,
            hinge:crate::articulation::Hinge{pivot:p,axis:((q-p)*side).norm(),drive:crate::articulation::Drive::Slat,side}});
    }
    a.body.v.extend(crate::hornet::exterior_tub(Type::Su27).v);
    a
}
pub(crate) fn brake_hinge() -> V3 {
    crate::flightgear::native(Type::Su27, v3(8.95, 1.176, 0.))
}
pub(crate) fn flap_hinge(side:f64)->(V3,V3) {
    let a=v3(side*2.16955,0.29362,3.44078);
    let b=v3(side*5.34686,0.31700,4.85649);
    (a,((b-a)*side).norm())
}
pub(crate) fn rudder_hinge(side:f64)->(V3,V3) {
    (v3(side*2.12870,0.26727,6.15312),v3(0.0,2.21600,0.46366).norm())
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
