//! Legacy F/A-18C exterior adapted from Fabrice Kauffmann's FlightGear model.
//! Editable originals, exact transforms and licence are in assets/models.
use crate::{
    math::*,
    mesh::{Frame, Mesh, Rgb},
    sim::Type,
};
pub const EYE: V3 = v3(0., 1.00395, -3.6275048);
pub const NOZZLES: [V3; 2] = [v3(-0.68, -0.001, 7.90), v3(0.68, -0.001, 7.90)];
pub const STAB_PIVOT: V3 = v3(1.06, -0.06, 6.48);
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (v3(0., -0.4, -1.5), V3::X, -95. * DEG),
    (v3(-0.9, -0.45, 1.7), V3::Z, -90. * DEG),
    (v3(0.9, -0.45, 1.7), V3::Z, 90. * DEG),
];
pub struct Airframe {
    pub body: Mesh,
    pub canopy: Mesh,
    pub stabs: [Mesh; 2],
    pub flaps: [Mesh; 2],
    pub rudders: [Mesh; 2],
    pub brake: Mesh,
    pub gear: [Mesh; 3],
}
pub fn build() -> Airframe {
    let mut a = crate::flightgear::build(Type::F18);
    // Single-store pylons replace the source's permanent twin Sidewinder racks.
    for side in [-1., 1.] {
        for (x, z) in [(2.5, 1.65), (3.75, 1.85)] {
            let outline = [
                v3(side * x - 0.045, 0.30, z - 0.70),
                v3(side * x - 0.045, 0.30, z + 0.60),
                v3(side * x - 0.045, -0.28, z + 0.48),
                v3(side * x - 0.045, -0.36, z - 0.42),
                v3(side * x - 0.045, -0.16, z - 0.66),
            ];
            let other = outline.map(|p| p + V3::X * 0.09);
            a.body.polygon(&outline, Rgb(0.55, 0.59, 0.61));
            a.body.polygon(&other, Rgb(0.55, 0.59, 0.61));
            for k in 0..5 {
                let j = (k + 1) % 5;
                a.body.quad(
                    outline[k],
                    other[k],
                    other[j],
                    outline[j],
                    Rgb(0.49, 0.53, 0.55),
                );
            }
            let f = Frame {
                o: v3(side * x, -0.38, z),
                x: V3::X,
                y: V3::Y,
                z: V3::Z,
            };
            a.body
                .block(&f, v3(0.075, 0.035, 0.70), Rgb(0.31, 0.34, 0.35));
        }
        let f = Frame {
            o: v3(side * 0.90, -0.61, 0.35),
            x: V3::X,
            y: V3::Y,
            z: V3::Z,
        };
        a.body
            .block(&f, v3(0.045, 0.04, 0.67), Rgb(0.35, 0.38, 0.39));
    }
    // Fit the curved door to the imported spine, including its transverse
    // section. A flat plate here visibly floats above the sloping rear body.
    let point = |u: f64, v: f64| {
        let z = mix(4.90, 6.20, u);
        let x = mix(-1.0, 1.0, v) * mix(0.40, 0.36, u);
        v3(x, skin_top(&a.body, x, z) + 0.008, z)
    };
    for i in 0..12 {
        for j in 0..8 {
            let (u, v) = (i as f64 / 12.0, j as f64 / 8.0);
            a.brake.quad(
                point(u, v),
                point(u, v + 0.125),
                point(u + 1.0 / 12.0, v + 0.125),
                point(u + 1.0 / 12.0, v),
                Rgb(0.55, 0.60, 0.63),
            );
        }
    }
    for j in 0..8 {
        for u in [0.0, 1.0] {
            let p = point(u, j as f64 / 8.0);
            let q = point(u, (j + 1) as f64 / 8.0);
            a.brake.quad(
                p,
                q,
                q - V3::Y * 0.015,
                p - V3::Y * 0.015,
                Rgb(0.25, 0.28, 0.30),
            );
        }
    }
    for i in 0..12 {
        for v in [0.0, 1.0] {
            let p = point(i as f64 / 12.0, v);
            let q = point((i + 1) as f64 / 12.0, v);
            a.brake.quad(
                p,
                q,
                q - V3::Y * 0.015,
                p - V3::Y * 0.015,
                Rgb(0.25, 0.28, 0.30),
            );
        }
    }
    a
}
fn skin_top(m: &Mesh, x: f64, z: f64) -> f64 {
    let mut top = f64::NEG_INFINITY;
    for t in m.v.chunks(crate::mesh::STRIDE * 3) {
        let p = |k: usize| v3(t[k * 11] as f64, t[k * 11 + 1] as f64, t[k * 11 + 2] as f64);
        let a = p(0);
        let b = p(1) - a;
        let c = p(2) - a;
        let q = v3(x - a.x, 0.0, z - a.z);
        let d = b.x * c.z - c.x * b.z;
        if d.abs() < 1e-12 {
            continue;
        }
        let u = (q.x * c.z - c.x * q.z) / d;
        let v = (b.x * q.z - q.x * b.z) / d;
        if u >= -1e-7 && v >= -1e-7 && u + v <= 1.0000001 {
            top = top.max(a.y + u * b.y + v * c.y);
        }
    }
    assert!(top.is_finite());
    top
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stowed_speedbrake_follows_the_imported_spine() {
        let a = build();
        for v in a.brake.v.chunks(crate::mesh::STRIDE) {
            let gap = v[1] as f64 - skin_top(&a.body, v[0] as f64, v[2] as f64);
            assert!(gap > -0.008 && gap < 0.009, "floating speedbrake: {gap}");
        }
        let hinge = brake_hinge(false);
        assert!((hinge.y - skin_top(&a.body, 0.0, hinge.z) - 0.008).abs() < 1e-6);
    }
}
pub(crate) fn canopy_point(flanker: bool, z: f64, phi: f64) -> V3 {
    let kind = if flanker { Type::Su27 } else { Type::F18 };
    let (w, sill, crown) = crate::flightgear::canopy_section(kind, z);
    v3(w * phi.cos(), sill + (crown - sill) * phi.sin().max(0.), z)
}
pub(crate) fn canopy_bow(flanker: bool) -> f64 {
    if flanker {
        -6.32
    } else {
        -4.43
    }
}
pub(crate) fn brake_hinge(flanker: bool) -> V3 {
    if flanker {
        crate::su27::brake_hinge()
    } else {
        v3(0., 0.680112279, 4.90)
    }
}
pub(crate) fn flap_hinge(flanker: bool, side: f64) -> (V3, V3) {
    if flanker {
        return crate::su27::flap_hinge(side);
    }
    (
        v3(side * 2.55, 0.45, 3.05),
        v3(1., side * 0.015, side * 0.028).norm(),
    )
}
pub(crate) fn rudder_hinge(flanker: bool, side: f64) -> (V3, V3) {
    if flanker {
        return crate::su27::rudder_hinge(side);
    }
    (
        v3(side * 1.08, 0.72, 5.24),
        v3(side * 0.35, 0.93, -0.13).norm(),
    )
}
/// Compatibility translation for station-only consumers. Store geometry uses
/// Station::position, which accounts for the missile's nose origin.
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    let side = at.x.signum();
    let to = if x > 4.6 {
        v3(5.78, 0.46, 1.90)
    } else if x > 3.8 {
        v3(3.75, -0.48, 1.85)
    } else if x > 2.9 {
        v3(2.50, -0.48, 1.65)
    } else {
        v3(0.90, -0.76, 0.35)
    };
    v3(to.x * side, to.y, to.z) - at
}
