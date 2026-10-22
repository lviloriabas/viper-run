//! Buildings, trees and power lines on the ground, so the valley does not
//! look empty: the airbase's control tower, hangars, hardened shelters, fuel
//! farm and radar dome on the apron; farming villages along the river with
//! houses, a church, barns and silos; groves of trees on the valley floor
//! and the lower slopes; and a power line down the valley side. Everything
//! is placed once at start-up from a fixed seed, on the terrain grid, and
//! merged into one mesh per cluster in cluster-local coordinates.
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
use crate::targets;
use crate::terrain::{self, river_x};

const CONCRETE: Rgb = Rgb(0.60, 0.59, 0.55);
const CONCRETE_DARK: Rgb = Rgb(0.45, 0.45, 0.42);
const STEEL: Rgb = Rgb(0.50, 0.53, 0.55);
const GLASS: Rgb = Rgb(0.14, 0.20, 0.24);
const OLIVE: Rgb = Rgb(0.36, 0.39, 0.27);
const WALLS: [Rgb; 4] = [Rgb(0.86, 0.83, 0.74), Rgb(0.80, 0.70, 0.52), Rgb(0.74, 0.74, 0.70), Rgb(0.82, 0.78, 0.66)];
const ROOFS: [Rgb; 3] = [Rgb(0.55, 0.24, 0.16), Rgb(0.36, 0.30, 0.28), Rgb(0.48, 0.30, 0.20)];
const BARN: Rgb = Rgb(0.52, 0.20, 0.15);
const LEAF: [Rgb; 3] = [Rgb(0.18, 0.32, 0.14), Rgb(0.24, 0.36, 0.16), Rgb(0.15, 0.27, 0.15)];
const TRUNK: Rgb = Rgb(0.30, 0.22, 0.14);

/// A group of objects drawn as one mesh, positioned at `origin`.
pub struct Cluster {
    pub origin: V3,
    pub mesh: Mesh,
}

/// Ground height under a footprint: the lowest grid point under it, so a
/// building on a slope sits into the hill instead of floating.
fn ground(p: V3, half: f64) -> f64 {
    [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 0.0)].iter().map(|&(a, b)| terrain::floor(p.x + a * half, p.z + b * half)).fold(f64::INFINITY, f64::min)
}

/// Frame at a ground point turned by `yaw` (radians about up).
fn site(o: V3, yaw: f64) -> Frame {
    let (s, c) = yaw.sin_cos();
    Frame { o, x: v3(c, 0.0, -s), y: V3::Y, z: v3(s, 0.0, c) }
}

/// Box standing on `f.o`: half width `w` along x, half depth `d` along z, height `h`.
fn house_box(m: &mut Mesh, f: &Frame, w: f64, d: f64, h: f64, col: Rgb) {
    m.block(&Frame { o: f.p(0.0, h * 0.5 - 1.0, 0.0), ..*f }, v3(w, h * 0.5 + 1.0, d), col);
}

/// Gable roof over a `w` x `d` box of height `h`, ridge along z, `rise` high.
fn gable(m: &mut Mesh, f: &Frame, w: f64, d: f64, h: f64, rise: f64, col: Rgb) {
    let (e, r) = (0.4, w + 0.4);
    let p = |x: f64, y: f64, z: f64| f.p(x, y, z);
    m.quad(p(-r, h - e, -d - e), p(0.0, h + rise, -d - e), p(0.0, h + rise, d + e), p(-r, h - e, d + e), col);
    m.quad(p(r, h - e, d + e), p(0.0, h + rise, d + e), p(0.0, h + rise, -d - e), p(r, h - e, -d - e), col.scale(0.85));
    m.tri(p(-w, h, -d), p(w, h, -d), p(0.0, h + rise, -d), WALLS[0].scale(0.9));
    m.tri(p(w, h, d), p(-w, h, d), p(0.0, h + rise, d), WALLS[0].scale(0.9));
}

fn house(m: &mut Mesh, f: &Frame, rnd: &mut Lcg) {
    let (w, d, h) = (3.5 + rnd.next() * 2.0, 4.5 + rnd.next() * 3.0, 3.0 + rnd.next() * 3.0);
    let wall = WALLS[(rnd.next() * 4.0) as usize % 4];
    let roof = ROOFS[(rnd.next() * 3.0) as usize % 3];
    house_box(m, f, w, d, h, wall);
    gable(m, f, w, d, h, 2.2 + rnd.next(), roof);
    // A dark doorway and two windows on the long side.
    let side = Frame { o: f.p(w + 0.05, 0.0, 0.0), x: f.z, y: V3::Y, z: f.x };
    m.block(&side.at(0.0, 1.0, 0.0), v3(0.5, 1.0, 0.05), GLASS.scale(0.6));
    for dz in [-d * 0.55, d * 0.55] {
        m.block(&side.at(dz, h * 0.6, 0.0), v3(0.45, 0.45, 0.05), GLASS);
    }
}

fn church(m: &mut Mesh, f: &Frame) {
    house_box(m, f, 5.0, 10.0, 7.0, WALLS[0]);
    gable(m, f, 5.0, 10.0, 7.0, 4.5, ROOFS[1]);
    let t = Frame { o: f.p(0.0, 0.0, -12.5), ..*f };
    house_box(m, &t, 2.5, 2.5, 16.0, WALLS[0]);
    // Spire: a four-sided pyramid on the tower.
    let top = t.p(0.0, 25.0, 0.0);
    let c = |x: f64, z: f64| t.p(x * 2.8, 16.0, z * 2.8);
    for (a, b) in [((-1.0, -1.0), (1.0, -1.0)), ((1.0, -1.0), (1.0, 1.0)), ((1.0, 1.0), (-1.0, 1.0)), ((-1.0, 1.0), (-1.0, -1.0))] {
        m.tri(c(a.0, a.1), c(b.0, b.1), top, ROOFS[1].scale(0.8));
    }
}

fn barn(m: &mut Mesh, f: &Frame, rnd: &mut Lcg) {
    house_box(m, f, 6.0, 11.0, 6.0, BARN);
    gable(m, f, 6.0, 11.0, 6.0, 4.0, ROOFS[1]);
    if rnd.next() < 0.7 {
        let c = f.p(9.5, 0.0, -6.0);
        m.lathe(c - V3::Y, V3::Y, &[(0.0, 0.0), (0.0, 2.8), (14.0, 2.8), (15.5, 2.0), (16.3, 0.0)], 8, Rgb(0.70, 0.71, 0.70));
    }
}

/// Low-poly conifer or broadleaf: a trunk under one or two cones.
fn tree(m: &mut Mesh, base: V3, h: f64, rnd: &mut Lcg) {
    let leaf = LEAF[(rnd.next() * 3.0) as usize % 3];
    m.lathe(base - V3::Y * 0.5, V3::Y, &[(0.0, 0.35), (h * 0.3, 0.3)], 4, TRUNK);
    if rnd.next() < 0.6 {
        m.lathe(base + V3::Y * (h * 0.2), V3::Y, &[(0.0, 0.0), (0.0, h * 0.28), (h * 0.8, 0.0)], 5, leaf);
    } else {
        m.lathe(base + V3::Y * (h * 0.25), V3::Y, &[(0.0, 0.0), (0.0, h * 0.22), (h * 0.35, h * 0.3), (h * 0.65, h * 0.2), (h * 0.75, 0.0)], 5, leaf);
    }
}

/// Lattice pylon reduced to its outline: four tapering legs, a waist and
/// the cross-arm with its insulators.
fn pylon(m: &mut Mesh, f: &Frame) {
    let leg = |sx: f64, sz: f64| (f.p(sx * 3.0, -1.0, sz * 3.0), f.p(sx * 0.8, 26.0, sz * 0.8));
    for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let (a, b) = leg(sx, sz);
        m.capsule(a, b, 0.25, 0.18, STEEL, 4);
    }
    m.block(&Frame { o: f.p(0.0, 26.5, 0.0), ..*f }, v3(9.0, 0.5, 0.6), STEEL);
    for x in [-8.0, 0.0, 8.0] {
        m.capsule(f.p(x, 26.0, 0.0), f.p(x, 23.5, 0.0), 0.25, 0.25, Rgb(0.30, 0.42, 0.36), 4);
    }
}

/// Arched hangar or hardened shelter on `f`: half span `w`, half depth `d`,
/// height `h`, its open front facing -z.
fn arch(m: &mut Mesh, f: &Frame, w: f64, d: f64, h: f64, col: Rgb, door: Rgb) {
    let n = 8;
    let p = |k: usize, z: f64| {
        let t = PI * k as f64 / n as f64;
        f.p(-w * t.cos(), h * t.sin() - 0.5, z)
    };
    for k in 0..n {
        m.quad(p(k, -d), p(k + 1, -d), p(k + 1, d), p(k, d), col.scale(0.9 + 0.1 * ((k % 2) as f64)));
    }
    let front: Vec<V3> = (0..=n).map(|k| p(k, -d)).collect();
    let back: Vec<V3> = (0..=n).map(|k| p(k, d)).collect();
    m.polygon(&front, door);
    m.polygon(&back, col.scale(0.8));
}

/// The airbase buildings along the apron, west of the runway.
fn airbase(rnd: &mut Lcg) -> Cluster {
    let e = terrain::base_elevation();
    let (cx, cz) = terrain::runway_point(0.0, -260.0);
    let origin = v3(cx, e, cz);
    let (ax, az) = terrain::runway_dir();
    // Facing the runway: buildings' fronts (-z) look east, toward it.
    let yaw = az.atan2(-ax);
    let at = |u: f64, v: f64, y: f64| {
        let (x, z) = terrain::runway_point(u, v);
        site(v3(x, e + y, z) - origin, yaw)
    };
    let mut m = Mesh::new();
    // Control tower: shaft, glazed cab, roof and mast.
    let t = at(-80.0, -205.0, 0.0);
    house_box(&mut m, &t, 3.0, 3.0, 18.0, CONCRETE);
    m.block(&Frame { o: t.p(0.0, 20.0, 0.0), ..t }, v3(4.5, 2.0, 4.5), GLASS);
    m.block(&Frame { o: t.p(0.0, 22.4, 0.0), ..t }, v3(5.0, 0.4, 5.0), CONCRETE_DARK);
    m.capsule(t.p(0.0, 22.8, 0.0), t.p(0.0, 28.0, 0.0), 0.15, 0.1, STEEL, 4);
    house_box(&mut m, &at(-80.0, -230.0, 0.0), 12.0, 8.0, 4.0, CONCRETE);
    // Three maintenance hangars and four hardened aircraft shelters.
    for u in [-420.0, -330.0, -240.0] {
        arch(&mut m, &at(u, -322.0, 0.0), 18.0, 24.0, 13.0, Rgb(0.55, 0.57, 0.54), Rgb(0.30, 0.31, 0.30));
    }
    for u in [60.0, 140.0, 220.0, 300.0] {
        arch(&mut m, &at(u, -318.0, 0.0), 12.0, 18.0, 8.5, OLIVE, CONCRETE_DARK);
    }
    // Fuel farm and the approach radar's dome on its tower.
    for (u, v) in [(430.0, -300.0), (460.0, -300.0), (445.0, -330.0)] {
        let c = at(u, v, 0.0).o;
        m.lathe(c - V3::Y, V3::Y, &[(0.0, 0.0), (0.0, 9.0), (8.0, 9.0), (9.5, 0.0)], 10, Rgb(0.78, 0.78, 0.74));
    }
    let r = at(520.0, -240.0, 0.0);
    house_box(&mut m, &r, 3.0, 3.0, 9.0, CONCRETE);
    m.ellipsoid(r.p(0.0, 13.0, 0.0), v3(4.5, 4.5, 4.5), Rgb(0.88, 0.88, 0.84), 5, 8);
    // Windsock beside the touchdown zone of runway A.
    let ws = at(-1100.0, -80.0, 0.0);
    m.capsule(ws.p(0.0, -0.5, 0.0), ws.p(0.0, 7.0, 0.0), 0.1, 0.08, STEEL, 4);
    m.lathe(ws.p(0.0, 7.0, 0.0), ws.x, &[(0.0, 0.0), (0.0, 0.6), (3.5, 0.35), (3.5, 0.0)], 5, Rgb(0.95, 0.45, 0.12));
    // A few trees round the buildings.
    for _ in 0..30 {
        let (u, v) = (-700.0 + rnd.next() * 1500.0, -420.0 - rnd.next() * 60.0);
        let p = at(u, v, 0.0).o + origin;
        tree(&mut m, v3(p.x, ground(p, 1.0), p.z) - origin, 9.0 + rnd.next() * 6.0, rnd);
    }
    m.facet();
    Cluster { origin, mesh: m }
}

/// Clear of the runway and the strike targets.
fn clear(p: V3, avoid: &[V3]) -> bool {
    let (u, v) = terrain::runway_local(p.x, p.z);
    let base = u.abs() < 1700.0 && v > -480.0 && v < 140.0;
    !base && avoid.iter().all(|a| (v3(a.x, 0.0, a.z) - v3(p.x, 0.0, p.z)).len() > 160.0)
}

/// Level enough to build on: the grid within 15 m varies by less than 4 m.
fn flat(x: f64, z: f64) -> bool {
    let h = [terrain::floor(x - 15.0, z), terrain::floor(x + 15.0, z), terrain::floor(x, z - 15.0), terrain::floor(x, z + 15.0)];
    let (lo, hi) = h.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &y| (a.min(y), b.max(y)));
    hi - lo < 4.0 && lo > 30.0
}

/// A village on the valley floor `off` metres from the river at `z`: houses
/// along a lane, a church, a couple of farms and trees round it.
fn village(z: f64, off: f64, rnd: &mut Lcg, avoid: &[V3]) -> Option<Cluster> {
    let x = river_x(z) + off;
    let origin = v3(x, terrain::floor(x, z), z);
    if !clear(origin, avoid) {
        return None;
    }
    let mut m = Mesh::new();
    let lane = rnd.next() * PI;
    let dir = v3(lane.cos(), 0.0, lane.sin());
    let side = v3(-dir.z, 0.0, dir.x);
    let mut placed = 0;
    for k in 0..26 {
        let along = (k as f64 - 13.0) * 19.0 + rnd.next() * 6.0;
        let across = if k % 2 == 0 { 14.0 } else { -14.0 } + rnd.next() * 8.0 - 4.0;
        let p = origin + dir * along + side * across;
        if !flat(p.x, p.z) || !clear(p, avoid) {
            continue;
        }
        let f = site(v3(p.x, ground(p, 6.0), p.z) - origin, lane + PI / 2.0 + (rnd.next() - 0.5) * 0.2);
        if placed == 6 {
            church(&mut m, &f);
        } else {
            house(&mut m, &f, rnd);
        }
        placed += 1;
    }
    for _ in 0..3 {
        let a = rnd.next() * TAU;
        let p = origin + v3(a.cos(), 0.0, a.sin()) * (260.0 + rnd.next() * 120.0);
        if flat(p.x, p.z) && clear(p, avoid) {
            barn(&mut m, &site(v3(p.x, ground(p, 10.0), p.z) - origin, rnd.next() * PI), rnd);
        }
    }
    for _ in 0..60 {
        let a = rnd.next() * TAU;
        let p = origin + v3(a.cos(), 0.0, a.sin()) * (60.0 + rnd.next() * 380.0);
        if clear(p, avoid) && (p.x - river_x(p.z)).abs() > 45.0 {
            tree(&mut m, v3(p.x, ground(p, 1.0), p.z) - origin, 8.0 + rnd.next() * 7.0, rnd);
        }
    }
    if placed < 6 {
        return None;
    }
    m.facet();
    Some(Cluster { origin, mesh: m })
}

/// Groves on the valley floor and the lower slopes, about a kilometre apart.
fn groves(rnd: &mut Lcg, avoid: &[V3]) -> Vec<Cluster> {
    let mut out = Vec::new();
    let mut z = 16000.0;
    while z > -12500.0 {
        for side in [-1.0, 1.0] {
            let x = river_x(z) + side * (250.0 + rnd.next() * 900.0);
            let origin = v3(x, terrain::floor(x, z), z);
            if !clear(origin, avoid) || origin.y > 1300.0 {
                continue;
            }
            let mut m = Mesh::new();
            let n = 12 + (rnd.next() * 30.0) as usize;
            for _ in 0..n {
                let a = rnd.next() * TAU;
                let r = rnd.next().sqrt() * 160.0;
                let p = origin + v3(a.cos(), 0.0, a.sin()) * r;
                let g = ground(p, 1.0);
                let slope = (terrain::floor(p.x + 8.0, p.z) - g).abs() / 8.0;
                if slope < 0.35 && clear(p, avoid) && (p.x - river_x(p.z)).abs() > 45.0 {
                    tree(&mut m, v3(p.x, g, p.z) - origin, 9.0 + rnd.next() * 9.0, rnd);
                }
            }
            if m.count() > 0 {
                m.facet();
                out.push(Cluster { origin, mesh: m });
            }
        }
        z -= 900.0 + rnd.next() * 400.0;
    }
    out
}

/// A power line along the west side of the valley, a pylon every 380 m,
/// in clusters of eight.
fn power_line() -> Vec<Cluster> {
    let mut out = Vec::new();
    let mut pts = Vec::new();
    let mut z = 15500.0;
    while z > -11000.0 {
        let x = river_x(z) - 520.0;
        pts.push(v3(x, terrain::floor(x, z), z));
        z -= 380.0;
    }
    for chunk in pts.chunks(8) {
        let origin = chunk[0];
        let mut m = Mesh::new();
        for (i, &p) in chunk.iter().enumerate() {
            let next = chunk.get(i + 1).copied().unwrap_or(p + V3::Z * -380.0);
            let d = v3(next.x - p.x, 0.0, next.z - p.z).norm();
            let yaw = d.x.atan2(d.z) + PI / 2.0;
            pylon(&mut m, &site(v3(p.x, ground(p, 3.0), p.z) - origin, yaw));
        }
        m.facet();
        out.push(Cluster { origin, mesh: m });
    }
    out
}

/// Every scenery cluster, from a fixed seed.
pub fn build() -> Vec<Cluster> {
    let mut rnd = Lcg(4711);
    let avoid: Vec<V3> = targets::place().iter().map(|s| s.pos).collect();
    let mut out = vec![airbase(&mut rnd)];
    for (z, off) in [(14300.0, -300.0), (6400.0, 260.0), (4300.0, -260.0), (-200.0, 290.0), (-4200.0, -280.0), (-6900.0, 250.0), (-10200.0, -240.0)] {
        if let Some(v) = village(z, off, &mut rnd, &avoid) {
            out.push(v);
        }
    }
    out.extend(groves(&mut rnd, &avoid));
    out.extend(power_line());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenery_stands_on_the_ground_and_off_the_runway() {
        terrain::init();
        let c = build();
        assert!(c.len() > 20, "only {} clusters", c.len());
        assert!(c.iter().filter(|k| k.mesh.count() > 600).count() >= 5, "too few villages");
        for k in &c {
            for v in k.mesh.v.chunks(crate::mesh::STRIDE).step_by(7) {
                let p = k.origin + v3(v[0] as f64, v[1] as f64, v[2] as f64);
                let (u, w) = terrain::runway_local(p.x, p.z);
                assert!(!(u.abs() < terrain::RUNWAY_HALF_LENGTH + 60.0 && w.abs() < 60.0), "something on the runway at {p:?}");
            }
        }
    }
}
