//! Windows capture compatibility exports and the procedural icon (src/icon.rs).
//!
//! The .ico is compiled with `windres` (mingw) or `zig rc`, whichever is
//! found; without either the exe simply keeps the default icon. A preview
//! sheet is also written to $OUT_DIR/icon-preview.png.

#[allow(dead_code)]
#[path = "src/icon.rs"]
mod icon;

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    let out = std::env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);

    let sheet = preview();
    std::fs::write(out.join("icon-preview.png"), png(sheet.0, sheet.1, &sheet.2)).unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Keep the capture compatibility marker in the executable's PE export
    // table, including stripped/LTO builds. It is data, not a callable API.
    let exports = out.join("capture-compat.def");
    std::fs::write(&exports, "EXPORTS\n    RTSSHooksCompatibility DATA\n").unwrap();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/DEF:{}", exports.display());
    } else {
        println!("cargo:rustc-link-arg-bins={}", exports.display());
    }
    std::fs::write(out.join("icon.ico"), ico(&[16, 24, 32, 48, 64, 256])).unwrap();
    std::fs::write(out.join("icon.rc"), "1 ICON \"icon.ico\"\n").unwrap();

    let ok = |c: &mut Command| c.current_dir(out).status().is_ok_and(|s| s.success());
    let linked = if ["x86_64-w64-mingw32-windres", "windres"]
        .iter()
        .any(|w| ok(Command::new(w).args(["icon.rc", "-O", "coff", "-o", "icon.o"])))
    {
        Some("icon.o")
    } else if ok(Command::new("zig").args(["rc", "/fo", "icon.res", "icon.rc"])) {
        Some("icon.res")
    } else {
        None
    };
    match linked {
        Some(f) => println!("cargo:rustc-link-arg-bins={}", out.join(f).display()),
        None => println!("cargo:warning=no resource compiler (windres or zig) found; exe has no icon"),
    }
}

/// 16, 32 and 64 px icons at 8x/4x/2x next to the 256 px one.
fn preview() -> (usize, usize, Vec<u8>) {
    let (w, h) = (128 * 3 + 256 + 40, 256 + 16);
    let mut px = vec![40u8; w * h * 4];
    let mut x0 = 8;
    for (size, zoom) in [(16, 8), (32, 4), (64, 2), (256, 1)] {
        let img = icon::render(size);
        for y in 0..size * zoom {
            for x in 0..size * zoom {
                let s = ((y / zoom) * size + x / zoom) * 4;
                if img[s + 3] > 0 {
                    let d = ((y + 8) * w + x0 + x) * 4;
                    px[d..d + 4].copy_from_slice(&img[s..s + 4]);
                }
            }
        }
        x0 += size * zoom + 8;
    }
    (w, h, px)
}

/// ICO with 32-bit BMP entries, plus a PNG entry for 256 px.
fn ico(sizes: &[usize]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes
        .iter()
        .map(|&s| if s >= 256 { png(s, s, &icon::render(s)) } else { bmp(s, &icon::render(s)) })
        .collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len();
    for (&s, img) in sizes.iter().zip(&images) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(img.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += img.len();
    }
    for img in images {
        out.extend_from_slice(&img);
    }
    out
}

fn bmp(s: usize, rgba: &[u8]) -> Vec<u8> {
    let mask_row = s.div_ceil(32) * 4;
    let mut out = Vec::new();
    for v in [40u32, s as u32, 2 * s as u32] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&[1, 0, 32, 0]);
    out.extend_from_slice(&[0; 24]);
    for y in (0..s).rev() {
        for p in rgba[y * s * 4..(y + 1) * s * 4].chunks(4) {
            out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    out.resize(out.len() + mask_row * s, 0);
    out
}

fn crc32(data: &[u8], mut c: u32) -> u32 {
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    c
}

/// RGBA PNG compressed with fixed-Huffman deflate. Matches are only tried
/// at the previous pixel and the row above, which suits upscaled pixel art.
fn png(w: usize, h: usize, rgba: &[u8]) -> Vec<u8> {
    let stride = w * 4 + 1;
    let mut raw = Vec::with_capacity(stride * h);
    for row in rgba.chunks(w * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut bits = Bits::default();
    bits.put(0b011, 3); // final block, fixed Huffman
    let mut i = 0;
    while i < raw.len() {
        let mut best = (0, 0);
        for dist in [4, stride] {
            if dist <= i {
                let len = (0..258.min(raw.len() - i)).take_while(|&k| raw[i + k] == raw[i + k - dist]).count();
                if len > best.0 {
                    best = (len, dist);
                }
            }
        }
        if best.0 >= 3 {
            bits.length(best.0);
            bits.distance(best.1);
            i += best.0;
        } else {
            bits.literal(raw[i] as u16);
            i += 1;
        }
    }
    bits.literal(256);
    let mut z = vec![0x78, 0x01];
    z.extend_from_slice(&bits.finish());
    let (mut a, mut b) = (1u32, 0u32);
    for &v in &raw {
        a = (a + v as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    for (kind, data) in [(b"IHDR", &ihdr[..]), (b"IDAT", &z[..]), (b"IEND", &[][..])] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let c = crc32(data, crc32(kind, 0xFFFF_FFFF)) ^ 0xFFFF_FFFF;
        out.extend_from_slice(&c.to_be_bytes());
    }
    out
}

#[derive(Default)]
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= v << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// Huffman codes are sent most significant bit first.
    fn code(&mut self, code: u32, n: u32) {
        self.put(code.reverse_bits() >> (32 - n), n);
    }
    fn literal(&mut self, v: u16) {
        let v = v as u32;
        match v {
            0..=143 => self.code(0x30 + v, 8),
            144..=255 => self.code(0x190 + v - 144, 9),
            256..=279 => self.code(v - 256, 7),
            _ => self.code(0xC0 + v - 280, 8),
        }
    }
    fn length(&mut self, len: usize) {
        const BASE: [usize; 29] =
            [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
        let i = BASE.iter().rposition(|&b| b <= len).unwrap();
        let extra = if i == 28 { 0 } else { (i.saturating_sub(4) / 4) as u32 };
        self.literal(257 + i as u16);
        self.put((len - BASE[i]) as u32, extra);
    }
    fn distance(&mut self, dist: usize) {
        let mut base = 1;
        for code in 0..30u32 {
            let extra = code.saturating_sub(2) / 2;
            if dist < base + (1 << extra) {
                self.code(code, 5);
                self.put((dist - base) as u32, extra);
                return;
            }
            base += 1 << extra;
        }
        unreachable!("distance {dist} out of range");
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}
