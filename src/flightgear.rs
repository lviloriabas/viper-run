//! Embedded, editable FlightGear meshes. See assets/models/README.md.
//! Import is offline; the game never downloads assets or needs Blender.
use crate::{
    hornet::Airframe,
    math::*,
    mesh::{Mesh, STRIDE},
    sim::Type,
};
use std::{collections::BTreeMap, sync::OnceLock};

pub struct TextureMesh {
    pub mesh: Mesh,
    pub image: &'static [u8],
}
struct Asset {
    parts: BTreeMap<String, Mesh>,
    textures: Vec<TextureMesh>,
}
fn u32_at(data: &[u8], at: &mut usize) -> usize {
    let n = u32::from_le_bytes(data[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    n
}
fn floats(data: &[u8], at: &mut usize, n: usize) -> Mesh {
    assert_eq!(n % (STRIDE * 3), 0);
    let v = data[*at..*at + n * 4]
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    *at += n * 4;
    Mesh { v }
}
fn decode(data: &'static [u8]) -> Asset {
    let data=if data.starts_with(b"VRZ1") {
        let expected=u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        assert!(expected<=64*1024*1024,"model exceeds decoded budget");
        let raw=miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&data[8..],expected)
            .expect("corrupt embedded model");
        assert_eq!(raw.len(),expected);
        // Assets are decoded once; texture slices share this immutable allocation.
        &*Box::leak(raw.into_boxed_slice())
    }else{data};
    assert_eq!(&data[..4], b"VRM1");
    let mut at = 4;
    let n = u32_at(data, &mut at);
    let mut parts = BTreeMap::new();
    for _ in 0..n {
        let len = u32_at(data, &mut at);
        let name = std::str::from_utf8(&data[at..at + len]).unwrap().to_owned();
        at += len;
        let len = u32_at(data, &mut at);
        parts.insert(name, floats(data, &mut at, len));
    }
    let n = u32_at(data, &mut at);
    let mut textures = Vec::new();
    for _ in 0..n {
        let len = u32_at(data, &mut at);
        let bytes = u32_at(data, &mut at);
        let mesh = floats(data, &mut at, len);
        let image = &data[at..at + bytes];
        at += bytes;
        textures.push(TextureMesh { mesh, image });
    }
    assert_eq!(at, data.len());
    Asset { parts, textures }
}
fn asset(kind: Type) -> &'static Asset {
    static F18: OnceLock<Asset> = OnceLock::new();
    static SU27: OnceLock<Asset> = OnceLock::new();
    match kind.base() {
        Type::F18 => F18.get_or_init(|| decode(include_bytes!("../assets/models/f18.vrz"))),
        Type::Su27 => SU27.get_or_init(|| decode(include_bytes!("../assets/models/su27.vrz"))),
        _ => panic!("FlightGear asset requested for {kind:?}"),
    }
}
pub fn part(kind: Type, name: &str) -> Mesh {
    Mesh {
        v: asset(kind).parts[name].v.clone(),
    }
}
pub fn textures(kind: Type) -> &'static [TextureMesh] {
    &asset(kind).textures
}
pub fn build(kind: Type) -> Airframe {
    let controls=if kind==Type::F18 {
        [-1.0,1.0].into_iter().enumerate().map(|(i,side)|crate::articulation::Surface{
            mesh:part(kind,&format!("aileron{i}")),hinge:crate::articulation::Hinge{
            pivot:v3(side*3.87,0.448,3.11839),axis:V3::X,drive:crate::articulation::Drive::Aileron,side}}).collect()
    }else{Vec::new()};
    Airframe {
        controls,
        body: part(kind, "body"),
        canopy: part(kind, "canopy"),
        stabs: [part(kind, "stab0"), part(kind, "stab1")],
        flaps: [part(kind, "flap0"), part(kind, "flap1")],
        rudders: [part(kind, "rudder0"), part(kind, "rudder1")],
        brake: part(kind, "brake"),
        gear: [
            part(kind, "gear0"),
            part(kind, "gear1"),
            part(kind, "gear2"),
        ],
    }
}
/// The model's datum adjustment is shared by geometry, cockpit and hardpoints.
pub fn native(kind: Type, p: V3) -> V3 {
    if kind == Type::F18 {
        v3(-p.z * 1.035, p.y * 1.035, p.x * (17.1 / 15.39131) + 0.15)
    } else {
        v3(
            -p.z * (14.7 / 15.25),
            p.y * (14.7 / 15.25),
            p.x * (21.9 / 22.07678) - 10.95587,
        )
    }
}
pub fn cockpit_point(kind: Type, p: V3) -> V3 {
    native(kind, p) + crate::airframe::EYE - kind.eye()
}
pub fn cockpit_mesh(kind: Type, name: &str) -> Mesh {
    let mut m = part(kind, name);
    let offset = crate::airframe::EYE - kind.eye();
    for v in m.v.chunks_mut(STRIDE) {
        v[0] += offset.x as f32;
        v[1] += offset.y as f32;
        v[2] += offset.z as f32;
    }
    m
}
pub fn canopy_range(kind: Type) -> (f64, f64) {
    asset(kind).parts["canopy"]
        .v
        .chunks(STRIDE)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v[2] as f64), hi.max(v[2] as f64))
        })
}
/// Intersect the actual canopy triangles with a station plane.
pub fn canopy_section(kind: Type, z: f64) -> (f64, f64, f64) {
    let mesh = &asset(kind).parts["canopy"];
    let mut w: f64 = 0.;
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for tri in mesh.v.chunks(STRIDE * 3) {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let a = &tri[a * STRIDE..];
            let b = &tri[b * STRIDE..];
            let (za, zb) = (a[2] as f64, b[2] as f64);
            if z < za.min(zb) || z > za.max(zb) || (zb - za).abs() < 1e-9 {
                continue;
            }
            let t = (z - za) / (zb - za);
            let x = mix(a[0] as f64, b[0] as f64, t);
            let y = mix(a[1] as f64, b[1] as f64, t);
            w = w.max(x.abs());
            lo = lo.min(y);
            hi = hi.max(y);
        }
    }
    assert!(lo.is_finite(), "canopy section outside {kind:?} at {z}");
    (w, lo, hi)
}
/// Decode lossless QOI source textures at their original pixel dimensions.
pub fn image(data: &[u8]) -> (u32, u32, Vec<u8>) {
    assert_eq!(&data[..4], b"qoif");
    let w = u32::from_be_bytes(data[4..8].try_into().unwrap());
    let h = u32::from_be_bytes(data[8..12].try_into().unwrap());
    assert!(w > 0 && h > 0 && w <= 8192 && h <= 8192);
    let mut px = [0u8, 0, 0, 255];
    let mut cache = [[0u8; 4]; 64];
    let mut at = 14;
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    let mut run = 0;
    while out.len() < w as usize * h as usize * 4 {
        if run > 0 {
            run -= 1;
        } else {
            let b = data[at];
            at += 1;
            match b {
                0xfe => {
                    px[..3].copy_from_slice(&data[at..at + 3]);
                    at += 3;
                }
                0xff => {
                    px.copy_from_slice(&data[at..at + 4]);
                    at += 4;
                }
                _ => match b >> 6 {
                    0 => px = cache[b as usize],
                    1 => {
                        for (k, shift) in [4, 2, 0].into_iter().enumerate() {
                            px[k] = px[k].wrapping_add(((b >> shift) & 3).wrapping_sub(2));
                        }
                    }
                    2 => {
                        let c = data[at];
                        at += 1;
                        let g = (b & 63).wrapping_sub(32);
                        px[0] = px[0].wrapping_add(g).wrapping_add((c >> 4).wrapping_sub(8));
                        px[1] = px[1].wrapping_add(g);
                        px[2] = px[2].wrapping_add(g).wrapping_add((c & 15).wrapping_sub(8));
                    }
                    _ => run = (b & 63) as usize,
                },
            }
            let hash = (px[0] as usize * 3
                + px[1] as usize * 5
                + px[2] as usize * 7
                + px[3] as usize * 11)
                % 64;
            cache[hash] = px;
        }
        out.extend_from_slice(&px);
    }
    assert_eq!(out.len(), w as usize * h as usize * 4);
    (w, h, out)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub fn extent(m: &Mesh) -> (V3, V3) {
        let mut lo = v3(1e9, 1e9, 1e9);
        let mut hi = -lo;
        for v in m.v.chunks(STRIDE) {
            let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
            lo = v3(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
            hi = v3(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
        }
        (lo, hi)
    }
    #[test]
    fn embedded_geometry_is_finite_and_preserves_source_part_boundaries() {
        for kind in [Type::F18, Type::Su27] {
            for (name, m) in &asset(kind).parts {
                assert_eq!(m.count() % 3, 0);
                assert!(m.v.iter().all(|v| v.is_finite()), "{kind:?} {name}");
                for v in m.v.chunks(STRIDE) {
                    let n = v3(v[3] as f64, v[4] as f64, v[5] as f64);
                    assert!((n.len() - 1.).abs() < 1e-5, "{kind:?} {name} normal");
                    if name.ends_with('0') && name != "gear0" {
                        assert!(v[0] < 0.05, "left part on right: {kind:?} {name}");
                    }
                    if name.ends_with('1') && !name.starts_with("gear") {
                        assert!(v[0] > -0.05, "right part on left: {kind:?} {name}");
                    }
                }
            }
        }
    }
    #[test]
    fn production_envelopes_match_legacy_hornet_and_baseline_flanker() {
        for (kind, span, length) in [(Type::F18, 11.43, 17.1), (Type::Su27, 14.7, 21.9)] {
            let mut m = part(kind, "body");
            for name in ["stab0", "stab1"] {
                m.v.extend(part(kind, name).v);
            }
            let (lo, hi) = extent(&m);
            assert!(
                (hi.x - lo.x - span).abs() < 0.04,
                "{kind:?} span {}",
                hi.x - lo.x
            );
            assert!(
                (hi.z - lo.z - length).abs() < 0.04,
                "{kind:?} length {}",
                hi.z - lo.z
            );
            let (_, glass) = extent(&part(kind, "canopy"));
            assert!(
                glass.y < if kind == Type::F18 { 1.40 } else { 1.67 },
                "inflated canopy"
            );
            assert!(glass.y > kind.eye().y + 0.20, "no helmet clearance");
        }
    }
    #[test]
    fn imported_tyres_meet_the_simulated_contact_points() {
        for kind in [Type::F18, Type::Su27] {
            for i in 0..3 {
                let m = part(kind, &format!("gear{i}"));
                let (lo, _) = extent(&m);
                assert!(
                    (lo.y - kind.wheels()[i].local.y).abs() < 1e-5,
                    "{kind:?} wheel {i}"
                );
                let p = kind.wheels()[i].local;
                assert!(
                    m.v.chunks(STRIDE)
                        .any(|v| (v3(v[0] as f64, v[1] as f64, v[2] as f64) - p).len() < 0.18),
                    "{kind:?} missing tyre at {p:?}"
                );
            }
        }
    }
    #[test]
    fn native_missile_noses_match_the_reference_rails() {
        use crate::combat::{Config, Loadout, MissionKind, Store};
        let su = Loadout::configure(&Config::for_jet(MissionKind::Dogfight, Type::Su27));
        for st in &su.stations {
            let p = st.position(Type::Su27);
            if st.tip {
                let expected = native(
                    Type::Su27,
                    v3(15.40 - 1.45, 0.147, -st.at.x.signum() * 7.555),
                );
                assert!(
                    (p - expected).len() < 0.05,
                    "R-73 nose {p:?}, expected {expected:?}"
                );
            } else if st.at.x.abs() > 2.0 && st.at.x.abs() < 2.6 {
                assert_eq!(st.store, Store::R27r);
                let expected = native(
                    Type::Su27,
                    v3(13.80 - 2.19896, -1.30, -st.at.x.signum() * 1.22),
                );
                assert!(
                    (p - expected).len() < 0.05,
                    "R-27 nose {p:?}, expected {expected:?}"
                );
            }
        }
        let f18 = Loadout::configure(&Config::for_jet(MissionKind::Dogfight, Type::F18));
        for st in &f18.stations {
            let p = st.position(Type::F18);
            if st.at.x.abs() < 3.0 {
                assert!(
                    (p.x.abs() - 0.90).abs() < 1e-8 && p.y < -0.70,
                    "cheek missile intersects the inlet"
                );
            }
            if st.tip {
                assert!((p.x.abs() - 5.78).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn native_bomb_racks_keep_distinct_release_positions() {
        use crate::combat::{Config, Loadout, MissionKind, Store};
        for kind in [Type::F18, Type::Su27] {
            let mut c = Config::for_jet(MissionKind::Strike, kind);
            c.outer = Some(if kind == Type::F18 {
                Store::Mk82Ter
            } else {
                Store::Fab250Twin
            });
            let l = Loadout::configure(&c);
            let rack: Vec<_> = l
                .stations
                .iter()
                .filter(|s| s.at.x.abs() > 3.8 && !s.tip)
                .collect();
            assert!(rack.len() >= 4);
            for (i, a) in rack.iter().enumerate() {
                for b in &rack[i + 1..] {
                    assert!(
                        (a.position(kind) - b.position(kind)).len() > 0.20,
                        "overlapping stores on {kind:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn baseline_flanker_has_one_nose_wheel_and_retracts_it_forward() {
        let m = part(Type::Su27, "gear0");
        let floor = Type::Su27.wheels()[0].local.y;
        // Check the whole tyre, rather than its narrower curved contact band.
        let tyre = part(Type::Su27, "nose_tyre");
        let (lo, hi) = extent(&tyre);
        assert!((hi.x - lo.x - 0.26).abs() < 1e-5 && (hi.y - lo.y - 0.68).abs() < 1e-5);
        assert!((lo.y - floor).abs() < 1e-5);
        let lower: Vec<_> = tyre.v.chunks(STRIDE).collect();
        assert!(
            lower.iter().all(|v| v[0].abs() < 0.14),
            "naval twin wheel on baseline Su-27"
        );
        assert!(lower.iter().any(|v| v[0] < -0.10) && lower.iter().any(|v| v[0] > 0.10));
        for v in lower {
            assert!(
                m.v.chunks(STRIDE).any(|p| p[..3] == v[..3]),
                "checked tyre missing from rendered leg"
            );
        }
        let (pivot, axis, swing) = crate::su27::GEAR_PIVOTS[0];
        let down = Type::Su27.wheels()[0].local + V3::Y * 0.34;
        let up = pivot + Quat::axis_angle(axis, swing).rotate(down - pivot);
        assert!(up.z < down.z - 1.0 && up.y > down.y + 1.0);
    }
    #[test]
    fn source_textures_decode_without_resampling() {
        let t = textures(Type::Su27);
        assert!(t.len() > 20);
        for t in t {
            let (w, h, p) = image(t.image);
            assert_eq!(p.len(), w as usize * h as usize * 4);
            assert!(w >= 64 && h >= 64);
        }
        assert!(t.iter().any(|t| image(t.image).0 == 2048));
    }
}
