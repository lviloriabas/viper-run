//! Minimal PNG writer (stored deflate blocks) for `--shot` screenshots.

fn crc32(data: &[u8], mut c: u32) -> u32 {
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let c = crc32(data, crc32(kind, 0xFFFF_FFFF)) ^ 0xFFFF_FFFF;
    out.extend_from_slice(&c.to_be_bytes());
}

/// Writes RGBA8 pixels as an RGB PNG. `flip` reverses rows (GL reads bottom-up).
pub fn write(path: &str, w: u32, h: u32, rgba: &[u8], flip: bool) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((w * 3 + 1) as usize * h as usize);
    for y in 0..h {
        let row = if flip { h - 1 - y } else { y };
        raw.push(0);
        for x in 0..w {
            let i = ((row * w + x) * 4) as usize;
            raw.extend_from_slice(&rgba[i..i + 3]);
        }
    }
    let mut z = vec![0x78, 0x01];
    let (mut a, mut b) = (1u32, 0u32);
    for &v in &raw {
        a = (a + v as u32) % 65521;
        b = (b + a) % 65521;
    }
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, blk) in blocks.iter().enumerate() {
        z.push(if i + 1 == blocks.len() { 1 } else { 0 });
        let n = blk.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(blk);
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    std::fs::write(path, out)
}
