//! MiG-29A (9.12) Fulcrum, low-poly, after the published three-view: 17.3 m
//! long, 11.4 m span, 4.7 m tall. A slim forward fuselage with the radome
//! and a bubble canopy blends through the leading-edge root extensions into
//! a flat lifting centre body between two widely spaced engine nacelles,
//! with the wedge intakes under the LERX, 42 degree swept wings, all-moving
//! stabilators and twin fins canted out 6 degrees. Vertex colours only, in
//! the two-tone grey of the type, with red stars on the fins and the wings.
//! Body axes as for the F-16 model: x right, y up, z aft, the centre of
//! gravity at the origin.
use crate::math::*;
use crate::mesh::{Mesh, Rgb};

const LIGHT: Rgb = Rgb(0.66, 0.70, 0.73);
const DARK: Rgb = Rgb(0.46, 0.52, 0.57);
const BELLY: Rgb = Rgb(0.74, 0.77, 0.79);
const RADOME: Rgb = Rgb(0.80, 0.80, 0.75);
const GLASS: Rgb = Rgb(0.16, 0.22, 0.26);
const FRAME: Rgb = Rgb(0.40, 0.45, 0.48);
const INTAKE: Rgb = Rgb(0.05, 0.05, 0.06);
const METAL: Rgb = Rgb(0.30, 0.29, 0.28);
const STAR: Rgb = Rgb(0.78, 0.12, 0.10);

/// Stabilator pivot (right side; the left mirrors it) and its hinge axis.
pub const STAB_PIVOT: V3 = v3(1.75, 0.0, 6.95);
/// Right nozzle exit (the left mirrors it).
pub const NOZZLE: V3 = v3(1.25, -0.30, 8.0);

/// Flat polygon facing `out`, wound so its normal points that way.
fn face(m: &mut Mesh, pts: &[V3], out: V3, col: Rgb) {
    let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
    if n.dot(out) >= 0.0 {
        m.polygon(pts, col);
    } else {
        let r: Vec<V3> = pts.iter().rev().copied().collect();
        m.polygon(&r, col);
    }
}

/// A thin lifting surface: top and bottom faces over the outline and the
/// edges between them. `up` is the surface normal on the top side.
fn panel(m: &mut Mesh, outline: &[V3], up: V3, half: f64, top: Rgb, bottom: Rgb) {
    let t: Vec<V3> = outline.iter().map(|&p| p + up * half).collect();
    let b: Vec<V3> = outline.iter().map(|&p| p - up * half).collect();
    face(m, &t, up, top);
    face(m, &b, -up, bottom);
    let c = outline.iter().fold(V3::ZERO, |s, &p| s + p) / outline.len() as f64;
    for k in 0..outline.len() {
        let j = (k + 1) % outline.len();
        let out = ((t[k] + t[j]) * 0.5 - c).norm();
        face(m, &[t[k], t[j], b[j], b[k]], out, top.scale(0.85));
    }
}

/// Red star of radius `r` centred on `c`, lying on a plane through `c`
/// with normal `n`, lifted a few millimetres off it.
fn star(m: &mut Mesh, c: V3, n: V3, r: f64) {
    let e1 = n.cross(if n.y.abs() > 0.9 { V3::X } else { V3::Y }).norm();
    let e2 = n.cross(e1);
    let p = |k: usize, rad: f64| {
        let a = k as f64 * PI / 5.0;
        c + n * 0.01 + (e2 * a.cos() + e1 * a.sin()) * rad
    };
    for k in 0..5 {
        face(m, &[c + n * 0.01, p(2 * k, r), p(2 * k + 1, r * 0.4)], n, STAR);
        face(m, &[c + n * 0.01, p(2 * k + 1, r * 0.4), p(2 * k + 2, r)], n, STAR);
    }
}

/// Fuselage cross-section half width and half height, and its centre
/// height, along z: the slim nose to the cockpit, widening into the centre
/// body. Behind the LERX the body is the flat deck between the nacelles.
const FUSE: [(f64, f64, f64, f64); 9] = [
    // z, half width, half height, centre y
    (-8.70, 0.00, 0.00, 0.30),
    (-8.10, 0.20, 0.20, 0.30),
    (-7.10, 0.40, 0.40, 0.32),
    (-5.80, 0.55, 0.55, 0.38),
    (-4.40, 0.62, 0.66, 0.44),
    (-3.00, 0.70, 0.66, 0.42),
    (-1.20, 0.95, 0.52, 0.36),
    (3.50, 1.05, 0.40, 0.30),
    (7.00, 0.55, 0.28, 0.30),
];

fn fuse_point(i: usize, k: usize, sides: usize) -> V3 {
    let (z, w, h, y) = FUSE[i];
    let t = k as f64 * TAU / sides as f64;
    // A squarish section (superellipse) that reads as the flat Fulcrum body.
    let (c, s) = (t.cos(), t.sin());
    let e = 0.6;
    v3(w * c.signum() * c.abs().powf(e), y + h * s.signum() * s.abs().powf(e), z)
}

/// The airframe without its stabilators, and the right and left stabilators.
pub fn build() -> (Mesh, [Mesh; 2]) {
    let mut m = Mesh::new();
    let sides = 10;
    for i in 0..FUSE.len() - 1 {
        for k in 0..sides {
            let (a, b) = (fuse_point(i, k, sides), fuse_point(i, k + 1, sides));
            let (c, d) = (fuse_point(i + 1, k + 1, sides), fuse_point(i + 1, k, sides));
            let mid = (a + b + c + d) * 0.25;
            let axis = v3(0.0, (FUSE[i].3 + FUSE[i + 1].3) * 0.5, mid.z);
            let col = if i == 0 { RADOME } else if mid.y < axis.y - 0.1 { BELLY } else if (k + i) % 3 == 0 { DARK } else { LIGHT };
            face(&mut m, &[a, b, c, d], (mid - axis).norm(), col);
        }
    }
    let tail: Vec<V3> = (0..sides).map(|k| fuse_point(FUSE.len() - 1, k, sides)).collect();
    face(&mut m, &tail, V3::Z, METAL);

    // Bubble canopy over the cockpit, and its frame bow.
    m.ellipsoid(v3(0.0, 1.02, -4.25), v3(0.40, 0.36, 1.25), GLASS, 5, 8);
    m.capsule(v3(-0.38, 0.95, -3.55), v3(0.38, 0.95, -3.55), 0.03, 0.03, FRAME, 5);
    // Spine behind the canopy, tapering down onto the deck.
    face(&mut m, &[v3(-0.32, 1.05, -3.1), v3(0.32, 1.05, -3.1), v3(0.25, 0.75, 3.0), v3(-0.25, 0.75, 3.0)], V3::Y, LIGHT);
    for s in [-1.0, 1.0] {
        face(&mut m, &[v3(s * 0.32, 1.05, -3.1), v3(s * 0.25, 0.75, 3.0), v3(s * 0.9, 0.62, 3.0), v3(s * 0.66, 0.92, -2.8)], v3(s, 1.0, 0.0), DARK);
    }

    // Leading-edge root extensions, from the cockpit sides out to the wing
    // roots, and the flat centre body deck between the nacelles.
    for s in [-1.0, 1.0] {
        let lerx = [v3(s * 0.60, 0.52, -5.4), v3(s * 2.10, 0.40, -0.6), v3(s * 2.10, 0.40, 1.6), v3(s * 0.95, 0.52, 1.6)];
        panel(&mut m, &lerx, V3::Y, 0.06, if s < 0.0 { DARK } else { LIGHT }, BELLY);
        let deck = [v3(s * 0.95, 0.55, 1.6), v3(s * 2.05, 0.42, 1.6), v3(s * 1.85, 0.36, 6.4), v3(s * 0.55, 0.45, 6.4)];
        panel(&mut m, &deck, V3::Y, 0.07, LIGHT, BELLY);
    }

    // Engine nacelles under the deck, with the wedge intakes under the LERX
    // and the afterburner nozzles.
    for s in [-1.0, 1.0] {
        let x = s * 1.25;
        let y = -0.30;
        // Intake box: open front, raked; the ramp splits it from the body.
        let (z0, z1) = (-2.4, -0.6);
        let (hw, hh) = (0.42, 0.40);
        let corners = |z: f64, rake: f64| [v3(x - hw, y + hh, z), v3(x + hw, y + hh, z), v3(x + hw, y - hh, z + rake), v3(x - hw, y - hh, z + rake)];
        let f = corners(z0, 0.35);
        let b = corners(z1, 0.0);
        face(&mut m, &[f[0], f[1], f[2], f[3]], -V3::Z, INTAKE);
        face(&mut m, &[f[0], f[1], b[1], b[0]], V3::Y, BELLY);
        face(&mut m, &[f[3], f[2], b[2], b[3]], -V3::Y, BELLY);
        face(&mut m, &[f[1], f[2], b[2], b[1]], V3::X, BELLY);
        face(&mut m, &[f[0], f[3], b[3], b[0]], -V3::X, BELLY);
        // Nacelle and nozzle.
        m.lathe(v3(x, y, z1), V3::Z, &[(0.0, 0.50), (3.0, 0.56), (6.6, 0.52), (7.9, 0.44)], 8, BELLY);
        m.lathe(v3(x, y, z1 + 7.9), V3::Z, &[(0.0, 0.44), (0.7, 0.40), (0.7, 0.30), (0.2, 0.0)], 8, METAL);
    }

    // Wings: 42 degree leading-edge sweep from the LERX junction to the tips.
    for s in [-1.0, 1.0] {
        let wing = [v3(s * 2.0, 0.38, -0.6), v3(s * 5.68, 0.28, 3.25), v3(s * 5.68, 0.28, 4.35), v3(s * 4.2, 0.30, 4.5), v3(s * 2.0, 0.36, 4.3)];
        panel(&mut m, &wing, V3::Y, 0.05, if s < 0.0 { LIGHT } else { DARK }, BELLY);
        star(&mut m, v3(s * 4.3, 0.33, 3.0), V3::Y, 0.42);
        star(&mut m, v3(s * 4.3, 0.23, 3.0), -V3::Y, 0.42);
    }

    // Twin fins on the nacelle shoulders, canted out 6 degrees.
    for s in [-1.0, 1.0] {
        let cant = 6.0 * DEG;
        let up = v3(s * cant.sin(), cant.cos(), 0.0);
        let root = v3(s * 1.30, 0.50, 0.0);
        let p = |dz: f64, h: f64| root + up * h + V3::Z * dz;
        let fin = [p(4.0, 0.0), p(7.6, 0.0), p(8.1, 2.75), p(6.9, 2.75)];
        let side = v3(s * cant.cos(), -cant.sin(), 0.0);
        panel(&mut m, &fin, side, 0.05, DARK, DARK);
        star(&mut m, p(6.6, 1.4) + side * 0.05, side, 0.36);
        star(&mut m, p(6.6, 1.4) - side * 0.05, -side, 0.36);
    }
    // Ventral strakes under the nacelles.
    for s in [-1.0, 1.0] {
        let x = s * 1.6;
        face(&mut m, &[v3(x, -0.55, 4.5), v3(x, -0.55, 6.8), v3(x + s * 0.12, -1.1, 6.6)], v3(s, 0.0, 0.0), DARK);
    }
    m.facet();

    let stab = |s: f64| {
        let mut st = Mesh::new();
        let outline = [v3(s * 1.75, 0.05, 6.0), v3(s * 3.85, -0.05, 7.95), v3(s * 3.85, -0.05, 8.65), v3(s * 1.75, 0.05, 8.45)];
        panel(&mut st, &outline, V3::Y, 0.04, LIGHT, BELLY);
        st.facet();
        st
    };
    (m, [stab(1.0), stab(-1.0)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fulcrum_matches_its_three_view() {
        let (m, stabs) = build();
        let (mut lo, mut hi) = (v3(f64::INFINITY, f64::INFINITY, f64::INFINITY), v3(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY));
        for mesh in std::iter::once(&m).chain(stabs.iter()) {
            for v in mesh.v.chunks(crate::mesh::STRIDE) {
                let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                lo = v3(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
                hi = v3(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
            }
        }
        let (span, length) = (hi.x - lo.x, hi.z - lo.z);
        assert!((span - 11.36).abs() < 0.3, "span {span:.2} m");
        assert!((length - 17.32).abs() < 0.5, "length {length:.2} m");
        // Fin tip 4.73 m above the ground with the sim's gear down (1.55 m).
        assert!((hi.y + 1.55 - 4.73).abs() < 0.3, "height {:.2} m", hi.y + 1.55);
    }
}
