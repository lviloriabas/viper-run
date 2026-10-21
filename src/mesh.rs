//! Triangle-soup mesh builder shared by the exterior and cockpit models.
//! Vertex layout: position(3) normal(3) color(3) uv(2); uv.x < 0 means untextured.
use crate::math::*;

pub const STRIDE: usize = 11;

#[derive(Clone, Copy)]
pub struct Rgb(pub f64, pub f64, pub f64);

impl Rgb {
    pub fn scale(self, s: f64) -> Rgb {
        Rgb(self.0 * s, self.1 * s, self.2 * s)
    }
}

/// Orthonormal local frame used to place panels and instruments.
#[derive(Clone, Copy)]
pub struct Frame {
    pub o: V3,
    pub x: V3,
    pub y: V3,
    pub z: V3,
}

impl Frame {
    pub fn p(&self, u: f64, v: f64, w: f64) -> V3 {
        self.o + self.x * u + self.y * v + self.z * w
    }
    pub fn at(&self, u: f64, v: f64, w: f64) -> Frame {
        Frame { o: self.p(u, v, w), ..*self }
    }
}

#[derive(Default)]
pub struct Mesh {
    pub v: Vec<f32>,
}

const NO_UV: (f64, f64) = (-1.0, -1.0);

impl Mesh {
    pub fn new() -> Mesh {
        Mesh { v: Vec::new() }
    }
    pub fn count(&self) -> usize {
        self.v.len() / STRIDE
    }
    pub fn clear(&mut self) {
        self.v.clear();
    }

    pub fn vert(&mut self, p: V3, n: V3, c: Rgb, uv: (f64, f64)) {
        self.v.extend_from_slice(&[
            p.x as f32, p.y as f32, p.z as f32, n.x as f32, n.y as f32, n.z as f32,
            c.0 as f32, c.1 as f32, c.2 as f32, uv.0 as f32, uv.1 as f32,
        ]);
    }

    pub fn tri(&mut self, a: V3, b: V3, c: V3, col: Rgb) {
        let n = (b - a).cross(c - a).norm();
        for p in [a, b, c] {
            self.vert(p, n, col, NO_UV);
        }
    }

    pub fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, col: Rgb) {
        self.tri(a, b, c, col);
        self.tri(a, c, d, col);
    }

    pub fn quad_uv(&mut self, a: V3, b: V3, c: V3, d: V3, uv: [(f64, f64); 4]) {
        let n = (b - a).cross(c - a).norm();
        let w = Rgb(1.0, 1.0, 1.0);
        for &(p, t) in &[(a, uv[0]), (b, uv[1]), (c, uv[2]), (a, uv[0]), (c, uv[2]), (d, uv[3])] {
            self.vert(p, n, w, t);
        }
    }

    /// Parametric surface over [0,1]^2 with smooth finite-difference normals.
    /// `outward` orients each normal away from the given reference point
    /// function; pass `None` to keep the parametric orientation.
    pub fn surface<F, C>(&mut self, nu: usize, nv: usize, f: F, color: C, outward: Option<&dyn Fn(V3) -> V3>)
    where
        F: Fn(f64, f64) -> V3,
        C: Fn(V3, V3) -> Rgb,
    {
        let e = 1e-4;
        let normal = |u: f64, v: f64| -> V3 {
            let du = f((u + e).min(1.0), v) - f((u - e).max(0.0), v);
            let dv = f(u, (v + e).min(1.0)) - f(u, (v - e).max(0.0));
            let mut n = du.cross(dv).norm();
            if n.len() < 0.5 {
                // Degenerate pole: take the normal from a nearby parameter.
                let u2 = if u < 0.5 { u + 0.01 } else { u - 0.01 };
                let du = f((u2 + e).min(1.0), v) - f((u2 - e).max(0.0), v);
                let dv = f(u2, (v + e).min(1.0)) - f(u2, (v - e).max(0.0));
                n = du.cross(dv).norm();
            }
            let p = f(u, v);
            if let Some(o) = outward {
                if n.dot(p - o(p)) < 0.0 {
                    n = -n;
                }
            }
            n
        };
        let mut grid = Vec::with_capacity((nu + 1) * (nv + 1));
        for i in 0..=nu {
            for j in 0..=nv {
                let (u, v) = (i as f64 / nu as f64, j as f64 / nv as f64);
                let p = f(u, v);
                let n = normal(u, v);
                grid.push((p, n, color(p, n)));
            }
        }
        let at = |i: usize, j: usize| grid[i * (nv + 1) + j];
        for i in 0..nu {
            for j in 0..nv {
                let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                for (p, n, col) in [a, b, c, a, c, d] {
                    self.vert(p, n, col, NO_UV);
                }
            }
        }
    }

    /// Surface of revolution around an arbitrary axis; profile is (along, radius).
    pub fn lathe(&mut self, base: V3, axis: V3, profile: &[(f64, f64)], sides: usize, col: Rgb) {
        let ax = axis.norm();
        let r0 = ax.cross(if ax.y.abs() > 0.9 { V3::X } else { V3::Y }).norm();
        let r1 = ax.cross(r0);
        for k in 0..profile.len() - 1 {
            let (a0, ra) = profile[k];
            let (a1, rb) = profile[k + 1];
            let slope = (ra - rb) / (a1 - a0).abs().max(1e-6);
            for s in 0..sides {
                let t0 = s as f64 * TAU / sides as f64;
                let t1 = (s + 1) as f64 * TAU / sides as f64;
                let dir = |t: f64| r0 * t.cos() + r1 * t.sin();
                let pa0 = base + ax * a0 + dir(t0) * ra;
                let pa1 = base + ax * a0 + dir(t1) * ra;
                let pb0 = base + ax * a1 + dir(t0) * rb;
                let pb1 = base + ax * a1 + dir(t1) * rb;
                let n0 = (dir(t0) + ax * slope * (a1 - a0).signum()).norm();
                let n1 = (dir(t1) + ax * slope * (a1 - a0).signum()).norm();
                for (p, n) in [(pa0, n0), (pb0, n0), (pb1, n1), (pa0, n0), (pb1, n1), (pa1, n1)] {
                    self.vert(p, n, col, NO_UV);
                }
            }
        }
    }

    /// Tapered cylinder with rounded caps.
    pub fn capsule(&mut self, a: V3, b: V3, r1: f64, r2: f64, col: Rgb, sides: usize) {
        let axis = b - a;
        let l = axis.len();
        if l < 1e-6 {
            self.ellipsoid(a, v3(r1, r1, r1), col, 6, sides);
            return;
        }
        let mut prof = Vec::new();
        let caps = 4;
        for i in 0..=caps {
            let t = (i as f64 / caps as f64) * PI * 0.5;
            prof.push((-r1 * t.cos(), r1 * t.sin()));
        }
        for i in 0..=caps {
            let t = (i as f64 / caps as f64) * PI * 0.5;
            prof.push((l + r2 * t.sin(), r2 * t.cos()));
        }
        prof.dedup_by(|x, y| (x.0 - y.0).abs() < 1e-9 && (x.1 - y.1).abs() < 1e-9);
        self.lathe(a, axis, &prof, sides, col);
    }

    pub fn ellipsoid(&mut self, c: V3, r: V3, col: Rgb, lat: usize, lon: usize) {
        self.ellipsoid_basis(c, V3::X * r.x, V3::Y * r.y, V3::Z * r.z, col, lat, lon);
    }

    /// Ellipsoid along arbitrary semi-axis vectors.
    pub fn ellipsoid_basis(&mut self, c: V3, ex: V3, ey: V3, ez: V3, col: Rgb, lat: usize, lon: usize) {
        let point = |i: usize, j: usize| -> (V3, V3) {
            let th = i as f64 * PI / lat as f64;
            let ph = j as f64 * TAU / lon as f64;
            let (s, cth) = (th.sin(), th.cos());
            let unit = v3(s * ph.cos(), cth, s * ph.sin());
            let p = c + ex * unit.x + ey * unit.y + ez * unit.z;
            // Normal of an ellipsoid: inverse-scaled along each semi-axis.
            let inv = |e: V3| e / e.dot(e).max(1e-12);
            let n = (inv(ex) * unit.x + inv(ey) * unit.y + inv(ez) * unit.z).norm();
            (p, n)
        };
        for i in 0..lat {
            for j in 0..lon {
                let q = [point(i, j), point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)];
                for k in [0, 1, 2, 0, 2, 3] {
                    self.vert(q[k].0, q[k].1, col, NO_UV);
                }
            }
        }
    }

    /// Box along a frame's axes, with half extents and a small bevel.
    pub fn block(&mut self, f: &Frame, half: V3, col: Rgb) {
        let b = (half.x.min(half.y).min(half.z) * 0.18).min(0.006);
        let p = |x: f64, y: f64, z: f64| f.p(x, y, z);
        let (hx, hy, hz) = (half.x, half.y, half.z);
        let faces: [(V3, [V3; 4], f64); 6] = [
            (f.z, [p(-hx + b, -hy + b, hz), p(hx - b, -hy + b, hz), p(hx - b, hy - b, hz), p(-hx + b, hy - b, hz)], 1.0),
            (-f.z, [p(hx - b, -hy + b, -hz), p(-hx + b, -hy + b, -hz), p(-hx + b, hy - b, -hz), p(hx - b, hy - b, -hz)], 0.7),
            (f.x, [p(hx, -hy + b, hz - b), p(hx, -hy + b, -hz + b), p(hx, hy - b, -hz + b), p(hx, hy - b, hz - b)], 0.85),
            (-f.x, [p(-hx, -hy + b, -hz + b), p(-hx, -hy + b, hz - b), p(-hx, hy - b, hz - b), p(-hx, hy - b, -hz + b)], 0.85),
            (f.y, [p(-hx + b, hy, hz - b), p(hx - b, hy, hz - b), p(hx - b, hy, -hz + b), p(-hx + b, hy, -hz + b)], 1.08),
            (-f.y, [p(-hx + b, -hy, -hz + b), p(hx - b, -hy, -hz + b), p(hx - b, -hy, hz - b), p(-hx + b, -hy, hz - b)], 0.6),
        ];
        for (n, q, _) in faces.iter() {
            for &k in &[0usize, 1, 2, 0, 2, 3] {
                self.vert(q[k], *n, col, NO_UV);
            }
        }
        if b > 0.0 {
            // Bevel strips along the twelve edges, lit as the average of both faces.
            let corner = |sx: f64, sy: f64, sz: f64, ix: bool, iy: bool, iz: bool| {
                p(sx * (hx - if ix { b } else { 0.0 }), sy * (hy - if iy { b } else { 0.0 }), sz * (hz - if iz { b } else { 0.0 }))
            };
            for &sx in &[-1.0, 1.0] {
                for &sy in &[-1.0, 1.0] {
                    let n = (f.x * sx + f.y * sy).norm();
                    let a0 = corner(sx, sy, -1.0, false, true, true);
                    let a1 = corner(sx, sy, 1.0, false, true, true);
                    let b0 = corner(sx, sy, -1.0, true, false, true);
                    let b1 = corner(sx, sy, 1.0, true, false, true);
                    for q in [a0, a1, b1, a0, b1, b0] {
                        self.vert(q, n, col, NO_UV);
                    }
                }
                for &sz in &[-1.0, 1.0] {
                    let n = (f.x * sx + f.z * sz).norm();
                    let a0 = corner(sx, -1.0, sz, false, true, true);
                    let a1 = corner(sx, 1.0, sz, false, true, true);
                    let b0 = corner(sx, -1.0, sz, true, true, false);
                    let b1 = corner(sx, 1.0, sz, true, true, false);
                    for q in [a0, a1, b1, a0, b1, b0] {
                        self.vert(q, n, col, NO_UV);
                    }
                }
            }
            for &sy in &[-1.0, 1.0] {
                for &sz in &[-1.0, 1.0] {
                    let n = (f.y * sy + f.z * sz).norm();
                    let a0 = corner(-1.0, sy, sz, true, false, true);
                    let a1 = corner(1.0, sy, sz, true, false, true);
                    let b0 = corner(-1.0, sy, sz, true, true, false);
                    let b1 = corner(1.0, sy, sz, true, true, false);
                    for q in [a0, a1, b1, a0, b1, b0] {
                        self.vert(q, n, col, NO_UV);
                    }
                }
            }
        }
    }

    /// Flat polygon (convex or star-shaped around its first vertex average).
    pub fn polygon(&mut self, pts: &[V3], col: Rgb) {
        if pts.len() < 3 {
            return;
        }
        let c = pts.iter().fold(V3::ZERO, |s, &p| s + p) / pts.len() as f64;
        for k in 0..pts.len() {
            self.tri(c, pts[k], pts[(k + 1) % pts.len()], col);
        }
    }

    /// Extruded ring between two closed outlines lying in a frame's xy plane:
    /// front face at w = 0, sides back to w = -depth.
    pub fn bezel(&mut self, f: &Frame, outer: &[(f64, f64)], inner: &[(f64, f64)], depth: f64, recess: f64, col: Rgb) {
        let n = outer.len();
        assert_eq!(n, inner.len());
        let side = col.scale(0.62);
        for k in 0..n {
            let k2 = (k + 1) % n;
            let (o0, o1) = (f.p(outer[k].0, outer[k].1, 0.0), f.p(outer[k2].0, outer[k2].1, 0.0));
            let (i0, i1) = (f.p(inner[k].0, inner[k].1, 0.0), f.p(inner[k2].0, inner[k2].1, 0.0));
            for p in [o0, o1, i1, o0, i1, i0] {
                self.vert(p, f.z, col, NO_UV);
            }
            let (ob0, ob1) = (f.p(outer[k].0, outer[k].1, -depth), f.p(outer[k2].0, outer[k2].1, -depth));
            let no = (o1 - o0).cross(f.z).norm();
            for p in [o0, ob0, ob1, o0, ob1, o1] {
                self.vert(p, -no, side, NO_UV);
            }
            if recess > 0.0 {
                let (ib0, ib1) = (f.p(inner[k].0, inner[k].1, -recess), f.p(inner[k2].0, inner[k2].1, -recess));
                for p in [i0, i1, ib1, i0, ib1, ib0] {
                    self.vert(p, no, side.scale(0.7), NO_UV);
                }
            }
        }
    }

    /// Textured disc facing the frame's +z, with atlas rect (u0, v0, u1, v1).
    pub fn disc_uv(&mut self, f: &Frame, r: f64, uv: (f64, f64, f64, f64), sides: usize) {
        let (u0, v0, u1, v1) = uv;
        let (cu, cv) = ((u0 + u1) * 0.5, (v0 + v1) * 0.5);
        let (ru, rv) = ((u1 - u0) * 0.5, (v1 - v0) * 0.5);
        let w = Rgb(1.0, 1.0, 1.0);
        for s in 0..sides {
            let a0 = s as f64 * TAU / sides as f64;
            let a1 = (s + 1) as f64 * TAU / sides as f64;
            let c = f.p(0.0, 0.0, 0.0);
            let p0 = f.p(r * a0.cos(), r * a0.sin(), 0.0);
            let p1 = f.p(r * a1.cos(), r * a1.sin(), 0.0);
            self.vert(c, f.z, w, (cu, cv));
            self.vert(p0, f.z, w, (cu + ru * a0.cos(), cv - rv * a0.sin()));
            self.vert(p1, f.z, w, (cu + ru * a1.cos(), cv - rv * a1.sin()));
        }
    }

    /// Ring bezel for round gauges.
    pub fn ring(&mut self, f: &Frame, r_in: f64, r_out: f64, depth: f64, col: Rgb, sides: usize) {
        let outer: Vec<(f64, f64)> = (0..sides).map(|s| {
            let a = s as f64 * TAU / sides as f64;
            (r_out * a.cos(), r_out * a.sin())
        }).collect();
        let inner: Vec<(f64, f64)> = outer.iter().map(|&(x, y)| (x * r_in / r_out, y * r_in / r_out)).collect();
        self.bezel(f, &outer, &inner, depth, depth * 0.6, col);
    }

    /// Thin ribbon along a polyline, offset along per-point normals.
    pub fn ribbon(&mut self, pts: &[(V3, V3)], half_width: f64, col: Rgb) {
        for k in 0..pts.len().saturating_sub(1) {
            let (p0, n0) = pts[k];
            let (p1, n1) = pts[k + 1];
            let t = (p1 - p0).norm();
            let (w0, w1) = (t.cross(n0).norm() * half_width, t.cross(n1).norm() * half_width);
            let (p0, p1) = (p0 + n0 * 0.004, p1 + n1 * 0.004);
            let (a, b, c, d) = (p0 - w0, p0 + w0, p1 + w1, p1 - w1);
            for (p, n) in [(a, n0), (b, n0), (c, n1), (a, n0), (c, n1), (d, n1)] {
                self.vert(p, n, col, NO_UV);
            }
        }
    }
}

/// Rounded rectangle outline centred on the origin.
pub fn rounded_rect(w: f64, h: f64, r: f64, per_corner: usize) -> Vec<(f64, f64)> {
    let r = r.min(w * 0.5).min(h * 0.5);
    let corners = [(w / 2.0 - r, h / 2.0 - r, 0.0), (-w / 2.0 + r, h / 2.0 - r, PI / 2.0), (-w / 2.0 + r, -h / 2.0 + r, PI), (w / 2.0 - r, -h / 2.0 + r, PI * 1.5)];
    let mut out = Vec::new();
    for (cx, cy, a0) in corners {
        for i in 0..=per_corner {
            let t = a0 + i as f64 * (PI / 2.0) / per_corner as f64;
            out.push((cx + t.cos() * r, cy + t.sin() * r));
        }
    }
    out
}
