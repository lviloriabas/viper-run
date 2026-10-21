//! Smooth, dimensioned F-16C exterior. Body axes: right (+x), up (+y), aft (+z), metres.
//! 15.06 m overall from pitot tip to stabilator tips, 9.96 m span over the
//! wingtip missiles, 5.09 m from the gear line to the fin cap. Lofted sections
//! and NACA thickness distributions are sampled coarsely and flat shaded, so
//! the jet reads as clean early-90s polygons rather than boxes or blobs.
use crate::math::*;
use crate::mesh::{Mesh, Rgb};

pub const DARK: Rgb = Rgb(0.37, 0.40, 0.43);
pub const MED: Rgb = Rgb(0.49, 0.52, 0.55);
pub const LIGHT: Rgb = Rgb(0.61, 0.63, 0.65);
const RADOME: Rgb = Rgb(0.42, 0.44, 0.45);
const SEAM: Rgb = Rgb(0.30, 0.33, 0.35);
const DUCT: Rgb = Rgb(0.05, 0.06, 0.065);
const METAL: Rgb = Rgb(0.33, 0.33, 0.32);

/// Pilot design eye. The seat is reclined 30 degrees, so the eye sits aft of the hips.
pub const EYE: V3 = v3(0.0, 1.25, -3.24);

// ------------------------------------------------------------------ fuselage
// z, half width, centre height, upper and lower half heights, superellipse exponents.
const FUSE: [[f64; 7]; 16] = [
    [-6.95, 0.015, 0.08, 0.015, 0.015, 2.0, 2.0],
    [-6.70, 0.17, 0.09, 0.17, 0.165, 2.0, 2.0],
    [-6.30, 0.31, 0.10, 0.31, 0.30, 2.0, 2.0],
    [-5.80, 0.43, 0.10, 0.43, 0.42, 2.05, 2.05],
    [-5.30, 0.47, 0.10, 0.50, 0.50, 2.1, 2.15],
    [-4.80, 0.54, 0.10, 0.56, 0.56, 2.3, 2.5],
    [-4.20, 0.59, 0.11, 0.63, 0.63, 2.5, 2.8],
    [-3.50, 0.64, 0.12, 0.68, 0.68, 2.6, 3.0],
    [-2.50, 0.72, 0.12, 0.72, 0.70, 2.7, 3.2],
    [-1.00, 0.80, 0.10, 0.70, 0.66, 2.8, 3.3],
    [0.50, 0.86, 0.08, 0.64, 0.62, 2.8, 3.3],
    [2.00, 0.82, 0.06, 0.60, 0.58, 2.7, 3.1],
    [3.50, 0.74, 0.05, 0.57, 0.53, 2.5, 2.7],
    [5.00, 0.64, 0.04, 0.54, 0.48, 2.2, 2.3],
    [6.20, 0.56, 0.03, 0.49, 0.47, 2.0, 2.0],
    [6.70, 0.51, 0.02, 0.48, 0.46, 2.0, 2.0],
];
// Facets of the fuselage loft: stations along the body and sides around it.
const FUSE_NZ: usize = 40;
const FUSE_NTH: usize = 16;
const FUSE_Z: (f64, f64) = (-6.95, 6.70);
const INTAKE_NTH: usize = 16;
// Ventral inlet duct, from the D-shaped mouth back into the lower fuselage.
// About as wide as the forebody above it, so it reads as the belly, not a pod.
const INTAKE: [[f64; 7]; 8] = [
    [-3.75, 0.54, -0.93, 0.31, 0.31, 4.0, 2.4],
    [-3.20, 0.56, -0.88, 0.36, 0.35, 3.5, 2.4],
    [-2.00, 0.61, -0.80, 0.40, 0.36, 3.0, 2.5],
    [-0.50, 0.65, -0.70, 0.42, 0.36, 3.0, 2.6],
    [1.00, 0.66, -0.58, 0.40, 0.34, 3.0, 2.6],
    [2.50, 0.62, -0.42, 0.36, 0.28, 3.0, 2.6],
    [3.80, 0.55, -0.30, 0.30, 0.20, 3.0, 2.6],
    [4.50, 0.50, -0.22, 0.25, 0.20, 3.0, 2.6],
];

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
    let h = lerp_table(&[(-2.30, 0.0), (-2.05, 0.24), (-1.5, 0.24), (-0.5, 0.20), (1.0, 0.15), (2.5, 0.12), (3.6, 0.12), (6.0, 0.10), (6.6, 0.06), (6.9, 0.0)], z);
    let w = lerp_table(&[(-2.3, 0.30), (-1.5, 0.30), (1.0, 0.26), (3.6, 0.21), (6.6, 0.17), (6.9, 0.15)], z);
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
pub const CANOPY_FRONT: f64 = -5.10;
pub const CANOPY_PEAK: f64 = -3.35;
pub const CANOPY_AFT: f64 = -1.95;
const CANOPY_TOP: f64 = 1.58;
/// Canopy facets, front to aft and from sill to sill.
const CANOPY_NU: usize = 14;
pub const CANOPY_NV: usize = 10;

/// Canopy half width, base height and top height at a station.
pub fn canopy_profile(z: f64) -> (f64, f64, f64) {
    let z = clamp(z, CANOPY_FRONT, CANOPY_AFT);
    let (w, top);
    if z <= CANOPY_PEAK {
        let t = (z - CANOPY_FRONT) / (CANOPY_PEAK - CANOPY_FRONT);
        let g = (1.0 - (1.0 - t).powi(2)).max(0.0);
        w = 0.50 * g.powf(0.62);
        top = mix(fuselage_top_at(z, 0.0), CANOPY_TOP, g.sqrt());
    } else {
        let t = (z - CANOPY_PEAK) / (CANOPY_AFT - CANOPY_PEAK);
        w = 0.50 - 0.18 * t.powf(1.6);
        top = CANOPY_TOP - 0.48 * t.powf(1.7);
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

/// 40-degree leading edge sweep, unswept trailing edge, 9.45 m wing span.
fn wing_le40(x: f64) -> f64 {
    -2.16 + 0.839 * x
}
const WING_TE: f64 = 2.90;
const WING_TIP: f64 = 4.72;
const WING_Y: f64 = 0.06;
const WING_NU: usize = 12;
const WING_NV: usize = 8;

/// Leading edge including the blended leading-edge root extension: a slender
/// strake from beside the windscreen that curves out to meet the 40-degree
/// edge about a third of the way to the tip.
const LERX_ROOT: (f64, f64) = (0.50, -4.95);
const LERX_JOIN: f64 = 1.65;
fn wing_le(x: f64) -> f64 {
    let le40 = wing_le40(x);
    if x >= LERX_JOIN {
        return le40;
    }
    let t = clamp(x - LERX_ROOT.0, 0.0, LERX_JOIN - LERX_ROOT.0) / (LERX_JOIN - LERX_ROOT.0);
    (LERX_ROOT.1 + (wing_le40(LERX_JOIN) - LERX_ROOT.1) * t.powf(1.0 / 1.6)).min(le40)
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
    let lift = 0.10 * clamp((wing_le40(x) - z) / (wing_le40(x) - LERX_ROOT.1), 0.0, 1.0).powf(1.5);
    v3(side * x, WING_Y + lift + if upper { half } else { -half }, z)
}

fn fin_point(u: f64, v: f64, side: f64) -> V3 {
    let y = 0.70 + (3.34 - 0.70) * u;
    let le = 3.55 + (6.29 - 3.55) * u;
    let te = 6.95 + (7.25 - 6.95) * u;
    let chord = te - le;
    let half = 5.0 * 0.05 * chord * naca(v) * (1.0 - 0.3 * u);
    v3(side * half, y, le + chord * v)
}

pub const STAB_PIVOT: V3 = v3(0.85, 0.05, 6.05);
const ANHEDRAL: f64 = 0.176; // tan(10 deg)

fn stab_point(side: f64, u: f64, v: f64, upper: bool) -> V3 {
    let x = 0.80 + (2.79 - 0.80) * u;
    let le = 4.95 + (6.66 - 4.95) * (x - 0.80) / 1.99;
    let te = 7.25 + 0.26 * u;
    let chord = te - le;
    let w = cosine_spacing(v);
    let half = 5.0 * 0.04 * chord * naca(w);
    let y = 0.05 - (x - 0.80) * ANHEDRAL;
    v3(side * x, y + if upper { half } else { -half }, le + chord * w)
}

pub fn stab_axis(side: f64) -> V3 {
    v3(side, -ANHEDRAL, 0.0).norm() * side
}

pub const RUDDER_HINGE: f64 = 0.72;
pub const RUDDER_TOP: f64 = 0.93;

pub fn rudder_axis() -> (V3, V3) {
    let a = fin_point(0.0, RUDDER_HINGE, 0.0);
    let b = fin_point(RUDDER_TOP, RUDDER_HINGE, 0.0);
    (a, (b - a).norm())
}

pub const BRAKE_HINGE_Z: f64 = 6.15;

pub struct Airframe {
    pub exterior: Mesh,
    pub canopy: Mesh,
    pub crew: Mesh,
    pub flame: Mesh,
    pub stab: [Mesh; 2],
    pub rudder: Mesh,
    /// Speedbrake petals: left upper, left lower, right upper, right lower.
    pub brakes: [Mesh; 4],
}

fn fuselage_color(p: V3, n: V3) -> Rgb {
    if p.z < -5.25 {
        return RADOME;
    }
    if n.y > 0.42 {
        DARK
    } else if n.y < -0.35 {
        LIGHT
    } else {
        MED
    }
}

pub fn build() -> Airframe {
    let mut ext = Mesh::new();
    let mut canopy = Mesh::new();
    let mut crew = Mesh::new();
    let mut flame = Mesh::new();

    // Fuselage loft.
    let axis_out = |p: V3| v3(0.0, section(p.z).yc + if p.y > 0.9 { 0.5 } else { 0.0 }, p.z);
    ext.surface(FUSE_NZ, FUSE_NTH, |u, v| fuselage_point(mix(FUSE_Z.0, FUSE_Z.1, u), v * TAU), fuselage_color, Some(&axis_out));

    // Panel lines on facet edges: frame rings and two longitudinal joints per side.
    let dz = (FUSE_Z.1 - FUSE_Z.0) / FUSE_NZ as f64;
    let station = |z: f64| ((z - FUSE_Z.0) / dz).round() as usize;
    let dth = TAU / FUSE_NTH as f64;
    for &z in &[-5.27, -4.62, -1.85, 0.95, 3.45, 5.70] {
        let z = FUSE_Z.0 + station(z) as f64 * dz;
        let pts: Vec<(V3, V3)> = (0..=FUSE_NTH)
            .map(|k| {
                let th = k as f64 * dth;
                let p = fuselage_point(z, th);
                let n = (fuselage_point(z, th + 0.01) - fuselage_point(z, th - 0.01)).cross(V3::Z).norm();
                (p, -n)
            })
            .collect();
        ext.ribbon(&pts, 0.006, SEAM);
    }
    for k in [1, FUSE_NTH / 2 - 1, FUSE_NTH / 2 + 1, FUSE_NTH - 1] {
        let th = k as f64 * dth;
        let pts: Vec<(V3, V3)> = (station(-1.8)..=station(3.8))
            .map(|i| {
                let z = FUSE_Z.0 + i as f64 * dz;
                let p = fuselage_point(z, th);
                let a = fuselage_point(z, th + 0.01) - fuselage_point(z, th - 0.01);
                let n = a.cross(V3::Z).norm();
                (p, -n)
            })
            .collect();
        ext.ribbon(&pts, 0.005, SEAM);
    }

    // Pitot boom ahead of the radome.
    ext.lathe(v3(0.0, 0.08, -6.93), v3(0.0, 0.0, -1.0), &[(0.0, 0.022), (0.25, 0.014), (0.58, 0.008), (0.60, 0.0)], 6, Rgb(0.6, 0.62, 0.62));

    // Ventral intake: duct fairing, rounded lip and dark interior.
    let intake_point = |u: f64, v: f64| {
        let z = mix(-3.75, 4.5, u);
        let s = catmull(&INTAKE, z);
        let (x, y) = superellipse(&s, v * TAU);
        v3(x, y, z)
    };
    let intake_out = |p: V3| v3(0.0, catmull(&INTAKE, p.z).yc, p.z);
    ext.surface(16, INTAKE_NTH, intake_point, |p, n| if p.z < -3.68 { LIGHT } else { fuselage_color(p, n) }, Some(&intake_out));
    let mouth = catmull(&INTAKE, -3.75);
    let ring = |scale: f64, z: f64, th: f64| {
        let (x, y) = superellipse(&mouth, th);
        v3(x * scale, mouth.yc + (y - mouth.yc) * scale, z)
    };
    for k in 0..INTAKE_NTH {
        let (t0, t1) = (k as f64 * TAU / INTAKE_NTH as f64, (k + 1) as f64 * TAU / INTAKE_NTH as f64);
        ext.quad(ring(1.0, -3.75, t0), ring(1.0, -3.75, t1), ring(0.90, -3.62, t1), ring(0.90, -3.62, t0), LIGHT.scale(0.85));
        ext.quad(ring(0.90, -3.62, t0), ring(0.90, -3.62, t1), ring(0.80, -2.70, t1), ring(0.80, -2.70, t0), DUCT);
        ext.tri(v3(0.0, mouth.yc, -2.68), ring(0.80, -2.70, t1), ring(0.80, -2.70, t0), Rgb(0.015, 0.02, 0.022));
    }
    // Boundary-layer diverter between the duct and the forebody.
    let div = crate::mesh::Frame { o: v3(0.0, -0.585, -3.40), x: V3::X, y: V3::Y, z: V3::Z };
    ext.block(&div, v3(0.018, 0.045, 0.36), DARK);

    // Wings with blended LERX, upper and lower skins and a tip cap.
    for &side in &[-1.0, 1.0] {
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            ext.surface(WING_NU, WING_NV, move |u, v| wing_point(side, u, v, upper), move |p, _| if upper { if p.x.abs() < 1.2 && p.z < -1.2 { DARK } else { MED } } else { LIGHT }, Some(&out));
        }
        for k in 0..WING_NV {
            let (v0, v1) = (k as f64 / WING_NV as f64, (k + 1) as f64 / WING_NV as f64);
            ext.quad(wing_point(side, 1.0, v0, true), wing_point(side, 1.0, v1, true), wing_point(side, 1.0, v1, false), wing_point(side, 1.0, v0, false), MED);
        }
        // Leading-edge flap and flaperon hinge lines; the flaperon follows facet edges.
        let on_wing = |u: f64, v: f64| (wing_point(side, u, v, true), V3::Y);
        let (gu, gv) = (|i: usize| i as f64 / WING_NU as f64, |j: usize| j as f64 / WING_NV as f64);
        let (flap_in, flap_out, hinge) = (2, WING_NU / 2 + 1, WING_NV - 2);
        let flap_hinge: Vec<(V3, V3)> = (flap_in..=flap_out).map(|i| on_wing(gu(i), gv(hinge))).collect();
        let flap_edge = |i: usize| -> Vec<(V3, V3)> { (hinge..=WING_NV).map(|j| on_wing(gu(i), gv(j))).collect() };
        let le_line: Vec<(V3, V3)> = (0..=24)
            .map(|k| {
                let u = mix((LERX_JOIN - 0.40) / (WING_TIP - 0.40), 1.0, k as f64 / 24.0);
                let x = 0.40 + (WING_TIP - 0.40) * u;
                let v = ((wing_le40(x) + 0.42 - wing_le(x)) / (WING_TE - wing_le(x))).clamp(0.0, 1.0);
                on_wing(u, v.sqrt())
            })
            .collect();
        ext.ribbon(&le_line, 0.006, SEAM);
        ext.ribbon(&flap_hinge, 0.006, SEAM);
        ext.ribbon(&flap_edge(flap_in), 0.005, SEAM);
        ext.ribbon(&flap_edge(flap_out), 0.005, SEAM);

        // Wingtip launcher with an AIM-9.
        let rail = crate::mesh::Frame { o: v3(side * 4.76, 0.04, 2.30), x: V3::X, y: V3::Y, z: V3::Z };
        ext.block(&rail, v3(0.035, 0.045, 0.70), MED.scale(0.9));
        let mx = side * 4.80;
        let my = -0.03;
        let white = Rgb(0.78, 0.78, 0.74);
        ext.lathe(v3(mx, my, 0.65), V3::Z, &[(0.0, 0.0), (0.06, 0.035), (0.16, 0.058), (0.24, 0.064), (2.80, 0.064), (2.86, 0.05), (2.87, 0.0)], 8, white);
        ext.lathe(v3(mx, my, 0.65), V3::Z, &[(-0.001, 0.0), (0.055, 0.033), (0.07, 0.0)], 8, Rgb(0.08, 0.09, 0.08));
        for f in 0..4 {
            let a = PI / 4.0 + f as f64 * PI / 2.0;
            let d = v3(a.cos(), a.sin(), 0.0);
            let c0 = v3(mx, my, 0.65) + d * 0.064;
            ext.quad(c0 + V3::Z * 0.36, c0 + V3::Z * 0.62, c0 + d * 0.14 + V3::Z * 0.58, c0 + d * 0.14 + V3::Z * 0.48, white.scale(0.9));
            ext.quad(c0 + V3::Z * 2.45, c0 + V3::Z * 2.84, c0 + d * 0.18 + V3::Z * 2.84, c0 + d * 0.18 + V3::Z * 2.62, white.scale(0.9));
        }

        // Two underwing pylons.
        for &(x, z0, z1) in &[(2.35, 0.25, 2.30), (3.55, 1.25, 2.55)] {
            let f = crate::mesh::Frame { o: v3(side * x, -0.10, (z0 + z1) * 0.5), x: V3::X, y: V3::Y, z: V3::Z };
            ext.block(&f, v3(0.05, 0.13, (z1 - z0) * 0.5), LIGHT.scale(0.95));
        }

        // Aft fuselage strakes carrying the stabilator pivots and speedbrakes.
        let boom_point = move |u: f64, v: f64, upper: bool| {
            let x = 0.30 + 0.65 * u;
            let z = mix(2.60, BRAKE_HINGE_Z, v);
            let half = 0.075 * (1.0 - u.powi(4)).max(0.0).sqrt() * smoothstep(0.0, 0.25, v);
            v3(side * x, 0.06 + if upper { half } else { -half }, z)
        };
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            ext.surface(3, 6, move |u, v| boom_point(u, v, upper), move |_, n| if n.y > 0.0 { MED } else { LIGHT }, Some(&out));
        }
        for k in 0..3 {
            let (u0, u1) = (k as f64 / 3.0, (k + 1) as f64 / 3.0);
            ext.quad(boom_point(u0, 1.0, true), boom_point(u1, 1.0, true), boom_point(u1, 1.0, false), boom_point(u0, 1.0, false), MED.scale(0.7));
        }

        // Ventral fins, canted outward.
        let vf = [v3(side * 0.60, -0.05, 4.85), v3(side * 0.60, -0.05, 6.35), v3(side * 0.74, -0.58, 6.20), v3(side * 0.72, -0.56, 5.75)];
        thin_plate(&mut ext, &vf, v3(side, 0.25, 0.0).norm() * 0.016, LIGHT);
    }

    // Vertical fin: main box, dark cap; the rudder is a separate moving surface.
    for &side in &[-1.0, 1.0] {
        let out = move |p: V3| v3(p.x - side, p.y, p.z);
        ext.surface(6, 5, move |u, v| fin_point(u * RUDDER_TOP, v * RUDDER_HINGE, side), |_, _| MED, Some(&out));
        ext.surface(1, 7, move |u, v| fin_point(mix(RUDDER_TOP, 1.0, u), v, side), |_, _| Rgb(0.16, 0.18, 0.20), Some(&out));
    }
    for k in 0..7 {
        let (v0, v1) = (k as f64 / 7.0, (k + 1) as f64 / 7.0);
        ext.quad(fin_point(1.0, v0, -1.0), fin_point(1.0, v1, -1.0), fin_point(1.0, v1, 1.0), fin_point(1.0, v0, 1.0), Rgb(0.16, 0.18, 0.20));
    }
    for k in 0..6 {
        let (u0, u1) = (k as f64 / 6.0 * RUDDER_TOP, (k + 1) as f64 / 6.0 * RUDDER_TOP);
        ext.quad(fin_point(u0, RUDDER_HINGE, -1.0), fin_point(u1, RUDDER_HINGE, -1.0), fin_point(u1, RUDDER_HINGE, 1.0), fin_point(u0, RUDDER_HINGE, 1.0), SEAM);
    }
    // Drag-chute housing at the fin root, sunk into the spine so it fairs the
    // fin trailing edge instead of floating above it, and two dorsal aerials.
    ext.lathe(v3(0.0, 0.70, 6.05), V3::Z, &[(0.0, 0.10), (0.6, 0.14), (1.2, 0.14), (1.36, 0.09), (1.38, 0.0)], 8, MED);
    for &z in &[-1.2, 0.8] {
        thin_plate(&mut ext, &[v3(0.0, 0.90, z), v3(0.0, 1.10, z + 0.16), v3(0.0, 1.08, z + 0.24), v3(0.0, 0.88, z + 0.40)], v3(0.012, 0.0, 0.0), Rgb(0.2, 0.22, 0.24));
    }

    // Nozzle: petalled outer shell, lip and dark core.
    for j in 0..16 {
        let (a, b) = (j as f64 * TAU / 16.0, (j + 1) as f64 * TAU / 16.0);
        let p = |z: f64, r: f64, t: f64| v3(t.cos() * r, t.sin() * r * 0.96 + 0.02, z);
        let metal = if j % 2 == 1 { METAL } else { METAL.scale(0.8) };
        ext.quad(p(6.62, 0.51, a), p(6.62, 0.51, b), p(7.30, 0.445, b), p(7.30, 0.445, a), metal);
        ext.quad(p(7.30, 0.445, a), p(7.30, 0.445, b), p(7.22, 0.38, b), p(7.22, 0.38, a), Rgb(0.10, 0.10, 0.10));
        ext.quad(p(7.22, 0.38, a), p(7.22, 0.38, b), p(6.80, 0.36, b), p(6.80, 0.36, a), Rgb(0.06, 0.055, 0.05));
        ext.tri(v3(0.0, 0.02, 6.78), p(6.80, 0.36, a), p(6.80, 0.36, b), Rgb(0.04, 0.03, 0.02));
        let f = |z: f64, r: f64, t: f64| v3(t.cos() * r, t.sin() * r * 0.96 + 0.02, z);
        flame.quad(f(7.30, 0.37, a), f(7.30, 0.37, b), f(9.4, 0.20, b), f(9.4, 0.20, a), Rgb(1.0, 0.55, 0.18));
        flame.quad(f(9.4, 0.20, a), f(9.4, 0.20, b), f(10.8, 0.02, b), f(10.8, 0.02, a), Rgb(0.9, 0.30, 0.08));
        flame.quad(f(7.25, 0.22, a), f(7.25, 0.22, b), f(8.4, 0.04, b), f(8.4, 0.04, a), Rgb(1.0, 0.85, 0.55));
    }

    // Canopy glass, sill rails, aft bow and bulkhead.
    canopy.surface(CANOPY_NU, CANOPY_NV, canopy_point, |_, _| Rgb(0.52, 0.42, 0.17), Some(&|p: V3| v3(0.0, 0.6, p.z)));
    for &v in &[0.0, 1.0] {
        let pts: Vec<V3> = (1..=CANOPY_NU).map(|k| canopy_point(k as f64 / CANOPY_NU as f64, v)).collect();
        for k in 0..pts.len() - 1 {
            ext.capsule(pts[k], pts[k + 1], 0.022, 0.022, Rgb(0.13, 0.15, 0.16), 6);
        }
    }
    for k in 0..CANOPY_NV {
        let (a, b) = (canopy_point(0.985, k as f64 / CANOPY_NV as f64), canopy_point(0.985, (k + 1) as f64 / CANOPY_NV as f64));
        ext.capsule(a, b, 0.03, 0.03, Rgb(0.13, 0.15, 0.16), 6);
    }
    let bulk: Vec<V3> = (0..=CANOPY_NV).map(|k| canopy_point(0.995, k as f64 / CANOPY_NV as f64)).collect();
    ext.polygon(&bulk, Rgb(0.10, 0.11, 0.12));

    // Pilot for the chase view: helmet, visor, shoulders and ACES II headrest.
    crew.ellipsoid(v3(0.0, 1.28, -3.14), v3(0.135, 0.145, 0.155), Rgb(0.70, 0.71, 0.68), 6, 10);
    crew.ellipsoid(v3(0.0, 1.27, -3.235), v3(0.112, 0.06, 0.06), Rgb(0.05, 0.06, 0.07), 4, 8);
    crew.ellipsoid(v3(0.0, 0.98, -3.18), v3(0.25, 0.24, 0.17), Rgb(0.26, 0.30, 0.20), 5, 10);
    let (c30, s30) = ((30.0 * DEG).cos(), (30.0 * DEG).sin());
    let rest = crate::mesh::Frame { o: v3(0.0, 1.18, -2.90), x: V3::X, y: v3(0.0, c30, s30), z: v3(0.0, -s30, c30) };
    crew.block(&rest, v3(0.17, 0.22, 0.07), Rgb(0.18, 0.2, 0.19));

    // Moving surfaces.
    let mut stab = [Mesh::new(), Mesh::new()];
    for (i, &side) in [-1.0, 1.0].iter().enumerate() {
        for &upper in &[true, false] {
            let out = move |p: V3| v3(p.x, if upper { p.y - 1.0 } else { p.y + 1.0 }, p.z);
            stab[i].surface(5, 6, move |u, v| stab_point(side, u, v, upper), move |_, _| if upper { MED } else { LIGHT }, Some(&out));
        }
        for k in 0..6 {
            let (v0, v1) = (k as f64 / 6.0, (k + 1) as f64 / 6.0);
            stab[i].quad(stab_point(side, 1.0, v0, true), stab_point(side, 1.0, v1, true), stab_point(side, 1.0, v1, false), stab_point(side, 1.0, v0, false), MED);
        }
    }
    let mut rudder = Mesh::new();
    for &side in &[-1.0, 1.0] {
        let out = move |p: V3| v3(p.x - side, p.y, p.z);
        rudder.surface(6, 2, move |u, v| fin_point(u * RUDDER_TOP, mix(RUDDER_HINGE, 1.0, v), side), |_, _| MED, Some(&out));
    }
    let mut brakes = [Mesh::new(), Mesh::new(), Mesh::new(), Mesh::new()];
    for (i, &side) in [-1.0, 1.0].iter().enumerate() {
        for (j, &dy) in [0.034, -0.034].iter().enumerate() {
            let f = crate::mesh::Frame { o: v3(side * 0.72, 0.06 + dy, (BRAKE_HINGE_Z + 7.10) * 0.5), x: V3::X, y: V3::Y, z: V3::Z };
            brakes[i * 2 + j].block(&f, v3(0.22, 0.032, 0.475), if dy > 0.0 { MED } else { LIGHT });
        }
    }

    for m in [&mut ext, &mut canopy, &mut crew, &mut rudder].into_iter().chain(stab.iter_mut()).chain(brakes.iter_mut()) {
        m.facet();
    }
    Airframe { exterior: ext, canopy, crew, flame, stab, rudder, brakes }
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
        for c in m.v.chunks(crate::mesh::STRIDE) {
            lo = v3(lo.x.min(c[0] as f64), lo.y.min(c[1] as f64), lo.z.min(c[2] as f64));
            hi = v3(hi.x.max(c[0] as f64), hi.y.max(c[1] as f64), hi.z.max(c[2] as f64));
        }
        (lo, hi)
    }

    #[test]
    fn measured_dimensions_match_the_f16() {
        let a = build();
        let mut all = Mesh::new();
        for m in [&a.exterior, &a.stab[0], &a.stab[1], &a.rudder] {
            all.v.extend_from_slice(&m.v);
        }
        let (lo, hi) = extent(&all);
        let length = hi.z - lo.z;
        let span = hi.x - lo.x;
        assert!((length - 15.06).abs() < 0.08, "length {length}");
        assert!((span - 9.96).abs() < 0.08, "span {span}");
        // Fin cap above the gear line (about 1.75 m below the datum).
        assert!((hi.y + 1.75 - 5.09).abs() < 0.08, "height {}", hi.y + 1.75);
    }

    #[test]
    fn geometry_is_finite_with_unit_normals() {
        let a = build();
        for m in [&a.exterior, &a.canopy, &a.crew, &a.flame, &a.rudder] {
            assert!(m.count() > 0);
            for c in m.v.chunks(crate::mesh::STRIDE) {
                assert!(c.iter().all(|x| x.is_finite()));
                let l = (c[3] * c[3] + c[4] * c[4] + c[5] * c[5]).sqrt();
                assert!((l - 1.0).abs() < 1e-3 || l == 0.0, "normal length {l}");
            }
        }
    }

    #[test]
    fn eye_is_inside_the_canopy() {
        let (w, base, top) = canopy_profile(EYE.z);
        assert!(EYE.y > base + 0.3 && EYE.y < top - 0.2 && w > 0.4);
    }
}
