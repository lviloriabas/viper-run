//! F-16C exterior in low-poly pixel-art style: a few hundred smooth
//! (Gouraud) polygons wearing a small hand-painted texture of panel lines and
//! markings, sampled with nearest filtering.
//!
//! Proportions follow a scale three-view of the F-16 (the same shapes the War
//! Thunder model reproduces): 15.06 m from pitot tip to fin tip, 9.96 m span
//! over the wingtip AIM-9s, 4.88 m high on the gear, a long radome, the canopy
//! and chin intake well aft of the nose, a shallow forebody and the engine
//! axis above the nose line. Body axes: right (+x), up (+y), aft (+z), metres;
//! the origin is the flight model's centre of gravity.
use crate::font;
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb, STRIDE};

const WHITE: Rgb = Rgb(1.0, 1.0, 1.0);
const DUCT: Rgb = Rgb(0.05, 0.06, 0.065);
const METAL: Rgb = Rgb(0.36, 0.35, 0.33);
const DARK_METAL: Rgb = Rgb(0.18, 0.17, 0.16);

/// Pilot design eye. The seat is reclined 30 degrees, so the eye sits aft of the hips.
pub const EYE: V3 = v3(0.0, 1.25, -3.24);
/// Ground under the wheels with the gear down and the jet at rest.
#[cfg(test)]
const GROUND_Y: f64 = -1.24;

// ------------------------------------------------------------------ fuselage
// z, half width, centre height, upper and lower half heights, superellipse exponents.
const FUSE: [[f64; 7]; 16] = [
    [-7.09, 0.02, 0.31, 0.02, 0.02, 2.0, 2.0],
    [-6.76, 0.14, 0.325, 0.125, 0.125, 2.0, 2.0],
    [-6.19, 0.28, 0.355, 0.225, 0.225, 2.0, 2.0],
    [-5.39, 0.40, 0.42, 0.34, 0.34, 2.1, 2.2],
    [-4.63, 0.52, 0.495, 0.435, 0.435, 2.3, 2.5],
    [-3.79, 0.58, 0.53, 0.45, 0.45, 2.5, 2.8],
    [-3.06, 0.62, 0.56, 0.44, 0.44, 2.6, 3.0],
    [-2.09, 0.68, 0.565, 0.465, 0.465, 2.7, 3.2],
    [-1.41, 0.72, 0.56, 0.48, 0.48, 2.8, 3.3],
    [-0.39, 0.78, 0.54, 0.49, 0.49, 2.8, 3.3],
    [1.21, 0.80, 0.535, 0.485, 0.485, 2.8, 3.3],
    [2.71, 0.78, 0.55, 0.48, 0.48, 2.7, 3.0],
    [3.91, 0.74, 0.55, 0.55, 0.55, 2.4, 2.6],
    [4.91, 0.66, 0.55, 0.51, 0.51, 2.2, 2.3],
    [5.91, 0.56, 0.55, 0.49, 0.49, 2.0, 2.0],
    [6.15, 0.53, 0.55, 0.48, 0.48, 2.0, 2.0],
];
const FUSE_Z: (f64, f64) = (-7.09, 6.15);
const FUSE_NZ: usize = 26;
const FUSE_NTH: usize = 12;
// Ventral inlet duct, from the D-shaped mouth back into the lower fuselage.
const INTAKE: [[f64; 7]; 8] = [
    [-3.06, 0.50, -0.15, 0.27, 0.28, 4.0, 2.4],
    [-2.49, 0.57, -0.12, 0.30, 0.31, 3.5, 2.4],
    [-1.09, 0.67, -0.06, 0.34, 0.37, 3.2, 2.6],
    [0.41, 0.73, 0.00, 0.36, 0.43, 3.2, 2.8],
    [1.91, 0.72, 0.05, 0.34, 0.46, 3.2, 2.8],
    [3.31, 0.66, 0.12, 0.30, 0.42, 3.0, 2.6],
    [4.51, 0.56, 0.20, 0.26, 0.34, 3.0, 2.6],
    [5.31, 0.48, 0.26, 0.22, 0.28, 3.0, 2.6],
];
const INTAKE_Z: (f64, f64) = (-3.06, 5.31);
const NOZZLE_Y: f64 = 0.55;
/// Nozzle exit plane, where the exhaust effects start.
pub const NOZZLE_EXIT: V3 = v3(0.0, NOZZLE_Y, 6.73);

#[derive(Clone, Copy)]
pub struct Section {
    pub w: f64,
    pub yc: f64,
    pub hu: f64,
    pub hl: f64,
    pub nu: f64,
    pub nl: f64,
}

fn catmull(table: &[[f64; 7]], z: f64) -> Section {
    let mut k = 0;
    while k < table.len() - 2 && z > table[k + 1][0] {
        k += 1;
    }
    let t = clamp((z - table[k][0]) / (table[k + 1][0] - table[k][0]), 0.0, 1.0);
    let a = &table[k.saturating_sub(1)];
    let b = &table[k];
    let c = &table[k + 1];
    let d = &table[(k + 2).min(table.len() - 1)];
    let f = |i: usize| {
        0.5 * ((2.0 * b[i]) + (-a[i] + c[i]) * t + (2.0 * a[i] - 5.0 * b[i] + 4.0 * c[i] - d[i]) * t * t
            + (-a[i] + 3.0 * b[i] - 3.0 * c[i] + d[i]) * t * t * t)
    };
    Section { w: f(1).max(0.0), yc: f(2), hu: f(3).max(0.0), hl: f(4).max(0.0), nu: f(5), nl: f(6) }
}

pub fn section(z: f64) -> Section {
    catmull(&FUSE, z)
}

fn lerp_table(t: &[(f64, f64)], z: f64) -> f64 {
    if z <= t[0].0 {
        return t[0].1;
    }
    for k in 0..t.len() - 1 {
        if z <= t[k + 1].0 {
            let s = (z - t[k].0) / (t[k + 1].0 - t[k].0);
            let s = s * s * (3.0 - 2.0 * s);
            return mix(t[k].1, t[k + 1].1, s);
        }
    }
    t[t.len() - 1].1
}

/// Dorsal spine behind the canopy, flowing back to the base of the fin.
fn spine(z: f64) -> (f64, f64) {
    let h = lerp_table(&[(-1.60, 0.0), (-1.30, 0.20), (0.0, 0.20), (2.0, 0.17), (3.9, 0.13), (5.5, 0.07), (6.1, 0.0)], z);
    let w = lerp_table(&[(-1.6, 0.30), (0.0, 0.28), (3.9, 0.20), (6.1, 0.15)], z);
    (h, w)
}

fn superellipse(s: &Section, th: f64) -> (f64, f64) {
    let (c, sn) = (th.cos(), th.sin());
    let up = sn >= 0.0;
    let n = if up { s.nu } else { s.nl };
    let x = s.w * c.signum() * c.abs().powf(2.0 / n);
    let y = s.yc + if up { s.hu } else { s.hl } * sn.signum() * sn.abs().powf(2.0 / n);
    (x, y)
}

pub fn fuselage_point(z: f64, th: f64) -> V3 {
    let s = section(z);
    let (x, mut y) = superellipse(&s, th);
    if th.sin() > 0.0 {
        let (hs, ws) = spine(z);
        let k = 1.0 - (x / ws).powi(2);
        if hs > 0.0 && k > 0.0 {
            y += hs * k.powf(1.5);
        }
    }
    v3(x, y, z)
}

/// Height of the upper fuselage skin at a lateral offset (no spine).
pub fn fuselage_top_at(z: f64, x: f64) -> f64 {
    let s = section(z);
    let r = clamp(x.abs() / s.w.max(1e-6), 0.0, 1.0);
    s.yc + s.hu * (1.0 - r.powf(s.nu)).max(0.0).powf(1.0 / s.nu)
}

// -------------------------------------------------------------------- canopy
pub const CANOPY_FRONT: f64 = -4.63;
pub const CANOPY_PEAK: f64 = -3.06;
pub const CANOPY_AFT: f64 = -1.41;
const CANOPY_TOP: f64 = 1.58;
const CANOPY_W: f64 = 0.46;
/// Canopy facets, front to aft and from sill to sill.
const CANOPY_NU: usize = 10;
pub const CANOPY_NV: usize = 8;

/// Canopy half width, base height and top height at a station.
pub fn canopy_profile(z: f64) -> (f64, f64, f64) {
    let z = clamp(z, CANOPY_FRONT, CANOPY_AFT);
    let (w, top);
    if z <= CANOPY_PEAK {
        let t = (z - CANOPY_FRONT) / (CANOPY_PEAK - CANOPY_FRONT);
        let g = (1.0 - (1.0 - t).powi(2)).max(0.0);
        w = CANOPY_W * g.powf(0.62);
        top = mix(fuselage_top_at(z, 0.0), CANOPY_TOP, g.sqrt());
    } else {
        let t = (z - CANOPY_PEAK) / (CANOPY_AFT - CANOPY_PEAK);
        w = CANOPY_W - 0.16 * t.powf(1.6);
        top = CANOPY_TOP - 0.36 * t.powf(1.7);
    }
    let base = fuselage_top_at(z, w) - 0.015;
    (w, base, top.max(base))
}

/// Bubble canopy surface; u runs front to aft, v from the right sill over to the left.
pub fn canopy_point(u: f64, v: f64) -> V3 {
    let z = mix(CANOPY_FRONT, CANOPY_AFT, u);
    let (w, base, top) = canopy_profile(z);
    let a = v * PI;
    let (c, s) = (a.cos(), a.sin());
    let n = 2.25;
    v3(w * c.signum() * c.abs().powf(2.0 / n), base + (top - base) * s.abs().powf(2.0 / n), z)
}

// ---------------------------------------------------------------- lifting surfaces
fn naca(v: f64) -> f64 {
    let v = clamp(v, 0.0, 1.0);
    0.2969 * v.sqrt() - 0.1260 * v - 0.3516 * v * v + 0.2843 * v * v * v - 0.1036 * v * v * v * v
}

fn cosine_spacing(s: f64) -> f64 {
    0.5 * (1.0 - (PI * s).cos())
}

const WING_Y: f64 = 0.33;
const WING_TE: f64 = 3.26;
const WING_TIP: f64 = 4.65;
const WING_NU: usize = 8;
const WING_NV: usize = 6;
/// The 40-degree leading edge meets the blended strake here.
const LERX_JOIN: (f64, f64) = (1.34, -0.41);
/// The strake starts beside the windscreen and curves out to the join.
const LERX_ROOT: (f64, f64) = (0.56, -4.33);

fn wing_le40(x: f64) -> f64 {
    LERX_JOIN.1 + 0.839 * (x - LERX_JOIN.0)
}

fn wing_le(x: f64) -> f64 {
    if x >= LERX_JOIN.0 {
        return wing_le40(x);
    }
    let t = (clamp(x - LERX_ROOT.0, 0.0, LERX_JOIN.0 - LERX_ROOT.0) / (LERX_JOIN.0 - LERX_ROOT.0)).powf(1.0 / 0.7);
    (LERX_ROOT.1 + (LERX_JOIN.1 - LERX_ROOT.1) * t).min(wing_le40(x))
}

fn wing_point(side: f64, u: f64, v: f64, upper: bool) -> V3 {
    let x = 0.40 + (WING_TIP - 0.40) * u;
    let le = wing_le(x);
    let chord = WING_TE - le;
    let chord40 = WING_TE - wing_le40(x);
    let t = 0.040 - 0.010 * u;
    let w = cosine_spacing(v);
    let half = 5.0 * t * chord40 * naca(w) * (chord40 / chord).sqrt();
    let z = le + chord * w;
    // The strake surface curls up slightly toward the forebody chine.
    let lift = 0.08 * clamp((wing_le40(x) - z) / (wing_le40(x) - LERX_ROOT.1), 0.0, 1.0).powf(1.5);
    v3(side * x, WING_Y + lift + if upper { half } else { -half }, z)
}

const FIN_ROOT_Y: f64 = 1.10;
const FIN_TIP_Y: f64 = 3.64;

fn fin_point(u: f64, v: f64, side: f64) -> V3 {
    let y = FIN_ROOT_Y + (FIN_TIP_Y - FIN_ROOT_Y) * u;
    let le = 3.84 + (6.04 - 3.84) * u;
    let te = 6.10 + (7.09 - 6.10) * u;
    let chord = te - le;
    let half = 5.0 * 0.05 * chord * naca(v) * (1.0 - 0.3 * u);
    v3(side * half, y, le + chord * v)
}

pub const STAB_PIVOT: V3 = v3(1.06, 0.55, 5.35);
const ANHEDRAL: f64 = 0.176; // tan(10 deg)
const STAB_ROOT: f64 = 1.06;
const STAB_TIP: f64 = 2.79;

fn stab_point(side: f64, u: f64, v: f64, upper: bool) -> V3 {
    let x = STAB_ROOT + (STAB_TIP - STAB_ROOT) * u;
    let le = 4.28 + (5.71 - 4.28) * u;
    let te = 6.54 + 0.11 * u;
    let chord = te - le;
    let w = cosine_spacing(v);
    let half = 5.0 * 0.04 * chord * naca(w);
    let y = STAB_PIVOT.y - (x - STAB_ROOT) * ANHEDRAL;
    v3(side * x, y + if upper { half } else { -half }, le + chord * w)
}

pub fn stab_axis(side: f64) -> V3 {
    v3(side, -ANHEDRAL, 0.0).norm() * side
}

pub const RUDDER_HINGE: f64 = 0.70;
pub const RUDDER_TOP: f64 = 0.92;

pub fn rudder_axis() -> (V3, V3) {
    let a = fin_point(0.0, RUDDER_HINGE, 0.0);
    let b = fin_point(RUDDER_TOP, RUDDER_HINGE, 0.0);
    (a, (b - a).norm())
}

pub const BRAKE_HINGE_Z: f64 = 5.95;
pub const BOOM_Y: f64 = 0.42;

pub struct Airframe {
    pub exterior: Mesh,
    pub canopy: Mesh,
    pub crew: Mesh,
    pub flame: Mesh,
    pub stab: [Mesh; 2],
    pub rudder: Mesh,
    /// Airbrake petals: left upper, left lower, right upper, right lower.
    pub brakes: [Mesh; 4],
    /// Left and right wingtip AIM-9s.
    pub tips: [Mesh; 2],
    /// Nose, left main and right main landing gear legs, extended.
    pub gear: [Mesh; 3],
    /// Gear well doors that hinge on the airframe (see `GEAR_DOORS`).
    pub gear_doors: [Mesh; 1],
    /// Paint texture, `SKIN_W` x `SKIN_H` RGBA.
    pub skin: Vec<u8>,
}

// ------------------------------------------------------------------ paint
// The skin texture holds four orthographic views: top and bottom on the
// left half, right and left sides on the right half. Every skin triangle
// takes the view its normal faces most.
pub const SKIN_W: usize = 512;
pub const SKIN_H: usize = 256;
const PAINT_Z: (f64, f64) = (-8.0, 7.6);
const PLAN_X: f64 = 5.0;
const SIDE_Y: (f64, f64) = (3.9, -0.7);

#[derive(Clone, Copy, PartialEq)]
enum View {
    Top,
    Bottom,
    Right,
    Left,
}

/// Texture coordinates of a body point in one of the painted views.
fn view_uv(view: View, p: V3) -> (f64, f64) {
    let u = (p.z - PAINT_Z.0) / (PAINT_Z.1 - PAINT_Z.0) * 0.5;
    let plan = (p.x + PLAN_X) / (2.0 * PLAN_X) * 0.5;
    let side = (SIDE_Y.0 - p.y) / (SIDE_Y.0 - SIDE_Y.1) * 0.5;
    match view {
        View::Top => (u, plan),
        View::Bottom => (u, 0.5 + plan),
        View::Right => (0.5 + u, side),
        View::Left => (0.5 + u, 0.5 + side),
    }
}

/// Projects the paint onto every triangle of a mesh from `first` on, and
/// makes those vertices white so the texture shows its own colours.
fn wrap_skin(m: &mut Mesh, first: usize) {
    let tris = (m.count() - first) / 3;
    for t in 0..tris {
        let base = (first + 3 * t) * STRIDE;
        let p: [V3; 3] = std::array::from_fn(|k| {
            let c = &m.v[base + k * STRIDE..];
            v3(c[0] as f64, c[1] as f64, c[2] as f64)
        });
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        let view = if n.y.abs() >= n.x.abs() * 0.8 {
            if n.y >= 0.0 { View::Top } else { View::Bottom }
        } else if n.x >= 0.0 {
            View::Right
        } else {
            View::Left
        };
        for (k, q) in p.iter().enumerate() {
            let (u, v) = view_uv(view, *q);
            let c = &mut m.v[base + k * STRIDE..base + (k + 1) * STRIDE];
            c[6..9].copy_from_slice(&[1.0, 1.0, 1.0]);
            c[9] = u as f32;
            c[10] = v as f32;
        }
    }
}

/// Smooth normals vanish where an airfoil's cosine spacing has no slope (the
/// leading and trailing edges); those vertices take their triangle's normal,
/// turned to agree with its neighbours.
fn mend_normals(m: &mut Mesh) {
    for t in 0..m.count() / 3 {
        let base = 3 * t * STRIDE;
        let get = |m: &Mesh, k: usize, o: usize| {
            let c = &m.v[base + k * STRIDE + o..];
            v3(c[0] as f64, c[1] as f64, c[2] as f64)
        };
        let normals: [V3; 3] = std::array::from_fn(|k| get(m, k, 3));
        if normals.iter().all(|n| n.len() > 0.5) {
            continue;
        }
        let p: [V3; 3] = std::array::from_fn(|k| get(m, k, 0));
        let mut face = (p[1] - p[0]).cross(p[2] - p[0]).norm();
        let hint = normals.iter().fold(V3::ZERO, |s, &n| s + n);
        if face.len() < 0.5 {
            // A sliver with no area of its own (a pole of a lathe or a loft).
            face = if hint.len() > 1e-6 { hint.norm() } else { V3::Y };
        } else if face.dot(hint) < 0.0 {
            face = -face;
        }
        for (k, n) in normals.iter().enumerate() {
            if n.len() < 0.5 {
                let o = base + k * STRIDE + 3;
                m.v[o..o + 3].copy_from_slice(&face.f32());
            }
        }
    }
}

const GREY_TOP: [f32; 3] = [126.0, 132.0, 138.0]; // FS 36270 medium grey
const GREY_DARK: [f32; 3] = [96.0, 101.0, 108.0]; // FS 36118 gunship grey
const GREY_LOW: [f32; 3] = [160.0, 166.0, 171.0]; // FS 36375 light ghost grey
const INSIGNIA: [f32; 3] = [84.0, 88.0, 94.0];
const RADOME: [f32; 3] = [112.0, 116.0, 120.0];

/// An aggressor camouflage: the three colours of a hard-edged splinter
/// pattern, the fin serial and the colour of the stencils.
pub struct Scheme {
    colours: [[f32; 3]; 3],
    serial: &'static str,
    ink: [f32; 3],
}

/// The paint schemes the 64th Aggressor Squadron's F-16s wear at Nellis,
/// each copying a threat jet's: "Flanker", the Su-27's three blues;
/// "Desert", tan, brown and drab; "Ghost", two blue-greys over light grey;
/// and "Arctic", white and greys. Bandits take them in turn.
pub const SCHEMES: [Scheme; 4] = [
    // Flanker
    Scheme { colours: [[176.0, 196.0, 214.0], [118.0, 146.0, 180.0], [64.0, 88.0, 124.0]], serial: "AF 86-0280", ink: [38.0, 44.0, 58.0] },
    // Desert
    Scheme { colours: [[198.0, 172.0, 126.0], [150.0, 110.0, 70.0], [104.0, 106.0, 74.0]], serial: "AF 86-0299", ink: [52.0, 44.0, 34.0] },
    // Ghost
    Scheme { colours: [[168.0, 176.0, 184.0], [118.0, 128.0, 140.0], [84.0, 94.0, 108.0]], serial: "AF 86-0320", ink: [40.0, 44.0, 50.0] },
    // Arctic
    Scheme { colours: [[226.0, 228.0, 230.0], [162.0, 168.0, 174.0], [74.0, 78.0, 84.0]], serial: "AF 86-0291", ink: [40.0, 42.0, 46.0] },
];

impl Scheme {
    /// Colour at a point of one painted view (0 top, 1 bottom, 2 and 3 the
    /// sides). Three families of straight lines, each at its own angle and
    /// spacing, cut the skin into angular patches, and each patch takes a
    /// colour from a hash of the three strips it lies in, so neighbours that
    /// draw the same colour merge into the long splinters of the real paint.
    fn at(&self, a: f64, b: f64, view: u32) -> [f32; 3] {
        const FAMILIES: [(f64, f64, f64); 3] = [(0.45, 2.9, 0.3), (1.75, 3.5, 0.7), (2.7, 4.4, 0.15)];
        let mut h = 0x9e37_79b9u32.wrapping_mul(view + 1) ^ self.serial.len() as u32;
        for (angle, spacing, offset) in FAMILIES {
            let d = a * f64::cos(angle) + b * f64::sin(angle);
            let k = (d / spacing + offset).floor() as i32 as u32;
            h = (h ^ k).wrapping_mul(0x85eb_ca6b);
            h ^= h >> 13;
        }
        self.colours[(h % 3) as usize]
    }
}

/// An aggressor's skin texture, painted like `Airframe::skin`.
pub fn aggressor_skin(scheme: &Scheme) -> Vec<u8> {
    paint_skin(Some(scheme))
}

fn in_star(dx: f64, dy: f64, r: f64) -> bool {
    // Five-pointed star, point up: even-odd test against its ten corners.
    let pts: Vec<(f64, f64)> = (0..10)
        .map(|k| {
            let a = PI / 2.0 + k as f64 * PI / 5.0;
            let rr = if k % 2 == 0 { r } else { r * 0.382 };
            (a.cos() * rr, a.sin() * rr)
        })
        .collect();
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if (yi > dy) != (yj > dy) && dx < (xj - xi) * (dy - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Low-visibility star and bar: a dark roundel with the star left in the
/// background grey and a bar each side. `a` runs along the view's horizontal.
fn insignia(a: f64, b: f64, ca: f64, cb: f64, r: f64) -> Option<bool> {
    let (da, db) = (a - ca, b - cb);
    if da * da + db * db <= r * r {
        return Some(!in_star(da, db, r * 0.95));
    }
    if da.abs() < r * 2.0 && db.abs() < r * 0.33 && db > -r * 0.33 {
        return Some(da.abs() > r * 1.02);
    }
    None
}

/// 5x7 pixel-font text painted in body units; `dir` is +1 or -1 along `a`.
fn text_pixel(text: &str, a: f64, b: f64, a0: f64, b0: f64, h: f64, dir: f64) -> bool {
    let px = h / 7.0;
    let along = (a - a0) * dir / px;
    let row = ((b0 - b) / px).floor();
    if along < 0.0 || !(0.0..7.0).contains(&row) {
        return false;
    }
    let col = along.floor() as usize;
    let (ch, gx) = (col / 6, col % 6);
    if gx >= 5 {
        return false;
    }
    let Some(c) = text.chars().nth(ch) else { return false };
    let cell = font::cell_for(c);
    let atlas = glyph_atlas();
    let (cx, cy) = ((cell % font::COLS) * font::CELL, (cell / font::COLS) * font::CELL);
    atlas[((cy + row as usize) * font::ATLAS_W + cx + gx) * 4 + 3] > 127
}

fn glyph_atlas() -> &'static [u8] {
    static ATLAS: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    ATLAS.get_or_init(font::atlas)
}

/// Colour of the paint at a body point seen in one view, in the Block 50's
/// two greys or, for an aggressor, in its camouflage.
fn paint(view: View, z: f64, a: f64, camo: Option<&Scheme>) -> [f32; 3] {
    let s = section(z);
    match view {
        View::Top | View::Bottom => {
            let x = a;
            let top = view == View::Top;
            let mut c = match camo {
                Some(k) => k.at(x, z, if top { 0 } else { 1 }),
                None if top => GREY_TOP,
                None => GREY_LOW,
            };
            if top {
                // Anti-glare panel ahead of the windscreen.
                if z > -6.2 && z < CANOPY_FRONT + 0.1 && x.abs() < s.w * 0.55 {
                    c = camo.map_or(GREY_DARK, |k| k.colours[2].map(|v| v * 0.8));
                }
                // Star points forward, bars run spanwise: top of the left wing.
                if insignia(x, -z, -3.05, -2.35, 0.42).is_some_and(|d| d) {
                    c = INSIGNIA;
                }
            } else if insignia(x, -z, 3.05, -2.35, 0.42).is_some_and(|d| d) {
                c = INSIGNIA;
            }
            if z < -5.95 {
                c = RADOME;
            }
            // Panel lines: fuselage frames, flaperons, leading-edge flaps.
            let on_body = x.abs() < s.w + 0.05;
            let frame = [-5.95, -4.85, -2.0, -0.35, 1.45, 3.25, 4.95, 5.9].iter().any(|&f| (z - f).abs() < 0.035);
            let wing = x.abs() > 1.1 && x.abs() < WING_TIP;
            let flaperon = wing && x.abs() < 3.5 && ((z - (WING_TE - 0.55)).abs() < 0.035 || (x.abs() - 3.5).abs() < 0.04 && z > WING_TE - 0.55);
            let lef = wing && (z - (wing_le40(x.abs()) + 0.38)).abs() < 0.04;
            let spine_line = top && (x.abs() - 0.32).abs() < 0.04 && z > -1.3 && z < 4.0;
            if (on_body && frame) || flaperon || lef || spine_line {
                c = c.map(|v| v * 0.72);
            }
            c
        }
        View::Right | View::Left => {
            let y = a;
            let dir = if view == View::Right { -1.0 } else { 1.0 };
            // Medium grey above the chine, light grey below, blended over a band.
            let split = s.yc + 0.05;
            let mut c = match camo {
                Some(k) => k.at(z, y, 2 + (view == View::Left) as u32),
                None if y > split + 0.06 => GREY_TOP,
                None if y < split - 0.06 => GREY_LOW,
                None => [143.0, 149.0, 154.0],
            };
            if z < -5.95 {
                c = RADOME;
            }
            if y > FIN_ROOT_Y + 0.1 && z > 3.6 {
                // Fin: the Aviano jet's green tip band, AV tail code and
                // serial; an aggressor's camouflage runs up the fin to Nellis's
                // WA code and its own serial.
                let (code, serial, ink) = match camo {
                    Some(k) => ("WA", k.serial, k.ink),
                    None => {
                        c = GREY_TOP;
                        if y > 3.20 {
                            c = [60.0, 112.0, 72.0];
                        }
                        if y > 3.52 {
                            c = [40.0, 42.0, 46.0];
                        }
                        ("AV", "AF 89-2001", INSIGNIA)
                    }
                };
                let code_start = if dir > 0.0 { 5.40 } else { 6.19 };
                if text_pixel(code, z, y, code_start, 2.70, 0.50, dir) {
                    c = ink;
                }
                let serial_start = if dir > 0.0 { 4.75 } else { 6.10 };
                if text_pixel(serial, z, y, serial_start, 1.88, 0.16, dir) {
                    c = ink;
                }
            }
            if let Some(dark) = insignia(z * dir, y, -0.55 * dir, -0.08, 0.27) {
                if dark {
                    c = INSIGNIA;
                }
            }
            // Ejection warning triangle below the canopy rail.
            let (tz, ty) = (z - (-3.95), y - 0.80);
            if ty > -0.07 && ty < 0.07 && tz.abs() < (0.07 - ty) * 0.6 {
                c = [200.0, 40.0, 30.0];
            }
            let frame = [-5.95, -4.85, -2.0, -0.35, 1.45, 3.25, 4.95, 5.9].iter().any(|&f| (z - f).abs() < 0.035);
            let rudder = z > 3.6 && y > FIN_ROOT_Y && (z - fin_point((y - FIN_ROOT_Y) / (FIN_TIP_Y - FIN_ROOT_Y), RUDDER_HINGE, 0.0).z).abs() < 0.04;
            let chine = (y - (s.yc - 0.2)).abs() < 0.02 && z > -5.0 && z < 3.0;
            if frame || rudder || chine {
                c = c.map(|v| v * 0.72);
            }
            c
        }
    }
}

/// Paints the four views into the skin texture, with a little grain.
fn paint_skin(camo: Option<&Scheme>) -> Vec<u8> {
    let mut out = vec![0u8; SKIN_W * SKIN_H * 4];
    for py in 0..SKIN_H {
        for px in 0..SKIN_W {
            let (u, v) = ((px as f64 + 0.5) / SKIN_W as f64, (py as f64 + 0.5) / SKIN_H as f64);
            let side = u >= 0.5;
            let lower = v >= 0.5;
            let view = match (side, lower) {
                (false, false) => View::Top,
                (false, true) => View::Bottom,
                (true, false) => View::Right,
                (true, true) => View::Left,
            };
            let z = PAINT_Z.0 + (u * 2.0).fract() * (PAINT_Z.1 - PAINT_Z.0);
            let f = (v * 2.0).fract();
            let a = if side { SIDE_Y.0 - f * (SIDE_Y.0 - SIDE_Y.1) } else { f * 2.0 * PLAN_X - PLAN_X };
            let c = paint(view, z, a, camo);
            let grain = ((px * 7 + py * 13) % 5) as f32 - 2.0;
            let o = (py * SKIN_W + px) * 4;
            for k in 0..3 {
                out[o + k] = (c[k] + grain).clamp(0.0, 255.0) as u8;
            }
            out[o + 3] = 255;
        }
    }
    out
}

pub fn build() -> Airframe {
    let mut ext = Mesh::new();
    let mut canopy = Mesh::new();
    let mut crew = Mesh::new();
    let mut flame = Mesh::new();

    // Painted skin: fuselage, intake, wings, fin, strakes and ventral fins.
    let axis_out = |p: V3| v3(0.0, section(p.z).yc + if p.y > 1.0 { 0.4 } else { 0.0 }, p.z);
    ext.surface(FUSE_NZ, FUSE_NTH, |u, v| fuselage_point(mix(FUSE_Z.0, FUSE_Z.1, u), v * TAU), |_, _| WHITE, Some(&axis_out));
    let intake_point = |u: f64, v: f64| {
        let z = mix(INTAKE_Z.0, INTAKE_Z.1, u);
        let s = catmull(&INTAKE, z);
        let (x, y) = superellipse(&s, v * TAU);
        v3(x, y, z)
    };
    let intake_out = |p: V3| v3(0.0, catmull(&INTAKE, p.z).yc, p.z);
    ext.surface(10, FUSE_NTH, intake_point, |_, _| WHITE, Some(&intake_out));
    for &side in &[-1.0, 1.0] {
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            ext.surface(WING_NU, WING_NV, move |u, v| wing_point(side, u, v, upper), |_, _| WHITE, Some(&out));
        }
        for k in 0..WING_NV {
            let (v0, v1) = (k as f64 / WING_NV as f64, (k + 1) as f64 / WING_NV as f64);
            ext.quad(wing_point(side, 1.0, v0, true), wing_point(side, 1.0, v1, true), wing_point(side, 1.0, v1, false), wing_point(side, 1.0, v0, false), WHITE);
        }
        // Aft strakes beside the engine, carrying the stabilators and airbrakes.
        let boom_point = move |u: f64, v: f64, upper: bool| {
            let x = 0.62 + 0.48 * u;
            let z = mix(WING_TE - 0.2, BRAKE_HINGE_Z, v);
            let half = 0.07 * (1.0 - u.powi(4)).max(0.0).sqrt() * smoothstep(0.0, 0.25, v);
            v3(side * x, BOOM_Y + if upper { half } else { -half }, z)
        };
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            ext.surface(2, 4, move |u, v| boom_point(u, v, upper), |_, _| WHITE, Some(&out));
        }
        for k in 0..2 {
            let (u0, u1) = (k as f64 / 2.0, (k + 1) as f64 / 2.0);
            ext.quad(boom_point(u0, 1.0, true), boom_point(u1, 1.0, true), boom_point(u1, 1.0, false), boom_point(u0, 1.0, false), WHITE);
        }
        // Ventral fins, canted outward.
        let vf = [v3(side * 0.52, 0.04, 4.40), v3(side * 0.52, 0.04, 5.60), v3(side * 0.66, -0.46, 5.50), v3(side * 0.64, -0.44, 5.05)];
        thin_plate(&mut ext, &vf, v3(side, 0.25, 0.0).norm() * 0.016, WHITE);
    }
    for &side in &[-1.0, 1.0] {
        let out = move |p: V3| v3(p.x - side, p.y, p.z);
        ext.surface(4, 4, move |u, v| fin_point(u * RUDDER_TOP, v * RUDDER_HINGE, side), |_, _| WHITE, Some(&out));
        ext.surface(1, 5, move |u, v| fin_point(mix(RUDDER_TOP, 1.0, u), v, side), |_, _| WHITE, Some(&out));
    }
    for k in 0..5 {
        let (v0, v1) = (k as f64 / 5.0, (k + 1) as f64 / 5.0);
        ext.quad(fin_point(1.0, v0, -1.0), fin_point(1.0, v1, -1.0), fin_point(1.0, v1, 1.0), fin_point(1.0, v0, 1.0), WHITE);
    }
    // Drag-chute housing at the fin root.
    ext.lathe(v3(0.0, 1.13, 5.65), V3::Z, &[(0.0, 0.10), (0.4, 0.14), (0.9, 0.14), (1.04, 0.09), (1.06, 0.0)], 8, WHITE);
    wrap_skin(&mut ext, 0);

    // Unpainted detail: pitot, intake lip and duct, diverter, nozzle, missiles.
    ext.lathe(v3(0.0, 0.31, -7.07), v3(0.0, 0.0, -1.0), &[(0.0, 0.03), (0.30, 0.016), (0.86, 0.008), (0.90, 0.0)], 6, Rgb(0.62, 0.63, 0.62));
    let mouth = catmull(&INTAKE, INTAKE_Z.0);
    let ring = |scale: f64, z: f64, th: f64| {
        let (x, y) = superellipse(&mouth, th);
        v3(x * scale, mouth.yc + (y - mouth.yc) * scale, z)
    };
    let mz = INTAKE_Z.0;
    for k in 0..FUSE_NTH {
        let (t0, t1) = (k as f64 * TAU / FUSE_NTH as f64, (k + 1) as f64 * TAU / FUSE_NTH as f64);
        ext.quad(ring(1.0, mz, t0), ring(1.0, mz, t1), ring(0.88, mz + 0.12, t1), ring(0.88, mz + 0.12, t0), Rgb(0.55, 0.57, 0.59));
        ext.quad(ring(0.88, mz + 0.12, t0), ring(0.88, mz + 0.12, t1), ring(0.78, mz + 1.0, t1), ring(0.78, mz + 1.0, t0), DUCT);
        ext.tri(v3(0.0, mouth.yc, mz + 1.02), ring(0.78, mz + 1.0, t1), ring(0.78, mz + 1.0, t0), Rgb(0.015, 0.02, 0.022));
    }
    let div = crate::mesh::Frame { o: v3(0.0, 0.10, mz + 0.30), x: V3::X, y: V3::Y, z: V3::Z };
    ext.block(&div, v3(0.02, 0.05, 0.34), Rgb(0.30, 0.31, 0.33));

    let nz = 8;
    for j in 0..nz {
        let (a, b) = (j as f64 * TAU / nz as f64, (j + 1) as f64 * TAU / nz as f64);
        let p = |z: f64, r: f64, t: f64| v3(t.cos() * r, NOZZLE_Y + t.sin() * r, z);
        let metal = if j % 2 == 1 { METAL } else { METAL.scale(0.82) };
        ext.quad(p(6.10, 0.51, a), p(6.10, 0.51, b), p(6.73, 0.42, b), p(6.73, 0.42, a), metal);
        ext.quad(p(6.73, 0.42, a), p(6.73, 0.42, b), p(6.66, 0.36, b), p(6.66, 0.36, a), DARK_METAL);
        ext.quad(p(6.66, 0.36, a), p(6.66, 0.36, b), p(6.30, 0.34, b), p(6.30, 0.34, a), Rgb(0.07, 0.06, 0.055));
        ext.tri(v3(0.0, NOZZLE_Y, 6.28), p(6.30, 0.34, a), p(6.30, 0.34, b), Rgb(0.04, 0.03, 0.02));
        let f = |z: f64, r: f64, t: f64| v3(t.cos() * r, NOZZLE_Y + t.sin() * r, z);
        flame.quad(f(6.73, 0.35, a), f(6.73, 0.35, b), f(8.7, 0.19, b), f(8.7, 0.19, a), Rgb(1.0, 0.55, 0.18));
        flame.quad(f(8.7, 0.19, a), f(8.7, 0.19, b), f(10.0, 0.02, b), f(10.0, 0.02, a), Rgb(0.9, 0.30, 0.08));
        flame.quad(f(6.70, 0.21, a), f(6.70, 0.21, b), f(7.8, 0.04, b), f(7.8, 0.04, a), Rgb(1.0, 0.85, 0.55));
    }

    let mut tips = [Mesh::new(), Mesh::new()];
    for (i, &side) in [-1.0, 1.0].iter().enumerate() {
        // Wingtip launcher; its AIM-9 is a separate part, gone once fired.
        let rail = crate::mesh::Frame { o: v3(side * 4.69, WING_Y, 2.55), x: V3::X, y: V3::Y, z: V3::Z };
        ext.block(&rail, v3(0.035, 0.045, 0.78), Rgb(0.46, 0.48, 0.50));
        aim9(&mut tips[i], v3(side * AIM9_TIP.x, AIM9_TIP.y, AIM9_TIP.z));
        // Two underwing pylons.
        for &(x, z0, z1) in &[(2.30, 0.70, 3.00), (3.50, 1.60, 3.10)] {
            let f = crate::mesh::Frame { o: v3(side * x, WING_Y - 0.13, (z0 + z1) * 0.5), x: V3::X, y: V3::Y, z: V3::Z };
            ext.block(&f, v3(0.05, 0.12, (z1 - z0) * 0.5), Rgb(0.60, 0.62, 0.64));
        }
    }

    // Canopy glass, sill rails, aft bow and bulkhead.
    canopy.surface(CANOPY_NU, CANOPY_NV, canopy_point, |_, _| Rgb(0.42, 0.36, 0.18), Some(&|p: V3| v3(0.0, 0.6, p.z)));
    let frame_col = Rgb(0.20, 0.22, 0.24);
    for &v in &[0.0, 1.0] {
        let pts: Vec<V3> = (1..=CANOPY_NU).map(|k| canopy_point(k as f64 / CANOPY_NU as f64, v)).collect();
        for k in 0..pts.len() - 1 {
            ext.capsule(pts[k], pts[k + 1], 0.022, 0.022, frame_col, 4);
        }
    }
    for k in 0..CANOPY_NV {
        let (a, b) = (canopy_point(0.985, k as f64 / CANOPY_NV as f64), canopy_point(0.985, (k + 1) as f64 / CANOPY_NV as f64));
        ext.capsule(a, b, 0.03, 0.03, frame_col, 4);
    }
    let bulk: Vec<V3> = (0..=CANOPY_NV).map(|k| canopy_point(0.995, k as f64 / CANOPY_NV as f64)).collect();
    ext.polygon(&bulk, Rgb(0.10, 0.11, 0.12));

    // Pilot for the chase view: helmet, visor, shoulders and ACES II headrest.
    crew.ellipsoid(v3(0.0, 1.28, -3.14), v3(0.135, 0.145, 0.155), Rgb(0.72, 0.73, 0.70), 5, 8);
    crew.ellipsoid(v3(0.0, 1.27, -3.235), v3(0.112, 0.06, 0.06), Rgb(0.05, 0.06, 0.07), 3, 6);
    crew.ellipsoid(v3(0.0, 0.98, -3.18), v3(0.25, 0.24, 0.17), Rgb(0.30, 0.34, 0.22), 4, 8);
    let (c30, s30) = ((30.0 * DEG).cos(), (30.0 * DEG).sin());
    let rest = crate::mesh::Frame { o: v3(0.0, 1.18, -2.90), x: V3::X, y: v3(0.0, c30, s30), z: v3(0.0, -s30, c30) };
    crew.block(&rest, v3(0.17, 0.22, 0.07), Rgb(0.18, 0.2, 0.19));

    // Moving surfaces, painted like the rest of the skin.
    let mut stab = [Mesh::new(), Mesh::new()];
    for (i, &side) in [-1.0, 1.0].iter().enumerate() {
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            stab[i].surface(4, 5, move |u, v| stab_point(side, u, v, upper), |_, _| WHITE, Some(&out));
        }
        for k in 0..5 {
            let (v0, v1) = (k as f64 / 5.0, (k + 1) as f64 / 5.0);
            stab[i].quad(stab_point(side, 1.0, v0, true), stab_point(side, 1.0, v1, true), stab_point(side, 1.0, v1, false), stab_point(side, 1.0, v0, false), WHITE);
        }
        wrap_skin(&mut stab[i], 0);
    }
    let mut rudder = Mesh::new();
    for &side in &[-1.0, 1.0] {
        let out = move |p: V3| v3(p.x - side, p.y, p.z);
        rudder.surface(4, 2, move |u, v| fin_point(u * RUDDER_TOP, mix(RUDDER_HINGE, 1.0, v), side), |_, _| WHITE, Some(&out));
    }
    wrap_skin(&mut rudder, 0);
    let mut brakes = [Mesh::new(), Mesh::new(), Mesh::new(), Mesh::new()];
    for (i, &side) in [-1.0, 1.0].iter().enumerate() {
        for (j, &dy) in [0.034, -0.034].iter().enumerate() {
            let f = crate::mesh::Frame { o: v3(side * 0.86, BOOM_Y + dy, (BRAKE_HINGE_Z + 6.75) * 0.5), x: V3::X, y: V3::Y, z: V3::Z };
            let m = &mut brakes[i * 2 + j];
            m.block(&f, v3(0.22, 0.032, 0.40), WHITE);
            wrap_skin(m, 0);
        }
    }

    for m in [&mut ext, &mut canopy, &mut crew, &mut flame, &mut rudder].into_iter().chain(stab.iter_mut()).chain(brakes.iter_mut()).chain(tips.iter_mut()) {
        mend_normals(m);
    }
    let (gear, gear_doors) = landing_gear();
    Airframe { exterior: ext, canopy, crew, flame, stab, rudder, brakes, tips, gear, gear_doors, skin: paint_skin(None) }
}

/// Retraction pivots of the nose and main gear legs, the axis each swings
/// about and the angle it swings through (negative: the wheel goes aft).
/// The nose leg hangs from the forebody just ahead of the intake and folds
/// aft; the mains hang from the sides of the intake trunk and fold forward
/// and inward, tucking the wheels in under the body.
pub const GEAR_PIVOTS: [(V3, V3, f64); 3] = [
    (v3(0.0, 0.08, -3.38), V3::X, -105.0 * DEG),
    (v3(-0.72, 0.0, 0.55), v3(0.912, 0.0, 0.410), 95.0 * DEG),
    (v3(0.72, 0.0, 0.55), v3(0.912, 0.0, -0.410), 95.0 * DEG),
];
/// Nose-well door: hinged on the right edge of the well, flush with the
/// skin when shut, and swung down beside the leg when open.
pub const GEAR_DOORS: [(V3, V3, f64); 1] = [(v3(0.17, 0.095, -3.20), V3::Z, 88.0 * DEG)];

/// Landing gear legs fully down: nose leg with its single wheel in a fork,
/// main legs with the larger wheels outboard of their struts and the strut
/// doors along them. The tyres meet the ground at the sim's wheel points
/// (sim::WHEELS), 1.55 m under the datum. Returns the legs and the doors.
fn landing_gear() -> ([Mesh; 3], [Mesh; 1]) {
    let strut = Rgb(0.78, 0.78, 0.76);
    let tyre = Rgb(0.07, 0.07, 0.075);
    let hub = Rgb(0.55, 0.56, 0.55);
    let door = Rgb(0.62, 0.64, 0.66);
    let wheel = |m: &mut Mesh, c: V3, r: f64, w: f64| {
        m.lathe(c - V3::X * (w * 0.5), V3::X, &[(0.0, 0.0), (0.0, r * 0.82), (0.02, r), (w - 0.02, r), (w, r * 0.82), (w, 0.0)], 10, tyre);
        m.lathe(c - V3::X * (w * 0.5 + 0.005), V3::X, &[(0.0, 0.0), (0.0, r * 0.5), (w + 0.01, r * 0.5), (w + 0.01, 0.0)], 8, hub);
    };
    let block = |m: &mut Mesh, o: V3, half: V3, col: Rgb| m.block(&Frame { o, x: V3::X, y: V3::Y, z: V3::Z }, half, col);

    let mut nose = Mesh::new();
    let (p, _, _) = GEAR_PIVOTS[0];
    let axle = v3(0.0, -1.33, -3.45);
    // Oleo from the forebody down to the fork crown just above the tyre.
    let crown = axle + V3::Y * 0.27;
    let l = (crown - p).len();
    nose.lathe(p, (crown - p).norm(), &[(0.0, 0.0), (0.0, 0.065), (0.62 * l, 0.065), (0.63 * l, 0.05), (l, 0.05), (l + 0.01, 0.0)], 6, strut);
    block(&mut nose, crown, v3(0.13, 0.025, 0.05), strut.scale(0.85));
    for s in [-1.0, 1.0] {
        block(&mut nose, axle + v3(s * 0.11, 0.13, 0.0), v3(0.012, 0.14, 0.04), strut.scale(0.85));
    }
    // Torque link behind the oleo.
    block(&mut nose, p.lerp(crown, 0.75) + V3::Z * 0.08, v3(0.02, 0.10, 0.03), strut.scale(0.8));
    wheel(&mut nose, axle, 0.22, 0.16);
    let mut nose_door = Mesh::new();
    let (h, _, _) = GEAR_DOORS[0];
    block(&mut nose_door, h - V3::X * 0.17, v3(0.17, 0.008, 0.26), door);

    let main = |side: f64| {
        let mut m = Mesh::new();
        let (p, _, _) = GEAR_PIVOTS[if side < 0.0 { 1 } else { 2 }];
        let foot = v3(side * 1.06, -1.20, 0.55);
        let dir = (foot - p).norm();
        let l = (foot - p).len();
        m.lathe(p, dir, &[(0.0, 0.0), (0.0, 0.075), (0.55 * l, 0.075), (0.56 * l, 0.06), (l, 0.06), (l + 0.01, 0.0)], 6, strut);
        // Axle stub out to the wheel.
        m.lathe(foot, V3::X * side, &[(0.0, 0.0), (0.0, 0.045), (0.14, 0.045), (0.14, 0.0)], 5, strut);
        // Drag brace from the leg forward into the well.
        let (brace, mid) = (v3(side * 0.70, -0.05, -0.15), p.lerp(foot, 0.55));
        let bl = (mid - brace).len();
        m.lathe(brace, (mid - brace).norm(), &[(0.0, 0.0), (0.0, 0.03), (bl, 0.03), (bl, 0.0)], 5, strut.scale(0.85));
        wheel(&mut m, v3(side * 1.18, -1.20, 0.55), 0.35, 0.21);
        // Strut door on the outboard face of the leg, closing the well.
        let out = v3(-dir.y * side, dir.x * side, 0.0).norm();
        m.block(&Frame { o: p + dir * 0.40 + out * 0.09, x: out, y: dir, z: V3::Z }, v3(0.008, 0.38, 0.34), door);
        m
    };
    ([nose, main(-1.0), main(1.0)], [nose_door])
}

/// Nose of the right wingtip AIM-9 (the left one mirrors it).
pub const AIM9_TIP: V3 = v3(4.80, WING_Y - 0.03, 0.55);

/// AIM-9 Sidewinder, 2.87 m long and 127 mm across, nose at `nose`, pointing
/// forward: seeker dome, canards near the nose and rollerons on the tail fins.
pub fn aim9(m: &mut Mesh, nose: V3) {
    let white = Rgb(0.80, 0.80, 0.76);
    m.lathe(nose, V3::Z, &[(0.0, 0.0), (0.08, 0.04), (0.20, 0.064), (2.80, 0.064), (2.86, 0.05), (2.87, 0.0)], 6, white);
    m.lathe(nose, V3::Z, &[(-0.001, 0.0), (0.07, 0.036), (0.08, 0.0)], 6, Rgb(0.08, 0.09, 0.08));
    for f in 0..4 {
        let a = PI / 4.0 + f as f64 * PI / 2.0;
        let d = v3(a.cos(), a.sin(), 0.0);
        let c0 = nose + d * 0.064;
        m.quad(c0 + V3::Z * 0.36, c0 + V3::Z * 0.62, c0 + d * 0.14 + V3::Z * 0.58, c0 + d * 0.14 + V3::Z * 0.48, white.scale(0.9));
        m.quad(c0 + V3::Z * 2.45, c0 + V3::Z * 2.84, c0 + d * 0.18 + V3::Z * 2.84, c0 + d * 0.18 + V3::Z * 2.62, white.scale(0.9));
    }
}

fn thin_plate(m: &mut Mesh, pts: &[V3], half: V3, col: Rgb) {
    let a: Vec<V3> = pts.iter().map(|&p| p - half).collect();
    let b: Vec<V3> = pts.iter().map(|&p| p + half).collect();
    for i in 1..pts.len() - 1 {
        m.tri(a[0], a[i], a[i + 1], col);
        m.tri(b[0], b[i + 1], b[i], col);
    }
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        m.quad(a[i], b[i], b[j], a[j], col.scale(0.9));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extent(m: &Mesh) -> (V3, V3) {
        let mut lo = v3(1e9, 1e9, 1e9);
        let mut hi = v3(-1e9, -1e9, -1e9);
        for c in m.v.chunks(STRIDE) {
            lo = v3(lo.x.min(c[0] as f64), lo.y.min(c[1] as f64), lo.z.min(c[2] as f64));
            hi = v3(hi.x.max(c[0] as f64), hi.y.max(c[1] as f64), hi.z.max(c[2] as f64));
        }
        (lo, hi)
    }

    #[test]
    fn measured_dimensions_match_the_f16() {
        let a = build();
        let mut all = Mesh::new();
        for m in [&a.exterior, &a.stab[0], &a.stab[1], &a.rudder, &a.tips[0], &a.tips[1]] {
            all.v.extend_from_slice(&m.v);
        }
        let (lo, hi) = extent(&all);
        let length = hi.z - lo.z;
        let span = hi.x - lo.x;
        assert!((length - 15.06).abs() < 0.08, "length {length}");
        assert!((span - 9.96).abs() < 0.08, "span {span}");
        assert!((hi.y - GROUND_Y - 4.88).abs() < 0.08, "height {}", hi.y - GROUND_Y);
    }

    /// Proportions from the three-view: where the canopy, intake and wing sit
    /// along the body, and how deep the forebody is.
    #[test]
    fn proportions_follow_the_three_view() {
        let nose = FUSE_Z.0;
        let fuselage = NOZZLE_EXIT.z - nose;
        let at = |z: f64| (z - nose) / fuselage;
        assert!((at(CANOPY_FRONT) - 0.178).abs() < 0.02);
        assert!((at(CANOPY_AFT) - 0.411).abs() < 0.02);
        assert!((at(INTAKE_Z.0) - 0.29).abs() < 0.02);
        assert!((at(WING_TE) - 0.75).abs() < 0.03);
        // Canopy top to intake bottom: about 0.15 of the fuselage length.
        let s = catmull(&INTAKE, INTAKE_Z.0);
        let depth = CANOPY_TOP - (s.yc - s.hl);
        assert!((depth / fuselage - 0.147).abs() < 0.015, "depth ratio {}", depth / fuselage);
    }

    #[test]
    fn geometry_is_finite_with_unit_normals() {
        let a = build();
        for m in [&a.exterior, &a.canopy, &a.crew, &a.flame, &a.rudder, &a.tips[0], &a.tips[1]] {
            assert!(m.count() > 0);
            for c in m.v.chunks(STRIDE) {
                assert!(c.iter().all(|x| x.is_finite()));
                let l = (c[3] * c[3] + c[4] * c[4] + c[5] * c[5]).sqrt();
                assert!((l - 1.0).abs() < 1e-3, "normal length {l}");
            }
        }
        assert_eq!(a.skin.len(), SKIN_W * SKIN_H * 4);
    }

    #[test]
    fn painted_triangles_map_inside_their_view() {
        let a = build();
        for c in a.exterior.v.chunks(STRIDE).filter(|c| c[9] >= 0.0) {
            assert!((0.0..=1.0).contains(&c[9]) && (0.0..=1.0).contains(&c[10]), "uv {} {}", c[9], c[10]);
        }
    }


    /// The gear legs hang from the skin, not from thin air, and fold their
    /// wheels into the body: the mains in beside the intake trunk, the nose
    /// wheel up behind the forebody.
    #[test]
    fn gear_legs_hang_from_the_skin_and_fold_into_the_body() {
        let below = |p: V3| {
            let s = section(p.z);
            let i = catmull(&INTAKE, p.z);
            let fuse = s.yc - s.hl;
            let duct = if p.z > INTAKE_Z.0 && p.x.abs() < i.w { i.yc - i.hl } else { fuse };
            p.y - fuse.min(duct)
        };
        for (i, &(p, axis, swing)) in GEAR_PIVOTS.iter().enumerate() {
            assert!(p.x.abs() < section(p.z).w && below(p) > -0.05 && p.y < 0.2, "leg {i} pivot {p:?} off the skin");
            let wheel = [v3(0.0, -1.33, -3.45), v3(-1.18, -1.20, 0.55), v3(1.18, -1.20, 0.55)][i];
            let up = p + Quat::axis_angle(axis, swing).rotate(wheel - p);
            println!("leg {i}: wheel folds to {up:?}");
            assert!(up.x.abs() < 0.8 && up.y > -0.55 && up.y < 0.5, "leg {i} wheel folds to {up:?}");
        }
    }
    #[test]
    fn eye_is_inside_the_canopy() {
        let (w, base, top) = canopy_profile(EYE.z);
        assert!(EYE.y > base + 0.3 && EYE.y < top - 0.2 && w > 0.4);
    }
}
