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
/// Catmull-Rom through measured longitudinal stations, with clamped radii.
fn section<const N: usize>(table: &[[f64; N]], z: f64) -> [f64; N] {
    let i = (0..table.len() - 1)
        .find(|&i| z <= table[i + 1][0])
        .unwrap_or(table.len() - 2);
    let t = ((z - table[i][0]) / (table[i + 1][0] - table[i][0])).clamp(0.0, 1.0);
    let (a, b, c, d) = (
        table[i.saturating_sub(1)],
        table[i],
        table[i + 1],
        table[(i + 2).min(table.len() - 1)],
    );
    std::array::from_fn(|k| {
        0.5 * (2.0 * b[k]
            + (-a[k] + c[k]) * t
            + (2.0 * a[k] - 5.0 * b[k] + 4.0 * c[k] - d[k]) * t * t
            + (-a[k] + 3.0 * b[k] - 3.0 * c[k] + d[k]) * t * t * t)
    })
}

/// Canopy stations: z, half-width, sill and crown. Body and cockpit use
/// this same shape, so the bow and rails line up in both views.
pub(crate) fn canopy_point(flanker: bool, z: f64, phi: f64) -> V3 {
    let table: &[[f64; 4]] = if flanker {
        &[
            [-6.22, 0.24, 0.98, 1.15],
            [-5.80, 0.41, 1.03, 1.54],
            [-5.28, 0.49, 1.04, 1.90],
            [-4.60, 0.52, 1.02, 2.03],
            [-3.85, 0.51, 1.00, 2.00],
            [-3.18, 0.43, 0.98, 1.78],
            [-2.85, 0.31, 0.97, 1.41],
        ]
    } else {
        &[
            [-5.42, 0.20, 0.87, 1.02],
            [-5.00, 0.37, 0.91, 1.39],
            [-4.52, 0.48, 0.93, 1.75],
            [-3.80, 0.52, 0.92, 1.88],
            [-3.10, 0.50, 0.90, 1.83],
            [-2.48, 0.40, 0.88, 1.54],
            [-2.18, 0.28, 0.86, 1.14],
        ]
    };
    let [_, w, sill, top] = section(table, z);
    v3(
        w * phi.cos(),
        sill + (top - sill) * phi.sin().max(0.0).powf(0.88),
        z,
    )
}
pub(crate) fn canopy_bow(flanker: bool) -> f64 {
    if flanker {
        -5.28
    } else {
        -4.52
    }
}
pub(crate) fn brake_hinge(flanker: bool) -> V3 {
    if flanker {
        v3(0.0, 1.11, 0.4)
    } else {
        v3(0.0, 0.90, 3.2)
    }
}

/// Trailing-edge flap hinge follows the swept trailing edge on either wing.
pub(crate) fn flap_hinge(flanker: bool, side: f64) -> (V3, V3) {
    let (root, tip, te0, te1, y, x0, x1) = if flanker {
        (2.0, 7.35, 5.5, 5.85, 0.32, 2.3, 5.1)
    } else {
        (1.2, 5.72, 3.58, 2.84, 0.30, 1.5, 4.8)
    };
    let point = |x: f64| {
        v3(
            side * x,
            mix(
                y,
                if flanker { 0.22 } else { 0.20 },
                (x - root) / (tip - root),
            ),
            mix(te0, te1, (x - root) / (tip - root)) - 0.65,
        )
    };
    let (a, b) = (point(x0), point(x1));
    ((a + b) * 0.5, ((b - a) * side).norm())
}
/// The rudder turns about the inclined fin hinge, including outward cant.
pub(crate) fn rudder_hinge(flanker: bool, side: f64) -> (V3, V3) {
    let (root, top, back, cant) = if flanker {
        (1.8, 3.9, 9.0, 6.0 * DEG)
    } else {
        (0.94, 3.05, 6.8, 20.0 * DEG)
    };
    (
        v3(side * root, 0.5, back - 0.6),
        v3(side * cant.tan(), 1.0, -0.05 / (top - 0.5)).norm(),
    )
}

/// Thin NACA section, cosine spaced in chord. There is real thickness at
/// the root, a closed leading edge, and a separate moving trailing edge.
fn wing_panel(m: &mut Mesh, a: V3, b: V3, c: V3, d: V3, depth: f64, col: Rgb) {
    for upper in [true, false] {
        m.surface(
            5,
            12,
            |u, v| {
                let le = a.lerp(b, u);
                let te = d.lerp(c, u);
                let t = 0.5 * (1.0 - (PI * v).cos());
                let h = 5.0
                    * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                        - 0.1036 * t.powi(4));
                le.lerp(te, t)
                    + V3::Y * (if upper { 1.0 } else { -1.0 }) * h * depth * mix(1.0, 0.45, u)
            },
            |_, _| if upper { col } else { BELLY },
            Some(&|p: V3| {
                let u = ((p.x - a.x) / (b.x - a.x)).clamp(0.0, 1.0);
                let y = mix((a.y + d.y) * 0.5, (b.y + c.y) * 0.5, u);
                v3(p.x, y - if upper { 0.001 } else { -0.001 }, p.z)
            }),
        );
    }
    for (le, te) in [(a, d), (b, c)] {
        face(m, &[le, te, te - V3::Y * 0.008], V3::X * le.x.signum(), col);
    }
}

/// An intake has a raked rectangular lip, a deep dark duct and its own
/// square-to-round nacelle. It is not a cylinder with a black cap.
fn nacelle(m: &mut Mesh, side: f64, flanker: bool, x: f64, y: f64, front: f64, rear: f64, r: f64) {
    let point = |u: f64, v: f64| {
        let z = mix(front, rear - 0.55, u);
        let shape = section(
            &[
                [front, r * 1.04, r * 0.83, 4.0],
                [front + 1.0, r * 1.12, r * 0.98, 3.8],
                [front + 3.0, r, r * 1.05, 2.8],
                [rear - 1.6, r * 0.98, r * 0.98, 2.0],
                [rear - 0.55, r * 0.95, r * 0.95, 2.0],
            ],
            z,
        );
        let ph = v * TAU;
        let (c, s) = (ph.cos(), ph.sin());
        let n = shape[3];
        v3(
            side * x + shape[1] * c.signum() * c.abs().powf(2.0 / n),
            y + shape[2] * s.signum() * s.abs().powf(2.0 / n),
            z,
        )
    };
    m.surface(
        26,
        24,
        point,
        |p, n| {
            if p.z > rear - 2.5 {
                Rgb(0.48, 0.47, 0.44)
            } else if n.y < -0.3 {
                BELLY
            } else if flanker {
                Rgb(0.50, 0.63, 0.71)
            } else {
                SKIN
            }
        },
        Some(&|p: V3| v3(side * x, y, p.z)),
    );
    let mouth = |k: usize, depth: f64| {
        let (sx, sy) = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][k];
        v3(
            side * x + sx * r * (1.0 - depth * 0.12),
            y + sy * r * 0.78,
            front - sy * if flanker { 0.24 } else { 0.13 } + depth,
        )
    };
    for k in 0..4 {
        let j = (k + 1) % 4;
        m.quad(
            mouth(k, 0.0),
            mouth(j, 0.0),
            mouth(j, 0.80),
            mouth(k, 0.80),
            Rgb(0.18, 0.21, 0.22),
        );
        m.capsule(mouth(k, -0.02), mouth(j, -0.02), 0.028, 0.028, SKIN, 6);
    }
    face(
        m,
        &(0..4).map(|k| mouth(k, 0.81)).collect::<Vec<_>>(),
        -V3::Z,
        BLACK,
    );
    if flanker {
        // FOD mesh/ramp slats recessed into the upper intake.
        for j in 0..5 {
            let z = front + 0.12 + j as f64 * 0.11;
            m.capsule(
                v3(side * x - r * 0.86, y + r * 0.66, z),
                v3(side * x + r * 0.86, y + r * 0.66, z),
                0.015,
                0.015,
                METAL,
                4,
            );
        }
    } else {
        // Legacy Hornet splitter plate, separated from the cheek.
        let sx = side * (x - r - 0.055);
        plate(
            m,
            &[
                v3(sx, y - r * 0.75, front - 0.25),
                v3(sx, y + r * 0.85, front - 0.43),
                v3(sx, y + r * 0.85, front + 1.3),
                v3(sx, y - r * 0.75, front + 1.3),
            ],
            V3::X * side,
            0.018,
            SKIN,
        );
    }
    let n = v3(side * x, y, rear);
    m.lathe(
        n - V3::Z * 0.60,
        V3::Z,
        &[(0.0, r * 0.95), (0.46, r * 0.87), (0.60, r * 0.82)],
        24,
        METAL,
    );
    m.lathe(
        n - V3::Z * 0.10,
        V3::Z,
        &[(0.0, r * 0.73), (0.10, r * 0.73)],
        24,
        BLACK,
    );
    // Individual nozzle petals and an inner exhaust tunnel.
    for j in 0..24 {
        let ph = j as f64 * TAU / 24.0;
        let dir = v3(ph.cos(), ph.sin(), 0.0);
        m.capsule(
            n + dir * r * 0.82,
            n - V3::Z * 0.54 + dir * r * 0.94,
            0.009,
            0.012,
            Rgb(0.53, 0.51, 0.47),
            4,
        );
    }
    m.lathe(
        n - V3::Z * 0.35,
        V3::Z,
        &[(0.0, r * 0.72), (0.01, 0.0)],
        24,
        BLACK,
    );
}

pub(crate) fn build_twin(flanker: bool) -> Airframe {
    let mut body = Mesh::new();
    // Own station sets rather than scaling either existing aircraft.
    // z, half-width, centre, upper/lower height, superellipse exponent.
    let profiles: &[[f64; 6]] = if flanker {
        crate::su27::FUSE
    } else {
        &[
            [-9.10, 0.015, 0.18, 0.02, 0.02, 2.0],
            [-8.45, 0.19, 0.21, 0.20, 0.20, 2.0],
            [-7.40, 0.36, 0.29, 0.34, 0.32, 2.0],
            [-6.35, 0.47, 0.38, 0.43, 0.40, 2.2],
            [-5.30, 0.54, 0.44, 0.48, 0.44, 2.4],
            [-4.20, 0.59, 0.45, 0.50, 0.46, 2.5],
            [-3.00, 0.65, 0.44, 0.48, 0.48, 2.5],
            [-1.80, 0.88, 0.37, 0.48, 0.61, 3.0],
            [0.00, 1.18, 0.30, 0.51, 0.65, 3.2],
            [2.00, 1.23, 0.28, 0.50, 0.63, 3.2],
            [3.80, 1.18, 0.23, 0.49, 0.59, 2.8],
            [5.35, 1.00, 0.18, 0.39, 0.50, 2.5],
            [6.75, 0.35, 0.20, 0.20, 0.20, 2.0],
            [7.10, 0.13, 0.23, 0.08, 0.08, 2.0],
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
        68,
        32,
        |u, v| {
            let z = mix(z0, z1, u);
            let s = section(profiles, z);
            let ph = v * TAU;
            let (c, t) = (ph.cos(), ph.sin());
            let n = s[5];
            let x = s[1].max(0.005) * c.signum() * c.abs().powf(2.0 / n);
            let spine = if flanker { 0.26 } else { 0.18 }
                * smoothstep(
                    if flanker { -3.2 } else { -2.5 },
                    if flanker { -2.0 } else { -1.3 },
                    z,
                )
                * (1.0 - smoothstep(2.0, if flanker { 6.5 } else { 4.7 }, z))
                * (1.0 - (x / 0.48).powi(2)).max(0.0).powi(2);
            v3(
                x,
                s[2] + if t >= 0.0 {
                    s[3].max(0.005) * t.powf(2.0 / n) + spine
                } else {
                    -s[4].max(0.005) * (-t).powf(2.0 / n)
                },
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
            v3(0.0, 1.21, -6.20),
            v3(0.16, 0.16, 0.20),
            Rgb(0.18, 0.28, 0.32),
            6,
            12,
        );
    } else {
        // Nose-mounted M61 muzzle and the forward IFF antenna array.
        for x in [-0.12, -0.04, 0.04, 0.12] {
            body.block(
                &frame(v3(x, 0.91, -5.88)),
                v3(0.014, 0.055, 0.12),
                SKIN.scale(0.8),
            );
        }
        body.ellipsoid(v3(0.0, 0.70, -6.92), v3(0.055, 0.045, 0.08), BLACK, 4, 8);
    }
    let (engine_x, engine_y, intake, nozzle, rad) = if flanker {
        (1.40, -0.20, -2.6, 8.85, 0.60)
    } else {
        (0.72, -0.15, -2.0, 6.8, 0.47)
    };
    for side in [-1.0, 1.0] {
        nacelle(
            &mut body, side, flanker, engine_x, engine_y, intake, nozzle, rad,
        );
    }
    let (canopy_start, canopy_end) = if flanker {
        (-6.22, -2.85)
    } else {
        (-5.42, -2.18)
    };
    let mut canopy = Mesh::new();
    let canopy_point =
        |u: f64, v: f64| canopy_point(flanker, mix(canopy_start, canopy_end, u), v * PI);
    canopy.surface(
        20,
        18,
        canopy_point,
        |_, _| Rgb(0.35, 0.49, 0.52),
        Some(&|p: V3| v3(0.0, 0.9, p.z)),
    );
    for u in [
        (canopy_bow(flanker) - canopy_start) / (canopy_end - canopy_start),
        0.98,
    ] {
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
            wing_panel(
                &mut body,
                leading(x0),
                leading(x1),
                trailing(x1) - recess,
                trailing(x0) - recess,
                if flanker { 0.20 } else { 0.16 },
                if flanker { Rgb(0.48, 0.61, 0.69) } else { SKIN },
            );
        }
        let lex: &[[f64; 4]] = if flanker {
            &[
                [-6.15, 0.39, 0.40, 0.90],
                [-5.40, 0.53, 0.68, 0.92],
                [-4.60, 0.59, 1.00, 0.90],
                [-3.40, 0.67, 1.50, 0.77],
                [-2.10, 0.91, 2.05, 0.58],
                [-0.60, 1.18, 2.48, 0.40],
                [1.80, 1.53, 2.0, 0.32],
            ]
        } else {
            &[
                [-5.30, 0.44, 0.45, 0.88],
                [-4.60, 0.56, 0.79, 0.81],
                [-3.70, 0.62, 1.10, 0.68],
                [-2.60, 0.74, 1.46, 0.54],
                [-1.40, 0.98, 1.78, 0.39],
                [-0.60, 1.16, 1.89, 0.32],
                [1.2, 1.18, 1.30, 0.30],
            ]
        };
        for up in [true, false] {
            body.surface(
                32,
                5,
                |u, v| {
                    let z = mix(lex[0][0], lex.last().unwrap()[0], u);
                    let [_, inner, outer, y] = section(lex, z);
                    v3(
                        side * mix(inner, outer, v),
                        y + (if up { 1.0 } else { -1.0 }) * 0.055 * (PI * v).sin(),
                        z,
                    )
                },
                |_, _| if flanker { Rgb(0.53, 0.63, 0.70) } else { SKIN },
                Some(&|p: V3| v3(p.x, p.y - if up { 0.2 } else { -0.2 }, p.z)),
            );
        }
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
        wing_panel(
            &mut stabs[i],
            tail[0],
            tail[1],
            tail[2],
            tail[3],
            0.13,
            if flanker { Rgb(0.51, 0.62, 0.69) } else { SKIN },
        );
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
                fin(top, front + if flanker { 2.4 } else { 1.95 }),
                fin(top - 0.05, back - 0.65),
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
                fin(top - 0.05, back - 0.65),
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
        let cant = cant * DEG;
        let n = v3(side * cant.cos(), -cant.sin(), 0.0);
        let c = fin(1.9, back - 0.9) + n * 0.045;
        if flanker {
            let e1 = V3::Z;
            let e2 = v3(side * cant.sin(), cant.cos(), 0.0);
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
    if !flanker {
        // Stowed carrier hook under the beaver tail, with the black hook shoe.
        body.capsule(
            v3(0.0, -0.38, 5.4),
            v3(0.0, -0.51, 7.1),
            0.045,
            0.035,
            METAL,
            8,
        );
        body.capsule(
            v3(0.0, -0.51, 7.1),
            v3(0.0, -0.62, 7.22),
            0.055,
            0.04,
            BLACK,
            6,
        );
    }
    for side in [-1.0, 1.0] {
        let tip = if flanker { 7.35 } else { 5.72 };
        let z = if flanker { 4.8 } else { 1.8 };
        body.ellipsoid(
            v3(side * tip, 0.24, z),
            v3(0.035, 0.035, 0.10),
            if side < 0.0 {
                Rgb(0.85, 0.07, 0.04)
            } else {
                Rgb(0.07, 0.65, 0.24)
            },
            5,
            8,
        );
        if !flanker {
            // Naval wing fold seam and tip panel, outside the moving flap.
            for (a, b) in [
                (v3(side * 4.65, 0.285, 0.65), v3(side * 4.65, 0.285, 2.92)),
                (v3(side * 0.565, 0.48, -5.15), v3(side * 0.63, 0.48, -3.75)),
            ] {
                body.capsule(a, b, 0.007, 0.007, SKIN.scale(0.65), 4);
            }
        }
        // Flush formation-light strip along each fuselage shoulder.
        body.capsule(
            v3(side * 0.62, 0.74, if flanker { -3.5 } else { -2.8 }),
            v3(side * 0.70, 0.72, if flanker { -2.8 } else { -2.2 }),
            0.014,
            0.014,
            Rgb(0.65, 0.67, 0.45),
            4,
        );
    }
    // Dorsal communication blade, aft of the canopy and clear of the brake.
    let z = if flanker { -1.6 } else { -0.8 };
    plate(
        &mut body,
        &[
            v3(-0.015, 1.02, z),
            v3(-0.015, 1.30, z + 0.10),
            v3(-0.015, 1.05, z + 0.42),
        ],
        V3::X,
        0.015,
        SKIN,
    );
    let mut brake = Mesh::new();
    let hinge = brake_hinge(flanker);
    let z = hinge.z;
    plate(
        &mut brake,
        &[
            v3(-0.47, hinge.y, z),
            v3(0.47, hinge.y, z),
            v3(0.38, hinge.y - 0.045, z + if flanker { 2.1 } else { 1.45 }),
            v3(-0.38, hinge.y - 0.045, z + if flanker { 2.1 } else { 1.45 }),
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
        let kind = if flanker {
            crate::sim::Type::Su27
        } else {
            crate::sim::Type::F18
        };
        let radius = kind.wheel_radius(i);
        let p = w.local + V3::Y * radius;
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
        gear[i].capsule(
            p + V3::Y * 0.12,
            p + V3::Y * 0.66,
            0.034,
            0.034,
            Rgb(0.78, 0.80, 0.80),
            10,
        );
        for h in [0.25, 0.45] {
            gear[i].capsule(
                p + v3(0.0, h, 0.0),
                p + v3(0.0, 0.35, 0.17),
                0.019,
                0.019,
                METAL,
                6,
            );
        }
        let wheel = |m: &mut Mesh, x: f64| {
            m.lathe(
                p + V3::X * x - V3::X * 0.10,
                V3::X,
                &[
                    (0.0, radius * 0.78),
                    (0.035, radius),
                    (0.17, radius),
                    (0.20, radius * 0.78),
                ],
                12,
                BLACK,
            );
            m.lathe(
                p + V3::X * x - V3::X * 0.102,
                V3::X,
                &[(0.0, radius * 0.48), (0.204, radius * 0.48)],
                12,
                Rgb(0.62, 0.64, 0.61),
            );
        };
        if i == 0 {
            for side in [-1.0, 1.0] {
                gear[i].capsule(
                    p + v3(side * 0.15, 0.03, 0.0),
                    p + v3(side * 0.15, 0.44, 0.10),
                    0.026,
                    0.026,
                    METAL,
                    6,
                );
            }
            if flanker {
                gear[i].block(&frame(p + v3(0.0, 0.40, 0.13)), v3(0.22, 0.025, 0.26), SKIN);
            } else {
                gear[i].capsule(
                    p + V3::Y * 0.28,
                    p + v3(0.0, -0.03, -0.56),
                    0.025,
                    0.025,
                    METAL,
                    6,
                );
            }
        }
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

#[cfg(test)]
mod fit_tests {
    use super::*;
    #[test]
    fn the_eye_and_helmet_fit_beneath_each_canopy() {
        for (flanker, eye) in [(false, EYE), (true, crate::su27::EYE)] {
            let sill = canopy_point(flanker, eye.z, 0.0);
            let crown = canopy_point(flanker, eye.z, PI * 0.5);
            assert!(eye.y > sill.y + 0.3 && crown.y > eye.y + 0.3);
            assert!(sill.x > 0.45 && sill.x < 0.56);
        }
    }
    #[test]
    fn sloping_wing_normals_point_out_of_each_skin() {
        let mut m = Mesh::new();
        wing_panel(
            &mut m,
            v3(1.0, 0.3, 0.0),
            v3(5.0, 0.2, 2.0),
            v3(5.0, 0.2, 4.0),
            v3(1.0, 0.3, 5.0),
            0.16,
            SKIN,
        );
        for v in m.v.chunks(crate::mesh::STRIDE) {
            let centre = mix(0.3, 0.2, (v[0] as f64 - 1.0) / 4.0);
            let height = v[1] as f64 - centre;
            if height.abs() > 0.01 {
                assert!(height * v[4] as f64 > 0.0, "inverted wing normal at {v:?}");
            }
        }
    }
    #[test]
    fn each_tyre_meets_its_simulated_contact_point() {
        for (flanker, kind) in [
            (false, crate::sim::Type::F18),
            (true, crate::sim::Type::Su27),
        ] {
            let a = build_twin(flanker);
            for (i, leg) in a.gear.iter().enumerate() {
                let bottom = leg
                    .v
                    .chunks(crate::mesh::STRIDE)
                    .map(|v| v[1] as f64)
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    (bottom - kind.wheels()[i].local.y).abs() < 0.001,
                    "{kind:?} leg {i}: {bottom}"
                );
            }
            for m in a
                .stabs
                .iter()
                .chain(a.flaps.iter())
                .chain(a.rudders.iter())
                .chain(std::iter::once(&a.brake))
            {
                assert!(m.v.iter().all(|v| v.is_finite()));
            }
        }
    }
}
