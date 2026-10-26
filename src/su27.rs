//! Su-27SK Flanker-B, procedural geometry in metres about the mass datum.
//! The baseline single-seater follows the Su-27/Su-33 family three-view but
//! omits the naval variant's canards and folding-wing hardware: a long
//! drooped radome with a centreline IRST, a broad blended lifting centre
//! body, 42-degree wings with leading-edge slats, widely separated AL-31F
//! nacelles, canted twin fins, ventral fins and the central tail sting.
//! Body axes: x right, y up, z aft.
use crate::hornet::{
    face, frame, gear_legs, plate, root_glove, section, wing_panel, wing_segment_chord, Airframe,
};
use crate::math::*;
use crate::mesh::{Mesh, Rgb};
use crate::sim::Type;

pub const EYE: V3 = v3(0.0, 1.48, -5.85);
pub const NOZZLES: [V3; 2] = [v3(-1.35, -0.24, 9.94), v3(1.35, -0.24, 9.94)];
pub const STAB_PIVOT: V3 = v3(1.85, 0.15, 8.50);
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (v3(0.0, -0.4, -3.4), V3::X, -95.0 * DEG),
    (v3(-1.5, -0.3, 1.3), V3::Z, -90.0 * DEG),
    (v3(1.5, -0.3, 1.3), V3::Z, 90.0 * DEG),
];
const BRAKE_FRONT: f64 = -2.0;
const BRAKE_LENGTH: f64 = 3.10;
pub(crate) fn brake_hinge() -> V3 {
    brake_point(0.0, 0.5)
}

const LIGHT: Rgb = Rgb(0.69, 0.72, 0.73);
const BELLY: Rgb = Rgb(0.76, 0.80, 0.82);
const RADOME: Rgb = Rgb(0.62, 0.66, 0.64);
const METAL: Rgb = Rgb(0.33, 0.33, 0.31);
const BARE: Rgb = Rgb(0.48, 0.46, 0.43);
const BLACK: Rgb = Rgb(0.055, 0.06, 0.062);
const DUCT: Rgb = Rgb(0.07, 0.075, 0.078);
const STAR: Rgb = Rgb(0.76, 0.11, 0.09);
const WHITE: Rgb = Rgb(0.90, 0.90, 0.87);

const ROOT_X: f64 = 2.0;
const TIP_X: f64 = 7.35;
const ROOT_LE: f64 = -0.95;
const TIP_LE: f64 = 3.55;
const ROOT_TE: f64 = 3.40;
const TIP_TE: f64 = 5.90;
const FLAP_CHORD: f64 = 1.20;

pub fn build() -> Airframe {
    build_flanker()
}

// ---------------------------------------------------------------- canopy
// z, half width, sill and crown.
const CANOPY: &[[f64; 4]] = &[
    [-7.8000, 0.18, 0.99, 1.08],
    [-7.4700, 0.33, 1.00, 1.40],
    [-7.2100, 0.41, 1.01, 1.68],
    [-7.0300, 0.45, 1.015, 1.82],
    [-6.6500, 0.48, 1.02, 1.94],
    [-6.2000, 0.505, 1.02, 2.02],
    [-5.6500, 0.515, 1.015, 2.05],
    [-5.0700, 0.50, 1.005, 1.99],
    [-4.5300, 0.45, 0.995, 1.83],
    [-4.0900, 0.36, 0.985, 1.55],
    [-3.7700, 0.24, 0.98, 1.18],
    [-3.6300, 0.15, 0.975, 1.04],
];

pub(crate) fn canopy_point(z: f64, phi: f64) -> V3 {
    let [_, w, sill, top] = section(CANOPY, z);
    v3(
        w * phi.cos(),
        sill + (top - sill) * phi.sin().max(0.0).powf(0.86),
        z,
    )
}
pub(crate) fn canopy_bow() -> f64 {
    -7.03
}
pub(crate) fn canopy_aft() -> f64 {
    CANOPY.last().unwrap()[0]
}

// --------------------------------------------------------------- fuselage
// z, half width, centre height, upper/lower half heights, exponents.
const FUSE: &[[f64; 7]] = &[
    [-11.10, 0.010, 0.20, 0.010, 0.010, 2.0, 2.0],
    [-10.78, 0.065, 0.21, 0.060, 0.060, 2.0, 2.0],
    [-10.30, 0.145, 0.23, 0.140, 0.130, 2.0, 2.0],
    [-9.65, 0.235, 0.27, 0.225, 0.210, 2.0, 2.0],
    [-8.95, 0.325, 0.33, 0.320, 0.295, 2.1, 2.1],
    [-8.20, 0.405, 0.39, 0.410, 0.370, 2.2, 2.2],
    [-7.35, 0.475, 0.45, 0.485, 0.435, 2.3, 2.3],
    [-6.50, 0.535, 0.50, 0.535, 0.480, 2.4, 2.4],
    [-5.65, 0.585, 0.535, 0.555, 0.500, 2.45, 2.45],
    [-4.80, 0.625, 0.545, 0.535, 0.485, 2.5, 2.5],
    [-3.90, 0.655, 0.525, 0.500, 0.455, 2.6, 2.55],
    [-2.90, 0.690, 0.470, 0.450, 0.410, 2.7, 2.5],
    [-1.70, 0.760, 0.405, 0.390, 0.355, 2.8, 2.45],
    [-0.30, 0.895, 0.335, 0.325, 0.295, 3.0, 2.5],
    [1.20, 1.070, 0.275, 0.270, 0.245, 3.2, 2.55],
    [2.80, 1.245, 0.225, 0.235, 0.205, 3.3, 2.6],
    [4.30, 1.345, 0.185, 0.215, 0.175, 3.2, 2.55],
    [5.70, 1.295, 0.150, 0.195, 0.150, 3.0, 2.45],
    [7.00, 1.145, 0.115, 0.180, 0.125, 2.7, 2.3],
    [8.20, 0.955, 0.080, 0.165, 0.105, 2.45, 2.2],
    [9.30, 0.735, 0.045, 0.145, 0.090, 2.25, 2.1],
    [10.15, 0.455, 0.015, 0.105, 0.070, 2.1, 2.0],
    [10.55, 0.155, 0.000, 0.060, 0.045, 2.0, 2.0],
];

/// Dorsal spine: height over the base skin and half width. The Su-27's
/// centre fuselage carries fuel and avionics behind the canopy.
fn spine(z: f64) -> (f64, f64) {
    let [_, h, w] = section(
        &[
            [-4.30, 0.00, 0.32],
            [-3.60, 0.08, 0.37],
            [-2.60, 0.15, 0.41],
            [-1.10, 0.26, 0.44],
            [0.20, 0.29, 0.44],
            [1.50, 0.24, 0.41],
            [2.80, 0.14, 0.36],
            [4.10, 0.05, 0.31],
            [5.10, 0.00, 0.28],
        ],
        z,
    );
    (h.max(0.0), w)
}

fn fuselage_point(z: f64, th: f64) -> V3 {
    let [_, w, yc, hu, hl, nu, nl] = section(FUSE, z);
    let (c, s) = (th.cos(), th.sin());
    let n = if s >= 0.0 { nu } else { nl };
    let x = w.max(0.004) * c.signum() * c.abs().powf(2.0 / n);
    let mut y = yc
        + if s >= 0.0 {
            hu.max(0.004) * s.abs().powf(2.0 / n)
        } else {
            -hl.max(0.004) * s.abs().powf(2.0 / n)
        };
    if s > 0.0 {
        let (h, ws) = spine(z);
        let k = 1.0 - (x / ws.max(0.05)).powi(2);
        if h > 0.0 && k > 0.0 {
            y += h * k.powf(1.6);
        }
    }
    v3(x, y, z)
}

/// LEX/centre-body deck: z, inner x, outer x, upper y, lower y.
const LEX: &[[f64; 5]] = &[
    [-5.60, 0.60, 0.60, 0.88, 0.80],
    [-5.05, 0.62, 0.82, 0.84, 0.74],
    [-4.45, 0.64, 1.06, 0.80, 0.70],
    [-3.82, 0.66, 1.34, 0.75, 0.65],
    [-3.12, 0.68, 1.62, 0.70, 0.60],
    [-2.32, 0.70, 1.88, 0.64, 0.54],
    [-1.42, 0.72, 2.04, 0.57, 0.47],
    [-0.52, 0.76, 2.14, 0.49, 0.39],
    [0.30, 0.84, 2.14, 0.40, 0.30],
    [0.66, 0.90, 2.10, 0.35, 0.26],
];

fn lex_surface(m: &mut Mesh, side: f64) {
    for upper in [true, false] {
        m.surface(
            34,
            7,
            |u, v| {
                let z = mix(LEX[0][0], LEX.last().unwrap()[0], u);
                let [_, inner, outer, up, lo] = section(LEX, z);
                let x = mix(inner, outer, v);
                let base = if upper { up } else { lo };
                let y = base
                    + (if upper { 1.0 } else { -1.0 })
                        * 0.045
                        * (1.0 - v).powf(0.7)
                        * (PI * v.min(0.999)).sin().max(0.0);
                v3(side * x, y, z)
            },
            |_, n| if n.y < -0.2 { BELLY } else { LIGHT },
            Some(&|p: V3| {
                v3(
                    0.0,
                    (section(LEX, p.z)[3] + section(LEX, p.z)[4]) * 0.5,
                    p.z,
                )
            }),
        );
    }
    for j in 0..34 {
        let (u0, u1) = (j as f64 / 34.0, (j + 1) as f64 / 34.0);
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
            LIGHT.scale(0.9),
        );
    }
    // A dark seam follows the outer edge of the centre body.
    let line: Vec<_> = (0..18)
        .map(|k| {
            let z = mix(-5.20, 0.42, k as f64 / 17.0);
            let [_, _, outer, up, _] = section(LEX, z);
            v3(side * (outer - 0.10).max(0.58), up + 0.004, z)
        })
        .collect();
    for w in line.windows(2) {
        m.capsule(w[0], w[1], 0.006, 0.006, LIGHT.scale(0.55), 4);
    }
}

/// AL-31F intake/nacelle stations: z, centre x, centre y, half width, half
/// height, superellipse exponent.
const NACELLE: &[[f64; 6]] = &[
    [-3.7800, 1.3000, -0.3400, 0.5320, 0.4650, 4.6000],
    [-3.5600, 1.3000, -0.3300, 0.5719, 0.5100, 4.2000],
    [-3.0500, 1.2900, -0.3200, 0.5985, 0.5400, 3.5000],
    [-1.8000, 1.2800, -0.3100, 0.6118, 0.5550, 2.9000],
    [0.2000, 1.2900, -0.2900, 0.6185, 0.5625, 2.6000],
    [2.3000, 1.3100, -0.2800, 0.6052, 0.5475, 2.4500],
    [4.6000, 1.3300, -0.2700, 0.5653, 0.5175, 2.3500],
    [6.7000, 1.3400, -0.2600, 0.5121, 0.4725, 2.2500],
    [8.3500, 1.3500, -0.2500, 0.4456, 0.4275, 2.1500],
    [9.5500, 1.3500, -0.2400, 0.4600, 0.4300, 2.1000],
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
        34,
        28,
        point,
        |p, n| {
            if p.z > 8.9 {
                BARE
            } else if n.y < -0.35 {
                BELLY.scale(0.96)
            } else if (p.z - 0.2).rem_euclid(1.35) < 0.055 {
                LIGHT.scale(0.86)
            } else {
                LIGHT
            }
        },
        Some(&|p: V3| v3(side * 1.31, -0.28, p.z)),
    );
    // Rectangular intake lip, dark duct and FOD ramp.
    let mouth = |k: usize, inset: f64| {
        let (sx, sy) = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][k];
        v3(
            side * 1.30 + sx * (0.53 - inset * 0.10),
            -0.34 + sy * (0.46 - inset * 0.075) - inset * 0.04,
            -3.76 + inset * 0.90 - sy * 0.16,
        )
    };
    for k in 0..4 {
        let j = (k + 1) % 4;
        m.quad(
            mouth(k, 0.0),
            mouth(j, 0.0),
            mouth(j, 0.82),
            mouth(k, 0.82),
            DUCT,
        );
        m.capsule(
            mouth(k, 0.0),
            mouth(j, 0.0),
            0.040,
            0.040,
            LIGHT.scale(0.93),
            6,
        );
        m.capsule(mouth(k, 0.08), mouth(j, 0.08), 0.009, 0.009, BLACK, 4);
    }
    face(
        m,
        &(0..4).map(|k| mouth(k, 0.86)).collect::<Vec<_>>(),
        -V3::Z,
        BLACK,
    );
    for j in 0..6 {
        let z = -3.62 + j as f64 * 0.105;
        m.capsule(
            v3(side * 1.30 - 0.34, -0.30 + 0.16, z),
            v3(side * 1.30 + 0.34, -0.30 + 0.16, z),
            0.012,
            0.012,
            METAL,
            4,
        );
    }
    // Close the space between the LEX/centre-body underside and the intake
    // shoulder, the surface that gives the Flanker its deep blended belly.
    m.surface(
        28,
        4,
        |u, v| {
            let z = mix(-3.70, 0.62, u);
            let [_, _, outer, _, low] = section(LEX, z);
            let [_, cx, cy, hw, hh, _] = section(NACELLE, z);
            let deck = v3(side * (outer - 0.01), low + 0.015, z);
            let shoulder = v3(side * (cx + hw * 0.70), cy + hh * 0.62, z);
            deck.lerp(shoulder, v)
        },
        |_, _| LIGHT.scale(0.96),
        Some(&|p: V3| v3(0.0, -0.12, p.z)),
    );
    // Splitter gap and inboard fairing.
    plate(
        m,
        &[
            v3(side * 0.91, -0.58, -3.62),
            v3(side * 0.91, 0.18, -3.72),
            v3(side * 0.91, 0.20, 1.90),
            v3(side * 0.91, -0.61, 2.10),
        ],
        V3::X * side,
        0.018,
        LIGHT.scale(0.88),
    );
    // Nozzle shroud, petals and dark exhaust tunnel.
    let n = v3(side * 1.35, -0.24, 9.55);
    m.lathe(
        n - V3::Z * 0.35,
        V3::Z,
        &[(0.0, 0.48), (0.30, 0.46), (0.55, 0.445)],
        24,
        BARE,
    );
    for j in 0..24 {
        let ph = j as f64 * TAU / 24.0;
        let dir = v3(ph.cos(), ph.sin(), 0.0);
        let edge = |k: f64, z: f64, radius: f64| {
            let angle = k * TAU / 24.0;
            n + v3(angle.cos() * radius, angle.sin() * radius, z)
        };
        let centre_angle = (j as f64 + 0.5) * TAU / 24.0;
        face(
            m,
            &[
                edge(j as f64, 0.0, 0.46),
                edge(j as f64 + 1.0, 0.0, 0.46),
                edge(j as f64 + 1.0, 0.38, 0.465),
                edge(j as f64, 0.38, 0.465),
            ],
            v3(centre_angle.cos(), centre_angle.sin(), 0.0),
            if j % 2 == 0 {
                BARE.scale(1.08)
            } else {
                BARE.scale(0.90)
            },
        );

        m.capsule(
            n + dir * 0.435,
            n + V3::Z * 0.38 + dir * 0.465,
            0.010,
            0.012,
            Rgb(0.50, 0.48, 0.45),
            4,
        );
    }
    m.lathe(
        n + V3::Z * 0.36,
        V3::Z,
        &[(0.0, 0.465), (0.02, 0.415)],
        24,
        METAL,
    );
    m.lathe(
        n + V3::Z * 0.38,
        V3::Z,
        &[(0.0, 0.41), (0.01, 0.0)],
        24,
        BLACK,
    );
}

fn fin(m: &mut Mesh, side: f64) {
    let cant = 6.0 * DEG;
    let at = |y: f64, z: f64| v3(side * (1.34 + (y - 0.48) * cant.tan()), y, z);
    // Fixed fin ends at the rudder hinge.
    plate(
        m,
        &[
            at(0.46, 6.30),
            at(3.98, 8.62),
            at(3.96, 9.08),
            at(0.46, 8.90),
        ],
        V3::X * side,
        0.035,
        LIGHT.scale(1.12),
    );
    // Root fairing, tip cap and leading-edge strip.
    m.ellipsoid(
        v3(side * 1.36, 0.50, 7.72),
        v3(0.15, 0.20, 1.40),
        LIGHT,
        5,
        10,
    );
    m.capsule(
        at(3.94, 8.66),
        at(3.94, 9.08),
        0.018,
        0.014,
        LIGHT.scale(0.95),
        5,
    );
    m.capsule(
        at(0.60, 6.45),
        at(3.82, 8.68),
        0.018,
        0.012,
        LIGHT.scale(0.90),
        5,
    );
    m.capsule(at(4.00, 8.68), at(4.00, 9.06), 0.012, 0.008, METAL, 4);
}

pub(crate) fn rudder_hinge(side: f64) -> (V3, V3) {
    let cant = 6.0 * DEG;
    let at = |y: f64, z: f64| v3(side * (1.34 + (y - 0.48) * cant.tan()), y, z);
    let (a, b) = (at(0.72, 8.90), at(3.96, 9.08));
    ((a + b) * 0.5, ((b - a) * side).norm())
}

pub(crate) fn flap_hinge(side: f64) -> (V3, V3) {
    let at = |x: f64| {
        let u = (x - ROOT_X) / (TIP_X - ROOT_X);
        let le = v3(ROOT_X, 0.18, ROOT_LE).lerp(v3(TIP_X, 0.10, TIP_LE), u);
        let te = v3(ROOT_X, 0.12, ROOT_TE).lerp(v3(TIP_X, 0.08, TIP_TE), u);
        let p = le.lerp(te, 1.0 - FLAP_CHORD / (te.z - le.z));
        v3(side * p.x, p.y, p.z)
    };
    let (p0, p1) = (at(2.55), at(6.55));
    ((p0 + p1) * 0.5, ((p1 - p0) * side).norm())
}

/// Red star with a white border on a fin or wing.
fn star(m: &mut Mesh, c: V3, n: V3, r: f64) {
    let e2 = if n.y.abs() > 0.9 {
        -V3::Z
    } else {
        (V3::Y - n * n.y).norm()
    };
    let e1 = e2.cross(n).norm();
    for (rad, lift, col) in [(r * 1.18, 0.010, WHITE), (r, 0.016, STAR)] {
        let o = c + n * lift;
        let p = |k: usize| {
            let a = k as f64 * PI / 5.0 + PI * 0.5;
            let radius = if k % 2 == 0 { rad } else { rad * 0.40 };
            o + (e1 * a.cos() + e2 * a.sin()) * radius
        };
        let outline: Vec<_> = (0..10).map(p).collect();
        face(m, &outline, n, col);
    }
}

/// Paint only the light upper-skin vertices with the shader's digital
/// blue-grey camouflage, exactly as the MiG-29 does.
fn flag_camo(m: &mut Mesh) {
    for c in m.v.chunks_mut(crate::mesh::STRIDE) {
        let k = c[7] as f64 / LIGHT.1;
        let grey =
            (c[6] as f64 - LIGHT.0 * k).abs() < 0.01 && (c[8] as f64 - LIGHT.2 * k).abs() < 0.01;
        if grey && k > 0.84 && k < 1.16 {
            c[9] = -3.0;
            c[10] = -3.0;
        }
    }
}

/// Mid-surface of the closed dorsal door, just above the curved spine.
fn brake_point(u: f64, v: f64) -> V3 {
    let z = BRAKE_FRONT + u * BRAKE_LENGTH;
    let x = (2.0 * v - 1.0) * mix(0.46, 0.38, u);
    let [_, _, yc, hu, ..] = section(FUSE, z);
    let (h, w) = spine(z);
    let y = yc + hu + h * (1.0 - (x / w).powi(2)).max(0.0).powf(1.6);
    v3(x, y + 0.011, z)
}

/// Panel joints, access covers and fasteners are actual shallow geometry.
fn surface_details(m: &mut Mesh) {
    for z in [-9.0, -7.35, -3.9, -1.7, 2.8, 5.7, 8.2] {
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
                LIGHT.scale(0.67),
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
                m.capsule(point(a), point(b), 0.003, 0.003, LIGHT.scale(0.62), 4);
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

fn build_flanker() -> Airframe {
    let mut body = Mesh::new();
    let (z0, z1) = (FUSE[0][0], FUSE.last().unwrap()[0]);
    let paint = |p: V3, n: V3| {
        if p.z < -9.0 {
            RADOME
        } else if n.y < -0.30 {
            BELLY
        } else {
            LIGHT
        }
    };
    body.surface(
        78,
        34,
        |u, v| fuselage_point(mix(z0, z1, u), v * TAU),
        paint,
        Some(&|p: V3| v3(0.0, section(FUSE, p.z)[2], p.z)),
    );
    // Pitot boom, IRST ball, AoA vanes and radome tip.
    body.capsule(
        v3(0.0, 0.20, -11.05),
        v3(0.0, 0.20, -11.38),
        0.016,
        0.003,
        METAL,
        5,
    );
    body.capsule(
        v3(0.0, 0.20, -10.95),
        v3(0.0, 0.20, -11.10),
        0.030,
        0.018,
        METAL,
        6,
    );
    body.ellipsoid(
        v3(0.0, 1.00, -8.02),
        v3(0.145, 0.145, 0.19),
        Rgb(0.16, 0.25, 0.28),
        8,
        14,
    );
    body.ellipsoid(v3(0.0, 1.00, -8.02), v3(0.075, 0.075, 0.105), BLACK, 5, 10);
    for side in [-1.0, 1.0] {
        body.capsule(
            v3(side * 0.31, 0.38, -8.55),
            v3(side * 0.42, 0.40, -8.47),
            0.009,
            0.004,
            METAL,
            4,
        );
    }
    // Gun bay and the starboard cannon blast panel.
    body.block(
        &frame(v3(0.73, 0.72, -4.90)),
        v3(0.075, 0.035, 0.30),
        LIGHT.scale(0.9),
    );

    let le = |x: f64| {
        v3(ROOT_X, 0.18, ROOT_LE).lerp(v3(TIP_X, 0.10, TIP_LE), (x - ROOT_X) / (TIP_X - ROOT_X))
    };
    let te = |x: f64| {
        v3(ROOT_X, 0.12, ROOT_TE).lerp(v3(TIP_X, 0.08, TIP_TE), (x - ROOT_X) / (TIP_X - ROOT_X))
    };
    let thickness = |x: f64| 0.215 * mix(1.0, 0.36, (x - ROOT_X) / (TIP_X - ROOT_X));

    let mut flaps = [Mesh::new(), Mesh::new()];
    let mut stabs = [Mesh::new(), Mesh::new()];
    let mut rudders = [Mesh::new(), Mesh::new()];
    for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
        lex_surface(&mut body, side);
        nacelle(&mut body, side);
        root_glove(
            &mut body, side, 1.48, 2.015, -0.98, 3.42, 0.10, 0.18, LIGHT, BELLY,
        );

        let panel = |p: V3| v3(side * p.x, p.y, p.z);
        let (flap_x0, flap_x1) = (2.55, 6.55);
        for (x0, x1, notch) in [
            (ROOT_X, flap_x0, false),
            (flap_x0, flap_x1, true),
            (flap_x1, TIP_X, false),
        ] {
            wing_segment_chord(
                &mut body,
                panel(le(x0)),
                panel(le(x1)),
                panel(te(x1)),
                panel(te(x0)),
                [thickness(x0), thickness(x1)],
                LIGHT,
                if notch { 1 } else { 0 },
                FLAP_CHORD,
            );
        }
        // Leading-edge slat and its track fairings.
        let (slat0, slat1) = (2.10, 6.92);
        body.surface(
            16,
            3,
            |u, v| {
                let x = mix(slat0, slat1, u);
                let p = le(x);
                let chord = te(x).z - p.z;
                let t = v * 0.085;
                let h = 5.0
                    * (0.2969 * t.sqrt() - 0.1260 * t - 0.3516 * t * t + 0.2843 * t.powi(3)
                        - 0.1036 * t.powi(4));
                v3(
                    side * x,
                    p.y - 0.015 + h * thickness(x),
                    p.z + chord * t - 0.012,
                )
            },
            |_, _| LIGHT.scale(0.85),
            Some(&|p: V3| v3(p.x, p.y + 0.1, p.z)),
        );
        wing_segment_chord(
            &mut flaps[i],
            panel(le(flap_x0)),
            panel(le(flap_x1)),
            panel(te(flap_x1)),
            panel(te(flap_x0)),
            [thickness(flap_x0), thickness(flap_x1)],
            LIGHT,
            2,
            FLAP_CHORD,
        );
        // Hinge seams and wing fold line.
        let hinge = |x: f64| {
            let (l, t) = (le(x), te(x));
            l.lerp(t, 1.0 - FLAP_CHORD / (t.z - l.z))
        };
        for x in [flap_x0, 4.35, 5.35, flap_x1] {
            let p = hinge(x);
            body.capsule(
                panel(p + v3(0.0, 0.004, 0.0)),
                panel(p + v3(0.0, 0.004, 0.25)),
                0.006,
                0.006,
                LIGHT.scale(0.60),
                4,
            );
        }
        body.capsule(
            panel(le(4.55) + v3(0.0, 0.010, 0.15)),
            panel(te(4.55) + v3(0.0, 0.004, -0.02)),
            0.005,
            0.005,
            LIGHT.scale(0.65),
            4,
        );
        // Pylons and wingtip launch rail.
        for (px, pz) in [(2.55, 1.15), (3.55, 1.35), (4.60, 1.55)] {
            let y = le(px).y - 0.22;
            body.block(
                &frame(v3(side * px, y, pz)),
                v3(0.060, 0.18, 0.65),
                LIGHT.scale(0.92),
            );
        }
        body.capsule(
            panel(v3(TIP_X + 0.09, 0.10, 4.55)),
            panel(v3(TIP_X + 0.09, 0.10, 5.86)),
            0.046,
            0.046,
            METAL.scale(0.78),
            6,
        );
        body.capsule(
            panel(v3(TIP_X + 0.11, 0.12, 4.62)),
            panel(v3(TIP_X + 0.11, 0.12, 5.74)),
            0.026,
            0.026,
            Rgb(0.13, 0.14, 0.15),
            5,
        );
        body.ellipsoid(
            panel(v3(TIP_X + 0.08, 0.12, 5.95)),
            v3(0.032, 0.032, 0.08),
            if side < 0.0 {
                Rgb(0.82, 0.07, 0.04)
            } else {
                Rgb(0.08, 0.62, 0.24)
            },
            5,
            8,
        );

        fin(&mut body, side);
        // Stabilator with a broad root and clipped, swept tip.
        let root_le = v3(side * 1.80, 0.16, 6.30);
        let tip_le = v3(side * 4.70, -0.02, 8.32);
        let tip_te = v3(side * 4.22, -0.02, 9.05);
        let root_te = v3(side * 1.80, 0.16, 9.02);
        wing_panel(
            &mut stabs[i],
            root_le,
            tip_le,
            tip_te,
            root_te,
            0.095,
            LIGHT,
        );
        let cant = 6.0 * DEG;
        let at = |y: f64, z: f64| v3(side * (1.34 + (y - 0.48) * cant.tan()), y, z);
        plate(
            &mut rudders[i],
            &[
                at(0.72, 8.90),
                at(3.96, 9.08),
                at(3.96, 9.42),
                at(0.72, 9.26),
            ],
            V3::X * side,
            0.028,
            LIGHT.scale(1.12),
        );
        // Red star with white outline on the fin, and a formation strip.
        star(
            &mut body,
            at(1.72, 7.65) + v3(side * 0.045, 0.0, 0.0),
            v3(side * cant.cos(), -cant.sin(), 0.0),
            0.30,
        );
        body.capsule(
            at(0.75, 6.55) + v3(side * 0.042, 0.0, 0.0),
            at(1.20, 7.30) + v3(side * 0.042, 0.0, 0.0),
            0.016,
            0.016,
            Rgb(0.56, 0.58, 0.45),
            4,
        );
        // Ventral fin below the tail boom.
        plate(
            &mut body,
            &[
                v3(side * 1.38, -0.26, 6.45),
                v3(side * 1.62, -1.08, 7.40),
                v3(side * 1.50, -1.12, 8.28),
                v3(side * 1.20, -0.26, 8.10),
            ],
            v3(side * 0.75, 0.25, 0.0).norm(),
            0.022,
            LIGHT.scale(0.9),
        );
    }

    // Canopy, frames and rear fairing.
    let mut canopy = Mesh::new();
    let (cz0, cz1) = (CANOPY[0][0], CANOPY.last().unwrap()[0]);
    canopy.surface(
        26,
        20,
        |u, v| canopy_point(mix(cz0, cz1, u), v * PI),
        |_, _| Rgb(0.25, 0.41, 0.43),
        Some(&|p: V3| v3(0.0, 1.02, p.z)),
    );
    for z in [cz0, canopy_bow(), cz1] {
        for j in 0..20 {
            body.capsule(
                canopy_point(z, j as f64 * PI / 20.0),
                canopy_point(z, (j + 1) as f64 * PI / 20.0),
                if z == cz0 || z == cz1 { 0.032 } else { 0.018 },
                0.026,
                if z == cz0 || z == cz1 {
                    LIGHT.scale(0.75)
                } else {
                    METAL.scale(0.9)
                },
                5,
            );
        }
    }
    for edge in [0.0, PI] {
        for j in 0..26 {
            body.capsule(
                canopy_point(mix(cz0, cz1, j as f64 / 26.0), edge),
                canopy_point(mix(cz0, cz1, (j + 1) as f64 / 26.0), edge),
                0.025,
                0.025,
                LIGHT.scale(0.8),
                5,
            );
        }
    }
    for phi in [0.70, PI - 0.70] {
        for j in 0..12 {
            let z = |k| mix(cz0, canopy_bow(), k as f64 / 12.0);
            body.capsule(
                canopy_point(z(j), phi),
                canopy_point(z(j + 1), phi),
                0.014,
                0.014,
                METAL,
                6,
            );
        }
    }
    body.ellipsoid(v3(0.0, 0.99, -3.60), v3(0.34, 0.115, 0.42), LIGHT, 5, 10);

    // Dorsal airbrake, closed on the spine.
    let mut brake = Mesh::new();
    for upper in [true, false] {
        brake.surface(
            20,
            8,
            |u, v| brake_point(u, v) + V3::Y * if upper { 0.007 } else { -0.007 },
            |_, _| if upper { LIGHT.scale(0.95) } else { METAL },
            Some(&|p: V3| p + V3::Y * if upper { -0.1 } else { 0.1 }),
        );
    }
    // Close the thin door's side walls and end caps.
    for edge in [0.0, 1.0] {
        for j in 0..20 {
            let a = brake_point(j as f64 / 20.0, edge);
            let b = brake_point((j + 1) as f64 / 20.0, edge);
            face(
                &mut brake,
                &[
                    a - V3::Y * 0.007,
                    b - V3::Y * 0.007,
                    b + V3::Y * 0.007,
                    a + V3::Y * 0.007,
                ],
                V3::X * (edge * 2.0 - 1.0),
                LIGHT.scale(0.85),
            );
        }
        for j in 0..8 {
            let a = brake_point(edge, j as f64 / 8.0);
            let b = brake_point(edge, (j + 1) as f64 / 8.0);
            face(
                &mut brake,
                &[
                    a - V3::Y * 0.007,
                    b - V3::Y * 0.007,
                    b + V3::Y * 0.007,
                    a + V3::Y * 0.007,
                ],
                V3::Z * (edge * 2.0 - 1.0),
                LIGHT.scale(0.85),
            );
        }
    }
    // Blade antenna and the tail sting between the nozzles.
    plate(
        &mut body,
        &[
            v3(-0.015, 0.98, -1.55),
            v3(-0.015, 1.24, -1.45),
            v3(-0.015, 1.01, -1.12),
        ],
        V3::X,
        0.015,
        LIGHT,
    );
    body.capsule(
        v3(0.0, 0.05, 8.95),
        v3(0.0, 0.03, 10.45),
        0.30,
        0.13,
        LIGHT,
        10,
    );
    body.capsule(
        v3(0.0, -0.10, 9.80),
        v3(0.0, -0.10, 10.50),
        0.045,
        0.025,
        METAL,
        6,
    );
    // Brake parachute container of the land-based Su-27.
    body.ellipsoid(
        v3(0.0, 0.10, 10.22),
        v3(0.20, 0.12, 0.42),
        LIGHT.scale(0.9),
        5,
        10,
    );
    // Wingtip formation lights and rear navigation lamps.
    for side in [-1.0, 1.0] {
        body.ellipsoid(
            v3(side * 0.20, 0.06, 10.38),
            v3(0.035, 0.035, 0.05),
            Rgb(0.80, 0.72, 0.55),
            4,
            8,
        );
    }

    surface_details(&mut body);
    flag_camo(&mut body);
    for m in flaps
        .iter_mut()
        .chain(stabs.iter_mut())
        .chain(rudders.iter_mut())
        .chain(std::iter::once(&mut brake))
    {
        flag_camo(m);
    }

    let gear = gear_legs(Type::Su27);
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

/// Offset that carries a store from the F-16 station `at` onto the Flanker's
/// matching pylon or wingtip rail.
pub fn store_shift(at: V3) -> V3 {
    let x = at.x.abs();
    if x < 0.1 {
        return V3::ZERO;
    }
    let side = at.x.signum();
    let (from, to) = if x > 4.6 {
        (v3(4.8, 0.30, 0.55), v3(7.32, 0.10, 4.70))
    } else if x > 3.8 {
        (v3(4.15, -0.06, 1.0), v3(4.60, -0.36, 1.78))
    } else if x > 2.9 {
        (v3(3.5, -0.06, 0.52), v3(3.55, -0.36, 1.60))
    } else {
        (v3(2.3, -0.08, -0.28), v3(2.55, -0.40, 1.32))
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
    fn flanker_matches_its_three_view() {
        let f = build();
        let (lo, hi) = extent(&[&f.body, &f.stabs[0], &f.stabs[1]]);
        let (span, length) = (hi.x - lo.x, hi.z - lo.z);
        assert!((span - 14.7).abs() < 0.35, "span {span:.2} m");
        assert!((length - 21.9).abs() < 0.35, "length {length:.2} m");
        assert!(f.body.count() > 14000, "body {} vertices", f.body.count());
        assert!(f.canopy.count() > 700);
        assert!(f.gear.iter().all(|m| m.count() > 200));
    }

    #[test]
    fn eye_and_helmet_fit_beneath_the_canopy() {
        let sill = canopy_point(EYE.z, 0.0);
        let crown = canopy_point(EYE.z, PI * 0.5);
        assert!(EYE.y > sill.y + 0.30, "eye {EYE:?} sill {sill:?}");
        assert!(crown.y > EYE.y + 0.30, "crown {crown:?}");
    }

    #[test]
    fn baseline_flanker_has_no_canards_or_outboard_foreplanes() {
        let f = build();
        for v in f.body.v.chunks(STRIDE) {
            if (v[2] as f64) < ROOT_LE - 0.05 {
                assert!(v[0].abs() < 2.20, "foreplane ahead of wing: {v:?}");
            }
        }
    }

    #[test]
    fn flap_skin_meets_the_fixed_wing_at_the_hinge() {
        let f = build();
        for i in 0..2 {
            let hinge_z = |x: f64| {
                let u = (x.abs() - ROOT_X) / (TIP_X - ROOT_X);
                let te = ROOT_TE + (TIP_TE - ROOT_TE) * u;
                te - FLAP_CHORD
            };
            let body: Vec<_> = f
                .body
                .v
                .chunks(STRIDE)
                .map(|v| v3(v[0] as f64, v[1] as f64, v[2] as f64))
                .collect();
            let mut checked = 0;
            for v in f.flaps[i].v.chunks(STRIDE) {
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
    fn fin_insignia_has_five_points_and_recessed_inner_corners() {
        let mut m = Mesh::new();
        star(&mut m, V3::ZERO, V3::Y, 1.0);
        let mut tips = Vec::new();
        let mut inner = 0;
        for v in
            m.v.chunks(STRIDE)
                .filter(|v| (v[6] - STAR.0 as f32).abs() < 1e-6)
        {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if (r - 1.0).abs() < 1e-5 {
                let point = (v[0], v[2]);
                if !tips.contains(&point) {
                    tips.push(point);
                }
            } else if (r - 0.40).abs() < 1e-5 {
                inner += 1;
            } else {
                assert!(r < 1e-5, "unexpected star radius {r}");
            }
        }
        assert_eq!(tips.len(), 5);
        assert!(inner >= 5);
    }

    #[test]
    fn closed_speedbrake_follows_the_spine_without_floating() {
        for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let p = brake_point(u, 0.5);
            let skin = fuselage_point(p.z, PI * 0.5);
            assert!((p.y - skin.y - 0.011).abs() < 1e-8);
            for v in [0.0, 0.25, 0.75, 1.0] {
                assert!(brake_point(u, v).y <= p.y);
            }
        }
    }

    #[test]
    fn each_tyre_meets_its_simulated_contact_point() {
        let f = build();
        for (i, leg) in f.gear.iter().enumerate() {
            let bottom = leg
                .v
                .chunks(STRIDE)
                .map(|v| v[1] as f64)
                .fold(f64::INFINITY, f64::min);
            assert!(
                (bottom - Type::Su27.wheels()[i].local.y).abs() < 0.001,
                "leg {i}: {bottom}"
            );
        }
        for m in f
            .stabs
            .iter()
            .chain(f.flaps.iter())
            .chain(f.rudders.iter())
            .chain(std::iter::once(&f.brake))
        {
            assert!(m.v.iter().all(|v| v.is_finite()));
        }
    }
}
