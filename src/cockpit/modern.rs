//! Reference-led single-seat F/A-18C with UFCD and baseline Su-27SK cockpits.
//! Geometry is in the shared design-eye space. All instrument faces use
//! their board's orthonormal frame; consoles clear the common pilot envelope.
use super::*;
use crate::sim::Type;
const DARK: Rgb = Rgb(0.075, 0.085, 0.09);
const IFEI: R = r(0.0, 1872.0, 256.0, 176.0);
const UFC: R = r(384.0, 1872.0, 160.0, 176.0);
const HUD_CONTROL: R = r(704.0, 1872.0, 256.0, 96.0);
const FLANKER_WEAPONS: R = r(1024.0, 1840.0, 448.0, 64.0);
const FLANKER_SYSTEMS: R = r(1472.0, 1840.0, 192.0, 64.0);
const FLANKER_WARN: R = r(1024.0, 640.0, 96.0, 320.0);

/// The outer cheeks toe toward the seat, while the central board stays flat.
fn instrument(kind: Type, x: f64, y: f64) -> Frame {
    let tilt = if kind == Type::Su27 { 12.0 } else { 8.5 } * DEG;
    let p = Frame {
        o: v3(0.0, 0.64, -4.40),
        x: V3::X,
        y: v3(0.0, tilt.cos(), -tilt.sin()),
        z: v3(0.0, tilt.sin(), tilt.cos()),
    };
    let cant = if kind == Type::Su27 { 18.0 } else { 10.0 } * DEG;
    let hinge = if kind == Type::Su27 { 0.29 } else { 0.40 };
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
/// Fit the complete face without stretching or cropping its pixel grid.
fn face_size(item: &Instrument) -> (f64, f64) {
    let margin = match item.shape {
        Shape::Display(true) => 0.69,
        Shape::Display(false) => 0.91,
        _ => 1.0,
    };
    let scale = (item.w * margin / item.rect.w as f64).min(item.h * margin / item.rect.h as f64);
    (item.rect.w as f64 * scale, item.rect.h as f64 * scale)
}

// Dense groups and proportions reconstructed from the supplied cockpit photos.
fn layout(kind: Type) -> Vec<Instrument> {
    if kind == Type::Su27 {
        vec![
            dial(-0.085, 0.805, 0.058, mig::R_ADI),
            dial(-0.085, 0.650, 0.058, mig::R_HSI),
            dial(-0.235, 0.805, 0.048, mig::R_ASI),
            dial(-0.235, 0.650, 0.048, mig::R_ALT),
            dial(-0.370, 0.805, 0.046, mig::R_UUAP),
            dial(-0.370, 0.650, 0.046, mig::R_RADALT),
            dial(0.060, 0.805, 0.044, mig::R_VVI),
            dial(0.195, 0.805, 0.048, mig::R_RPM),
            dial(0.080, 0.650, 0.035, mig::R_EGT),
            dial(0.195, 0.650, 0.035, mig::R_EGT_R),
            dial(-0.370, 0.505, 0.037, mig::R_CLOCK),
            display_face(0.355, 0.980, 0.185, 0.165, mig::R_TV, false),
            dial(0.390, 0.650, 0.047, mig::R_SPO),
            label(0.335, 0.505, 0.180, 0.055, mig::R_HYD),
            label(0.292, 0.725, 0.060, 0.200, mig::R_FUEL),
            label(-0.250, 0.505, 0.065, 0.070, mig::R_GEAR),
            label(-0.080, 0.915, 0.300, 0.045, FLANKER_WEAPONS),
            label(-0.025, 1.035, 0.235, 0.090, HUD_CONTROL),
            label(-0.305, 0.950, 0.100, 0.050, FLANKER_SYSTEMS),
            label(0.075, 0.505, 0.105, 0.105, mig::R_EKRAN),
            label(0.452, 0.530, 0.035, 0.120, FLANKER_WARN),
        ]
    } else {
        // The first supplied photo has the upgraded UFCD, not a keypad UFC.
        vec![
            display_face(-0.250, 0.885, 0.245, 0.245, R_MFD_L, true),
            display_face(0.250, 0.885, 0.245, 0.245, R_MFD_R, true),
            display_face(0.0, 1.005, 0.215, 0.240, UFC, true),
            label(0.0, 0.812, 0.210, 0.080, HUD_CONTROL),
            display_face(0.0, 0.590, 0.225, 0.225, R_HSI, true),
            label(-0.265, 0.595, 0.215, 0.150, IFEI),
            dial(0.200, 0.650, 0.041, R_DIALS[2]),
            dial(0.335, 0.650, 0.041, R_DIALS[0]),
            dial(0.190, 0.515, 0.031, R_DIALS[1]),
            dial(0.300, 0.515, 0.031, R_DIALS[3]),
            dial(0.410, 0.515, 0.031, R_VVI),
            dial(-0.432, 0.690, 0.029, R_RWR),
            label(-0.432, 0.880, 0.040, 0.170, R_ARM),
            label(-0.420, 1.035, 0.034, 0.075, R_AOA),
            label(-0.270, 1.065, 0.150, 0.050, R_CAUTION),
            label(0.270, 1.065, 0.150, 0.050, R_CAUTION),
        ]
    }
}
/// Physical aperture used both by the mesh and HUD clipping.
pub(super) fn hud_glass(kind: Type) -> [V3; 4] {
    if kind == Type::Su27 {
        [
            v3(-0.150, 1.115, -4.36),
            v3(0.150, 1.115, -4.36),
            v3(0.175, 1.445, -4.43),
            v3(-0.175, 1.445, -4.43),
        ]
    } else {
        [
            v3(-0.145, 1.180, -4.42),
            v3(0.145, 1.180, -4.42),
            v3(0.155, 1.445, -4.47),
            v3(-0.155, 1.445, -4.47),
        ]
    }
}
// Projector controls are on the front of its housing, above the main board.
fn mount(kind: Type, item: &Instrument) -> Frame {
    let mut f = instrument(kind, item.x, item.y);
    if kind == Type::Su27 && item.rect.x == HUD_CONTROL.x && item.rect.y == HUD_CONTROL.y {
        f.o.z += 0.17;
    }
    f
}
// The console atlas occupies the F-16 console area only for these aircraft;
// the active type draws its own legends after the common atlas pass.
fn console_rect(side: f64, row: usize) -> R {
    let i = row + if side > 0.0 { 6 } else { 0 };
    r(
        1024.0 + (i % 4) as f32 * 256.0,
        (i / 4) as f32 * 192.0,
        if side < 0.0 && (1..=3).contains(&row) {
            128.0
        } else {
            256.0
        },
        192.0,
    )
}
fn console_labels(russian: bool, side: f64, row: usize) -> [&'static str; 9] {
    if russian {
        if side < 0.0 {
            [
                [
                    "SHASSI / ZAKRYLKI",
                    "SHASSI",
                    "ZAKR",
                    "NOS",
                    "TORM",
                    "VYP",
                    "UBR",
                    "AVAR",
                    "TEST",
                ],
                [
                    "DVIGATELI",
                    "LEV",
                    "PRAV",
                    "ZAP",
                    "STOP",
                    "TOPL",
                    "NASOS",
                    "REZH",
                    "OTKL",
                ],
                [
                    "SAU", "KREN", "TANG", "VYS", "KURS", "STAB", "MARSH", "POS", "TEST",
                ],
                [
                    "VOORUZHENIE",
                    "PUSK",
                    "SBROS",
                    "SER",
                    "PUSH",
                    "BORT",
                    "ZALP",
                    "AVAR",
                    "OTKL",
                ],
                [
                    "RADIO", "VHF", "UHF", "VOL", "KAN", "RSBN", "ARK", "SAU", "SVYAZ",
                ],
                [
                    "TOPLIVO", "LEV", "PRAV", "NASOS", "BAKI", "PODV", "OSN", "AVAR", "TEST",
                ],
            ][row]
        } else {
            [
                [
                    "ELEKTRO", "AKK", "GEN L", "GEN R", "AC", "SET", "PIT", "AVAR", "TEST",
                ],
                [
                    "KONDITSIONER",
                    "TEMP",
                    "DAVL",
                    "KAB",
                    "VENT",
                    "OBDUV",
                    "OTOP",
                    "STEK",
                    "TEST",
                ],
                [
                    "NAVIGATSIYA",
                    "RSBN",
                    "ARK",
                    "KURS",
                    "REZH",
                    "KAN",
                    "MARSH",
                    "POS",
                    "TEST",
                ],
                [
                    "OSVESHCHENIE",
                    "PRIB",
                    "PULT",
                    "FARA",
                    "BORT",
                    "DEN",
                    "NOCH",
                    "YARK",
                    "TEST",
                ],
                [
                    "KISLOROD", "PIT", "DAVL", "REZH", "AVAR", "SMES", "OBDUV", "MASKA", "TEST",
                ],
                [
                    "AVTOMATY", "RLS", "OEPS", "SAU", "SVYAZ", "PIT", "PRIB", "NAV", "REZ",
                ],
            ][row]
        }
    } else if side < 0.0 {
        [
            [
                "FLIGHT CONTROL",
                "FLAPS",
                "TRIM",
                "FCS",
                "T/O",
                "LAUNCH",
                "BAR",
                "FOLD",
                "RESET",
            ],
            [
                "ENGINES", "L ENG", "R ENG", "START", "CRANK", "FUEL", "BOOST", "IGN", "STOP",
            ],
            [
                "THROTTLE", "IDLE", "MIL", "AB", "CUT", "L ENG", "R ENG", "FUEL", "TEST",
            ],
            [
                "COMMUNICATION",
                "COMM1",
                "COMM2",
                "VOL",
                "ICS",
                "IFF",
                "TCN",
                "ILS",
                "TEST",
            ],
            [
                "FUEL", "PROBE", "DUMP", "EXT", "BINGO", "WING", "CTR", "FEED", "TEST",
            ],
            [
                "EXTERIOR LIGHTS",
                "POS",
                "FORM",
                "STROBE",
                "TAXI",
                "NAV",
                "BRT",
                "DIM",
                "TEST",
            ],
        ][row]
    } else {
        [
            [
                "ELECTRICAL",
                "BATT",
                "L GEN",
                "R GEN",
                "AC",
                "RESET",
                "DC",
                "EXT",
                "TEST",
            ],
            [
                "ENVIRONMENT",
                "TEMP",
                "CABIN",
                "BLEED",
                "DEFOG",
                "RAM",
                "ANTI-I",
                "VENT",
                "TEST",
            ],
            [
                "NAVIGATION",
                "INS",
                "RADAR",
                "ALT",
                "MAP",
                "ALIGN",
                "NAV",
                "ATT",
                "TEST",
            ],
            [
                "INTERIOR LIGHTS",
                "INST",
                "CONSL",
                "FLOOD",
                "CHART",
                "DAY",
                "NIGHT",
                "BRT",
                "TEST",
            ],
            [
                "OXYGEN", "SUPPLY", "FLOW", "MASK", "EMERG", "NORMAL", "100%", "PRESS", "TEST",
            ],
            [
                "CIRCUIT BREAKERS",
                "AVION",
                "COMM",
                "NAV",
                "HUD",
                "DDI L",
                "DDI R",
                "FCS",
                "AUX",
            ],
        ][row]
    }
}
fn draw_consoles(cv: &mut Canvas, russian: bool) {
    for side in [-1.0, 1.0] {
        for row in 0..6 {
            let r = console_rect(side, row);
            let names = console_labels(russian, side, row);
            let ink = rgba(0xc9, 0xd1, 0xcb, 1.0);
            cv.rect(
                r.x,
                r.y,
                r.w,
                r.h,
                if russian {
                    rgba(0x28, 0x39, 0x3c, 1.0)
                } else {
                    rgba(0x20, 0x24, 0x25, 1.0)
                },
            );
            cv.text(
                names[0],
                r.x + r.w * 0.5,
                r.y + 16.0,
                1.0,
                ink,
                Align::Center,
            );
            cv.line(
                r.x + 10.0,
                r.y + 29.0,
                r.x + r.w - 10.0,
                r.y + 29.0,
                1.0,
                DIM,
            );
            let cols = if r.w < 200.0 { 2 } else { 4 };
            for k in 0..2 {
                for j in 0..cols {
                    let x = r.x
                        + if cols == 2 {
                            28.8 + j as f32 * 70.4
                        } else {
                            34.0 + j as f32 * 61.4
                        };
                    let y = r.y + if k == 0 { 64.4 } else { 140.7 };
                    cv.text(
                        names[1 + k * cols + j],
                        x,
                        y - 20.0,
                        1.0,
                        ink,
                        Align::Center,
                    );
                    cv.circle(x, y, 9.0, 1.0, DIM);
                    cv.text(
                        if k == 0 { "ON" } else { "OFF" },
                        x,
                        y + 22.0,
                        1.0,
                        ink,
                        Align::Center,
                    );
                }
            }
        }
    }
}
pub fn build(kind: Type) -> Mesh {
    let mut m = Mesh::new();
    let russian = kind == Type::Su27;
    let panel = if russian {
        Rgb(0.37, 0.62, 0.66)
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
            v3(0.018, 0.38, 1.09),
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
            let narrow = side < 0.0 && z > -3.74 && z < -3.23;
            let x = side * if narrow { 0.445 } else { 0.375 };
            let across = v3(1.0, side * 0.14, 0.0).norm();
            let cf = Frame {
                o: c(x.abs(), z) + V3::Y * 0.005,
                x: across,
                y: -V3::Z,
                z: across.cross(-V3::Z),
            };
            m.block(
                &cf,
                v3(if narrow { 0.040 } else { 0.098 }, 0.073, 0.004),
                DARK.scale(1.7),
            );
            let rect = console_rect(side, row);
            let width = if narrow { 0.080 } else { 0.196 };
            let height = (width * rect.h as f64 / rect.w as f64).min(0.146);
            let width = height * rect.w as f64 / rect.h as f64;
            screen(&mut m, &cf.at(0.0, 0.0, 0.0045), width, height, rect);
            for k in 0..2 {
                for j in 0..if narrow { 2 } else { 4 } {
                    if j == 0 && k == 0 && (row == 0 || side > 0.0 && row == 2) {
                        continue;
                    }
                    let at = cf.p(
                        if narrow {
                            -0.022 + j as f64 * 0.044
                        } else {
                            -0.072 + j as f64 * 0.047
                        },
                        if k == 0 { 0.024 } else { -0.034 },
                        0.006,
                    );
                    if (row + j + k) % 4 == 0 {
                        m.capsule(at, at + cf.z * 0.015, 0.008, 0.007, DARK, 10);
                        m.capsule(
                            at + cf.z * 0.016,
                            at + cf.z * 0.016 + cf.y * 0.005,
                            0.001,
                            0.001,
                            METALC,
                            4,
                        );
                    } else if russian && side > 0.0 && row == 5 || row == 4 && j == 0 {
                        m.block(
                            &Frame {
                                o: at + cf.z * 0.008,
                                ..cf
                            },
                            v3(0.006, 0.012, 0.009),
                            Rgb(0.65, 0.10, 0.06),
                        );
                    } else {
                        m.capsule(
                            at,
                            at + cf.z * 0.017 - cf.y * 0.005,
                            0.0028,
                            0.0018,
                            METALC,
                            6,
                        );
                    }
                }
            }
            for x in if narrow {
                [-0.033, 0.033]
            } else {
                [-0.087, 0.087]
            } {
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
        (-0.47, 0.47, 0.63, if russian { 0.90 } else { 0.97 }),
        (-0.47, -0.145, 0.43, 0.63),
        (0.145, 0.47, 0.43, 0.63),
        (-0.145, 0.145, 0.36, 0.63),
    ] {
        let mut xs = vec![x0];
        let hinge = if kind == Type::Su27 { 0.29 } else { 0.40 };
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
        let x = x.abs();
        if russian {
            if x <= 0.16 {
                1.10
            } else if x <= 0.29 {
                mix(1.10, 1.00, (x - 0.16) / 0.13)
            } else {
                mix(1.00, 0.90, (x - 0.29) / 0.18)
            }
        } else if x < 0.13 {
            1.15
        } else if x < 0.37 {
            1.11
        } else {
            mix(1.11, 0.97, (x - 0.37) / 0.10)
        }
    };
    let xs = [
        -0.47, -0.40, -0.37, -0.29, -0.16, -0.13, 0.0, 0.13, 0.16, 0.29, 0.37, 0.40, 0.47,
    ];
    for pair in xs.windows(2) {
        let (l, r) = (pair[0], pair[1]);
        let (a, b) = (
            instrument(kind, l, top(l)).p(0.0, 0.0, -0.018),
            instrument(kind, r, top(r)).p(0.0, 0.0, -0.018),
        );
        m.quad(
            instrument(kind, l, if russian { 0.90 } else { 0.97 }).p(0.0, 0.0, -0.018),
            instrument(kind, r, if russian { 0.90 } else { 0.97 }).p(0.0, 0.0, -0.018),
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
    // Close the tub to the panel sides; the forward board must not float
    // above the external fuselage visible through an open side triangle.
    for side in [-1.0, 1.0] {
        m.quad(
            instrument(kind, side * 0.47, 0.43).p(0.0, 0.0, -0.02),
            instrument(kind, side * 0.47, top(0.47)).p(0.0, 0.0, -0.02),
            v3(side * 0.515, 0.90, -3.94),
            v3(side * 0.515, 0.43, -3.94),
            wall,
        );
    }
    for item in layout(kind) {
        let f = mount(kind, &item);
        let (face_w, face_h) = face_size(&item);
        match item.shape {
            Shape::Dial => gauge_mount(&mut m, &f, item.w * 0.5, item.rect, 0.008),
            Shape::Display(buttons) => display(
                &mut m, &f, item.w, item.h, face_w, face_h, item.rect, buttons,
            ),
            Shape::Label => plate(&mut m, &f, face_w, face_h, item.rect),
        }
    }
    // Recessed radar hood and deep projector housing reproduce their silhouettes.
    if russian {
        let f = instrument(kind, 0.355, 0.980);
        for side in [-1.0, 1.0] {
            m.block(
                &f.at(side * 0.100, 0.010, 0.016),
                v3(0.008, 0.096, 0.030),
                DARK,
            );
        }
        m.block(&f.at(0.0, 0.103, 0.016), v3(0.108, 0.008, 0.030), DARK);
    }
    let glass = hud_glass(kind);
    let foot = (glass[0] + glass[1]) * 0.5;
    m.block(
        &plain(foot - v3(0.0, 0.038, 0.075)),
        v3(0.130, 0.033, 0.105),
        DARK,
    );
    for (a, b) in [(glass[0], glass[3]), (glass[1], glass[2])] {
        m.capsule(a, b, if russian { 0.014 } else { 0.008 }, 0.011, DARK, 8);
    }
    if russian {
        m.capsule(glass[3], glass[2], 0.018, 0.018, DARK, 8);
        for k in 0..12 {
            let p = |t: f64| {
                v3(
                    -0.16 - 0.09 * t,
                    1.08 + 0.08 * (PI * t).sin(),
                    -4.45 + 0.04 * t,
                )
            };
            m.capsule(
                p(k as f64 / 12.0),
                p((k + 1) as f64 / 12.0),
                0.015,
                0.015,
                DARK,
                6,
            );
        }
    } else {
        for side in [-1.0, 1.0] {
            let f = instrument(kind, side * 0.466, 0.78);
            m.capsule(
                f.p(0.0, -0.075, 0.03),
                f.p(0.0, 0.075, 0.03),
                0.009,
                0.009,
                Rgb(0.92, 0.68, 0.10),
                8,
            );
            for j in 0..5 {
                let at = f.p(0.0, -0.06 + j as f64 * 0.03, 0.031);
                m.capsule(at - f.y * 0.004, at + f.y * 0.004, 0.0095, 0.0095, DARK, 8);
            }
        }
    }
    // Native canopy geometry carried into the common eye frame. Sills,
    // windscreen bow, mirrors and rear bulkhead all meet the exterior.
    let offset = crate::airframe::EYE - kind.eye();
    let native = |z: f64, phi: f64| crate::hornet::canopy_point(russian, z, phi) + offset;
    let bow = crate::hornet::canopy_bow(russian);
    let aft = crate::hornet::canopy_aft(russian);
    for k in 0..24 {
        let (a, b) = (
            native(bow, k as f64 * PI / 24.0),
            native(bow, (k + 1) as f64 * PI / 24.0),
        );
        m.capsule(a, b, 0.024, 0.024, if russian { panel } else { DARK }, 8);
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
    if russian {
        // MiG-style three-pane installation: small centre panorama and two
        // angled shoulder panes on the canopy bow.
        for phi in [0.88, PI / 2.0, PI - 0.88] {
            let at = native(bow, phi);
            let tangent = v3(phi.sin(), -phi.cos(), 0.0);
            let radial = v3(phi.cos(), phi.sin(), 0.0);
            curved_mirror(
                &mut m,
                if phi == PI / 2.0 { 0.095 } else { 0.086 },
                0.035,
                &|x, y, d| at + tangent * x + radial * (y - 0.038) + V3::Z * (0.045 + d),
                DARK,
                -0.01,
            );
        }
    } else {
        // The Hornet carries two large rectangular panes, one either side of
        // the centreline, on short brackets under the windscreen bow. They
        // are deliberately much larger than the Flanker/MiG shoulder panes.
        for (i, phi) in [0.74, PI - 0.74].into_iter().enumerate() {
            let at = native(bow, phi);
            let tangent = v3(phi.sin(), -phi.cos(), 0.0);
            let radial = v3(phi.cos(), phi.sin(), 0.0);
            let centre = at + radial * -0.058 + V3::Z * 0.050;
            m.capsule(
                at + V3::Z * 0.035,
                centre + radial * 0.036 + V3::Z * 0.035,
                0.009,
                0.009,
                DARK,
                6,
            );
            curved_mirror(
                &mut m,
                0.156,
                0.070,
                &|x, y, d| at + tangent * x + radial * (y - 0.058) + V3::Z * (0.050 + d),
                DARK,
                if i == 0 { -0.42 } else { 0.42 },
            );
        }
    }
    let back_z = -2.70;
    m.block(&plain(v3(0.0, 0.63, back_z)), v3(0.49, 0.47, 0.025), wall);
    pilot::build_static_mig(&mut m);
    m
}
/// Main levers and selector respond to the same commands as the flight model.
pub(super) fn moving_controls(m: &mut Mesh, kind: Type, h: &Hands) {
    let f = instrument(kind, -0.438, 0.555);
    m.block(&f.at(0.0, 0.0, 0.012), v3(0.018, 0.048, 0.012), DARK);
    let base = f.p(0.0, mix(0.030, -0.030, h.gear), 0.024);
    let tip = base + f.z * 0.065;
    m.capsule(base, tip, 0.005, 0.005, METALC, 8);
    m.lathe(
        tip - f.z * 0.008,
        f.z,
        &[(0.0, 0.022), (0.016, 0.022), (0.016, 0.0)],
        12,
        Rgb(0.85, 0.86, 0.80),
    );
    let lamp = if h.gear_warn {
        Rgb(1.2, 0.18, 0.08)
    } else {
        Rgb(0.12, 0.06, 0.04)
    };
    m.ellipsoid(tip + f.z * 0.010, v3(0.007, 0.007, 0.004), lamp, 4, 8);
    let switch = |m: &mut Mesh, at: V3, value: f64, col: Rgb| {
        let to = at + v3(0.0, 0.024, mix(-0.010, 0.010, value));
        m.capsule(at, to, 0.0027, 0.0027, METALC, 6);
        m.ellipsoid(to, v3(0.005, 0.005, 0.008), col, 4, 8);
    };
    switch(m, v3(-0.447, 0.650, -3.805), h.flaps, DARK);
    // Speedbrake rocker on the outboard side of the moving throttle grip.
    let throttle = v3(-0.355, 0.747, -3.40 - h.throttle * 0.22);
    switch(m, throttle, h.airbrake, Rgb(0.50, 0.51, 0.48));
    if kind == Type::Su27 {
        switch(
            m,
            v3(0.445, 0.650, -3.805),
            if h.irst { 1.0 } else { 0.0 },
            DARK,
        );
    }
    let f = Frame {
        o: v3(0.445, 0.653, -3.485),
        x: V3::X,
        y: -V3::Z,
        z: V3::Y,
    };
    m.lathe(f.o, f.z, &[(0.0, 0.012), (0.012, 0.011)], 10, DARK);
    let angle = h.weapon_mode as f64 * 45.0 * DEG;
    m.capsule(
        f.p(0.0, 0.0, 0.013),
        f.p(angle.sin() * 0.009, angle.cos() * 0.009, 0.013),
        0.0012,
        0.0012,
        Rgb(0.85, 0.85, 0.77),
        4,
    );
}

pub fn hornet_instruments(
    cv: &mut Canvas,
    a: &Aircraft,
    ad: &AirData,
    m: &Mission,
    time: f64,
    c: &Combat,
) {
    draw_consoles(cv, false);
    mfd(cv, R_HSI, Page::Hsd, a, ad, m, time, c);
    let r = UFC;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x02, 0x0d, 0x08, 1.0));
    cv.text(
        "COM 1     COM 2",
        r.x + r.w * 0.5,
        r.y + 18.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.text(
        "305.000   127.500",
        r.x + r.w * 0.5,
        r.y + 39.0,
        1.2,
        GREEN,
        Align::Center,
    );
    for (i, name) in ["IFF", "TCN", "ILS", "D/L"].into_iter().enumerate() {
        cv.text(
            name,
            r.x + 12.0,
            r.y + 65.0 + i as f32 * 22.0,
            1.0,
            GREEN,
            Align::Left,
        );
    }
    for (i, name) in ["A/P", "WPT", "TIME", "MENU"].into_iter().enumerate() {
        cv.text(
            name,
            r.x + r.w - 12.0,
            r.y + 65.0 + i as f32 * 22.0,
            1.0,
            GREEN,
            Align::Right,
        );
    }
    cv.text(
        &format!("HDG {:03.0}", a.heading),
        r.x + r.w * 0.5,
        r.y + 92.0,
        1.2,
        GREEN,
        Align::Center,
    );
    cv.text(
        c.mode.label(),
        r.x + r.w * 0.5,
        r.y + 119.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.stroke_rect(r.x + 49.0, r.y + 145.0, r.w - 98.0, 24.0, 1.0, GREEN);
    let r = HUD_CONTROL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x24, 0x2a, 0x2a, 1.0));
    for (i, name) in ["HUD", "BRT", "DAY", "ATT", "ALT"].into_iter().enumerate() {
        let x = r.x + 28.0 + i as f32 * 50.0;
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
        ("NOZ", format!("{:.0}", a.nozzle)),
        ("OIL", format!("{:.0}", a.oil_psi)),
    ]
    .into_iter()
    .enumerate()
    {
        let y = r.y + 17.0 + i as f32 * 26.0;
        cv.text(name, r.x + 80.0, y, 1.0, DIM, Align::Center);
        for x in [r.x + 27.0, r.x + 130.0] {
            cv.text(&value, x, y, 1.0, GREEN, Align::Center);
        }
    }
    cv.text(
        "L   ENGINE   R",
        r.x + 80.0,
        r.y + 148.0,
        1.0,
        DIM,
        Align::Center,
    );
    cv.text("FUEL", r.x + 211.0, r.y + 20.0, 1.1, DIM, Align::Center);
    cv.text(
        &format!("{:.0}", a.fuel * 2.20462),
        r.x + 211.0,
        r.y + 48.0,
        1.6,
        GREEN,
        Align::Center,
    );
    cv.text("BINGO", r.x + 211.0, r.y + 91.0, 1.0, DIM, Align::Center);
    cv.text("2000", r.x + 211.0, r.y + 116.0, 1.4, GREEN, Align::Center);
}
pub fn flanker_instruments(cv: &mut Canvas, a: &Aircraft, c: &Combat) {
    draw_consoles(cv, true);
    let r = FLANKER_WEAPONS;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x24, 0x36, 0x39, 1.0));
    let weapon = symbology::weapon_line(c);
    cv.text(
        &weapon,
        r.x + r.w * 0.5,
        r.y + 19.0,
        1.0,
        GREEN,
        Align::Center,
    );
    cv.text(
        if c.irst {
            "OEPS-27 / IRST"
        } else {
            "N001 / RLS"
        },
        r.x + 12.0,
        r.y + 46.0,
        1.0,
        WHITE,
        Align::Left,
    );
    cv.text(
        if c.lock.is_some() { "ZAKHVAT" } else { "POISK" },
        r.x + r.w - 12.0,
        r.y + 46.0,
        1.0,
        GREEN,
        Align::Right,
    );
    let r = FLANKER_SYSTEMS;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x32, 0x48, 0x4b, 1.0));
    for (i, (name, on)) in [
        ("SHASSI", a.gear > 0.98),
        ("ZAKR", a.flaps > 0.05),
        ("TORM", a.brake > 0.05),
    ]
    .into_iter()
    .enumerate()
    {
        let x = r.x + 32.0 + i as f32 * 64.0;
        cv.text(name, x, r.y + 15.0, 1.0, WHITE, Align::Center);
        cv.disc(
            x,
            r.y + 42.0,
            7.0,
            if on {
                GREEN
            } else {
                rgba(0x12, 0x24, 0x1b, 1.0)
            },
        );
    }
    let r = FLANKER_WARN;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x11, 0x19, 0x1a, 1.0));
    for (i, (name, on)) in [
        ("POZHAR", a.battle.fire),
        ("MASLO", a.oil_psi < 20.0),
        ("TOPLIVO", a.fuel < 800.0),
        ("SHASSI", a.gear > 0.02 && a.gear < 0.98),
        ("TORMOZ", a.brake > 0.1),
        ("ALFA", a.alpha_deg > 22.0),
    ]
    .into_iter()
    .enumerate()
    {
        let height = r.h / 6.0;
        let y = r.y + i as f32 * height;
        cv.rect(
            r.x + 3.0,
            y + 2.0,
            r.w - 6.0,
            height - 4.0,
            if on {
                AMBER
            } else {
                rgba(0x16, 0x22, 0x1b, 1.0)
            },
        );
        cv.text(
            name,
            r.x + r.w * 0.5,
            y + height * 0.5,
            1.0,
            if on { rgba(0x12, 0x10, 0x08, 1.0) } else { DIM },
            Align::Center,
        );
    }
    let r = HUD_CONTROL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x1b, 0x22, 0x22, 1.0));
    for (i, name) in ["СЕТКА", "ЯРК", "НОЧЬ", "ДЕНЬ"].into_iter().enumerate() {
        let x = r.x + 32.0 + i as f32 * 64.0;
        cv.text(name, x, r.y + 18.0, 1.0, WHITE, Align::Center);
        cv.circle(x, r.y + 56.0, 12.0, 2.0, DIM);
        cv.line(x, r.y + 56.0, x + 4.0, r.y + 45.0, 2.0, WHITE);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hornet_has_two_large_mirrors_while_flanker_keeps_three_small_panes() {
        for kind in [Type::F18, Type::Su27] {
            let m = build(kind);
            let glass: Vec<_> =
                m.v.chunks(crate::mesh::STRIDE)
                    .filter(|v| {
                        let x = v[9] as f64 * AW;
                        let y = (1.0 - v[10] as f64) * A;
                        x >= R_MIRROR.x as f64
                            && x <= (R_MIRROR.x + R_MIRROR.w) as f64
                            && y >= R_MIRROR.y as f64
                            && y <= (R_MIRROR.y + R_MIRROR.h) as f64
                    })
                    .collect();
            assert!(glass.len() > 100);
            assert!(glass.iter().any(|v| v[0] < -0.20));
            assert!(glass.iter().any(|v| v[0] > 0.20));
            assert_eq!(glass.iter().any(|v| v[0].abs() < 0.10), kind == Type::Su27);
            let width = |side: f32| {
                let mut lo = f32::INFINITY;
                let mut hi = f32::NEG_INFINITY;
                for v in glass.iter().filter(|v| side * v[0] > 0.20) {
                    lo = lo.min(v[0]);
                    hi = hi.max(v[0]);
                }
                hi - lo
            };
            for side in [-1.0, 1.0] {
                assert!(width(side) > if kind == Type::F18 { 0.09 } else { 0.04 });
            }
        }
    }

    #[test]
    fn moving_controls_follow_gear_flaps_brake_sensor_and_weapon_state() {
        for kind in [Type::F18, Type::Su27] {
            let initial = Hands::default();
            let mut base = Mesh::new();
            moving_controls(&mut base, kind, &initial);
            for changed in [
                Hands {
                    gear: 1.0,
                    ..initial
                },
                Hands {
                    flaps: 1.0,
                    ..initial
                },
                Hands {
                    airbrake: 1.0,
                    ..initial
                },
                Hands {
                    weapon_mode: 2,
                    ..initial
                },
            ] {
                let mut m = Mesh::new();
                moving_controls(&mut m, kind, &changed);
                assert_ne!(m.v, base.v, "unresponsive control {kind:?}");
                assert!(m.v.iter().all(|v| v.is_finite()));
            }
            let mut sensor = Mesh::new();
            moving_controls(
                &mut sensor,
                kind,
                &Hands {
                    irst: true,
                    ..initial
                },
            );
            assert_eq!(
                sensor.v == base.v,
                kind == Type::F18,
                "IRST selector on wrong aircraft"
            );
        }
    }

    #[test]
    fn active_instrument_and_console_atlas_regions_do_not_overlap() {
        for kind in [Type::F18, Type::Su27] {
            let mut rects: Vec<_> = layout(kind).iter().map(|i| i.rect).collect();
            rects.push(R_MIRROR);
            for side in [-1.0, 1.0] {
                for row in 0..6 {
                    rects.push(console_rect(side, row));
                }
            }
            for (i, a) in rects.iter().enumerate() {
                assert!(
                    a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= AW as f32 && a.y + a.h <= A as f32
                );
                for b in &rects[i + 1..] {
                    // Identical tiles can be reused (the Hornet caution eyebrows).
                    if a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h {
                        continue;
                    }
                    assert!(
                        a.x + a.w <= b.x
                            || b.x + b.w <= a.x
                            || a.y + a.h <= b.y
                            || b.y + b.h <= a.y,
                        "atlas faces overlap in {kind:?}: {},{} and {},{}",
                        a.x,
                        a.y,
                        b.x,
                        b.y
                    );
                }
            }
        }
    }

    #[test]
    fn instrument_images_keep_their_native_pixel_aspect() {
        for kind in [Type::F18, Type::Su27] {
            for item in layout(kind) {
                let (w, h) = face_size(&item);
                assert!(
                    (w / h - item.rect.w as f64 / item.rect.h as f64).abs() < 1e-9,
                    "stretched face {kind:?} at {},{}",
                    item.x,
                    item.y
                );
                assert!(w <= item.w + 1e-9 && h <= item.h + 1e-9);
            }
        }
    }

    #[test]
    fn fixed_readouts_fit_inside_the_model_specific_hud_glass() {
        for (kind, ys) in [
            (Type::F18, [-62.0, 0.0, 119.0]),
            (Type::Su27, [-84.0, 0.0, 81.0]),
        ] {
            let g = hud_glass(kind);
            let normal = (g[1] - g[0]).cross(g[3] - g[0]).norm();
            for y in ys {
                for x in [-79.0, 0.0, 79.0] {
                    let (x, y) = fixed_hud_coords(kind, x, y);
                    let ray = v3(x / 650.0, -y / 650.0, -1.0);
                    let eye = crate::airframe::EYE;
                    let p = eye + ray * ((g[0] - eye).dot(normal) / ray.dot(normal));
                    let t = (p.y - g[0].y) / (g[3].y - g[0].y);
                    let half = mix(g[1].x, g[2].x, t);
                    assert!(
                        t > 0.015 && t < 0.985 && p.x.abs() < half - 0.005,
                        "clipped HUD readout {kind:?}: {p:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn instrument_faces_are_visible_and_do_not_overlap() {
        for kind in [Type::F18, Type::Su27] {
            let m = build(kind);
            assert!(m.count() > 1000 && m.v.iter().all(|x| x.is_finite()));
            let items = layout(kind);
            for (i, item) in items.iter().enumerate() {
                let f = mount(kind, &item);
                assert!(f.x.dot(f.y).abs() < 1e-8 && (f.z.len() - 1.0).abs() < 1e-8);
                let (w, h, depth) = match item.shape {
                    Shape::Dial => (item.w, item.h, 0.0135),
                    Shape::Display(_) => {
                        let (w, h) = face_size(item);
                        (w, h, 0.006)
                    }
                    Shape::Label => {
                        let (w, h) = face_size(item);
                        (w, h, 0.0085)
                    }
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
