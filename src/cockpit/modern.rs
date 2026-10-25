//! Reference-led single-seat F/A-18C and Su-27SK cockpits.
//! Geometry is in the shared design-eye space. All instrument faces use
//! their board's orthonormal frame; consoles clear the common pilot envelope.
use super::*;
use crate::sim::Type;
const DARK: Rgb = Rgb(0.075, 0.085, 0.09);
const IFEI: R = r(0.0, 1872.0, 384.0, 160.0);
const UFC: R = r(384.0, 1872.0, 320.0, 176.0);
const HUD_CONTROL: R = r(704.0, 1872.0, 320.0, 96.0);

/// The outer cheeks toe toward the seat, while the central board stays flat.
fn instrument(kind: Type, x: f64, y: f64) -> Frame {
    let tilt = if kind == Type::Su27 { 12.0 } else { 8.5 } * DEG;
    let p = Frame {
        o: v3(0.0, 0.64, -4.28),
        x: V3::X,
        y: v3(0.0, tilt.cos(), -tilt.sin()),
        z: v3(0.0, tilt.sin(), tilt.cos()),
    };
    let cant = if kind == Type::Su27 { 18.0 } else { 10.0 } * DEG;
    let hinge = if kind == Type::Su27 { 0.27 } else { 0.46 };
    let side = x.signum();
    if x.abs() <= hinge {
        return p.at(x, (y - 0.64) / tilt.cos(), 0.0);
    }
    let across = p.x * cant.cos() + p.z * (side * cant.sin());
    Frame {
        o: p.p(side * hinge, (y - 0.64) / tilt.cos(), 0.0) + across * (x - side * hinge),
        x: across,
        y: p.y,
        z: across.cross(p.y),
    }
}
#[derive(Clone, Copy)]
enum Shape {
    Dial,
    Display(bool),
    Label,
}
#[derive(Clone, Copy)]
struct Instrument {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rect: R,
    shape: Shape,
}
fn dial(x: f64, y: f64, radius: f64, rect: R) -> Instrument {
    Instrument {
        x,
        y,
        w: radius * 2.0,
        h: radius * 2.0,
        rect,
        shape: Shape::Dial,
    }
}
fn display_face(x: f64, y: f64, w: f64, h: f64, rect: R, buttons: bool) -> Instrument {
    Instrument {
        x,
        y,
        w,
        h,
        rect,
        shape: Shape::Display(buttons),
    }
}
fn label(x: f64, y: f64, w: f64, h: f64, rect: R) -> Instrument {
    Instrument {
        x,
        y,
        w,
        h,
        rect,
        shape: Shape::Label,
    }
}
fn layout(kind: Type) -> Vec<Instrument> {
    if kind == Type::Su27 {
        vec![
            dial(-0.12, 0.84, 0.070, mig::R_ADI),
            dial(-0.12, 0.65, 0.070, mig::R_HSI),
            dial(-0.31, 0.85, 0.058, mig::R_ASI),
            dial(-0.31, 0.66, 0.058, mig::R_ALT),
            dial(-0.44, 0.99, 0.037, mig::R_UUAP),
            dial(0.065, 0.84, 0.052, mig::R_VVI),
            dial(0.215, 0.84, 0.049, mig::R_RPM),
            dial(0.065, 0.65, 0.049, mig::R_EGT),
            dial(0.215, 0.65, 0.049, mig::R_EGT_R),
            dial(-0.31, 0.49, 0.045, mig::R_RADALT),
            dial(0.065, 0.49, 0.043, mig::R_CLOCK),
            display_face(0.365, 0.99, 0.185, 0.170, mig::R_TV, false),
            dial(0.39, 0.65, 0.060, mig::R_SPO),
            label(0.365, 0.49, 0.170, 0.065, mig::R_HYD),
            label(0.49, 0.82, 0.036, 0.25, mig::R_FUEL),
            label(-0.44, 0.59, 0.065, 0.07, mig::R_GEAR),
            label(0.04, 1.02, 0.195, 0.045, mig::R_WEAPON),
        ]
    } else {
        // NATOPS FO-5: IFEI beneath the left DDI; standby group beneath
        // the right one; UFC/HUD controls over the low centre AMPCD.
        vec![
            display_face(-0.27, 0.86, 0.240, 0.245, R_MFD_L, true),
            display_face(0.27, 0.86, 0.240, 0.245, R_MFD_R, true),
            display_face(0.0, 0.97, 0.220, 0.205, UFC, false),
            label(0.0, 0.795, 0.215, 0.093, HUD_CONTROL),
            display_face(0.0, 0.54, 0.235, 0.245, R_HSI, true),
            label(-0.285, 0.56, 0.225, 0.150, IFEI),
            dial(0.205, 0.61, 0.050, R_DIALS[2]),
            dial(0.37, 0.61, 0.050, R_DIALS[0]),
            dial(0.19, 0.46, 0.037, R_DIALS[1]),
            dial(0.305, 0.46, 0.037, R_DIALS[3]),
            dial(0.42, 0.46, 0.037, R_VVI),
            label(-0.47, 0.86, 0.044, 0.170, R_ARM),
            label(-0.42, 1.07, 0.040, 0.085, R_AOA),
            label(-0.29, 1.06, 0.155, 0.065, R_CAUTION),
            label(0.30, 1.06, 0.155, 0.065, R_CAUTION),
        ]
    }
}
pub fn build(kind: Type) -> Mesh {
    let mut m = Mesh::new();
    let russian = kind == Type::Su27;
    let panel = if russian {
        Rgb(0.32, 0.43, 0.46)
    } else {
        Rgb(0.19, 0.21, 0.22)
    };
    let wall = panel.scale(0.77);
    let plain = |at| Frame {
        o: at,
        x: V3::X,
        y: V3::Y,
        z: V3::Z,
    };
    // Floor beneath the boots and pedals, parallel console inboard edges
    // outside the seat bucket, and an open central stick/leg volume.
    m.block(&plain(v3(0.0, 0.125, -3.65)), v3(0.50, 0.025, 0.90), wall);
    for side in [-1.0, 1.0] {
        m.block(
            &plain(v3(side * 0.515, 0.52, -3.42)),
            v3(0.018, 0.38, 0.91),
            wall,
        );
        let c = |x: f64, z: f64| v3(side * x, 0.62 + (x - 0.262) * 0.14, z);
        let (inner, outer, zf, za) = (0.262, 0.49, -3.94, -2.72);
        m.quad(
            c(inner, zf),
            c(outer, zf),
            c(outer, za),
            c(inner, za),
            panel,
        );
        m.quad(
            c(inner, zf),
            c(inner, za),
            v3(side * inner, 0.15, za),
            v3(side * inner, 0.15, zf),
            wall,
        );
        // Sloped joining cheeks outboard of the knees, not a transverse block.
        let f = instrument(kind, side * 0.49, 0.30);
        m.quad(
            c(inner, zf),
            c(outer, zf),
            f.o,
            instrument(kind, side * inner, 0.30).o,
            panel.scale(0.8),
        );
        for (row, z) in [-3.83, -3.65, -3.47, -3.29, -3.11, -2.93]
            .into_iter()
            .enumerate()
        {
            // Leave the whole throttle slot and hand travel clear.
            if side < 0.0 && z > -3.74 && z < -3.23 {
                continue;
            }
            let x = side * 0.375;
            let across = v3(1.0, side * 0.14, 0.0).norm();
            let cf = Frame {
                o: c(x.abs(), z) + V3::Y * 0.005,
                x: across,
                y: -V3::Z,
                z: across.cross(-V3::Z),
            };
            m.block(&cf, v3(0.098, 0.073, 0.004), DARK.scale(1.7));
            for j in 0..4 {
                let at = cf.p(-0.072 + j as f64 * 0.047, 0.0, 0.006);
                if (row + j) % 3 == 0 {
                    m.capsule(at, at + cf.z * 0.017, 0.009, 0.009, DARK, 8);
                } else {
                    m.capsule(
                        at,
                        at + cf.z * 0.020 - cf.y * 0.006,
                        0.003,
                        0.002,
                        METALC,
                        5,
                    );
                }
            }
            for x in [-0.087, 0.087] {
                m.block(&cf.at(x, 0.058, 0.006), v3(0.002, 0.002, 0.001), METALC);
            }
        }
    }
    // Correct twin-throttle quadrant beneath the moving common controls.
    m.block(
        &plain(v3(-0.315, 0.637, -3.465)),
        v3(0.041, 0.018, 0.205),
        DARK,
    );
    m.block(
        &plain(v3(-0.315, 0.656, -3.465)),
        v3(0.022, 0.001, 0.184),
        Rgb(0.02, 0.02, 0.02),
    );
    // Board assembled around two open knee wells and a narrow centre pedestal.
    // A piecewise board avoids fanning triangles over the concave knee recess.
    for (x0, x1, y0, y1) in [
        (-0.51, 0.51, 0.63, 0.97),
        (-0.51, -0.145, 0.43, 0.63),
        (0.145, 0.51, 0.43, 0.63),
        (-0.145, 0.145, 0.36, 0.63),
    ] {
        let mut xs = vec![x0];
        let hinge = if kind == Type::Su27 { 0.27 } else { 0.46 };
        for fold in [-hinge, hinge] {
            if fold > x0 && fold < x1 {
                xs.push(fold);
            }
        }
        xs.push(x1);
        for pair in xs.windows(2) {
            let (l, r) = (pair[0], pair[1]);
            m.quad(
                instrument(kind, l, y0).p(0.0, 0.0, -0.018),
                instrument(kind, r, y0).p(0.0, 0.0, -0.018),
                instrument(kind, r, y1).p(0.0, 0.0, -0.018),
                instrument(kind, l, y1).p(0.0, 0.0, -0.018),
                panel,
            );
        }
    }
    let top = |x: f64| {
        if x.abs() <= 0.40 {
            mix(1.12, 1.10, ((x.abs() - 0.15) / 0.25).max(0.0))
        } else {
            mix(1.10, 0.97, (x.abs() - 0.40) / 0.11)
        }
    };
    let xs = [
        -0.51, -0.46, -0.40, -0.27, -0.15, 0.15, 0.27, 0.40, 0.46, 0.51,
    ];
    for pair in xs.windows(2) {
        let (l, r) = (pair[0], pair[1]);
        let (a, b) = (
            instrument(kind, l, top(l)).p(0.0, 0.0, -0.018),
            instrument(kind, r, top(r)).p(0.0, 0.0, -0.018),
        );
        m.quad(
            instrument(kind, l, 0.97).p(0.0, 0.0, -0.018),
            instrument(kind, r, 0.97).p(0.0, 0.0, -0.018),
            b,
            a,
            panel,
        );
        m.quad(
            a + V3::Z * 0.025,
            b + V3::Z * 0.025,
            b - V3::Z * 0.16 + V3::Y * 0.015,
            a - V3::Z * 0.16 + V3::Y * 0.015,
            DARK,
        );
    }
    for item in layout(kind) {
        let f = instrument(kind, item.x, item.y);
        match item.shape {
            Shape::Dial => gauge(&mut m, &f, item.w * 0.5, item.rect),
            Shape::Display(buttons) => display(
                &mut m,
                &f,
                item.w,
                item.h,
                item.w * if buttons { 0.69 } else { 0.91 },
                item.h * if buttons { 0.69 } else { 0.91 },
                item.rect,
                buttons,
            ),
            Shape::Label => plate(&mut m, &f, item.w, item.h, item.rect),
        }
    }
    // HUD projector and supports around the shared collimated combiner.
    m.block(&plain(v3(0.0, 1.12, -4.46)), v3(0.11, 0.035, 0.12), DARK);
    for side in [-1.0, 1.0] {
        m.capsule(
            v3(side * 0.113, 1.07, -4.015),
            v3(side * 0.113, 1.36, -3.957),
            0.006,
            0.006,
            DARK,
            6,
        );
    }
    // Native canopy geometry carried into the common eye frame. Sills,
    // windscreen bow, mirrors and rear bulkhead all meet the exterior.
    let offset = crate::airframe::EYE - kind.eye();
    let native = |z: f64, phi: f64| crate::hornet::canopy_point(russian, z, phi) + offset;
    let bow = crate::hornet::canopy_bow(russian);
    let aft = if russian { -2.85 } else { -2.18 };
    for k in 0..24 {
        let (a, b) = (
            native(bow, k as f64 * PI / 24.0),
            native(bow, (k + 1) as f64 * PI / 24.0),
        );
        m.capsule(a, b, 0.018, 0.018, DARK, 6);
    }
    for edge in [0.0, PI] {
        for k in 0..16 {
            let (z0, z1) = (
                mix(bow, aft, k as f64 / 16.0),
                mix(bow, aft, (k + 1) as f64 / 16.0),
            );
            m.capsule(native(z0, edge), native(z1, edge), 0.025, 0.025, DARK, 6);
        }
    }
    for phi in [0.55, PI / 2.0, PI - 0.55] {
        let at = native(bow, phi);
        curved_mirror(
            &mut m,
            if phi == PI / 2.0 { 0.092 } else { 0.075 },
            0.034,
            &|x, y, d| at + v3(x, y - 0.045, 0.045 + d),
            DARK,
            -0.01,
        );
    }
    let back_z = -2.70;
    m.block(&plain(v3(0.0, 0.63, back_z)), v3(0.49, 0.47, 0.025), wall);
    pilot::build_static_mig(&mut m);
    m
}
pub fn hornet_instruments(
    cv: &mut Canvas,
    a: &Aircraft,
    ad: &AirData,
    m: &Mission,
    time: f64,
    c: &Combat,
) {
    mfd(cv, R_HSI, Page::Hsd, a, ad, m, time, c);
    let r = UFC;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x23, 0x28, 0x29, 1.0));
    cv.rect(
        r.x + 10.0,
        r.y + 7.0,
        166.0,
        22.0,
        rgba(0x02, 0x09, 0x05, 1.0),
    );
    cv.text("  305.000", r.x + 18.0, r.y + 18.0, 1.2, GREEN, Align::Left);
    let keys = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "CLR", "0", "ENT",
    ];
    for row in 0..4 {
        for col in 0..3 {
            let (x, y) = (
                r.x + 17.0 + col as f32 * 48.0,
                r.y + 38.0 + row as f32 * 28.0,
            );
            cv.rect(x, y, 37.0, 23.0, rgba(0x43, 0x49, 0x49, 1.0));
            cv.text(
                keys[row * 3 + col],
                x + 18.5,
                y + 11.5,
                1.0,
                WHITE,
                Align::Center,
            );
        }
    }
    for (i, name) in ["COM1", "COM2", "IFF", "TCN", "ILS"]
        .into_iter()
        .enumerate()
    {
        let y = r.y + 20.0 + i as f32 * 28.0;
        cv.rect(
            r.x + 201.0,
            y - 8.0,
            92.0,
            18.0,
            rgba(0x02, 0x09, 0x05, 1.0),
        );
        cv.text(name, r.x + 245.0, y + 1.0, 1.0, GREEN, Align::Center);
        cv.rect(
            r.x + 183.0,
            y - 7.0,
            11.0,
            16.0,
            rgba(0x5c, 0x61, 0x60, 1.0),
        );
    }
    let r = HUD_CONTROL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x24, 0x2a, 0x2a, 1.0));
    for (i, name) in ["HUD", "BRT", "DAY", "ATT", "ALT"].into_iter().enumerate() {
        let x = r.x + 32.0 + i as f32 * 63.0;
        cv.text(name, x, r.y + 13.0, 1.0, WHITE, Align::Center);
        cv.disc(x, r.y + 49.0, 13.0, rgba(0x09, 0x0b, 0x0b, 1.0));
        cv.line(x, r.y + 49.0, x + 4.0, r.y + 38.0, 2.0, WHITE);
    }
    let r = IFEI;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x13, 0x1b, 0x18, 1.0));
    for (i, (name, value)) in [
        ("RPM", format!("{:.0}", a.rpm)),
        ("TEMP", format!("{:.0}", a.ftit)),
        ("FF", format!("{:.1}", a.fuel_flow * 7936.64 * 0.5 / 1000.0)),
        ("NOZ", format!("{:.0}", if a.ab { 100.0 } else { 28.0 })),
        ("OIL", "95".to_string()),
    ]
    .into_iter()
    .enumerate()
    {
        let y = r.y + 17.0 + i as f32 * 26.0;
        cv.text(name, r.x + 104.0, y, 1.0, DIM, Align::Center);
        for x in [r.x + 40.0, r.x + 163.0] {
            cv.text(&value, x, y, 1.4, GREEN, Align::Center);
        }
    }
    cv.text(
        "L   ENGINE   R",
        r.x + 103.0,
        r.y + 148.0,
        1.0,
        DIM,
        Align::Center,
    );
    cv.text("FUEL", r.x + 295.0, r.y + 20.0, 1.1, DIM, Align::Center);
    cv.text(
        &format!("{:.0}", a.fuel * 2.20462),
        r.x + 295.0,
        r.y + 48.0,
        1.6,
        GREEN,
        Align::Center,
    );
    cv.text("BINGO", r.x + 295.0, r.y + 91.0, 1.0, DIM, Align::Center);
    cv.text("2000", r.x + 295.0, r.y + 116.0, 1.4, GREEN, Align::Center);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instrument_faces_are_visible_and_do_not_overlap() {
        for kind in [Type::F18, Type::Su27] {
            let m = build(kind);
            assert!(m.count() > 1000 && m.v.iter().all(|x| x.is_finite()));
            let items = layout(kind);
            for (i, item) in items.iter().enumerate() {
                let f = instrument(kind, item.x, item.y);
                assert!(f.x.dot(f.y).abs() < 1e-8 && (f.z.len() - 1.0).abs() < 1e-8);
                let (w, h, depth) = match item.shape {
                    Shape::Dial => (item.w, item.h, 0.0135),
                    Shape::Display(buttons) => (
                        item.w * if buttons { 0.69 } else { 0.91 },
                        item.h * if buttons { 0.69 } else { 0.91 },
                        0.006,
                    ),
                    Shape::Label => (item.w, item.h, 0.0085),
                };
                for x in [-0.7, 0.0, 0.7] {
                    for y in [-0.7, 0.0, 0.7] {
                        let target = f.p(x * w * 0.5, y * h * 0.5, depth);
                        let hit =
                            m.v.chunks(crate::mesh::STRIDE * 3)
                                .find(|tri| blocked(crate::airframe::EYE, target, tri));
                        assert!(hit.is_none(),"{kind:?}: face {i} hidden at {x},{y}, target {target:?}, obstruction {hit:?}");
                    }
                }
                for next in &items[i + 1..] {
                    let gap_x = (item.x - next.x).abs() - (item.w + next.w) * 0.5;
                    let gap_y = (item.y - next.y).abs() - (item.h + next.h) * 0.5;
                    assert!(
                        gap_x >= 0.008 || gap_y >= 0.008,
                        "{kind:?}: overlapping instruments at {},{} and {},{}",
                        item.x,
                        item.y,
                        next.x,
                        next.y
                    );
                }
            }
        }
    }
    #[test]
    fn consoles_leave_the_pilot_and_throttle_envelopes_open() {
        for kind in [Type::F18, Type::Su27] {
            let mut pilot_mesh = Mesh::new();
            pilot::build_dynamic_center(
                &mut pilot_mesh,
                &Hands {
                    rudder: 1.0,
                    throttle: 1.0,
                    ..Default::default()
                },
                kind == Type::Su27,
            );
            let cockpit = build(kind);
            // Floor and console edges lie below the boot soles and outside the bucket.
            assert!(cockpit.v.iter().all(|v| v.is_finite()));
            for v in pilot_mesh.v.chunks(crate::mesh::STRIDE) {
                if v[1] < 0.60 && v[2] > -3.9 && v[2] < -3.0 {
                    assert!(
                        v[0].abs() < 0.262 + 0.012,
                        "pilot intersects console: {v:?}"
                    );
                    assert!(v[1] > 0.15, "boot goes through the cockpit floor");
                }
            }
        }
    }
}
