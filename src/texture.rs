//! Procedural pixel-art textures: small tiles in a few shades each, sampled
//! with nearest filtering for the chunky look of mid-90s flight games.
use crate::math::Lcg;

pub const TILE: usize = 64;

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Tileable value noise with a period of `cells` across the tile.
fn noise(x: usize, y: usize, cells: usize, seed: u32) -> f32 {
    let s = TILE as f32 / cells as f32;
    let (fx, fy) = (x as f32 / s, y as f32 / s);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let c = cells as i32;
    let at = |a: i32, b: i32| hash(a.rem_euclid(c), b.rem_euclid(c), seed);
    let top = at(ix, iy) + (at(ix + 1, iy) - at(ix, iy)) * tx;
    let bot = at(ix, iy + 1) + (at(ix + 1, iy + 1) - at(ix, iy + 1)) * tx;
    top + (bot - top) * ty
}

fn fbm(x: usize, y: usize, seed: u32) -> f32 {
    0.5 * noise(x, y, 4, seed) + 0.3 * noise(x, y, 8, seed + 1) + 0.2 * noise(x, y, 16, seed + 2)
}

/// Picks one of the palette shades from a 0..1 value; no blending between them.
fn shade(palette: &[[u8; 3]], v: f32) -> [u8; 3] {
    let i = ((v.clamp(0.0, 0.999)) * palette.len() as f32) as usize;
    palette[i]
}

fn tile(f: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(TILE * TILE * 4);
    for y in 0..TILE {
        for x in 0..TILE {
            let [r, g, b] = f(x, y);
            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
    out
}

/// Terrain materials: grass, dry scrub, rock, snow, river sand and crop fields.
pub fn terrain() -> Vec<Vec<u8>> {
    let grass = tile(|x, y| {
        let v = fbm(x, y, 11) * 0.8 + hash(x as i32, y as i32, 5) * 0.35;
        shade(&[[58, 92, 34], [72, 108, 40], [86, 122, 46], [104, 134, 52]], v - 0.1)
    });
    let scrub = tile(|x, y| {
        let v = fbm(x, y, 21) * 0.7 + hash(x as i32, y as i32, 6) * 0.4;
        let bush = noise(x, y, 16, 23) > 0.72 && hash(x as i32, y as i32, 7) > 0.3;
        if bush {
            [62, 84, 38]
        } else {
            shade(&[[122, 108, 70], [138, 122, 78], [152, 136, 88], [168, 150, 100]], v - 0.1)
        }
    });
    let rock = tile(|x, y| {
        // Strata: bands that wander a little, broken by cracks.
        let band = ((y as f32 + noise(x, y, 4, 31) * 9.0) / 16.0).fract();
        let v = fbm(x, y, 33) * 0.6 + band * 0.35 + hash(x as i32, y as i32, 8) * 0.2;
        if band < 0.08 && hash(x as i32 / 2, y as i32, 9) > 0.5 {
            [70, 58, 46]
        } else {
            shade(&[[96, 80, 62], [114, 96, 74], [132, 112, 86], [150, 128, 100]], v - 0.05)
        }
    });
    let snow = tile(|x, y| {
        let v = fbm(x, y, 41) + hash(x as i32, y as i32, 10) * 0.15;
        shade(&[[196, 206, 218], [214, 222, 232], [230, 236, 242], [244, 246, 250]], v)
    });
    let sand = tile(|x, y| {
        let v = fbm(x, y, 51) * 0.7 + hash(x as i32, y as i32, 11) * 0.4;
        shade(&[[150, 132, 96], [168, 150, 110], [184, 166, 124], [198, 182, 140]], v - 0.1)
    });
    // Crop rows; the shader tints each field and turns the rows across fields.
    let crops = tile(|x, y| {
        let row = (x / 3) % 2 == 0;
        let v = hash(x as i32, y as i32, 12) * 0.5 + if row { 0.45 } else { 0.0 };
        shade(&[[150, 150, 150], [180, 180, 180], [205, 205, 205], [230, 230, 230]], v)
    });
    vec![grass, scrub, rock, snow, sand, crops]
}

/// Soft-edged cloud puff alpha for the dithered cloud billboards.
pub fn cloud() -> Vec<u8> {
    let (w, h) = (128usize, 64usize);
    let mut a = vec![0f32; w * h];
    let mut rnd = Lcg(9127);
    for _ in 0..22 {
        let x = 18.0 + rnd.next() * 92.0;
        let spread = 1.0 - (x - 64.0).abs() / 75.0;
        let y = 36.0 + (rnd.next() - 0.5) * 16.0 * spread;
        let r = 9.0 + rnd.next() * 15.0 * (0.6 + spread);
        for py in 0..h {
            for px in 0..w {
                let d = ((px as f64 - x).powi(2) + ((py as f64 - y) * 1.3).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let o = &mut a[py * w + px];
                    *o = o.max((1.0 - d * d) as f32);
                }
            }
        }
    }
    a.iter()
        .enumerate()
        .flat_map(|(i, &v)| {
            // Darker undersides: the lower half of each puff is a grey-blue shade.
            let y = i / w;
            let s = if y > 38 { 214 } else { 255 };
            [s, s, (s as u32 + 6).min(255) as u8, (v.clamp(0.0, 1.0) * 255.0) as u8]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_are_full_size_and_wrap() {
        for t in terrain() {
            assert_eq!(t.len(), TILE * TILE * 4);
        }
        // Value noise repeats across the tile edge.
        assert_eq!(noise(0, 5, 8, 3), noise(TILE, 5, 8, 3));
    }
}
