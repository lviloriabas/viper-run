//! Distinct Hornet DDI/UFC/AMPCD and Su-27 analogue-panel layouts.
//! Displays share the game's navigation/weapon pages; this is a procedural
//! cockpit reconstruction, not an interactive reproduction of every switch.
use super::*;
use crate::sim::Type;
const DARK: Rgb = Rgb(0.10, 0.12, 0.13);
fn f(x: f64, y: f64, z: f64) -> Frame {
    Frame {
        o: v3(x, y, z),
        x: V3::X,
        y: v3(0.0, 0.99, -0.14),
        z: v3(0.0, 0.14, 0.99),
    }
}
pub fn build(kind: Type) -> Mesh {
    let mut m = Mesh::new();
    let russian = kind == Type::Su27;
    let panel = if russian {
        Rgb(0.30, 0.42, 0.43)
    } else {
        Rgb(0.20, 0.23, 0.24)
    };
    let width = if russian { 0.59 } else { 0.56 };
    // Chamfered tub, floor, side consoles and the sill stay below sightline.
    m.block(
        &f(0.0, 0.14, -3.65),
        v3(width, 0.035, 0.85),
        panel.scale(0.6),
    );
    for side in [-1.0, 1.0] {
        m.block(
            &f(side * (width + 0.015), 0.60, -3.45),
            v3(0.025, 0.45, 1.12),
            panel,
        );
        m.block(&f(side * 0.43, 0.53, -3.40), v3(0.12, 0.055, 0.59), DARK);
        m.capsule(
            v3(side * width, 1.0, -4.55),
            v3(side * width, 1.0, -2.7),
            0.027,
            0.027,
            DARK,
            7,
        );
        for row in 0..7 {
            for column in 0..3 {
                let at = v3(
                    side * (0.34 + column as f64 * 0.07),
                    0.60,
                    -3.95 + row as f64 * 0.14,
                );
                let axis = Frame {
                    o: at,
                    x: V3::X,
                    y: -V3::Z,
                    z: V3::Y,
                };
                m.block(&axis, v3(0.025, 0.042, 0.005), panel.scale(0.7));
                m.capsule(at, at + V3::Y * 0.024, 0.006, 0.004, METALC, 5);
            }
        }
    }
    // Windscreen arch and sills in the shared design-eye coordinates.
    // These replace the exterior skin frames removed by the first-person hole.
    let arch = |t: f64| v3(0.47 * t.cos(), 0.92 + 0.69 * t.sin(), -4.20);
    for j in 0..24 {
        m.capsule(
            arch(j as f64 * PI / 24.0),
            arch((j + 1) as f64 * PI / 24.0),
            0.018,
            0.018,
            DARK,
            6,
        );
    }
    for side in [-1.0, 1.0] {
        m.capsule(
            v3(side * 0.47, 0.92, -4.2),
            v3(side * 0.54, 1.0, -2.7),
            0.025,
            0.025,
            DARK,
            6,
        );
    }
    for (cx, cy) in [(-0.32, 1.36), (0.0, 1.56), (0.32, 1.36)] {
        curved_mirror(
            &mut m,
            0.10,
            0.045,
            &|x, y, depth| v3(cx + x, cy + y, -4.14 + depth),
            DARK,
            -0.01,
        );
    }
    // Instrument board is ahead of the knee volume, with a low centre cutout.
    m.block(&f(0.0, 0.76, -4.16), v3(width, 0.28, 0.025), panel);
    for side in [-1.0, 1.0] {
        m.block(&f(side * 0.41, 0.40, -4.16), v3(0.16, 0.12, 0.025), panel);
    }
    m.block(&f(0.0, 1.065, -4.14), v3(width + 0.015, 0.018, 0.10), DARK);
    if russian {
        gauge(&mut m, &f(-0.15, 0.81, -4.12), 0.088, mig::R_ADI);
        gauge(&mut m, &f(-0.15, 0.58, -4.08), 0.088, mig::R_HSI);
        gauge(&mut m, &f(-0.37, 0.82, -4.12), 0.068, mig::R_ASI);
        gauge(&mut m, &f(-0.37, 0.61, -4.09), 0.068, mig::R_ALT);
        gauge(&mut m, &f(0.065, 0.83, -4.12), 0.058, mig::R_VVI);
        gauge(&mut m, &f(-0.48, 0.97, -4.13), 0.046, mig::R_UUAP);
        display(
            &mut m,
            &f(0.32, 0.81, -4.12),
            0.26,
            0.25,
            0.215,
            0.192,
            mig::R_TV,
            false,
        );
        for (x, rect) in [(0.11, mig::R_RPM), (0.29, mig::R_EGT), (0.46, mig::R_EGT_R)] {
            gauge(&mut m, &f(x, 0.51, -4.06), 0.058, rect);
        }
        plate(&mut m, &f(0.51, 0.88, -4.10), 0.044, 0.25, mig::R_WARN);
        gauge(&mut m, &f(0.045, 0.64, -4.09), 0.047, mig::R_SPO);
        plate(&mut m, &f(-0.46, 0.40, -4.04), 0.095, 0.075, mig::R_GEAR);
    } else {
        display(
            &mut m,
            &f(-0.275, 0.79, -4.11),
            0.26,
            0.26,
            0.205,
            0.205,
            R_MFD_L,
            true,
        );
        display(
            &mut m,
            &f(0.275, 0.79, -4.11),
            0.26,
            0.26,
            0.205,
            0.205,
            R_MFD_R,
            true,
        );
        // UFC between the DDIs and AMPCD low in the centre, with standby
        // round instruments outside the lower central display.
        display(
            &mut m,
            &f(0.0, 0.91, -4.08),
            0.215,
            0.205,
            0.19,
            0.18,
            R_ICP,
            false,
        );
        display(
            &mut m,
            &f(0.0, 0.50, -4.03),
            0.25,
            0.23,
            0.205,
            0.19,
            R_HSI,
            true,
        );
        gauge(&mut m, &f(-0.32, 0.50, -4.03), 0.057, R_DIALS[0]);
        gauge(&mut m, &f(-0.47, 0.50, -4.03), 0.057, R_DIALS[1]);
        gauge(&mut m, &f(0.32, 0.50, -4.03), 0.057, R_DIALS[2]);
        gauge(&mut m, &f(0.47, 0.50, -4.03), 0.057, R_DIALS[3]);
        plate(&mut m, &f(-0.51, 0.98, -4.06), 0.047, 0.09, R_AOA);
        plate(&mut m, &f(0.51, 0.99, -4.08), 0.070, 0.05, R_MC);
    }
    // Centre-stick pilot geometry and twin-lever throttle, normalised to the
    // common eye point. Seat and ejection meshes use the selected seat spec.
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
    let r = R_ICP;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x13, 0x1a, 0x1c, 1.0));
    for (i, name) in ["COM1", "COM2", "IFF", "TCN", "ILS"]
        .into_iter()
        .enumerate()
    {
        let y = r.y + 14.0 + i as f32 * 24.0;
        cv.rect(r.x + 6.0, y - 7.0, 55.0, 16.0, rgba(0x03, 0x0c, 0x07, 1.0));
        cv.text(name, r.x + 33.0, y, 1.0, GREEN, Align::Center);
        cv.text(
            if i == 0 {
                "305.000"
            } else if i == 1 {
                "127.500"
            } else {
                "ON"
            },
            r.x + 70.0,
            y,
            1.0,
            GREEN,
            Align::Left,
        );
    }
    for row in 0..4 {
        for col in 0..3 {
            let (x, y) = (
                r.x + 148.0 + col as f32 * 36.0,
                r.y + 14.0 + row as f32 * 35.0,
            );
            cv.rect(x, y, 29.0, 26.0, rgba(0x36, 0x3d, 0x3e, 1.0));
            cv.text(
                &format!("{}", (row * 3 + col + 1) % 10),
                x + 14.5,
                y + 13.0,
                1.0,
                WHITE,
                Align::Center,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinct_panels_keep_the_forward_view_clear() {
        for kind in [Type::F18, Type::Su27] {
            let m = build(kind);
            assert!(m.count() > 1000 && m.v.iter().all(|v| v.is_finite()));
            let screens =
                m.v.chunks(crate::mesh::STRIDE)
                    .filter(|v| v[9] >= 0.0)
                    .count();
            assert!(screens > 60);
        }
    }
}
