//! F/A-18C Lot 20 Hornet, procedural geometry in metres about the mass datum.
//! The shape follows the legacy Hornet's production three-view: a slender
//! pointed radome and forward fuselage, a broad curved LEX that blends into
//! the 20-degree wing, twin rectangular intake trunks under the LEX, canted
//! twin fins with separate rudders, low stabilators and the naval gear.
//! Body axes as everywhere else: x right, y up, z aft.
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
use crate::sim::Type;

pub const EYE: V3 = v3(0.0, 1.35, -3.8);
pub const NOZZLES: [V3; 2] = [v3(-0.72, -0.14, 7.54), v3(0.72, -0.14, 7.54)];
pub const STAB_PIVOT: V3 = v3(1.05, 0.10, 6.05);
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

const SKIN: Rgb = Rgb(0.58, 0.62, 0.64);
const BELLY: Rgb = Rgb(0.70, 0.73, 0.74);
const RADOME: Rgb = Rgb(0.63, 0.65, 0.64);
const METAL: Rgb = Rgb(0.31, 0.33, 0.34);
const BARE: Rgb = Rgb(0.48, 0.46, 0.43);
const BLACK: Rgb = Rgb(0.045, 0.052, 0.055);
const DUCT: Rgb = Rgb(0.075, 0.082, 0.085);
const MARK_DARK: Rgb = Rgb(0.31, 0.34, 0.36);
const MARK_LIGHT: Rgb = Rgb(0.88, 0.90, 0.89);

/// Hornet wing planform, root to structural tip. The wingtip rails add the
/// last 0.25 m of the 12.2 m span quoted by the game.
const WING_ROOT_X: f64 = 1.42;
const WING_TIP_X: f64 = 5.72;
const WING_ROOT_LE: f64 = -0.35;
const WING_TIP_LE: f64 = 1.60;
const WING_ROOT_TE: f64 = 3.58;
const WING_TIP_TE: f64 = 3.22;
/// Chord of the moving trailing-edge flap.
const FLAP_CHORD: f64 = 0.90;
/// Legacy Hornet dorsal speedbrake, between the tails. The panel is hinged at
/// its forward edge; the Super Hornet deleted this door.
const BRAKE_Z0: f64 = 4.96;
const BRAKE_Z1: f64 = 6.58;
const BRAKE_Y0: f64 = 0.419;
const BRAKE_Y1: f64 = 0.246;
const BRAKE_HALF0: f64 = 0.46;
const BRAKE_HALF1: f64 = 0.43;

pub fn build() -> Airframe {
    build_hornet()
}

pub(crate) fn frame(at: V3) -> Frame {
    Frame {
        o: at,
        x: V3::X,
        y: V3::Y,
        z: V3::Z,
    }
}

/// Flat polygon facing `normal`.
pub(crate) fn face(m: &mut Mesh, pts: &[V3], normal: V3, col: Rgb) {
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

/// Thin solid plate around a polygon, with darker lower skin and rim.
pub(crate) fn plate(m: &mut Mesh, points: &[V3], normal: V3, thickness: f64, col: Rgb) {
    let hi: Vec<_> = points.iter().map(|p| *p + normal * thickness).collect();
    let lo: Vec<_> = points.iter().map(|p| *p - normal * thickness).collect();
    face(m, &hi, normal, col);
    face(m, &lo, -normal, col.scale(0.83));
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        m.quad(hi[i], hi[j], lo[j], lo[i], col.scale(0.9));
    }
}

/// Catmull-Rom through measured longitudinal stations, with clamped ends.
pub(crate) fn section<const N: usize>(table: &[[f64; N]], z: f64) -> [f64; N] {
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

// ---------------------------------------------------------------- canopy
// z, half width, sill and crown.
const CANOPY: &[[f64; 4]] = &[
    [-5.58, 0.22, 0.93, 1.02],
    [-5.28, 0.35, 0.95, 1.39],
    [-4.98, 0.45, 0.96, 1.60],
    [-4.78, 0.50, 0.965, 1.71],
    [-4.38, 0.530, 0.965, 1.79],
    [-3.78, 0.540, 0.96, 1.81],
    [-3.12, 0.515, 0.95, 1.75],
    [-2.62, 0.455, 0.945, 1.54],
    [-2.28, 0.345, 0.94, 1.22],
    [-2.10, 0.220, 0.935, 1.02],
];

pub(crate) fn canopy_point(flanker: bool, z: f64, phi: f64) -> V3 {
    if flanker {
        return crate::su27::canopy_point(z, phi);
    }
    let [_, w, sill, top] = section(CANOPY, z);
    v3(
        w * phi.cos(),
        sill + (top - sill) * phi.sin().max(0.0).powf(0.84),
        z,
    )
}
pub(crate) fn canopy_bow(flanker: bool) -> f64 {
    if flanker {
        crate::su27::canopy_bow()
    } else {
        -4.78
    }
}
pub(crate) fn canopy_aft(flanker: bool) -> f64 {
    if flanker {
        crate::su27::canopy_aft()
    } else {
        CANOPY.last().unwrap()[0]
    }
}
pub(crate) fn brake_hinge(flanker: bool) -> V3 {
    if flanker {
        crate::su27::brake_hinge()
    } else {
        v3(0.0, BRAKE_Y0, BRAKE_Z0)
    }
}
pub(crate) fn flap_hinge(flanker: bool, side: f64) -> (V3, V3) {
    if flanker {
        crate::su27::flap_hinge(side)
    } else {
        hornet_flap_hinge(side)
    }
}
pub(crate) fn rudder_hinge(flanker: bool, side: f64) -> (V3, V3) {
    if flanker {
        crate::su27::rudder_hinge(side)
    } else {
        hornet_rudder_hinge(side)
    }
}

fn wing_edges() -> (V3, V3, V3, V3) {
    (
        v3(WING_ROOT_X, 0.16, WING_ROOT_LE),
        v3(WING_TIP_X, 0.28, WING_TIP_LE),
        v3(WING_TIP_X, 0.25, WING_TIP_TE),
        v3(WING_ROOT_X, 0.10, WING_ROOT_TE),
    )
}

fn hornet_flap_hinge(side: f64) -> (V3, V3) {
    let (a, b, _, _) = wing_edges();
    let at = |x: f64| {
        let u = (x - WING_ROOT_X) / (WING_TIP_X - WING_ROOT_X);
        let le = a.lerp(b, u);
        let te = v3(WING_ROOT_X, 0.10, WING_ROOT_TE).lerp(v3(WING_TIP_X, 0.25, WING_TIP_TE), u);
        let p = le.lerp(te, 1.0 - FLAP_CHORD / (te.z - le.z));
        v3(side * p.x, p.y, p.z)
    };
    let (p0, p1) = (at(1.55), at(5.12));
    ((p0 + p1) * 0.5, ((p1 - p0) * side).norm())
}

fn hornet_rudder_hinge(side: f64) -> (V3, V3) {
    let cant = 20.0 * DEG;
    let root_x = 0.93;
    let point = |y: f64, z: f64| v3(side * (root_x + (y - 0.48) * cant.tan()), y, z);
    let (a, b) = (point(0.85, 6.62), point(3.03, 6.77));
    ((a + b) * 0.5, ((b - a) * side).norm())
}

// ------------------------------------------------------------------ wing
/// Thin NACA section, cosine spaced in chord.
pub(crate) fn wing_panel(m: &mut Mesh, a: V3, b: V3, c: V3, d: V3, depth: f64, col: Rgb) {
    wing_segment(m, a, b, c, d, [depth, depth * 0.45], col, 0);
}

/// segment 0: whole chord, 1: fixed skin up to the movable flap, 2: flap skin.
/// All segments sample the same thickness law, so the stowed surfaces meet.
pub(crate) fn wing_segment(
    m: &mut Mesh,
    a: V3,
    b: V3,
    c: V3,
    d: V3,
    depth: [f64; 2],
    col: Rgb,
    segment: u8,
) {
    wing_segment_chord(m, a, b, c, d, depth, col, segment, FLAP_CHORD);
}

pub(crate) fn wing_segment_chord(
    m: &mut Mesh,
    a: V3,
    b: V3,
    c: V3,
    d: V3,
    depth: [f64; 2],
    col: Rgb,
    segment: u8,
    flap_chord: f64,
) {
    for upper in [true, false] {
        m.surface(
            5,
            14,
            |u, v| {
                let le = a.lerp(b, u);
                let te = d.lerp(c, u);
                let fraction = 0.5 * (1.0 - (PI * v).cos());
                let hinge = 1.0 - flap_chord / (te.z - le.z).max(0.2);
                let t = match segment {
                    1 => fraction * hinge,
                    2 => mix(hinge, 1.0, fraction),
                    _ => fraction,
                };
                let h = 5.0
                    * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                        - 0.1036 * t.powi(4));
                le.lerp(te, t)
                    + V3::Y * (if upper { 1.0 } else { -1.0 }) * h * mix(depth[0], depth[1], u)
            },
            |_, _| if upper { col } else { BELLY },
            Some(&|p: V3| {
                let u = ((p.x.abs() - a.x.abs()) / (b.x.abs() - a.x.abs())).clamp(0.0, 1.0);
                let y = mix((a.y + d.y) * 0.5, (b.y + c.y) * 0.5, u);
                v3(p.x, y - if upper { 0.001 } else { -0.001 }, p.z)
            }),
        );
    }
    for (i, (le, te)) in [(a, d), (b, c)].into_iter().enumerate() {
        let hinge = 1.0 - flap_chord / (te.z - le.z).max(0.2);
        let at = |v: f64, sign: f64| {
            let t = match segment {
                1 => v * hinge,
                2 => mix(hinge, 1.0, v),
                _ => v,
            };
            let h = 5.0
                * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                    - 0.1036 * t.powi(4));
            le.lerp(te, t) + V3::Y * (sign * h * depth[i])
        };
        for j in 0..14 {
            let (u, v) = (j as f64 / 14.0, (j + 1) as f64 / 14.0);
            face(
                m,
                &[at(u, 1.0), at(v, 1.0), at(v, -1.0), at(u, -1.0)],
                V3::X * le.x.signum() * if i == 0 { -1.0 } else { 1.0 },
                col,
            );
        }
    }
}

/// A tapered glove that fills the slot between the inboard wing root and
/// the fuselage/intake shoulder. Real Hornets and Flankers carry a broad
/// fillet here; without it the thin wing looks pinned to a tube.
pub(crate) fn root_glove(
    m: &mut Mesh,
    side: f64,
    inner: f64,
    outer: f64,
    z0: f64,
    z1: f64,
    centre_y: f64,
    half_height: f64,
    upper: Rgb,
    lower: Rgb,
) {
    let n = 2.6;
    let (cx, hw) = ((inner + outer) * 0.5, (outer - inner) * 0.5);
    m.surface(
        26,
        14,
        |u, v| {
            let z = mix(z0, z1, u);
            // Collapse the section at both ends so the glove fairs into the
            // wing instead of ending in a blunt slab.
            let s = smoothstep(0.0, 0.055, u) * (1.0 - smoothstep(0.945, 1.0, u));
            let hh = 0.006 + half_height * s;
            let ph = v * TAU;
            let (c, ss) = (ph.cos(), ph.sin());
            v3(
                side * (cx + hw * c.signum() * c.abs().powf(2.0 / n)),
                centre_y + hh * ss.signum() * ss.abs().powf(2.0 / n),
                z,
            )
        },
        |_p, n| if n.y < -0.25 { lower } else { upper },
        Some(&|p: V3| v3(0.0, centre_y, p.z)),
    );
}

// --------------------------------------------------------------- fuselage
// z, half width, centre height, upper/lower half heights, exponents.
const FUSE: &[[f64; 7]] = &[
    [-9.05, 0.030, 0.16, 0.030, 0.030, 2.0, 2.0],
    [-8.72, 0.120, 0.17, 0.110, 0.100, 2.0, 2.0],
    [-8.28, 0.235, 0.18, 0.220, 0.200, 2.0, 2.0],
    [-7.68, 0.355, 0.20, 0.340, 0.290, 2.1, 2.1],
    [-6.98, 0.445, 0.24, 0.460, 0.370, 2.2, 2.2],
    [-6.20, 0.505, 0.30, 0.540, 0.440, 2.3, 2.3],
    [-5.35, 0.550, 0.37, 0.590, 0.490, 2.4, 2.4],
    [-4.45, 0.600, 0.39, 0.610, 0.520, 2.5, 2.5],
    [-3.55, 0.630, 0.37, 0.610, 0.530, 2.6, 2.55],
    [-2.60, 0.660, 0.34, 0.580, 0.500, 2.65, 2.5],
    [-1.35, 0.700, 0.30, 0.520, 0.450, 2.7, 2.45],
    [0.30, 0.740, 0.25, 0.480, 0.400, 2.7, 2.4],
    [1.80, 0.780, 0.20, 0.450, 0.400, 2.6, 2.35],
    [3.35, 0.840, 0.15, 0.380, 0.360, 2.45, 2.3],
    [4.75, 0.820, 0.10, 0.320, 0.300, 2.3, 2.2],
    [5.85, 0.720, 0.05, 0.260, 0.240, 2.2, 2.1],
    [6.80, 0.520, 0.01, 0.190, 0.170, 2.1, 2.0],
    [7.38, 0.280, 0.00, 0.110, 0.080, 2.0, 2.0],
    [7.62, 0.060, 0.00, 0.050, 0.050, 2.0, 2.0],
];

fn fuselage_point(z: f64, th: f64) -> V3 {
    let [_, w, yc, hu, hl, nu, nl] = section(FUSE, z);
    let (c, s) = (th.cos(), th.sin());
    let n = if s >= 0.0 { nu } else { nl };
    v3(
        w.max(0.004) * c.signum() * c.abs().powf(2.0 / n),
        yc + if s >= 0.0 {
            hu.max(0.004) * s.abs().powf(2.0 / n)
        } else {
            -hl.max(0.004) * s.abs().powf(2.0 / n)
        },
        z,
    )
}

/// LEX planform and deck height. z, inner x, outer x, upper y, lower y.
const LEX: &[[f64; 5]] = &[
    [-5.10, 0.48, 0.48, 0.76, 0.68],
    [-4.72, 0.50, 0.60, 0.72, 0.63],
    [-4.28, 0.53, 0.79, 0.68, 0.59],
    [-3.76, 0.55, 1.01, 0.64, 0.55],
    [-3.18, 0.57, 1.20, 0.60, 0.51],
    [-2.52, 0.59, 1.35, 0.56, 0.47],
    [-1.74, 0.61, 1.45, 0.52, 0.43],
    [-0.88, 0.62, 1.50, 0.49, 0.39],
    [0.02, 0.63, 1.53, 0.46, 0.35],
    [0.55, 0.63, 1.50, 0.44, 0.33],
];

fn lex_surface(m: &mut Mesh, side: f64) {
    for upper in [true, false] {
        m.surface(
            30,
            7,
            |u, v| {
                let z = mix(LEX[0][0], LEX.last().unwrap()[0], u);
                let [_, inner, outer, up, lo] = section(LEX, z);
                let x = mix(inner, outer, v);
                let base = if upper { up } else { lo };
                let y = base
                    + (if upper { 1.0 } else { -1.0 })
                        * 0.055
                        * (1.0 - v).powf(0.65)
                        * (PI * v.min(0.999)).sin().max(0.0);
                v3(side * x, y, z)
            },
            |_, n| {
                if n.y < -0.2 {
                    BELLY
                } else {
                    SKIN
                }
            },
            Some(&|p: V3| {
                v3(
                    0.0,
                    (section(LEX, p.z)[3] + section(LEX, p.z)[4]) * 0.5,
                    p.z,
                )
            }),
        );
    }
    // Rounded outer rim closes the upper and lower skins along the planform.
    for j in 0..30 {
        let (u0, u1) = (j as f64 / 30.0, (j + 1) as f64 / 30.0);
        let at = |u: f64| {
            let z = mix(LEX[0][0], LEX.last().unwrap()[0], u);
            let [_, _, outer, up, lo] = section(LEX, z);
            (z, outer, up, lo)
        };
        let (z0, x0, u0y, l0y) = at(u0);
        let (z1, x1, u1y, l1y) = at(u1);
        face(
            m,
            &[
                v3(side * x0, u0y, z0),
                v3(side * x1, u1y, z1),
                v3(side * x1, l1y, z1),
                v3(side * x0, l0y, z0),
            ],
            V3::X * side,
            SKIN.scale(0.9),
        );
    }
    // Panel line along the LEX shoulder.
    let line: Vec<_> = (0..16)
        .map(|k| {
            let u = k as f64 / 15.0;
            let z = mix(-4.75, 0.42, u);
            let [_, _, outer, up, _] = section(LEX, z);
            v3(side * (outer - 0.10).max(0.5), up + 0.004, z)
        })
        .collect();
    for w in line.windows(2) {
        m.capsule(w[0], w[1], 0.006, 0.006, SKIN.scale(0.55), 4);
    }
}

/// Intake/nacelle stations: z, centre x, centre y, half width, half
/// height, superellipse exponent.
const NACELLE: &[[f64; 6]] = &[
    [-2.7200, 0.8000, -0.2800, 0.4590, 0.4125, 4.6000],
    [-2.5500, 0.8000, -0.2700, 0.4860, 0.4455, 4.4000],
    [-2.1000, 0.7900, -0.2500, 0.5063, 0.4702, 3.8000],
    [-1.2000, 0.7800, -0.2300, 0.5130, 0.4867, 3.2000],
    [0.4000, 0.7600, -0.2000, 0.5063, 0.4950, 2.8000],
    [2.3000, 0.7400, -0.1700, 0.4860, 0.4867, 2.5000],
    [4.3000, 0.7300, -0.1500, 0.4590, 0.4620, 2.3000],
    [5.9000, 0.7200, -0.1400, 0.4050, 0.4125, 2.1000],
    [7.2000, 0.7200, -0.1400, 0.4300, 0.4000, 2.0000],
];

fn nacelle(m: &mut Mesh, side: f64) {
    let shape: &[[f64; 6]] = NACELLE;
    let point = |u: f64, v: f64| {
        let z = mix(shape[0][0], shape.last().unwrap()[0], u);
        let [_, cx, cy, hw, hh, n] = section(shape, z);
        let ph = v * TAU;
        let (c, s) = (ph.cos(), ph.sin());
        v3(
            side * cx + hw * c.signum() * c.abs().powf(2.0 / n),
            cy + hh * s.signum() * s.abs().powf(2.0 / n),
            z,
        )
    };
    m.surface(
        30,
        26,
        point,
        |p, n| {
            if p.z > 6.7 {
                BARE
            } else if n.y < -0.35 {
                BELLY.scale(0.96)
            } else if (p.z - 0.4).rem_euclid(1.15) < 0.06 {
                SKIN.scale(0.88)
            } else {
                SKIN
            }
        },
        Some(&|p: V3| v3(side * 0.78, -0.08, p.z)),
    );
    // Lip, recessed dark duct and the black compressor face.
    let mouth = |k: usize, inset: f64| {
        let (sx, sy) = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][k];
        v3(
            side * 0.80 + sx * (0.415 - inset * 0.10),
            -0.28 + sy * (0.365 - inset * 0.075) - inset * 0.06,
            -2.70 + inset * 0.95 - sy * 0.12,
        )
    };
    for k in 0..4 {
        let j = (k + 1) % 4;
        m.quad(
            mouth(k, 0.0),
            mouth(j, 0.0),
            mouth(j, 0.80),
            mouth(k, 0.80),
            DUCT,
        );
        m.capsule(
            mouth(k, 0.0),
            mouth(j, 0.0),
            0.040,
            0.040,
            SKIN.scale(0.94),
            6,
        );
        m.capsule(mouth(k, 0.08), mouth(j, 0.08), 0.010, 0.010, BLACK, 4);
    }
    face(
        m,
        &(0..4).map(|k| mouth(k, 0.84)).collect::<Vec<_>>(),
        -V3::Z,
        BLACK,
    );
    // The LEX underside rolls down into the intake shoulder. Without this
    // canted fairing the deck floats above the trunk and shows a black void
    // from every three-quarter view.
    m.surface(
        24,
        4,
        |u, v| {
            let z = mix(-2.60, 0.58, u);
            let [_, _, outer, _, low] = section(LEX, z);
            let [_, cx, cy, hw, hh, _] = section(NACELLE, z);
            let deck = v3(side * (outer - 0.01), low + 0.015, z);
            let shoulder = v3(side * (cx + hw * 0.72), cy + hh * 0.60, z);
            deck.lerp(shoulder, v)
        },
        |_, _| SKIN.scale(0.97),
        Some(&|p: V3| v3(0.0, -0.10, p.z)),
    );
    // Boundary-layer diverter plate outboard of the duct.
    let sx = side * 1.20;
    plate(
        m,
        &[
            v3(sx, -0.55, -2.48),
            v3(sx, 0.15, -2.62),
            v3(sx, 0.12, 1.30),
            v3(sx, -0.59, 1.40),
        ],
        V3::X * side,
        0.018,
        SKIN.scale(0.93),
    );
    // Nozzle shroud and individual F404 petals.
    let n = v3(side * 0.72, -0.14, 7.20);
    m.lathe(
        n - V3::Z * 0.35,
        V3::Z,
        &[(0.0, 0.45), (0.30, 0.43), (0.52, 0.405)],
        22,
        BARE,
    );
    for j in 0..22 {
        let ph = j as f64 * TAU / 22.0;
        let dir = v3(ph.cos(), ph.sin(), 0.0);
        let edge = |k: f64, z: f64, radius: f64| {
            let angle = k * TAU / 22.0;
            n + v3(angle.cos() * radius, angle.sin() * radius, z)
        };
        let centre_angle = (j as f64 + 0.5) * TAU / 22.0;
        face(
            m,
            &[
                edge(j as f64, 0.0, 0.43),
                edge(j as f64 + 1.0, 0.0, 0.43),
                edge(j as f64 + 1.0, 0.33, 0.425),
                edge(j as f64, 0.33, 0.425),
            ],
            v3(centre_angle.cos(), centre_angle.sin(), 0.0),
            if j % 2 == 0 {
                BARE.scale(1.08)
            } else {
                BARE.scale(0.90)
            },
        );

        m.capsule(
            n + dir * 0.395,
            n + V3::Z * 0.33 + dir * 0.425,
            0.010,
            0.012,
            Rgb(0.52, 0.50, 0.46),
            4,
        );
    }
    m.lathe(
        n + V3::Z * 0.31,
        V3::Z,
        &[(0.0, 0.425), (0.02, 0.375)],
        22,
        METAL,
    );
    m.lathe(
        n + V3::Z * 0.33,
        V3::Z,
        &[(0.0, 0.37), (0.01, 0.0)],
        22,
        BLACK,
    );
}

fn fin(m: &mut Mesh, side: f64) {
    let cant = 20.0 * DEG;
    let root_x = 0.93;
    let at = |y: f64, z: f64| v3(side * (root_x + (y - 0.48) * cant.tan()), y, z);
    // The fixed skin stops at the rudder hinge; otherwise a second,
    // immobile rudder remains visible when the control surface deflects.
    let outline = [
        at(0.48, 4.62),
        at(3.03, 6.12),
        at(3.03, 6.77),
        at(0.48, 6.62),
    ];
    plate(m, &outline, V3::X * side, 0.032, SKIN.scale(1.30));
    // Leading-edge darker strip and tip fairing.
    m.capsule(
        at(0.62, 4.75),
        at(2.93, 6.18),
        0.020,
        0.014,
        SKIN.scale(0.78),
        5,
    );
    m.capsule(
        at(3.00, 6.16),
        at(3.00, 6.77),
        0.018,
        0.014,
        SKIN.scale(0.85),
        5,
    );
    // Fin-root fairing over the fuselage.
    m.ellipsoid(
        v3(side * 0.95, 0.52, 5.78),
        v3(0.12, 0.15, 1.20),
        SKIN,
        5,
        10,
    );
}

fn star_and_bar(m: &mut Mesh, c: V3, n: V3, r: f64, fill: Rgb) {
    let e2 = if n.y.abs() > 0.9 {
        -V3::Z
    } else {
        (V3::Y - n * n.y).norm()
    };
    let e1 = e2.cross(n).norm();
    let o = c + n * 0.012;
    let mut star = Vec::with_capacity(10);
    for k in 0..10 {
        let a = k as f64 * PI / 5.0 + PI * 0.5;
        let q = if k % 2 == 0 { r } else { r * 0.40 };
        star.push(o + (e1 * a.cos() + e2 * a.sin()) * q);
    }
    face(m, &star, n, fill);
    for s in [-1.0, 1.0] {
        let inner = o + e1 * s * r * 0.92;
        let outer = o + e1 * s * r * 2.14;
        let h = r * 0.26;
        face(
            m,
            &[
                inner - e2 * h,
                outer - e2 * h,
                outer + e2 * h,
                inner + e2 * h,
            ],
            n,
            fill,
        );
    }
}

// ------------------------------------------------------------------ model
/// Panel joints, access covers and fasteners are actual shallow geometry.
fn surface_details(m: &mut Mesh) {
    for z in [-6.2, -5.35, -2.6, -1.35, 0.3, 3.35, 5.85] {
        for k in 0..32 {
            let at = |angle: f64| {
                let p = fuselage_point(z, angle);
                let centre = v3(0.0, section(FUSE, z)[2], z);
                p + (p - centre).norm() * 0.004
            };
            m.capsule(
                at(k as f64 * TAU / 32.0),
                at((k + 1) as f64 * TAU / 32.0),
                0.003,
                0.003,
                SKIN.scale(0.67),
                4,
            );
        }
    }
    for side in [-1.0, 1.0] {
        // Small flush access-cover rectangles on the forward cheeks.
        let theta = if side > 0.0 { 0.28 } else { PI - 0.28 };
        for (z0, z1) in [(-6.0, -5.55), (-3.0, -2.55)] {
            for (a, b) in [
                (v3(z0, theta - 0.14, 0.0), v3(z1, theta - 0.14, 0.0)),
                (v3(z1, theta - 0.14, 0.0), v3(z1, theta + 0.14, 0.0)),
                (v3(z1, theta + 0.14, 0.0), v3(z0, theta + 0.14, 0.0)),
                (v3(z0, theta + 0.14, 0.0), v3(z0, theta - 0.14, 0.0)),
            ] {
                let point = |q: V3| {
                    let p = fuselage_point(q.x, q.y);
                    p + (p - v3(0.0, section(FUSE, q.x)[2], q.x)).norm() * 0.005
                };
                m.capsule(point(a), point(b), 0.003, 0.003, SKIN.scale(0.62), 4);
            }
        }
        // Louvered ventilation grilles lie on the upper LEX shoulder.
        for k in 0..7 {
            let z = -2.1 + k as f64 * 0.065;
            let [_, inner, outer, top, _] = section(LEX, z);
            let x = mix(inner, outer, 0.60);
            m.capsule(
                v3(side * (x - 0.075), top + 0.015, z),
                v3(side * (x + 0.075), top + 0.015, z),
                0.006,
                0.006,
                BLACK,
                4,
            );
        }
    }
}

fn build_hornet() -> Airframe {
    let mut body = Mesh::new();
    let (z0, z1) = (FUSE[0][0], FUSE.last().unwrap()[0]);
    let paint = |p: V3, n: V3| {
        if p.z < -6.95 {
            RADOME
        } else if n.y < -0.30 {
            BELLY
        } else if p.z < -5.45 && p.z > -6.35 && p.y > 0.54 && p.x.abs() < 0.30 {
            // Matte anti-glare panel ahead of the windscreen.
            Rgb(0.15, 0.16, 0.17)
        } else {
            SKIN
        }
    };
    body.surface(
        72,
        32,
        |u, v| fuselage_point(mix(z0, z1, u), v * TAU),
        paint,
        Some(&|p: V3| v3(0.0, section(FUSE, p.z)[2], p.z)),
    );
    // Radome tip, pitot and angle-of-attack vanes.
    body.capsule(
        v3(0.0, 0.18, -9.05),
        v3(0.0, 0.18, -9.42),
        0.012,
        0.003,
        METAL,
        5,
    );
    for side in [-1.0, 1.0] {
        body.capsule(
            v3(side * 0.29, 0.29, -7.28),
            v3(side * 0.39, 0.31, -7.20),
            0.008,
            0.004,
            METAL,
            4,
        );
    }
    // Radome joint ring, a very visible production break on the Hornet.
    for k in 0..28 {
        let a = fuselage_point(-6.95, k as f64 * TAU / 28.0);
        let b = fuselage_point(-6.95, (k + 1) as f64 * TAU / 28.0);
        body.capsule(
            a + v3(a.x, a.y - section(FUSE, -6.95)[2], 0.0).norm() * 0.004,
            b + v3(b.x, b.y - section(FUSE, -6.95)[2], 0.0).norm() * 0.004,
            0.005,
            0.005,
            SKIN.scale(0.72),
            4,
        );
    }
    // M61A2 muzzle above the radome joint.
    for (k, x) in [-0.10, -0.035, 0.035, 0.10].into_iter().enumerate() {
        body.block(
            &frame(v3(x, 0.88, -6.34 + k as f64 * 0.008)),
            v3(0.012, 0.055, 0.10),
            BLACK,
        );
    }
    body.ellipsoid(
        v3(0.0, 0.63, -5.90),
        v3(0.10, 0.055, 0.14),
        SKIN.scale(0.92),
        4,
        8,
    );

    let (a, b, c, d) = wing_edges();
    let root_x = WING_ROOT_X;
    let tip_x = WING_TIP_X;
    let leading = |x: f64| a.lerp(b, (x - root_x) / (tip_x - root_x));
    let trailing = |x: f64| d.lerp(c, (x - root_x) / (tip_x - root_x));
    let thickness = |x: f64| 0.225 * mix(1.0, 0.34, (x - root_x) / (tip_x - root_x));
    let wing_surface = |x: f64, t: f64| {
        let le = leading(x);
        let te = trailing(x);
        let h = 5.0
            * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                - 0.1036 * t.powi(4));
        le.lerp(te, t) + V3::Y * h * thickness(x)
    };

    let mut flaps = [Mesh::new(), Mesh::new()];
    let mut stabs = [Mesh::new(), Mesh::new()];
    let mut rudders = [Mesh::new(), Mesh::new()];
    for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
        lex_surface(&mut body, side);
        nacelle(&mut body, side);
        root_glove(
            &mut body, side, 1.00, 1.435, -0.38, 3.60, 0.14, 0.13, SKIN, BELLY,
        );

        let panel = |_x: f64, p: V3| v3(side * p.x, p.y, p.z);
        let (flap_x0, flap_x1) = (1.55, 5.12);
        for (x0, x1, notch) in [
            (root_x, flap_x0, false),
            (flap_x0, flap_x1, true),
            (flap_x1, tip_x, false),
        ] {
            wing_segment(
                &mut body,
                panel(x0, leading(x0)),
                panel(x1, leading(x1)),
                panel(x1, trailing(x1)),
                panel(x0, trailing(x0)),
                [thickness(x0), thickness(x1)],
                SKIN,
                if notch { 1 } else { 0 },
            );
        }
        // Leading-edge flap: a thin drooped strip along the fixed wing.
        let (slat0, slat1) = (1.70, 5.38);
        body.surface(
            14,
            3,
            |u, v| {
                let x = mix(slat0, slat1, u);
                let le = leading(x);
                let chord = trailing(x).z - le.z;
                let t = v * 0.075;
                let h = 5.0
                    * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                        - 0.1036 * t.powi(4));
                v3(
                    side * x,
                    le.y - 0.012 + h * thickness(x),
                    le.z + chord * t - 0.010,
                )
            },
            |_, _| SKIN.scale(0.92),
            Some(&|p: V3| v3(p.x, p.y + 0.1, p.z)),
        );
        wing_segment(
            &mut flaps[i],
            panel(flap_x0, leading(flap_x0)),
            panel(flap_x1, leading(flap_x1)),
            panel(flap_x1, trailing(flap_x1)),
            panel(flap_x0, trailing(flap_x0)),
            [thickness(flap_x0), thickness(flap_x1)],
            SKIN,
            2,
        );
        // Flap and aileron hinge lines, wing fold and walkway.
        let hinge = |x: f64| {
            let le = leading(x);
            let te = trailing(x);
            le.lerp(te, 1.0 - FLAP_CHORD / (te.z - le.z))
        };
        for x in [flap_x0, 4.82, flap_x1] {
            let p = hinge(x);
            body.capsule(
                panel(x, p + v3(0.0, 0.004, 0.0)),
                panel(x, p + v3(0.0, 0.004, 0.22)),
                0.006,
                0.006,
                SKIN.scale(0.65),
                4,
            );
        }
        body.capsule(
            panel(4.82, leading(4.82) + v3(0.0, 0.015, 0.12)),
            panel(4.82, trailing(4.82) + v3(0.0, 0.005, -0.02)),
            0.005,
            0.005,
            SKIN.scale(0.6),
            4,
        );
        // Non-slip walkway and chordwise panel breaks.
        body.surface(
            3,
            5,
            |u, v| {
                let x = side * mix(2.05, 3.02, u);
                let t = mix(0.13, 0.58, v);
                v3(x, wing_surface(x.abs(), t).y, wing_surface(x.abs(), t).z) + V3::Y * 0.006
            },
            |_, _| SKIN.scale(0.76),
            Some(&|p: V3| p - V3::Y * 0.02),
        );
        for x in [2.95, 4.35] {
            for k in 0..5 {
                let (t0, t1) = (k as f64 / 5.0, (k + 1) as f64 / 5.0);
                body.capsule(
                    panel(x, wing_surface(x, t0 * 0.94 + 0.03) + V3::Y * 0.006),
                    panel(x, wing_surface(x, t1 * 0.94 + 0.03) + V3::Y * 0.006),
                    0.0045,
                    0.0045,
                    SKIN.scale(0.65),
                    4,
                );
            }
        }
        // Formation strip on the LEX shoulder.
        let [_, _, lx, uy, _] = section(LEX, -3.35);
        body.ellipsoid(
            v3(side * (lx - 0.13), uy + 0.018, -3.35),
            v3(0.045, 0.010, 0.16),
            Rgb(0.62, 0.67, 0.38),
            4,
            8,
        );
        // Wingtip launch rail.
        body.capsule(
            v3(side * 5.98, 0.24, 1.12),
            v3(side * 5.98, 0.24, 3.95),
            0.050,
            0.050,
            METAL.scale(0.78),
            6,
        );
        body.capsule(
            v3(side * 6.00, 0.27, 1.15),
            v3(side * 6.00, 0.27, 3.78),
            0.030,
            0.030,
            Rgb(0.14, 0.15, 0.16),
            5,
        );
        // Underwing pylons, aligned with the F-16 loadout stations through
        // `store_shift`: inner, middle and outer.
        for (px, pz) in [(2.65, 1.45), (4.05, 1.70), (4.95, 1.85)] {
            let y = leading(px).y - 0.20;
            body.block(
                &frame(v3(side * px, y, pz)),
                v3(0.055, 0.16, 0.55),
                SKIN.scale(0.95),
            );
        }
        // National star and bar on the upper wing.
        let c = panel(4.25, wing_surface(4.25, 0.38));
        let marking_start = body.v.len();
        star_and_bar(&mut body, c, V3::Y, 0.34, MARK_DARK);
        // Drape the marking over the airfoil. A flat decal at the chord
        // plane is buried by the cambered upper skin.
        for v in body.v[marking_start..].chunks_mut(crate::mesh::STRIDE) {
            let x = v[0].abs() as f64;
            let le = leading(x);
            let te = trailing(x);
            let t = ((v[2] as f64 - le.z) / (te.z - le.z)).clamp(0.0, 1.0);
            v[1] = (wing_surface(x, t).y + 0.008) as f32;
        }
        body.ellipsoid(
            v3(side * 6.00, 0.24, 2.85),
            v3(0.035, 0.035, 0.085),
            if side < 0.0 {
                Rgb(0.82, 0.07, 0.04)
            } else {
                Rgb(0.07, 0.62, 0.22)
            },
            5,
            8,
        );

        fin(&mut body, side);
        let stab_root_le = v3(side * 0.72, 0.10, 4.70);
        let stab_tip_le = v3(side * 3.22, -0.05, 6.74);
        let stab_tip_te = v3(side * 3.22, -0.05, 7.66);
        let stab_root_te = v3(side * 0.72, 0.10, 7.18);
        wing_panel(
            &mut stabs[i],
            stab_root_le,
            stab_tip_le,
            stab_tip_te,
            stab_root_te,
            0.105,
            SKIN,
        );
        let cant = 20.0 * DEG;
        let at = |y: f64, z: f64| v3(side * (0.93 + (y - 0.48) * cant.tan()), y, z);
        plate(
            &mut rudders[i],
            &[
                at(0.60, 6.62),
                at(3.02, 6.77),
                at(3.02, 7.03),
                at(0.60, 6.99),
            ],
            V3::X * side,
            0.026,
            SKIN.scale(1.30),
        );
        // Star-and-bar and formation strip on the fin.
        star_and_bar(
            &mut body,
            at(1.60, 6.13) + v3(side * 0.040, 0.0, 0.0),
            v3(side * cant.cos(), -cant.sin(), 0.0),
            0.24,
            MARK_LIGHT,
        );
        body.capsule(
            at(0.85, 5.55) + v3(side * 0.04, 0.0, 0.0),
            at(1.35, 5.85) + v3(side * 0.04, 0.0, 0.0),
            0.020,
            0.020,
            Rgb(0.55, 0.58, 0.45),
            4,
        );
        // Tailhook recess and ventral strake.
        body.capsule(
            v3(side * 0.35, -0.34, 5.05),
            v3(side * 0.30, -0.40, 6.70),
            0.018,
            0.012,
            METAL,
            5,
        );
    }

    // Flat ventral engine-bay panel between the F404 trunks. It turns the
    // two tubes into one aft fuselage with only a shallow tunnel, as on the
    // production aircraft.
    body.surface(
        20,
        6,
        |u, v| {
            let z = mix(3.25, 7.28, u);
            let w = 0.77 * (1.0 - 0.24 * u);
            let x = (v * 2.0 - 1.0) * w;
            v3(x, -0.34 + 0.025 * (x / w).powi(2) - 0.01 * (1.0 - u), z)
        },
        |_, _| BELLY.scale(0.90),
        Some(&|p: V3| v3(p.x, p.y + 0.30, p.z)),
    );
    // Canopy: windscreen and bubble, one surface with the cockpit's shared
    // stations. Frames are separate so the glass can be rendered tinted.
    let mut canopy = Mesh::new();
    let (cz0, cz1) = (CANOPY[0][0], CANOPY.last().unwrap()[0]);
    canopy.surface(
        24,
        20,
        |u, v| canopy_point(false, mix(cz0, cz1, u), v * PI),
        |_, _| Rgb(0.28, 0.42, 0.45),
        Some(&|p: V3| v3(0.0, 0.92, p.z)),
    );
    for z in [cz0, canopy_bow(false), cz1] {
        for j in 0..20 {
            body.capsule(
                canopy_point(false, z, j as f64 * PI / 20.0),
                canopy_point(false, z, (j + 1) as f64 * PI / 20.0),
                if z == cz0 || z == cz1 { 0.030 } else { 0.018 },
                0.024,
                if z == cz0 || z == cz1 {
                    SKIN.scale(0.8)
                } else {
                    METAL.scale(0.9)
                },
                5,
            );
        }
    }
    for edge in [0.0, PI] {
        for j in 0..24 {
            body.capsule(
                canopy_point(false, mix(cz0, cz1, j as f64 / 24.0), edge),
                canopy_point(false, mix(cz0, cz1, (j + 1) as f64 / 24.0), edge),
                0.024,
                0.024,
                SKIN.scale(0.8),
                5,
            );
        }
    }
    for phi in [0.70, PI - 0.70] {
        for j in 0..12 {
            let z = |k| mix(cz0, canopy_bow(false), k as f64 / 12.0);
            body.capsule(
                canopy_point(false, z(j), phi),
                canopy_point(false, z(j + 1), phi),
                0.014,
                0.014,
                METAL,
                6,
            );
        }
    }
    // Canopy aft fairing: an opaque turtle deck that carries the bubble
    // down into the spine, as on the real aircraft. It also hides the open
    // rear of the glass canopy.
    body.surface(
        16,
        18,
        |u, v| {
            let z = mix(-2.14, -0.18, u);
            let [_, w0, sill0, top0] = section(CANOPY, -2.10);
            let ease = smoothstep(0.0, 1.0, u);
            let w = mix(w0, 0.33, ease);
            let sill = mix(sill0, 0.735, ease);
            let top = mix(top0, 0.785, ease);
            let phi = v * PI;
            v3(
                w * phi.cos(),
                sill + (top - sill) * phi.sin().max(0.0).powf(0.84),
                z,
            )
        },
        |_, _| SKIN,
        Some(&|p: V3| v3(0.0, 0.84, p.z)),
    );

    // Dorsal blade antenna and IFF.
    plate(
        &mut body,
        &[
            v3(-0.014, 0.84, -1.20),
            v3(-0.014, 1.10, -1.10),
            v3(-0.014, 0.88, -0.76),
        ],
        V3::X,
        0.014,
        SKIN,
    );
    // Arresting hook under the beavertail.
    body.capsule(
        v3(0.0, -0.30, 5.10),
        v3(0.0, -0.44, 6.76),
        0.040,
        0.032,
        METAL,
        7,
    );
    body.capsule(
        v3(0.0, -0.44, 6.76),
        v3(0.0, -0.56, 6.90),
        0.050,
        0.036,
        BLACK,
        6,
    );

    // Legacy Hornet dorsal speedbrake. The A-D series carries a broad
    // rectangular door between the vertical tails, hinged at its forward
    // edge; the Super Hornet deleted this and uses LEX spoilers instead.
    let door = |z: f64| {
        let t = ((z - BRAKE_Z0) / (BRAKE_Z1 - BRAKE_Z0)).clamp(0.0, 1.0);
        (
            BRAKE_Y0 + (BRAKE_Y1 - BRAKE_Y0) * t,
            BRAKE_HALF0 + (BRAKE_HALF1 - BRAKE_HALF0) * t,
        )
    };
    let mut brake = Mesh::new();
    plate(
        &mut brake,
        &[
            v3(-BRAKE_HALF0, BRAKE_Y0, BRAKE_Z0),
            v3(BRAKE_HALF0, BRAKE_Y0, BRAKE_Z0),
            v3(BRAKE_HALF1, BRAKE_Y1, BRAKE_Z1),
            v3(-BRAKE_HALF1, BRAKE_Y1, BRAKE_Z1),
        ],
        V3::Y,
        0.012,
        SKIN.scale(0.97),
    );
    // Underside stiffeners, visible when the door swings open.
    for x in [-0.30, 0.0, 0.30] {
        brake.capsule(
            v3(x, BRAKE_Y0 - 0.016, BRAKE_Z0 + 0.08),
            v3(x * 0.94, BRAKE_Y1 - 0.016, BRAKE_Z1 - 0.08),
            0.009,
            0.009,
            SKIN.scale(0.76),
            4,
        );
    }
    // Dark perimeter joint and hinge lugs keep the stowed door readable
    // from above, without adding a raised box around it.
    let top_at = |z: f64| {
        let [_, _, yc, hu, _, _, _] = section(FUSE, z);
        yc + hu
    };
    let outline = |z: f64| {
        let (_, half) = door(z);
        (half + 0.012, top_at(z) + 0.004)
    };
    for side in [-1.0, 1.0] {
        for k in 0..12 {
            let z0 = mix(BRAKE_Z0 - 0.01, BRAKE_Z1 + 0.01, k as f64 / 12.0);
            let z1 = mix(BRAKE_Z0 - 0.01, BRAKE_Z1 + 0.01, (k + 1) as f64 / 12.0);
            let (w0, y0) = outline(z0);
            let (w1, y1) = outline(z1);
            body.capsule(
                v3(side * w0, y0, z0),
                v3(side * w1, y1, z1),
                0.0045,
                0.0045,
                SKIN.scale(0.48),
                4,
            );
        }
    }
    for z in [BRAKE_Z0 - 0.008, BRAKE_Z1 + 0.008] {
        let (w, y) = outline(z);
        body.capsule(
            v3(-w - 0.01, y, z),
            v3(w + 0.01, y, z),
            0.0045,
            0.0045,
            SKIN.scale(0.48),
            4,
        );
    }
    for x in [-0.28, 0.28] {
        body.block(
            &frame(v3(x, BRAKE_Y0 - 0.012, BRAKE_Z0 - 0.012)),
            v3(0.036, 0.016, 0.022),
            METAL,
        );
    }

    surface_details(&mut body);
    let gear = gear_legs(Type::F18);
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

/// Shared landing gear, placed by the simulation's published wheel contact
/// points. A twin nose wheel for the Hornet, a single one for the Flanker.
pub(crate) fn gear_legs(kind: Type) -> [Mesh; 3] {
    let wheels = kind.wheels();
    let mut gear = [Mesh::new(), Mesh::new(), Mesh::new()];
    for (i, w) in wheels.iter().enumerate() {
        let radius = kind.wheel_radius(i);
        let p = w.local + V3::Y * radius;
        let top = v3(p.x * 0.67, -0.2, p.z + if i == 0 { 0.55 } else { -0.15 });
        gear[i].capsule(top, p, 0.070, 0.078, Rgb(0.72, 0.73, 0.70), 8);
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
            p + V3::Y * 0.70,
            0.036,
            0.036,
            Rgb(0.78, 0.80, 0.80),
            10,
        );
        for h in [0.26, 0.48] {
            gear[i].capsule(
                p + v3(0.0, h, 0.0),
                p + v3(0.0, 0.36, 0.17),
                0.020,
                0.020,
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
            if kind == Type::F18 {
                gear[i].capsule(
                    p + V3::Y * 0.28,
                    p + v3(0.0, -0.03, -0.56),
                    0.025,
                    0.025,
                    METAL,
                    6,
                );
                wheel(&mut gear[i], -0.15);
                wheel(&mut gear[i], 0.15);
            } else {
                gear[i].block(&frame(p + v3(0.0, 0.40, 0.13)), v3(0.22, 0.025, 0.26), SKIN);
                wheel(&mut gear[i], 0.0);
            }
        } else {
            wheel(&mut gear[i], 0.0);
        }
        gear[i].block(
            &frame(top + v3(0.0, -0.15, -0.15)),
            v3(0.15, 0.35, 0.035),
            SKIN,
        );
    }
    gear
}

/// Offset that carries a store from the F-16 station `at` onto the Hornet's
/// matching pylon or wingtip rail.
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    let side = at.x.signum();
    let (from, to) = if x > 4.6 {
        (v3(4.8, 0.30, 0.55), v3(5.98, 0.22, 2.50))
    } else if x > 3.8 {
        (v3(4.15, -0.06, 1.0), v3(4.05, -0.30, 1.62))
    } else if x > 2.9 {
        (v3(3.5, -0.06, 0.52), v3(4.95, -0.30, 1.74))
    } else {
        (v3(2.3, -0.08, -0.28), v3(2.65, -0.30, 1.37))
    };
    let d = to - from;
    v3(side * d.x, d.y, d.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::STRIDE;

    fn extent(meshes: &[&Mesh]) -> (V3, V3) {
        let (mut lo, mut hi) = (
            v3(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            v3(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for m in meshes {
            for v in m.v.chunks(STRIDE) {
                let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                lo = v3(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
                hi = v3(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
            }
        }
        (lo, hi)
    }

    #[test]
    fn outboard_wing_chords_match_the_orthographic_reference_envelopes() {
        // Independent samples traced in metres from the three-view drawings.
        // These catch an overlong trailing edge even when total span/length fit.
        for (kind, x, tolerance, expected_le, expected_te) in [
            (Type::F18, 4.40, 0.035, 1.00, 3.33),
            (Type::Su27, 5.75, 0.060, 2.20, 5.15),
        ] {
            let a = if kind == Type::F18 {
                build()
            } else {
                crate::su27::build()
            };
            for side in [-1.0, 1.0] {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for m in [&a.body, &a.flaps[0], &a.flaps[1]] {
                    for v in m.v.chunks(STRIDE) {
                        if (v[0] as f64 - side * x).abs() < tolerance && v[1] > -0.02 && v[1] < 0.40
                        {
                            lo = lo.min(v[2] as f64);
                            hi = hi.max(v[2] as f64);
                        }
                    }
                }
                assert!(
                    (lo - expected_le).abs() < 0.18,
                    "{kind:?} leading edge {lo}"
                );
                assert!(
                    (hi - expected_te).abs() < 0.18,
                    "{kind:?} trailing edge {hi}"
                );
            }
        }
    }

    #[test]
    fn hornet_matches_its_production_three_view() {
        let a = build();
        let (lo, hi) = extent(&[&a.body, &a.stabs[0], &a.stabs[1]]);
        let (span, length) = (hi.x - lo.x, hi.z - lo.z);
        assert!((span - 12.2).abs() < 0.25, "span {span:.2} m");
        assert!((length - 17.0).abs() < 0.30, "length {length:.2} m");
        assert!(a.body.count() > 12000, "body {} vertices", a.body.count());
        assert!(a.canopy.count() > 700);
        assert!(a.gear.iter().all(|m| m.count() > 200));
    }

    #[test]
    fn legacy_dorsal_speedbrake_fits_between_the_tails() {
        let a = build();
        let (lo, hi) = extent(&[&a.brake]);
        assert!((lo.z - BRAKE_Z0).abs() < 0.02, "brake front {lo:?}");
        assert!((hi.z - BRAKE_Z1).abs() < 0.02, "brake rear {hi:?}");
        assert!(lo.x < -0.40 && hi.x > 0.40, "brake width {lo:?}..{hi:?}");
        assert!(lo.y > 0.20 && hi.y < 0.46, "brake height {lo:?}..{hi:?}");
        let hinge = brake_hinge(false);
        assert!((hinge.z - BRAKE_Z0).abs() < 1e-9);
        assert!((hinge.y - BRAKE_Y0).abs() < 1e-9);
        assert!(
            hi.z < 6.98,
            "brake must finish ahead of the fin trailing edge"
        );
        assert!(a.brake.count() > 60);
    }

    #[test]
    fn eye_and_helmet_fit_beneath_the_canopy() {
        let sill = canopy_point(false, EYE.z, 0.0);
        let crown = canopy_point(false, EYE.z, PI * 0.5);
        assert!(eye_clearance());
        assert!(sill.x > 0.46 && sill.x < 0.56, "sill {sill:?}");
        assert!(crown.y > EYE.y + 0.30, "crown {crown:?}");
    }
    fn eye_clearance() -> bool {
        let sill = canopy_point(false, EYE.z, 0.0);
        EYE.y > sill.y + 0.30
    }

    #[test]
    fn wing_normal_and_flap_join_are_continuous() {
        let a = build();
        for i in 0..2 {
            let hinge_z = |x: f64| {
                let u = (x.abs() - WING_ROOT_X) / (WING_TIP_X - WING_ROOT_X);
                let te = WING_ROOT_TE + (WING_TIP_TE - WING_ROOT_TE) * u;
                te - FLAP_CHORD
            };
            let body: Vec<_> = a
                .body
                .v
                .chunks(STRIDE)
                .map(|v| v3(v[0] as f64, v[1] as f64, v[2] as f64))
                .collect();
            let mut checked = 0;
            for v in a.flaps[i].v.chunks(STRIDE) {
                let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                assert!(p.z >= hinge_z(p.x) - 2e-3, "flap ahead of hinge: {p:?}");
                if (p.z - hinge_z(p.x)).abs() < 2e-3 {
                    assert!(body.iter().any(|q| (*q - p).len() < 3e-3), "gap at {p:?}");
                    checked += 1;
                }
            }
            assert!(checked >= 12);
        }
    }

    #[test]
    fn fixed_fin_leaves_the_rudder_free_to_move() {
        for side in [-1.0, 1.0] {
            let mut m = Mesh::new();
            fin(&mut m, side);
            for v in m.v.chunks(STRIDE) {
                if v[1] > 0.80 {
                    assert!(v[2] < 6.80, "fixed skin behind rudder hinge: {v:?}");
                }
            }
        }
    }

    #[test]
    fn canopy_rails_follow_both_sills_and_leave_the_bubble_clear() {
        for flanker in [false, true] {
            let a = if flanker {
                crate::su27::build()
            } else {
                build()
            };
            let z = if flanker { crate::su27::EYE.z } else { EYE.z };
            for phi in [0.0, PI] {
                let sill = canopy_point(flanker, z, phi);
                let nearest = a
                    .body
                    .v
                    .chunks(STRIDE)
                    .map(|v| (v3(v[0] as f64, v[1] as f64, v[2] as f64) - sill).len())
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest < 0.10,
                    "missing sill rail {flanker} {phi}: {nearest}"
                );
            }
            let crown = canopy_point(flanker, z, PI * 0.5);
            assert!(
                a.body
                    .v
                    .chunks(STRIDE)
                    .all(|v| (v3(v[0] as f64, v[1] as f64, v[2] as f64) - crown).len() > 0.10),
                "opaque frame crosses the clear bubble {flanker}"
            );
        }
    }

    #[test]
    fn each_tyre_meets_its_simulated_contact_point() {
        let a = build();
        for (i, leg) in a.gear.iter().enumerate() {
            let bottom = leg
                .v
                .chunks(STRIDE)
                .map(|v| v[1] as f64)
                .fold(f64::INFINITY, f64::min);
            assert!(
                (bottom - Type::F18.wheels()[i].local.y).abs() < 0.001,
                "leg {i}: {bottom}"
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

    #[test]
    fn canopy_covers_the_shared_cockpit_cutout() {
        for z in [-4.7, -4.2, -3.6, -3.0, -2.4] {
            let sill = canopy_point(false, z, 0.0);
            assert!(sill.x > 0.25, "canopy too narrow at {z}");
            assert!(sill.y > 0.90, "canopy sill too low at {z}");
        }
    }
}
