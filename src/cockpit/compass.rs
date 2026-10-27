//! Aircraft-specific standby compasses with moving 3D cards and fixed lubber
//! lines: spherical KI-13 bowls and the Hornet horizontal reading window.
use super::*;
use crate::sim::Type;
pub(super) const CARD: R = r(1024.0, 1984.0, 1024.0, 64.0);
const CASE: Rgb = Rgb(0.075, 0.08, 0.075);
const RADIUS: f64 = 0.030;
pub(super) fn frame(kind: Type) -> (V3, Frame) {
    if kind == Type::Mig29 {
        return mig::standby_compass_frame();
    }
    let (c, anchor) = if kind == Type::Su27 {
        (
            v3(0.265, 1.205, -4.02),
            v3(0.287465397, 1.273293526, -3.893414542),
        )
    } else {
        (
            v3(0.365, 1.015, -3.93),
            v3(0.340926539, 0.970513553, -4.070615653),
        )
    };
    let z = (EYE - c).norm();
    let x = V3::Y.cross(z).norm();
    (
        anchor,
        Frame {
            o: c,
            x,
            y: z.cross(x),
            z,
        },
    )
}
pub(super) fn housing(m: &mut Mesh, kind: Type) {
    let (anchor, f) = frame(kind);
    let root = Frame {
        o: anchor,
        x: f.x,
        y: f.y,
        z: f.z,
    };
    m.block(&root, v3(0.014, 0.010, 0.004), CASE);
    m.ellipsoid(anchor + f.z * 0.005, v3(0.002, 0.002, 0.002), METALC, 4, 8);
    pilot::bar(
        m,
        anchor,
        f.p(0.0, 0.025, -0.018),
        0.007,
        0.007,
        V3::Z,
        CASE,
    );
    m.block(
        &f.at(0.0, 0.038, -0.020),
        v3(0.020, 0.005, 0.018),
        METALC.scale(0.5),
    );
    if kind == Type::F18 {
        // Hornet standby unit: compact case with a horizontal reading window.
        // This is separate from the round KI-13 bowl used by the Soviet jets.
        m.block(&f.at(0.0, 0.0, -0.026), v3(0.036, 0.027, 0.012), CASE);
        m.bezel(
            &f.at(0.0, 0.0, 0.015),
            &rounded_rect(0.078, 0.060, 0.012, 3),
            &rounded_rect(0.054, 0.024, 0.004, 3),
            0.030,
            0.004,
            CASE,
        );
        for x in [-0.032, 0.032] {
            for y in [-0.023, 0.023] {
                m.ellipsoid(
                    f.p(x, y, 0.017),
                    v3(0.002, 0.002, 0.0015),
                    METALC.scale(0.6),
                    4,
                    8,
                );
            }
        }
        m.capsule(
            f.p(0.0, -0.011, 0.016),
            f.p(0.0, 0.011, 0.016),
            0.0005,
            0.0005,
            Rgb(0.86, 0.78, 0.56),
            4,
        );
    } else {
        m.lathe(
            f.p(0.0, 0.0, -0.042),
            f.z,
            &[
                (0.0, 0.0),
                (0.004, 0.024),
                (0.014, 0.036),
                (0.047, 0.039),
                (0.052, 0.036),
                (0.052, 0.031),
            ],
            32,
            CASE,
        );
        m.ring(
            &f.at(0.0, 0.0, 0.011),
            0.0305,
            0.037,
            0.003,
            METALC.scale(0.35),
            32,
        );
        for s in [-1.0, 1.0] {
            m.ellipsoid(
                f.p(s * 0.028, 0.028, 0.008),
                v3(0.002, 0.002, 0.002),
                METALC,
                4,
                8,
            );
        }
        m.capsule(
            f.p(0.0, -0.017, 0.023),
            f.p(0.0, 0.018, 0.023),
            0.0005,
            0.0005,
            Rgb(0.82, 0.35, 0.12),
            4,
        );
        m.block(&f.at(0.0, 0.034, 0.004), v3(0.029, 0.002, 0.009), CASE);
    }
}
fn ball_point(f: Frame, heading: f64, pitch: f64, bank: f64, az: f64, lat: f64) -> (V3, V3) {
    let t = az - heading * DEG;
    let p = v3(t.sin() * lat.cos(), lat.sin(), t.cos() * lat.cos());
    // Limited freedom in the bowl, as in a gimballed liquid compass.
    let q = Quat::axis_angle(V3::Z, -clamp(bank, -25.0, 25.0) * DEG)
        .mul(Quat::axis_angle(V3::X, -clamp(pitch, -25.0, 25.0) * DEG));
    let p = q.rotate(p);
    let n = f.x * p.x + f.y * p.y + f.z * p.z;
    (f.o - f.z * 0.009 + n * RADIUS, n)
}
pub(super) fn dynamic(m: &mut Mesh, kind: Type, heading: f64, pitch: f64, bank: f64) {
    if kind == Type::F16 {
        return;
    }
    let (_, f) = frame(kind);
    if kind == Type::F18 {
        let r = 0.0245;
        let ys = [
            -0.015, -0.010, -0.0075, -0.005, -0.0025, 0.0, 0.0025, 0.005, 0.0075, 0.010, 0.015,
        ];
        for pair in ys.windows(2) {
            for i in 0..72 {
                let u = i as f64 / 72.0;
                let v = (i + 1) as f64 / 72.0;
                for (az, y) in [
                    (u, pair[0]),
                    (v, pair[0]),
                    (v, pair[1]),
                    (u, pair[0]),
                    (v, pair[1]),
                    (u, pair[1]),
                ] {
                    let t = az * TAU - heading * DEG;
                    let n = f.x * t.sin() + f.z * t.cos();
                    let p = f.o - f.z * 0.014 + n * r + f.y * y;
                    let ty = clamp(
                        CARD.h as f64 * 0.5 - y / (r * TAU) * CARD.w as f64,
                        0.5,
                        CARD.h as f64 - 0.5,
                    );
                    m.vert(
                        p,
                        n,
                        Rgb(1.0, 1.0, 1.0),
                        (
                            (CARD.x as f64 + az * CARD.w as f64) / AW,
                            1.0 - (CARD.y as f64 + ty) / A,
                        ),
                    );
                }
            }
        }
        return;
    }
    let lats = [
        -PI / 2.0,
        -1.2,
        -0.9,
        -0.6,
        -0.3,
        -0.2,
        -0.15,
        -0.1,
        -0.05,
        0.0,
        0.05,
        0.1,
        0.15,
        0.2,
        0.3,
        0.6,
        0.9,
        1.2,
        PI / 2.0,
    ];
    for lat_pair in lats.windows(2) {
        let (a, b) = (lat_pair[0], lat_pair[1]);
        for i in 0..72 {
            let u = i as f64 / 72.0;
            let v = (i + 1) as f64 / 72.0;
            for (az, lat) in [(u, a), (v, a), (v, b), (u, a), (v, b), (u, b)] {
                let (p, n) = ball_point(f, heading, pitch, bank, az * TAU, lat);
                let tex_y = clamp(
                    CARD.h as f64 * 0.5 - lat / TAU * CARD.w as f64,
                    0.5,
                    CARD.h as f64 - 0.5,
                );
                m.vert(
                    p,
                    n,
                    Rgb(1.0, 1.0, 1.0),
                    (
                        (CARD.x as f64 + az * CARD.w as f64) / AW,
                        1.0 - (CARD.y as f64 + tex_y) / A,
                    ),
                );
            }
        }
    }
}
pub(super) fn draw(cv: &mut Canvas) {
    let r = CARD;
    let ink = rgba(0xeb, 0xe7, 0xcb, 1.0);
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x16, 0x17, 0x14, 1.0));
    for deg in (0..=360).step_by(5) {
        let x = r.x + deg as f32 * r.w / 360.0;
        let h = if deg % 30 == 0 {
            12.0
        } else if deg % 10 == 0 {
            8.0
        } else {
            4.0
        };
        cv.line(x, r.y + 44.0, x, r.y + 44.0 + h, 1.0, ink);
        if deg % 30 == 0 {
            let label = match deg % 360 {
                0 => "N".to_owned(),
                90 => "E".to_owned(),
                180 => "S".to_owned(),
                270 => "W".to_owned(),
                n => format!("{}", n / 10),
            };
            cv.text(&label, x, r.y + 29.0, 3.0, ink, Align::Center);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_full_sphere_fits_the_dynamic_face_buffer() {
        for kind in [Type::F18, Type::Su27, Type::Mig29] {
            let mut m = Mesh::new();
            pilot::build_dynamic_twin(&mut m, &Hands::default(), kind.russian());
            dynamic(&mut m, kind, 0.0, 0.0, 0.0);
            assert!(super::super::faces(&m).count() <= super::super::DYN_FACE_CAP);
            assert!(m.count() < 60000);
        }
    }
    #[test]
    fn hornet_has_a_horizontal_window_and_its_own_curved_scale() {
        let mut case = Mesh::new();
        housing(&mut case, Type::F18);
        let (_, f) = frame(Type::F18);
        let mut card = Mesh::new();
        dynamic(&mut card, Type::F18, 90.0, 0.0, 0.0);
        for v in card.v.chunks(crate::mesh::STRIDE) {
            let p = v3(v[0] as f64, v[1] as f64, v[2] as f64) - f.o + f.z * 0.014;
            assert!((p.dot(f.x).hypot(p.dot(f.z)) - 0.0245).abs() < 0.000001);
        }
        assert!(case.count() > 500);
        let visible = card
            .v
            .chunks(crate::mesh::STRIDE)
            .max_by(|a, b| {
                let z = |v: &[f32]| v3(v[0] as f64, v[1] as f64, v[2] as f64).dot(f.z);
                z(a).total_cmp(&z(b))
            })
            .unwrap();
        let heading = (visible[9] as f64 * AW - CARD.x as f64) / CARD.w as f64 * 360.0;
        assert!((heading - 90.0).abs() < 0.001);
    }
    #[test]
    fn card_is_a_ball_and_rotates_through_all_headings() {
        for kind in [Type::Su27, Type::Mig29] {
            let (_, f) = frame(kind);
            let centre = f.o - f.z * 0.009;
            for h in [0.0, 90.0, 180.0, 270.0, 359.0] {
                let mut m = Mesh::new();
                dynamic(&mut m, kind, h, 0.0, 0.0);
                for v in m.v.chunks(crate::mesh::STRIDE) {
                    let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                    assert!(((p - centre).len() - RADIUS).abs() < 0.000001);
                }
                let (_, n) = ball_point(f, h, 0.0, 0.0, h * DEG, 0.0);
                assert!(
                    n.dot(f.z) > 0.999999,
                    "heading is not under the lubber line"
                );
                let front = (EYE - centre).norm();
                let nearest = m
                    .v
                    .chunks(crate::mesh::STRIDE)
                    .max_by(|a, b| {
                        let p = |v: &[f32]| v3(v[0] as f64, v[1] as f64, v[2] as f64).dot(front);
                        p(a).total_cmp(&p(b))
                    })
                    .unwrap();
                let visible = (nearest[9] as f64 * AW - CARD.x as f64) / CARD.w as f64 * 360.0;
                assert!(
                    wrap180(visible - h).abs() < 6.0,
                    "wrong compass heading {visible} versus {h}"
                );
            }
        }
    }
}
