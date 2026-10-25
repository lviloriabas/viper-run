//! F/A-18C Lot 20, procedural geometry in metres about the mass datum.
//! Distinct LEX, moderately swept wing, canted twin fins, twin F404 bays,
//! nose gun, centre stick cockpit and naval undercarriage. Shared mesh
//! primitives also construct the Su-27 from its own stations and profiles.
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
pub const EYE: V3 = v3(0.0, 1.35, -3.8);
pub const NOZZLES: [V3; 2] = [v3(-0.72, -0.15, 6.8), v3(0.72, -0.15, 6.8)];
pub const STAB_PIVOT: V3 = v3(1.05, 0.15, 6.05);
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (v3(0.0, -0.3, -3.0), V3::X, -95.0 * DEG),
    (v3(-1.0, -0.15, 1.25), V3::Z, -90.0 * DEG),
    (v3(1.0, -0.15, 1.25), V3::Z, 90.0 * DEG),
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
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    let side = at.x.signum();
    let (from, to) = if x > 4.6 {
        (v3(4.8, 0.30, 0.55), v3(6.10, 0.20, 0.70))
    } else if x > 3.8 {
        (v3(4.15, -0.06, 1.0), v3(4.1, -0.20, 0.85))
    } else if x > 2.9 {
        (v3(3.5, -0.06, 0.52), v3(2.65, -0.35, 0.15))
    } else {
        (v3(2.3, -0.08, -0.28), v3(1.25, -0.62, -1.1))
    };
    let d = to - from;
    v3(side * d.x, d.y, d.z)
}
const SKIN: Rgb = Rgb(0.58, 0.62, 0.65);
const BELLY: Rgb = Rgb(0.70, 0.73, 0.75);
const METAL: Rgb = Rgb(0.31, 0.33, 0.34);
const BLACK: Rgb = Rgb(0.045, 0.055, 0.06);
pub fn build() -> Airframe {
    build_twin(false)
}
pub(crate) fn frame(at: V3) -> Frame {
    Frame {
        o: at,
        x: V3::X,
        y: V3::Y,
        z: V3::Z,
    }
}
fn face(m: &mut Mesh, pts: &[V3], normal: V3, col: Rgb) {
    let centre = pts.iter().copied().fold(V3::ZERO, |a, b| a + b) / pts.len() as f64;
    let winding = pts.iter().enumerate().fold(V3::ZERO, |sum, (i, p)| {
        sum + (*p - centre).cross(pts[(i + 1) % pts.len()] - centre)
    });
    if winding.dot(normal) >= 0.0 {
        m.polygon(pts, col)
    } else {
        m.polygon(&pts.iter().rev().copied().collect::<Vec<_>>(), col)
    }
}
fn plate(m: &mut Mesh, points: &[V3], normal: V3, thickness: f64, col: Rgb) {
    let hi: Vec<_> = points.iter().map(|p| *p + normal * thickness).collect();
    let lo: Vec<_> = points.iter().map(|p| *p - normal * thickness).collect();
    face(m, &hi, normal, col);
    face(m, &lo, -normal, col.scale(0.85));
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        m.quad(hi[i], hi[j], lo[j], lo[i], col.scale(0.9));
    }
}
fn section(table: &[[f64; 5]], z: f64) -> [f64; 5] {
    let i = (0..table.len() - 1)
        .find(|i| z <= table[i + 1][0])
        .unwrap_or(table.len() - 2);
    let t = ((z - table[i][0]) / (table[i + 1][0] - table[i][0])).clamp(0.0, 1.0);
    std::array::from_fn(|k| mix(table[i][k], table[i + 1][k], t))
}
pub(crate) fn build_twin(flanker: bool) -> Airframe {
    let mut body = Mesh::new();
    let profiles: &[[f64; 5]] = if flanker {
        &[
            [-10.2, 0.025, 0.20, 0.03, 0.03],
            [-9.3, 0.26, 0.25, 0.28, 0.25],
            [-7.5, 0.47, 0.36, 0.48, 0.42],
            [-6.2, 0.55, 0.47, 0.58, 0.45],
            [-4.5, 0.62, 0.58, 0.55, 0.45],
            [-2.5, 0.85, 0.55, 0.50, 0.48],
            [0.0, 1.15, 0.48, 0.48, 0.45],
            [3.0, 1.1, 0.45, 0.42, 0.38],
            [6.0, 0.8, 0.42, 0.32, 0.30],
            [9.0, 0.50, 0.40, 0.20, 0.20],
            [10.4, 0.10, 0.40, 0.06, 0.06],
        ]
    } else {
        &[
            [-8.35, 0.025, 0.20, 0.03, 0.03],
            [-7.5, 0.25, 0.24, 0.25, 0.25],
            [-6.3, 0.43, 0.32, 0.40, 0.36],
            [-5.0, 0.51, 0.43, 0.48, 0.42],
            [-3.8, 0.56, 0.47, 0.50, 0.45],
            [-2.0, 0.68, 0.45, 0.44, 0.45],
            [0.0, 0.88, 0.38, 0.46, 0.45],
            [2.5, 0.85, 0.33, 0.40, 0.40],
            [5.0, 0.68, 0.25, 0.33, 0.30],
            [6.9, 0.48, 0.2, 0.20, 0.18],
        ]
    };
    let (z0, z1) = (profiles[0][0], profiles.last().unwrap()[0]);
    let upper = |p: V3, n: V3| {
        if p.z < if flanker { -7.2 } else { -6.4 } {
            if flanker {
                Rgb(0.74, 0.78, 0.71)
            } else {
                Rgb(0.64, 0.66, 0.65)
            }
        } else if n.y < -0.3 {
            BELLY
        } else if flanker {
            let pattern = (p.x * 1.1 + p.z * 0.5).sin() + (p.z * 0.9).cos();
            if pattern > 0.6 {
                Rgb(0.38, 0.50, 0.60)
            } else if pattern < -0.6 {
                Rgb(0.62, 0.69, 0.73)
            } else {
                Rgb(0.49, 0.61, 0.69)
            }
        } else {
            SKIN
        }
    };
    body.surface(
        42,
        24,
        |u, v| {
            let z = mix(z0, z1, u);
            let s = section(profiles, z);
            let ph = v * TAU;
            v3(
                s[1] * ph.cos(),
                s[2] + if ph.sin() >= 0.0 { s[3] } else { s[4] } * ph.sin(),
                z,
            )
        },
        upper,
        Some(&|p: V3| v3(0.0, section(profiles, p.z)[2], p.z)),
    );
    if flanker {
        body.capsule(
            v3(0.0, 0.2, -10.15),
            v3(0.0, 0.2, -11.45),
            0.035,
            0.008,
            METAL,
            5,
        );
        body.ellipsoid(
            v3(0.0, 1.12, -6.0),
            v3(0.16, 0.16, 0.20),
            Rgb(0.18, 0.28, 0.32),
            6,
            12,
        );
    } else {
        body.capsule(
            v3(0.0, 0.2, -8.30),
            v3(0.0, 0.2, -9.02),
            0.035,
            0.008,
            METAL,
            5,
        );
        body.ellipsoid(v3(0.0, 0.70, -6.92), v3(0.055, 0.045, 0.08), BLACK, 4, 8);
    }
    let (engine_x, engine_y, intake, nozzle, rad) = if flanker {
        (1.40, -0.20, -2.6, 8.85, 0.60)
    } else {
        (0.72, -0.15, -2.0, 6.8, 0.47)
    };
    for side in [-1.0, 1.0] {
        let n = v3(side * engine_x, engine_y, nozzle);
        body.lathe(
            v3(side * engine_x, engine_y, intake),
            V3::Z,
            &[
                (0.0, rad * 0.98),
                (1.2, rad),
                (nozzle - intake - 0.7, rad * 0.94),
                (nozzle - intake, rad * 0.84),
            ],
            18,
            if flanker { Rgb(0.47, 0.53, 0.57) } else { SKIN },
        );
        body.lathe(
            n - V3::Z * 0.55,
            V3::Z,
            &[(0.0, rad * 0.94), (0.55, rad * 0.84)],
            20,
            METAL,
        );
        body.lathe(
            n + V3::Z * 0.012,
            V3::Z,
            &[(0.0, rad * 0.76), (0.02, 0.0)],
            20,
            BLACK,
        );
        let f = frame(v3(side * engine_x, engine_y, intake - 0.03));
        body.block(&f, v3(rad * 0.88, rad * 0.72, 0.015), BLACK);
        for sy in [-1.0, 1.0] {
            body.capsule(
                f.p(-rad, sy * rad * 0.78, 0.0),
                f.p(rad, sy * rad * 0.78, 0.0),
                0.055,
                0.055,
                SKIN,
                5,
            );
        }
        for sx in [-1.0, 1.0] {
            body.capsule(
                f.p(sx * rad, -rad * 0.78, 0.0),
                f.p(sx * rad, rad * 0.78, 0.0),
                0.055,
                0.055,
                SKIN,
                5,
            );
        }
    }
    let eye = if flanker { crate::su27::EYE } else { EYE };
    let canopy_start = eye.z - 1.45;
    let canopy_end = eye.z + 1.55;
    let mut canopy = Mesh::new();
    let canopy_point = |u: f64, v: f64| {
        let z = mix(canopy_start, canopy_end, u);
        let shape = (PI * u).sin().max(0.0).powf(0.45);
        let radius = 0.49 * shape + 0.06;
        let base = section(profiles, z);
        let y = base[2] + base[3] - 0.06;
        v3(
            radius * (v * PI).cos(),
            y + (0.75 * shape + 0.08) * (v * PI).sin(),
            z,
        )
    };
    canopy.surface(
        20,
        18,
        canopy_point,
        |_, _| Rgb(0.35, 0.49, 0.52),
        Some(&|p: V3| v3(0.0, 0.9, p.z)),
    );
    for u in [0.18, 0.98] {
        for j in 0..18 {
            body.capsule(
                canopy_point(u, j as f64 / 18.0),
                canopy_point(u, (j + 1) as f64 / 18.0),
                0.022,
                0.022,
                METAL,
                5,
            );
        }
    }
    for edge in [0.0, 1.0] {
        for j in 0..20 {
            body.capsule(
                canopy_point(j as f64 / 20.0, edge),
                canopy_point((j + 1) as f64 / 20.0, edge),
                0.035,
                0.035,
                SKIN,
                5,
            );
        }
    }
    let mut stabs = [Mesh::new(), Mesh::new()];
    let mut flaps = [Mesh::new(), Mesh::new()];
    let mut rudders = [Mesh::new(), Mesh::new()];
    for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
        let wing: Vec<_> = if flanker {
            [
                (2.0, 0.32, -0.6),
                (7.35, 0.22, 4.25),
                (7.35, 0.22, 5.85),
                (2.0, 0.32, 5.5),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        } else {
            [
                (1.20, 0.30, -0.6),
                (5.72, 0.20, 1.04),
                (5.72, 0.20, 2.84),
                (1.20, 0.30, 3.58),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        };
        let (root_x, tip_x) = (wing[0].x.abs(), wing[1].x.abs());
        let (flap_x0, flap_x1) = if flanker { (2.3, 5.1) } else { (1.5, 4.8) };
        let leading = |x: f64| wing[0].lerp(wing[1], (x - root_x) / (tip_x - root_x));
        let trailing = |x: f64| wing[3].lerp(wing[2], (x - root_x) / (tip_x - root_x));
        // Three solid panels leave the moving trailing-edge flap's actual
        // footprint empty, rather than drawing another wing through it.
        for (x0, x1, notch) in [
            (root_x, flap_x0, false),
            (flap_x0, flap_x1, true),
            (flap_x1, tip_x, false),
        ] {
            let recess = if notch { V3::Z * 0.65 } else { V3::ZERO };
            plate(
                &mut body,
                &[
                    leading(x0),
                    leading(x1),
                    trailing(x1) - recess,
                    trailing(x0) - recess,
                ],
                V3::Y,
                0.065,
                if flanker { Rgb(0.48, 0.61, 0.69) } else { SKIN },
            );
        }
        let lerx: Vec<_> = if flanker {
            [
                (0.48, 0.83, -5.45),
                (1.2, 0.75, -3.8),
                (2.5, 0.4, -0.4),
                (2.0, 0.32, 2.0),
                (0.7, 0.55, 1.8),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        } else {
            [
                (0.44, 0.91, -4.9),
                (0.90, 0.70, -3.7),
                (1.68, 0.35, -1.1),
                (1.20, 0.3, 1.2),
                (0.65, 0.52, 0.8),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        };
        plate(
            &mut body,
            &lerx,
            V3::Y,
            0.025,
            if flanker { Rgb(0.53, 0.63, 0.70) } else { SKIN },
        );
        let tail: Vec<_> = if flanker {
            [
                (1.3, 0.18, 6.5),
                (4.8, 0.12, 8.35),
                (4.75, 0.12, 10.35),
                (1.3, 0.18, 9.25),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        } else {
            [
                (0.85, 0.16, 4.6),
                (3.29, 0.12, 6.1),
                (3.14, 0.12, 7.95),
                (0.85, 0.16, 6.95),
            ]
            .map(|(x, y, z)| v3(side * x, y, z))
            .to_vec()
        };
        plate(&mut stabs[i], &tail, V3::Y, 0.045, SKIN);
        let (root, top, front, back, cant) = if flanker {
            (1.8, 3.9, 5.0, 9.0, 6.0_f64)
        } else {
            (0.94, 3.05, 3.1, 6.8, 20.0_f64)
        };
        let fin = |y: f64, z: f64| v3(side * (root + (y - 0.5) * cant.to_radians().tan()), y, z);
        plate(
            &mut body,
            &[
                fin(0.5, front),
                fin(top, front + 1.6),
                fin(top - 0.05, back - 0.45),
                fin(0.5, back - 0.6),
            ],
            V3::X * side,
            0.035,
            if flanker { Rgb(0.52, 0.63, 0.70) } else { SKIN },
        );
        // Rudder trailing strips are independent moving meshes.
        plate(
            &mut rudders[i],
            &[
                fin(0.65, back - 0.6),
                fin(top - 0.12, back - 0.45),
                fin(top - 0.05, back - 0.2),
                fin(0.5, back),
            ],
            V3::X * side,
            0.04,
            SKIN,
        );
        if flanker {
            plate(
                &mut body,
                &[
                    v3(side * 1.55, -0.45, 6.0),
                    v3(side * 2.1, -1.0, 8.0),
                    v3(side * 1.55, -0.45, 8.4),
                ],
                V3::X * side,
                0.025,
                SKIN,
            );
        }
        plate(
            &mut flaps[i],
            &[
                trailing(flap_x0) - V3::Z * 0.65,
                trailing(flap_x1) - V3::Z * 0.65,
                trailing(flap_x1),
                trailing(flap_x0),
            ],
            V3::Y,
            0.035,
            SKIN.scale(0.95),
        );
        // Pylons and wingtip rails.
        let pylons: &[f64] = if flanker {
            &[1.4, 3.1, 4.8]
        } else {
            &[2.65, 4.1]
        };
        for &x in pylons {
            body.block(
                &frame(v3(side * x, -0.04, if flanker { 2.2 } else { 1.7 })),
                v3(0.045, 0.23, 0.70),
                SKIN,
            );
        }
        let tip = if flanker { 7.35 } else { 6.10 };
        if !flanker {
            body.capsule(
                v3(side * 5.72, 0.2, 1.2),
                v3(side * 6.1, 0.2, 1.2),
                0.055,
                0.055,
                METAL,
                6,
            );
        }
        body.capsule(
            v3(side * tip, 0.2, if flanker { 3.4 } else { 0.4 }),
            v3(side * tip, 0.2, if flanker { 5.4 } else { 2.4 }),
            0.055,
            0.055,
            METAL,
            6,
        );
        // Visible national markings; no external texture or asset dependency.
        let c = v3(side * (root + 0.10), 1.9, back - 0.9);
        if flanker {
            let n = V3::X * side;
            let e1 = V3::Z;
            let e2 = V3::Y;
            for k in 0..5 {
                let a = k as f64 * TAU / 5.0;
                let b = a + TAU / 10.0;
                let c2 = a + TAU / 5.0;
                face(
                    &mut body,
                    &[
                        c,
                        c + (e1 * a.sin() + e2 * a.cos()) * 0.33,
                        c + (e1 * b.sin() + e2 * b.cos()) * 0.14,
                    ],
                    n,
                    Rgb(0.78, 0.12, 0.12),
                );
                face(
                    &mut body,
                    &[
                        c,
                        c + (e1 * b.sin() + e2 * b.cos()) * 0.14,
                        c + (e1 * c2.sin() + e2 * c2.cos()) * 0.33,
                    ],
                    n,
                    Rgb(0.78, 0.12, 0.12),
                );
            }
        }
    }
    let mut brake = Mesh::new();
    let z = if flanker { 2.8 } else { 3.2 };
    plate(
        &mut brake,
        &[
            v3(-0.47, 0.87, z),
            v3(0.47, 0.87, z),
            v3(0.38, 0.84, z + 1.45),
            v3(-0.38, 0.84, z + 1.45),
        ],
        V3::Y,
        0.035,
        SKIN,
    );
    let mut gear = [Mesh::new(), Mesh::new(), Mesh::new()];
    let wheels = if flanker {
        crate::sim::Type::Su27.wheels()
    } else {
        crate::sim::Type::F18.wheels()
    };
    for (i, w) in wheels.iter().enumerate() {
        let p = w.local + V3::Y * 0.30;
        let top = v3(p.x * 0.67, -0.2, p.z + if i == 0 { 0.55 } else { -0.15 });
        gear[i].capsule(top, p, 0.065, 0.075, Rgb(0.72, 0.73, 0.70), 8);
        gear[i].capsule(
            top + v3(if i == 0 { 0.0 } else { -p.x.signum() * 0.4 }, 0.12, 0.35),
            p + V3::Y * 0.35,
            0.035,
            0.035,
            METAL,
            6,
        );
        let wheel = |m: &mut Mesh, x: f64| {
            m.lathe(
                p + V3::X * x - V3::X * 0.10,
                V3::X,
                &[(0.0, 0.23), (0.035, 0.30), (0.17, 0.30), (0.20, 0.23)],
                12,
                BLACK,
            );
            m.lathe(
                p + V3::X * x - V3::X * 0.102,
                V3::X,
                &[(0.0, 0.14), (0.204, 0.14)],
                12,
                Rgb(0.62, 0.64, 0.61),
            );
        };
        if i == 0 && !flanker {
            wheel(&mut gear[i], -0.15);
            wheel(&mut gear[i], 0.15);
        } else {
            wheel(&mut gear[i], 0.0);
        }
        gear[i].block(
            &frame(top + v3(0.0, -0.15, -0.15)),
            v3(0.15, 0.35, 0.035),
            SKIN,
        );
    }
    Airframe {
        body,
        canopy,
        stabs,
        flaps,
        rudders,
        brake,
        gear,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silhouettes_are_finite_distinct_and_correctly_sized() {
        for (flanker, span, length) in [(false, 12.2, 17.0), (true, 14.7, 21.8)] {
            let a = build_twin(flanker);
            let xs: Vec<_> = a
                .body
                .v
                .chunks(crate::mesh::STRIDE)
                .map(|v| v[0] as f64)
                .collect();
            let zs: Vec<_> = a
                .body
                .v
                .chunks(crate::mesh::STRIDE)
                .chain(a.stabs.iter().flat_map(|m| m.v.chunks(crate::mesh::STRIDE)))
                .map(|v| v[2] as f64)
                .collect();
            let range = |v: &[f64]| {
                v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    - v.iter().copied().fold(f64::INFINITY, f64::min)
            };
            assert!((range(&xs) - span).abs() < 0.4, "span {}", range(&xs));
            assert!(range(&zs) > length - 1.0, "length {}", range(&zs));
            assert!(a.body.v.iter().all(|v| v.is_finite()));
            assert!(
                a.body.count() > 3000
                    && a.canopy.count() > 1000
                    && a.gear.iter().all(|m| m.count() > 200)
            );
        }
    }
}
