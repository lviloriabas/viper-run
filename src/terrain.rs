//! Procedural gorge terrain. The analytic height function is evaluated once at
//! startup into a 32 m grid; rendering and collision both use that grid, so
//! what you see is exactly what you can hit.
use crate::math::{mix, smoothstep};
use std::sync::OnceLock;

#[inline]
fn fract(x: f64) -> f64 {
    x - x.floor()
}

#[inline]
fn hash21(x: f64, y: f64) -> f64 {
    let mut px = fract(x * 123.34);
    let mut py = fract(y * 456.21);
    let d = px * px + py * py + 45.32 * (px + py);
    px += d;
    py += d;
    fract(px * py)
}

fn vnoise(x: f64, y: f64) -> f64 {
    let ix = x.floor();
    let iy = y.floor();
    let mut fx = x - ix;
    let mut fy = y - iy;
    fx = fx * fx * (3.0 - 2.0 * fx);
    fy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash21(ix, iy);
    let b = hash21(ix + 1.0, iy);
    let c = hash21(ix, iy + 1.0);
    let d = hash21(ix + 1.0, iy + 1.0);
    mix(mix(a, b, fx), mix(c, d, fx), fy)
}

fn fbm(mut x: f64, mut y: f64) -> f64 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    for _ in 0..5 {
        sum += amp * vnoise(x, y);
        x = x * 2.03 + 17.3;
        y = y * 2.03 + 9.2;
        amp *= 0.5;
    }
    sum
}

pub fn river_x(z: f64) -> f64 {
    1500.0 * (z / 5200.0).sin() + 420.0 * (z / 1900.0 + 0.7).sin()
}

/// Analytic terrain height in metres above mean sea level.
pub fn terrain_at(x: f64, z: f64) -> f64 {
    let wx = (fbm(x / 17000.0 + 4.2, z / 17000.0 + 8.1) - 0.5) * 1800.0;
    let wz = (fbm(x / 14000.0 - 3.3, z / 14000.0 + 5.8) - 0.5) * 1400.0;
    let xx = x + wx;
    let zz = z + wz;
    let base = fbm(xx / 13000.0 + 2.0, zz / 13000.0 + 9.4);
    let ridge_n = fbm(xx / 4600.0 + 1.7, zz / 4600.0 - 3.2);
    let ridge = 1.0 - (ridge_n * 2.0 - 1.0).abs();
    let mut h = -380.0 + base * 1050.0 + ridge.powf(2.3) * 2550.0;
    h += (fbm(x / 1000.0 + 8.0, z / 1000.0 + 3.0) - 0.5) * 110.0;
    h += (fbm(x / 300.0 + 3.0, z / 300.0 + 7.0) - 0.5) * 22.0;
    let coast = smoothstep(-13500.0, -9000.0, z);
    let sea = -320.0 + (fbm(x / 5000.0, z / 5000.0) - 0.5) * 80.0;
    h = mix(sea, h, coast);
    let d = (x - river_x(z)).abs();
    let river_n = fbm(x / 1300.0 + 5.5, z / 1300.0 + 1.7);
    let floor = 300.0 + river_n * 130.0 + 0.012 * (z + 3000.0);
    let wall = smoothstep(420.0, 1650.0, d);
    mix(floor, h, wall)
}

// ----------------------------------------------------------------- grid
pub const CELL: f64 = 32.0;
pub const CHUNK_CELLS: usize = 128;
pub const CHUNKS: usize = 18;
pub const N: usize = CHUNKS * CHUNK_CELLS + 1;
pub const CHUNK_M: f64 = CHUNK_CELLS as f64 * CELL;
pub const X0: f64 = -(CHUNKS as f64) * CHUNK_M / 2.0;
pub const Z0: f64 = X0 + 1024.0;
/// Grid stride, in cells, of the coarsest-allowed detail outside the corridor.
pub const COARSE_STRIDE: usize = 2;

/// Chunks covering the flyable gorge corridor carry full 32 m detail.
pub fn is_fine_chunk(cx: usize, cz: usize) -> bool {
    (7..=10).contains(&cx) && (5..=12).contains(&cz)
}

pub struct Heightmap {
    pub h: Vec<f32>,
    pub chunk_min: Vec<f32>,
    pub chunk_max: Vec<f32>,
}

static TERRAIN: OnceLock<Heightmap> = OnceLock::new();

pub fn heightmap() -> Option<&'static Heightmap> {
    TERRAIN.get()
}

/// Builds the grid on all cores. Only samples that some mesh uses are computed.
pub fn init() -> &'static Heightmap {
    TERRAIN.get_or_init(|| {
        let mut h = vec![f32::NAN; N * N];
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 32);
        let rows_per = N.div_ceil(threads);
        std::thread::scope(|s| {
            for (t, slice) in h.chunks_mut(rows_per * N).enumerate() {
                s.spawn(move || {
                    let row0 = t * rows_per;
                    for (r, row) in slice.chunks_mut(N).enumerate() {
                        let j = row0 + r;
                        let z = Z0 + j as f64 * CELL;
                        let cz = (j / CHUNK_CELLS).min(CHUNKS - 1);
                        let cz_edge = j % CHUNK_CELLS == 0 && j > 0;
                        for (i, out) in row.iter_mut().enumerate() {
                            let cx = (i / CHUNK_CELLS).min(CHUNKS - 1);
                            let cx_edge = i % CHUNK_CELLS == 0 && i > 0;
                            let mut fine = is_fine_chunk(cx, cz);
                            // Samples on a chunk edge also belong to the chunk before it.
                            if cx_edge && is_fine_chunk(cx - 1, cz) {
                                fine = true;
                            }
                            if cz_edge && is_fine_chunk(cx, cz - 1) {
                                fine = true;
                            }
                            if cx_edge && cz_edge && is_fine_chunk(cx - 1, cz - 1) {
                                fine = true;
                            }
                            let coarse = i % COARSE_STRIDE == 0 && j % COARSE_STRIDE == 0;
                            if fine || coarse {
                                *out = terrain_at(X0 + i as f64 * CELL, z) as f32;
                            }
                        }
                    }
                });
            }
        });
        let mut chunk_min = vec![0f32; CHUNKS * CHUNKS];
        let mut chunk_max = vec![0f32; CHUNKS * CHUNKS];
        for cz in 0..CHUNKS {
            for cx in 0..CHUNKS {
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for j in (cz * CHUNK_CELLS..=(cz + 1) * CHUNK_CELLS).step_by(COARSE_STRIDE) {
                    for i in (cx * CHUNK_CELLS..=(cx + 1) * CHUNK_CELLS).step_by(COARSE_STRIDE) {
                        let v = h[j * N + i];
                        lo = lo.min(v);
                        hi = hi.max(v);
                    }
                }
                chunk_min[cz * CHUNKS + cx] = lo;
                chunk_max[cz * CHUNKS + cx] = hi;
            }
        }
        Heightmap { h, chunk_min, chunk_max }
    })
}

impl Heightmap {
    #[inline]
    pub fn sample(&self, i: usize, j: usize) -> f32 {
        self.h[j * N + i]
    }

    /// Height of the rendered surface: the same triangles the finest mesh draws.
    pub fn surface(&self, x: f64, z: f64) -> f64 {
        let gx = (x - X0) / CELL;
        let gz = (z - Z0) / CELL;
        let max = (N - 1) as f64;
        if !(0.0..max).contains(&gx) || !(0.0..max).contains(&gz) {
            return terrain_at(x, z);
        }
        let cx = (gx as usize / CHUNK_CELLS).min(CHUNKS - 1);
        let cz = (gz as usize / CHUNK_CELLS).min(CHUNKS - 1);
        let s = if is_fine_chunk(cx, cz) { 1 } else { COARSE_STRIDE };
        let sf = s as f64;
        let i = (gx / sf).floor() as usize * s;
        let j = (gz / sf).floor() as usize * s;
        let fx = (gx - i as f64) / sf;
        let fz = (gz - j as f64) / sf;
        let a = self.sample(i, j) as f64;
        let b = self.sample(i + s, j) as f64;
        let c = self.sample(i, j + s) as f64;
        let d = self.sample(i + s, j + s) as f64;
        // Same diagonal as the index buffer: triangles (a, c, b) and (b, c, d).
        if fx + fz <= 1.0 {
            a + (b - a) * fx + (c - a) * fz
        } else {
            d + (c - d) * (1.0 - fx) + (b - d) * (1.0 - fz)
        }
    }

    /// A grid sample that falls back to the coarse grid outside the corridor,
    /// where only every other sample is computed.
    fn sample_any(&self, i: isize, j: isize) -> f32 {
        let (i, j) = (i.clamp(0, N as isize - 1) as usize, j.clamp(0, N as isize - 1) as usize);
        let v = self.sample(i, j);
        if v.is_nan() {
            self.sample(i & !1, j & !1)
        } else {
            v
        }
    }

    /// Vertex positions and smooth normals for one chunk at a stride, followed
    /// by its skirt ring: x, y, z, nx, ny, nz per vertex.
    pub fn chunk_vertices(&self, cx: usize, cz: usize, stride: usize) -> Vec<f32> {
        let n = CHUNK_CELLS / stride;
        let mut out = Vec::with_capacity(((n + 1) * (n + 1) + 4 * n) * 6);
        let i0 = cx * CHUNK_CELLS;
        let j0 = cz * CHUNK_CELLS;
        let s = stride as isize;
        let span = (2 * stride) as f64 * CELL;
        for b in 0..=n {
            for a in 0..=n {
                let i = i0 + a * stride;
                let j = j0 + b * stride;
                let (ii, jj) = (i as isize, j as isize);
                let dx = (self.sample_any(ii + s, jj) - self.sample_any(ii - s, jj)) as f64 / span;
                let dz = (self.sample_any(ii, jj + s) - self.sample_any(ii, jj - s)) as f64 / span;
                let l = (dx * dx + 1.0 + dz * dz).sqrt();
                out.extend_from_slice(&[
                    (X0 + i as f64 * CELL) as f32,
                    self.sample(i, j),
                    (Z0 + j as f64 * CELL) as f32,
                    (-dx / l) as f32,
                    (1.0 / l) as f32,
                    (-dz / l) as f32,
                ]);
            }
        }
        let depth = 30.0 + stride as f32 * 18.0;
        for k in perimeter(n) {
            let p = k as usize * 6;
            let v: [f32; 6] = out[p..p + 6].try_into().unwrap();
            out.extend_from_slice(&[v[0], v[1] - depth, v[2], v[3], v[4], v[5]]);
        }
        out
    }
}

fn perimeter(n: usize) -> Vec<u16> {
    let w = n + 1;
    let mut ring = Vec::with_capacity(4 * n);
    for a in 0..n {
        ring.push(a as u16);
    }
    for b in 0..n {
        ring.push((b * w + n) as u16);
    }
    for a in (1..=n).rev() {
        ring.push((n * w + a) as u16);
    }
    for b in (1..=n).rev() {
        ring.push((b * w) as u16);
    }
    ring
}

/// Shared index buffer for every chunk at one stride.
pub fn chunk_indices(stride: usize) -> Vec<u16> {
    let n = CHUNK_CELLS / stride;
    let w = n + 1;
    let mut idx = Vec::with_capacity(n * n * 6 + 4 * n * 6);
    for b in 0..n {
        for a in 0..n {
            let ia = (b * w + a) as u16;
            let ib = ia + 1;
            let ic = ia + w as u16;
            let id = ic + 1;
            idx.extend_from_slice(&[ia, ic, ib, ib, ic, id]);
        }
    }
    let ring = perimeter(n);
    let base = (w * w) as u16;
    let m = ring.len();
    for k in 0..m {
        let k2 = (k + 1) % m;
        let (t0, t1) = (ring[k], ring[k2]);
        let (s0, s1) = (base + k as u16, base + k2 as u16);
        idx.extend_from_slice(&[t0, s0, t1, t1, s0, s1]);
    }
    idx
}

/// Ground under a point, clamped to the sea surface.
pub fn floor(x: f64, z: f64) -> f64 {
    let h = match heightmap() {
        Some(m) => m.surface(x, z),
        None => terrain_at(x, z),
    };
    h.max(0.0)
}

/// Raw surface height, negative over the sea.
pub fn height(x: f64, z: f64) -> f64 {
    match heightmap() {
        Some(m) => m.surface(x, z),
        None => terrain_at(x, z),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_surface_matches_analytic_height_at_samples() {
        let m = init();
        for &(x, z) in &[(0.0, 0.0), (river_x(4200.0), 4200.0), (-2000.0, -6000.0)] {
            let gx = ((x - X0) / CELL).round();
            let gz = ((z - Z0) / CELL).round();
            let (sx, sz) = (X0 + gx * CELL, Z0 + gz * CELL);
            assert!((m.surface(sx, sz) - terrain_at(sx, sz)).abs() < 0.01);
        }
    }

    #[test]
    fn grid_interpolation_stays_close_to_the_analytic_surface() {
        let m = init();
        let mut worst: f64 = 0.0;
        for k in 0..2000 {
            let z = -9000.0 + k as f64 * 8.7;
            let x = river_x(z) + ((k * 37) % 900) as f64 - 450.0;
            worst = worst.max((m.surface(x, z) - terrain_at(x, z)).abs());
        }
        assert!(worst < 6.0, "grid error {worst}");
    }
}
