//! MiG-29S (9.13S) cockpit, after photos of 9.12 cockpits on Wikimedia
//! Commons, which the 9.13 kept nearly unchanged (see
//! docs/MIG29-REFERENCES.md): a narrow tub painted blue-grey,
//! an analogue panel of black-faced round instruments with the attitude
//! director (KPP) over the horizontal situation indicator (PNP) in the
//! left flight group, airspeed and altimeter on the left, the variometer and engine
//! gauges on the right; the ILS-31 head-up display on a black coaming with
//! the IT-23 radar/TV screen beside it, the column of warning lamps along
//! the right sill, a centre stick and a twin-lever throttle.
//!
//! It is built in the F-16 cockpit's space, with the design eye at
//! airframe::EYE, so the seat, the pilot and the head-up display are
//! shared; `OFFSET` takes the MiG's body axes there.
use super::{adi, gauge, gauge_face, pad, plate, r, screen, spoke, spoke_text, POINTER, R, RED};
use crate::air::AirData;
use crate::canvas::{rgba, Align, Canvas, Col};
use crate::combat::Combat;
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
use crate::sim::Aircraft;

/// From the MiG's body axes to the shared cockpit space.
pub const OFFSET: V3 = v3(crate::airframe::EYE.x - crate::mig29::EYE.x, crate::airframe::EYE.y - crate::mig29::EYE.y, crate::airframe::EYE.z - crate::mig29::EYE.z);

/// Paint: the blue-grey of the 9.12's panels and tub, a darker console
/// top, and the black coaming.
const PANEL: Rgb = Rgb(0.36, 0.43, 0.47);
const WALL: Rgb = Rgb(0.31, 0.37, 0.40);
const CONSOLE: Rgb = Rgb(0.33, 0.39, 0.43);
const COAMING: Rgb = Rgb(0.07, 0.075, 0.08);
const FRAME: Rgb = Rgb(0.28, 0.32, 0.33);
const KNOB: Rgb = Rgb(0.06, 0.06, 0.065);
const METAL: Rgb = Rgb(0.55, 0.57, 0.56);

/// Console tops, the tub wall and floor.
const CONSOLE_Y: f64 = 0.62;
const CONSOLE_IN: f64 = 0.262;
const WALL_X: f64 = 0.40;
// Below the soles and pedal blocks, including full rudder travel.
const FLOOR: f64 = 0.16;
/// Top of the throttle quadrant housing on the left console.
pub(super) const QUADRANT_TOP: f64 = 0.655;
/// Bulkhead behind the headrest.
const BULKHEAD_Z: f64 = -2.75;

// Atlas faces below the F-16's instruments and beside the mirror texture.
const R_ADI: R = r(0.0, 1280.0, 192.0, 192.0);
const R_HSI: R = r(192.0, 1280.0, 192.0, 192.0);
const R_ASI: R = r(384.0, 1280.0, 160.0, 160.0);
const R_ALT: R = r(544.0, 1280.0, 160.0, 160.0);
const R_VVI: R = r(704.0, 1280.0, 160.0, 160.0);
const R_UUAP: R = r(864.0, 1280.0, 160.0, 160.0);
const R_RPM: R = r(0.0, 1472.0, 160.0, 160.0);
const R_EGT: R = r(160.0, 1472.0, 160.0, 160.0);
const R_FUEL: R = r(1664.0, 1280.0, 96.0, 320.0);
const R_HYD: R = r(1760.0, 1280.0, 224.0, 96.0);
const R_CLOCK: R = r(608.0, 1472.0, 128.0, 128.0);
const R_WARN: R = r(736.0, 1472.0, 96.0, 320.0);
const R_GEAR: R = r(832.0, 1472.0, 192.0, 128.0);
const R_TV: R = r(0.0, 1664.0, 256.0, 208.0);
const R_LEFT: R = r(256.0, 1664.0, 224.0, 240.0);
const R_COMPASS: R = r(480.0, 1664.0, 128.0, 128.0);
const R_EGT_R: R = r(1024.0, 1472.0, 160.0, 160.0);
const R_WEAPON: R = r(1024.0, 1664.0, 224.0, 160.0);
const R_EKRAN: R = r(1248.0, 1664.0, 160.0, 160.0);
const R_SPO: R = r(1408.0, 1664.0, 160.0, 160.0);
const R_RADALT: R = r(1568.0, 1664.0, 160.0, 160.0);
const R_ASIMACH: R = r(1728.0, 1664.0, 160.0, 160.0);
const R_COUNTER: R = r(1888.0, 1664.0, 160.0, 96.0);

// ================================================================ panel
/// Main panel: tilted back 12 degrees, its foot 0.66 m above the datum just
/// ahead of the knees; the outer parts fold 22 degrees toward the pilot.
const TILT: f64 = 12.0 * DEG;
const HINGE: f64 = 0.29;
const CANT: f64 = 22.0 * DEG;
const BASE: f64 = 0.66;
// Leave room between the unchanged pilot's knees and the flight instruments.
// Only the panel moves; HUD glass, coaming and canopy keep their eye datums.
const PANEL_FORWARD: f64 = 0.15;

fn panel() -> Frame {
    Frame { o: v3(0.0, BASE, -3.95 - PANEL_FORWARD), x: V3::X, y: v3(0.0, TILT.cos(), -TILT.sin()), z: v3(0.0, TILT.sin(), TILT.cos()) }
}
fn pv(y: f64) -> f64 {
    (y - BASE) / TILT.cos()
}
fn wing_axes(s: f64) -> (V3, V3) {
    let p = panel();
    let x = p.x * CANT.cos() + p.z * (s * CANT.sin());
    (x, x.cross(p.y))
}
fn pp(u: f64, v: f64, w: f64) -> V3 {
    let p = panel();
    if u.abs() <= HINGE {
        return p.p(u, v, w);
    }
    let s = u.signum();
    let (x, n) = wing_axes(s);
    p.p(s * HINGE, v, 0.0) + x * (u - s * HINGE) + n * w
}
fn panel_at(u: f64, y: f64) -> Frame {
    if u.abs() <= HINGE {
        return panel().at(u, pv(y), 0.0);
    }
    let (x, n) = wing_axes(u.signum());
    Frame { o: pp(u, pv(y), 0.0), x, y: panel().y, z: n }
}

/// Panel outline in face coordinates: the centre up to 1.0 m, with shallow
/// shoulders falling to 0.98 m below the raised side equipment.
fn outline() -> [(f64, f64); 8] {
    let (top, side) = (pv(1.0), pv(0.98));
    [(-0.38, 0.0), (-HINGE, 0.0), (HINGE, 0.0), (0.38, 0.0), (0.38, side), (HINGE, top), (-HINGE, top), (-0.38, side)]
}

// Figure 15 of the Eagle Dynamics MiG-29 manual, checked against real
// cockpit photographs. KPP/PNP sit LEFT of the HUD axis; the engine group
// has two separate temperature gauges below the twin-pointer tachometer.
// Positions are visual proportions in metres, not surveyed dimensions.
#[derive(Clone, Copy)]
struct Dial { u: f64, y: f64, radius: f64, face: R }
const DIALS: [Dial; 12] = [
    Dial { u: -0.105, y: 0.905, radius: 0.049, face: R_ADI },
    Dial { u: -0.105, y: 0.770, radius: 0.045, face: R_HSI },
    Dial { u: -0.235, y: 0.910, radius: 0.038, face: R_ASI },
    Dial { u: -0.235, y: 0.800, radius: 0.038, face: R_ALT },
    Dial { u: 0.020, y: 0.910, radius: 0.034, face: R_VVI },
    Dial { u: 0.120, y: 0.910, radius: 0.033, face: R_RADALT },
    Dial { u: 0.220, y: 0.910, radius: 0.034, face: R_RPM },
    Dial { u: 0.020, y: 0.800, radius: 0.030, face: R_ASIMACH },
    Dial { u: 0.130, y: 0.800, radius: 0.030, face: R_EGT },
    Dial { u: 0.225, y: 0.800, radius: 0.030, face: R_EGT_R },
    Dial { u: 0.020, y: 0.705, radius: 0.026, face: R_CLOCK },
    Dial { u: 0.335, y: 0.715, radius: 0.029, face: R_SPO },
];

/// Right-handed frame on the console surface, with the panel top toward
/// the nose. Both sides have the same roll as their supporting console.
fn console_at(s: f64, x: f64, z: f64) -> Frame {
    let across = v3(1.0, s * 0.14, 0.0).norm();
    let y = -V3::Z;
    Frame { o: v3(s * x, CONSOLE_Y + (x - CONSOLE_IN) * 0.14, z), x: across, y, z: across.cross(y) }
}

/// Canopy point in cockpit space (see mig29::canopy_point).
fn cp(z: f64, phi: f64) -> V3 {
    crate::mig29::canopy_point(z - OFFSET.z, phi) + OFFSET
}

/// Sill on side `s` at cockpit `z`.
fn sill(s: f64, z: f64) -> V3 {
    cp(z, if s > 0.0 { 0.0 } else { PI })
}

fn raised_at(u: f64, y: f64) -> Frame {
    let mut f = panel_at(u, y);
    f.o.z += PANEL_FORWARD + 0.060;
    f
}

fn warning_frame() -> Frame {
    let n = v3(-0.85, 0.15, 0.50).norm();
    let y = (V3::Y - n * n.y).norm();
    Frame { o: v3(0.350, 0.850, -3.86), x: y.cross(n), y, z: n }
}

pub fn build_static(m: &mut Mesh) {
    let o = outline();
    let face = |ix: &[usize]| -> Vec<V3> { ix.iter().map(|&i| pp(o[i].0, o[i].1, 0.0)).collect() };
    m.polygon(&face(&[1, 2, 5, 6]), PANEL);
    m.polygon(&face(&[2, 3, 4, 5]), PANEL.scale(0.95));
    m.polygon(&face(&[0, 1, 6, 7]), PANEL.scale(0.95));
    for k in 0..o.len() {
        let (a, b) = (o[k], o[(k + 1) % o.len()]);
        m.quad(pp(a.0, a.1, 0.0), pp(b.0, b.1, 0.0), pp(b.0, b.1, -0.15), pp(a.0, a.1, -0.15), PANEL.scale(0.6));
    }
    // Black coaming from the panel top forward to the windscreen.
    let top = [o[3], o[4], o[5], o[6], o[7], o[0]];
    let hood = |u: f64, v: f64, t: f64| pp(u * (1.0 - 0.10 * t), v + 0.015 + 0.06 * t, 0.04 - 0.44 * t) + V3::Z * PANEL_FORWARD;
    for k in 1..top.len() - 2 {
        let (a, b) = (top[k], top[k + 1]);
        for i in 0..3 {
            let (t0, t1) = (i as f64 / 3.0, (i + 1) as f64 / 3.0);
            m.quad(hood(a.0, a.1, t0), hood(b.0, b.1, t0), hood(b.0, b.1, t1), hood(a.0, a.1, t1), COAMING);
        }
        m.quad(pp(a.0, a.1, 0.0), pp(b.0, b.1, 0.0), hood(b.0, b.1, 0.0), hood(a.0, a.1, 0.0), COAMING.scale(1.4));
        m.capsule(hood(a.0, a.1, 0.0), hood(b.0, b.1, 0.0), 0.012, 0.012, COAMING.scale(2.0), 6);
    }

    // ILS-31 head-up display: the big body on the coaming, the collimator
    // box and a heavy frame round the combiner glass (cockpit::HUD_GLASS).
    let up = Frame { o: v3(0.0, 1.035, -4.17), x: V3::X, y: V3::Y, z: V3::Z };
    m.block(&up, v3(0.125, 0.05, 0.15), COAMING.scale(1.5));
    m.block(&Frame { o: v3(0.0, 1.072, -4.05), ..up }, v3(0.11, 0.018, 0.05), COAMING.scale(2.2));
    for s in [-1.0, 1.0] {
        let (b, t) = (v3(s * 0.118, 1.06, -4.025), v3(s * 0.118, 1.365, -3.955));
        crate::cockpit::pilot::bar(m, b, t, 0.012, 0.016, V3::Z, COAMING.scale(1.8));
    }
    crate::cockpit::pilot::bar(m, v3(-0.13, 1.37, -3.955), v3(0.13, 1.37, -3.955), 0.010, 0.012, V3::Z, COAMING.scale(1.8));

    // IT-23 screen in its hood right of the head-up display.
    let tv = Frame { o: v3(0.245, 1.07, -4.08), x: V3::X, y: v3(0.0, (20.0 * DEG).cos(), -(20.0 * DEG).sin()), z: v3(0.0, (20.0 * DEG).sin(), (20.0 * DEG).cos()) };
    m.block(&tv, v3(0.075, 0.062, 0.10), COAMING.scale(1.3));
    m.block(&tv.at(0.0, 0.07, 0.07), v3(0.078, 0.008, 0.04), COAMING.scale(1.1));
    screen(m, &tv.at(0.0, 0.0, 0.1005), 0.115, 0.093, R_TV);

    // Primary flight group to port; systems and engines to starboard.
    for d in DIALS {
        if d.face.x == R_SPO.x && d.face.y == R_SPO.y {
            plate(m, &panel_at(d.u, d.y), 0.060, 0.062, d.face);
        } else {
            gauge(m, &panel_at(d.u, d.y), d.radius, d.face);
        }
    }
    plate(m, &panel_at(-0.235, 0.696), 0.090, 0.045, R_GEAR);
    plate(m, &panel_at(0.178, 0.695), 0.150, 0.040, R_HYD);
    plate(m, &panel_at(0.330, 0.877), 0.037, 0.178, R_FUEL);

    // Raised left cheek beside the HUD: UUAP, caution lamp and weapons
    // controls, supported by a blue-grey backing rather than floating.
    let cheek = raised_at(-0.205, 1.052);
    m.block(&cheek.at(0.0, 0.0, -0.015), v3(0.052, 0.058, 0.015), PANEL);
    gauge(m, &cheek, 0.031, R_UUAP);
    m.capsule(cheek.p(0.0, 0.047, 0.002), cheek.p(0.0, 0.047, 0.012), 0.009, 0.009, Rgb(0.6, 0.12, 0.08), 10);
    let weapons = raised_at(-0.320, 1.060);
    m.block(&weapons.at(0.0, 0.0, -0.012), v3(0.057, 0.043, 0.012), PANEL);
    plate(m, &weapons, 0.100, 0.065, R_WEAPON);

    // Ekran beside the engine group and the warning column under the rail.
    let warning = warning_frame();
    plate(m, &warning, 0.052, 0.225, R_WARN);
    let ekran = raised_at(0.350, 1.025);
    m.block(&ekran.at(0.0, -0.010, -0.018), v3(0.032, 0.032, 0.018), PANEL);
    plate(m, &ekran, 0.050, 0.045, R_EKRAN);
    // Countermeasure panel below the ILS-31 housing, as on the reference.
    let counter = raised_at(0.0, 1.035);
    m.block(&counter.at(0.0, 0.0, -0.020), v3(0.100, 0.032, 0.020), PANEL);
    plate(m, &counter, 0.180, 0.048, R_COUNTER);

    // Tub: floor, walls up to the sills, the canopy rails and the consoles.
    m.quad(v3(-WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, BULKHEAD_Z), v3(-WALL_X, FLOOR, BULKHEAD_Z), WALL.scale(0.5));
    m.quad(v3(-WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, -4.45), v3(WALL_X, BASE, panel().o.z - 0.02), v3(-WALL_X, BASE, panel().o.z - 0.02), WALL.scale(0.45));
    let nz = 12;
    for s in [-1.0, 1.0] {
        for k in 0..nz {
            let (z0, z1) = (mix(-4.30, BULKHEAD_Z, k as f64 / nz as f64), mix(-4.30, BULKHEAD_Z, (k + 1) as f64 / nz as f64));
            let (a, b) = (sill(s, z0), sill(s, z1));
            m.quad(v3(s * WALL_X, FLOOR, z0), v3(s * WALL_X, FLOOR, z1), v3(s * WALL_X, CONSOLE_Y, z1), v3(s * WALL_X, CONSOLE_Y, z0), WALL);
            m.quad(v3(s * WALL_X, CONSOLE_Y, z0), v3(s * WALL_X, CONSOLE_Y, z1), b - V3::Y * 0.02, a - V3::Y * 0.02, WALL.scale(1.1));
            crate::cockpit::pilot::bar(m, a - V3::Y * 0.01, b - V3::Y * 0.01, 0.03, 0.016, V3::Y, FRAME.scale(0.7));
        }
        // Panel wing edge out to the wall, under the coaming.
        let e = pp(s * 0.38, 0.0, 0.0);
        let et = pp(s * 0.38, pv(0.98), 0.0);
        let st = sill(s, et.z);
        m.polygon(&[e, v3(s * WALL_X, e.y, e.z), v3(s * WALL_X, st.y - 0.02, et.z), et], PANEL.scale(0.8));
        // Console: top, inboard face and front face down to the floor.
        let (zf, za) = (-3.85, BULKHEAD_Z);
        let c = |x: f64, z: f64| v3(s * x, CONSOLE_Y + (x - CONSOLE_IN) * 0.14, z);
        m.quad(c(CONSOLE_IN, zf), c(WALL_X, zf), c(WALL_X, za), c(CONSOLE_IN, za), CONSOLE);
        m.quad(c(CONSOLE_IN, zf), c(CONSOLE_IN, za), v3(s * CONSOLE_IN, FLOOR, za), v3(s * CONSOLE_IN, FLOOR, zf), WALL.scale(0.7));
        m.quad(c(CONSOLE_IN, zf), c(WALL_X, zf), v3(s * WALL_X, FLOOR, zf), v3(s * CONSOLE_IN, FLOOR, zf), WALL.scale(0.6));
        // Close the gap from the console nose to the lower panel, following
        // the same parallel inboard edge and the panel's folded outer edge.
        m.quad(c(CONSOLE_IN, zf), c(WALL_X, zf), e, pp(s * CONSOLE_IN, 0.0, 0.0), PANEL.scale(0.9));
        if s < 0.0 {
            plate(m, &console_at(s, 0.330, -3.77), 0.104, 0.115, R_LEFT);
        }
        // Switch rows and knobs, in the photos' rhythm of small black
        // panels with toggles.
        let rows = if s < 0.0 { [-3.14, -2.99, -2.84] } else { [-3.76, -3.60, -3.44] };
        for (i, z) in rows.iter().enumerate() {
            let f = console_at(s, 0.330, *z);
            m.block(&f.at(0.0, 0.0, 0.003), v3(0.055, 0.055, 0.003), KNOB.scale(2.0));
            for j in 0..4 {
                let p = f.p(-0.045 + j as f64 * 0.030, 0.0, 0.006);
                if (i + j) % 3 == 0 {
                    m.capsule(p, p + f.z * 0.015, 0.008, 0.008, KNOB, 6);
                } else {
                    m.capsule(p, p + f.z * 0.018 - f.y * 0.006, 0.0025, 0.002, METAL, 5);
                }
            }
        }
    }
    // Throttle quadrant on the left console, with its slot.
    let q = Frame { o: v3(-0.315, (CONSOLE_Y + QUADRANT_TOP) * 0.5 + 0.01, -3.465), x: V3::X, y: V3::Y, z: V3::Z };
    m.block(&q, v3(0.04, (QUADRANT_TOP - CONSOLE_Y) * 0.5 + 0.01, 0.205), CONSOLE.scale(0.8));
    m.quad(v3(-0.335, QUADRANT_TOP + 0.001, -3.64), v3(-0.295, QUADRANT_TOP + 0.001, -3.64), v3(-0.295, QUADRANT_TOP + 0.001, -3.28), v3(-0.335, QUADRANT_TOP + 0.001, -3.28), Rgb(0.02, 0.02, 0.022));

    // Bulkhead behind the seat, and the deck under the rear of the canopy.
    let nphi = 10;
    let ring = |z: f64, lift: f64| -> Vec<V3> {
        (0..=nphi).map(|k| {
            let p = cp(z, k as f64 / nphi as f64 * PI);
            let base = sill(1.0, z).y;
            v3(p.x, base + (p.y - base) * lift, z)
        }).collect()
    };
    let mut wall = vec![v3(WALL_X, FLOOR, BULKHEAD_Z)];
    wall.extend(ring(BULKHEAD_Z, 0.35));
    wall.push(v3(-WALL_X, FLOOR, BULKHEAD_Z));
    m.polygon(&wall, WALL.scale(0.8));
    let zs = [BULKHEAD_Z, -2.3, -1.95, -1.62];
    let lifts = [0.35, 0.55, 0.75, 0.97];
    for i in 0..zs.len() - 1 {
        let (a, b) = (ring(zs[i], lifts[i]), ring(zs[i + 1], lifts[i + 1]));
        for k in 0..nphi {
            m.quad(a[k], a[k + 1], b[k + 1], b[k], WALL.scale(0.6));
        }
    }

    // Canopy frames: the windscreen posts, the heavy arch over the knees
    // with its three rear-view mirrors, and the sill frames.
    let (zw, za) = (crate::mig29::CANOPY_Z[0] + OFFSET.z, (crate::mig29::CANOPY_Z[2] + crate::mig29::CANOPY_Z[3]) * 0.5 + OFFSET.z);
    let post = (crate::mig29::CANOPY_POST.0 + crate::mig29::CANOPY_POST.1) * 0.5;
    for phi in [post, PI - post] {
        // Set in as deep as the arch and stopped inside it, so no end shows.
        let z_of = |k: usize| mix(zw, za - 0.012, k as f64 / 4.0);
        let pts: Vec<V3> = (0..=4).map(|k| cp(z_of(k), phi) - inward(cp(z_of(k), phi), 0.025)).collect();
        crate::cockpit::pilot::band(m, &pts, 0.008, 0.016, (crate::airframe::EYE - pts[2]).norm(), FRAME);
    }
    let arch: Vec<V3> = (0..=nphi).map(|k| {
        let p = cp(za, k as f64 / nphi as f64 * PI);
        p - inward(p, 0.025)
    }).collect();
    crate::cockpit::pilot::band(m, &arch, 0.012, 0.022, V3::Z, FRAME);
    // Three panes on the pilot-facing side of the bow, beyond the canopy's
    // full painted arch thickness. Their lower rims overlap its front face
    // instead of sitting inside the bow and appearing behind it.
    for (lo, hi) in [(1.08, PI - 1.08), (0.30, 1.00), (PI - 1.00, PI - 0.30)] {
        let z0 = za + 0.025;
        let mid = (lo + hi) * 0.5;
        let radius = (cp(z0, mid + 0.001) - cp(z0, mid - 0.001)).len() / 0.002;
        let h = 0.075;
        let place = |x: f64, y: f64, d: f64| {
            let phi = mid - x / radius;
            let p = cp(z0, phi);
            let tangent = (cp(z0, phi - 0.001) - cp(z0, phi + 0.001)).norm();
            let n = (crate::airframe::EYE - p).norm();
            let up = n.cross(tangent).norm();
            // The pane lies outside the arch in the pilot's view instead
            // of hanging across the forward view below it.
            p - inward(p, 0.018) + up * (y + h * 0.35) + n * d
        };
        let centre = place(0.0, 0.0, 0.0);
        let anchor = cp(za, mid) - inward(cp(za, mid), 0.025);
        let up = (centre - anchor).norm();
        let mut mount = anchor + up * 0.012;
        mount.z = crate::mig29::CANOPY_Z[3] + OFFSET.z + 0.035;
        let smaller = |x: f64, y: f64, d: f64| mount + (place(x, y, d) - centre) * (0.78 * 0.70);
        super::curved_mirror(m, (hi - lo) * radius, h, &smaller, COAMING.scale(1.8), (mid - PI / 2.0).sin());
        for t in [-0.28, 0.28] {
            let x = t * (hi - lo) * radius;
            let phi = mid - x / radius;
            let p = cp(za, phi);
            crate::cockpit::pilot::bar(m, p - inward(p, 0.025), smaller(x, -h * 0.4, -0.018), 0.006, 0.006, V3::Z, FRAME);
        }
    }

    // KI-13 standby compass on its bracket at the top of the right
    // windscreen post, the bowl's window turned to the pilot.
    let (q, p) = (mix(zw, za, 0.9), post + 0.30);
    let mount = cp(q, p) - inward(cp(q, p), 0.03);
    let c = mount - inward(mount, 0.06) + V3::Z * 0.035;
    let n = (crate::airframe::EYE - c).norm();
    let x = V3::Y.cross(n).norm();
    let f = Frame { o: c, x, y: n.cross(x), z: n };
    crate::cockpit::pilot::bar(m, mount, f.p(0.0, 0.0, -0.03), 0.008, 0.008, V3::Z, FRAME);
    m.lathe(f.p(0.0, 0.0, -0.04), n, &[(0.0, 0.0), (0.0, 0.03), (0.01, 0.034), (0.04, 0.036), (0.04, 0.0)], 10, KNOB.scale(1.6));
    m.ring(&f.at(0.0, 0.0, 0.003), 0.028, 0.036, 0.006, KNOB.scale(2.2), 12);
    m.disc_uv(&f.at(0.0, 0.0, 0.002), 0.028, super::uv(R_COMPASS), 16);

    crate::cockpit::pilot::build_static_mig(m);
    m.facet();
}

/// Offset `d` from the canopy surface point `p` toward the cockpit's axis.
fn inward(p: V3, d: f64) -> V3 {
    let c = v3(0.0, sill(1.0, p.z).y, p.z);
    (p - c).norm() * d
}

// ================================================================ faces
const INK: Col = rgba(0xe6, 0xe6, 0xdc, 1.0);
const NEEDLE: Col = rgba(0xf2, 0xf0, 0xe4, 1.0);
const LAMP_OFF: Col = rgba(0x2a, 0x26, 0x14, 1.0);
const LAMP_ON: Col = rgba(0xf0, 0xc8, 0x3c, 1.0);
const LAMP_RED: Col = rgba(0xff, 0x50, 0x3c, 1.0);
const GEAR_GREEN: Col = rgba(0x5c, 0xf0, 0x6a, 1.0);

/// Black dial face: angle from the scale fraction, clockwise from
/// `a0` degrees (canvas, 0 at three o'clock) through `sweep`.
struct Scale {
    x: f32,
    y: f32,
    rad: f32,
    a0: f64,
    sweep: f64,
}

impl Scale {
    fn new(cv: &mut Canvas, r: R, a0: f64, sweep: f64) -> Scale {
        let (x, y, rad) = gauge_face(cv, r);
        Scale { x, y, rad, a0, sweep }
    }
    fn ang(&self, t: f64) -> f64 {
        (self.a0 + self.sweep * clamp(t, 0.0, 1.0)) * DEG
    }
    /// `n` major ticks with their labels, `minor` ticks between each pair.
    fn ticks(&self, cv: &mut Canvas, n: usize, minor: usize, label: impl Fn(usize) -> String) {
        let steps = n * (minor + 1);
        for k in 0..=steps {
            let t = k as f64 / steps as f64;
            let major = k % (minor + 1) == 0;
            spoke(cv, self.x, self.y, self.ang(t), self.rad * if major { 0.74 } else { 0.83 }, self.rad * 0.92, if major { 2.0 } else { 1.0 }, INK);
            if major {
                spoke_text(cv, self.x, self.y, self.ang(t), self.rad * 0.58, &label(k / (minor + 1)), INK);
            }
        }
    }
    fn needle(&self, cv: &mut Canvas, t: f64, len: f32, w: f32, col: Col) {
        spoke(cv, self.x, self.y, self.ang(t), -self.rad * 0.15, self.rad * len, w, col);
    }
    fn hub(&self, cv: &mut Canvas) {
        cv.disc(self.x, self.y, 5.0, rgba(0x30, 0x32, 0x30, 1.0));
    }
}

/// Bearing of a horizontal vector, degrees clockwise from north (-z).
fn bearing(v: V3) -> f64 {
    v.x.atan2(-v.z) / DEG
}

/// Small physical panels and the SPO-15 bearing lamps. The latter uses
/// actual radar emitters, rather than a decorative threat picture.
fn system_faces(cv: &mut Canvas, a: &Aircraft, c: &Combat) {
    let r = R_WEAPON;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x18, 0x1c, 0x1c, 1.0));
    for (i, label) in ["SUV", "RLS", "OLS"].iter().enumerate() {
        let x = r.x + 38.0 + i as f32 * 74.0;
        cv.text(label, x, r.y + 20.0, 1.0, INK, Align::Center);
        cv.circle(x, r.y + 57.0, 19.0, 2.0, INK);
        cv.line(x, r.y + 57.0, x + 9.0, r.y + 44.0, 4.0, INK);
        let lit = i == 0 || (i == 2) == c.irst;
        cv.rect(x - 18.0, r.y + 100.0, 36.0, 18.0, if lit { LAMP_ON } else { LAMP_OFF });
    }
    let r = R_EKRAN;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x12, 0x14, 0x14, 1.0));
    cv.text("EKRAN", r.x + r.w * 0.5, r.y + 14.0, 1.0, INK, Align::Center);
    cv.stroke_rect(r.x + 15.0, r.y + 32.0, r.w - 30.0, 65.0, 2.0, rgba(0x40, 0x48, 0x3c, 1.0));
    let warning = if a.battle.fire { "POZHAR" } else if a.fuel < 800.0 { "TOPLIVO" } else { "" };
    cv.text(warning, r.x + r.w * 0.5, r.y + 63.0, 1.0, LAMP_ON, Align::Center);
    cv.text("VYZOV", r.x + r.w * 0.5, r.y + 127.0, 1.0, INK, Align::Center);

    let (x, y, rad) = gauge_face(cv, R_SPO);
    // Aircraft silhouette surrounded by discrete bearing lamps.
    cv.polyline(&[[x, y - 28.0], [x - 6.0, y + 22.0], [x, y + 12.0], [x + 6.0, y + 22.0]], 2.0, true, INK);
    cv.line(x - 25.0, y + 8.0, x + 25.0, y + 8.0, 3.0, INK);
    let mut sectors = [false; 12];
    let mut locked = false;
    for (pos, _, lock) in c.emitters(a) {
        let angle = (bearing(pos - a.pos) - a.heading).rem_euclid(360.0);
        sectors[((angle / 30.0).round() as usize) % 12] = true;
        locked |= lock;
    }
    for (k, lit) in sectors.iter().enumerate() {
        let th = (k as f64 * 30.0 - 90.0) * DEG;
        cv.disc(x + th.cos() as f32 * rad * 0.77, y + th.sin() as f32 * rad * 0.77, 5.0, if *lit { LAMP_ON } else { LAMP_OFF });
    }
    cv.disc(x, y + 35.0, 5.0, if locked { LAMP_RED } else { LAMP_OFF });
}

/// Redraws the MiG's instrument faces into the atlas.
pub(super) fn draw(cv: &mut Canvas, a: &Aircraft, ad: &AirData, m: &crate::sim::Mission, c: &Combat) {
    adi(cv, R_ADI, a);

    // PNP: compass card turning under the lubber line, the course arrow to
    // the next steerpoint.
    let s = Scale::new(cv, R_HSI, -90.0, 360.0);
    cv.set_transform(s.x, s.y, (-a.heading * DEG) as f32);
    for k in 0..36 {
        let t = (k as f64 * 10.0 - 90.0) * DEG;
        let len = if k % 3 == 0 { 0.74 } else { 0.83 };
        spoke(cv, 0.0, 0.0, t, s.rad * len, s.rad * 0.92, if k % 3 == 0 { 2.0 } else { 1.0 }, INK);
        if k % 3 == 0 {
            spoke_text(cv, 0.0, 0.0, t, s.rad * 0.60, &format!("{}", k), INK);
        }
    }
    let brg = bearing(m.current_gate().pos - a.pos) * DEG;
    let (cs, sn) = (((brg - 90.0 * DEG) as f32).cos(), ((brg - 90.0 * DEG) as f32).sin());
    cv.line(-cs * s.rad * 0.5, -sn * s.rad * 0.5, cs * s.rad * 0.5, sn * s.rad * 0.5, 3.0, rgba(0xf0, 0xc0, 0x40, 1.0));
    cv.disc(cs * s.rad * 0.5, sn * s.rad * 0.5, 5.0, rgba(0xf0, 0xc0, 0x40, 1.0));
    cv.reset_transform();
    cv.polyline(&[[s.x - 7.0, s.y - s.rad + 3.0], [s.x, s.y - s.rad + 14.0], [s.x + 7.0, s.y - s.rad + 3.0]], 2.0, true, NEEDLE);
    cv.polyline(&[[s.x, s.y - 14.0], [s.x, s.y + 12.0]], 3.0, false, NEEDLE);
    cv.line(s.x - 14.0, s.y - 2.0, s.x + 14.0, s.y - 2.0, 3.0, NEEDLE);

    // Low-speed IAS on the left; the separate high-speed/Mach instrument
    // sits below the variometer, as in the original analogue cockpit.
    let s = Scale::new(cv, R_ASI, 135.0, 270.0);
    s.ticks(cv, 8, 1, |k| format!("{}", k + 1));
    cv.text("X100 KM/H", s.x, s.y + s.rad * 0.30, 1.0, INK, Align::Center);
    s.needle(cv, (ad.kcas * 1.852 - 100.0) / 800.0, 0.86, 3.0, NEEDLE);
    s.hub(cv);
    let s = Scale::new(cv, R_ASIMACH, 135.0, 270.0);
    s.ticks(cv, 8, 1, |k| format!("{}", k * 2));
    cv.text("KM/H", s.x, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
    cv.rect(s.x - 22.0, s.y + s.rad * 0.22, 44.0, 16.0, rgba(0, 0, 0, 1.0));
    cv.text(&format!("{:.2}", ad.mach), s.x, s.y + s.rad * 0.22 + 8.0, 1.0, INK, Align::Center);
    s.needle(cv, ad.kcas * 1.852 / 1600.0, 0.86, 3.0, NEEDLE);
    s.hub(cv);

    let s = Scale::new(cv, R_RADALT, 135.0, 270.0);
    s.ticks(cv, 6, 1, |k| format!("{}", [0, 10, 20, 50, 100, 500, 1000][k]));
    // Piecewise scale, with more room devoted to low-altitude flying.
    let metres = ad.radar_ft * 0.3048;
    let marks = [0.0, 10.0, 20.0, 50.0, 100.0, 500.0, 1000.0];
    let k = marks.windows(2).position(|p| metres < p[1]).unwrap_or(5);
    let value = (k as f64 + clamp((metres - marks[k]) / (marks[k + 1] - marks[k]), 0.0, 1.0)) / 6.0;
    cv.text("RV M", s.x, s.y + s.rad * 0.30, 1.0, INK, Align::Center);
    s.needle(cv, value, 0.86, 3.0, NEEDLE);
    s.hub(cv);

    // Altimeter in metres: the long needle a turn per kilometre, the short
    // one per ten, and the drum counter.
    let alt = ad.baro_ft * 0.3048;
    let s = Scale::new(cv, R_ALT, -90.0, 360.0);
    for k in 0..50 {
        let t = (-90.0 + k as f64 * 7.2) * DEG;
        spoke(cv, s.x, s.y, t, s.rad * if k % 5 == 0 { 0.74 } else { 0.84 }, s.rad * 0.92, if k % 5 == 0 { 2.0 } else { 1.0 }, INK);
        if k % 5 == 0 {
            spoke_text(cv, s.x, s.y, t, s.rad * 0.58, &format!("{}", k / 5), INK);
        }
    }
    cv.rect(s.x - 26.0, s.y + s.rad * 0.22, 52.0, 16.0, rgba(0, 0, 0, 1.0));
    cv.text(&pad(alt, 5), s.x, s.y + s.rad * 0.22 + 8.0, 1.0, INK, Align::Center);
    cv.text("M", s.x, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
    s.needle(cv, (alt / 10000.0).fract(), 0.50, 5.0, NEEDLE);
    s.needle(cv, (alt / 1000.0).fract(), 0.88, 3.0, NEEDLE);
    s.hub(cv);

    // Variometer, m/s, zero at nine o'clock, compressed toward 150.
    let vs = ad.vs_fpm * 0.00508;
    let vt = |v: f64| 0.5 + 0.5 * v.signum() * (v.abs() / 150.0).min(1.0).sqrt();
    let s = Scale::new(cv, R_VVI, 30.0, 300.0);
    for v in [-150.0, -100.0, -50.0, -20.0, -10.0, -5.0, 0.0, 5.0, 10.0, 20.0, 50.0, 100.0, 150.0] {
        spoke(cv, s.x, s.y, s.ang(1.0 - vt(v)), s.rad * 0.76, s.rad * 0.92, 2.0, INK);
        if (v as f64).abs() != 5.0 && (v as f64).abs() != 20.0 {
            spoke_text(cv, s.x, s.y, s.ang(1.0 - vt(v)), s.rad * 0.58, &format!("{}", (v as f64).abs()), INK);
        }
    }
    cv.text("M/S", s.x + s.rad * 0.25, s.y, 1.0, INK, Align::Center);
    s.needle(cv, 1.0 - vt(vs), 0.86, 3.0, NEEDLE);
    s.hub(cv);

    // UUAP: angle of attack over the top, load factor under it.
    let s = Scale::new(cv, R_UUAP, 200.0, 140.0);
    s.ticks(cv, 3, 1, |k| format!("{}", k * 10));
    s.needle(cv, a.alpha_deg / 30.0, 0.86, 3.0, NEEDLE);
    let g = Scale { a0: 160.0, sweep: -140.0, ..s };
    g.ticks(cv, 6, 0, |k| format!("{}", k as i64 * 2 - 2));
    g.needle(cv, (a.g_load + 2.0) / 12.0, 0.86, 3.0, rgba(0xf0, 0xc0, 0x40, 1.0));
    cv.text("A", g.x - g.rad * 0.22, g.y - 4.0, 1.0, INK, Align::Center);
    cv.text("G", g.x + g.rad * 0.22, g.y + 6.0, 1.0, rgba(0xf0, 0xc0, 0x40, 1.0), Align::Center);
    g.hub(cv);

    // Twin-pointer tachometer, followed by two separate EGT instruments.
    // The flight model has one engine RPM/temperature value: both engine
    // indications use it, without inventing a difference between engines.
    let s = Scale::new(cv, R_RPM, 135.0, 270.0);
    s.ticks(cv, 11, 1, |k| format!("{}", k * 10));
    cv.text("%", s.x, s.y + s.rad * 0.35, 1.0, INK, Align::Center);
    s.needle(cv, a.rpm / 110.0, 0.86, 3.0, NEEDLE);
    s.needle(cv, a.rpm / 110.0, 0.65, 5.0, rgba(0xb0, 0xb8, 0xb0, 1.0));
    s.hub(cv);
    for (face, label) in [(R_EGT, "1"), (R_EGT_R, "2")] {
        let s = Scale::new(cv, face, 135.0, 270.0);
        s.ticks(cv, 9, 0, |k| format!("{}", k + 2));
        cv.text("X100 C", s.x, s.y + s.rad * 0.32, 1.0, INK, Align::Center);
        cv.text(label, s.x, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
        s.needle(cv, (a.ftit - 200.0) / 900.0, 0.86, 3.0, NEEDLE);
        s.hub(cv);
    }

    // Narrow vertical fuel tape, not a round fuel dial.
    let r = R_FUEL;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x12, 0x14, 0x14, 1.0));
    cv.text("KG", r.x + r.w * 0.5, r.y + 16.0, 1.0, INK, Align::Center);
    cv.text(&pad(a.fuel, 4), r.x + r.w * 0.5, r.y + 38.0, 1.0, INK, Align::Center);
    let (x, top, bottom) = (r.x + 48.0, r.y + 65.0, r.y + r.h - 22.0);
    for k in 0..=20 {
        let y = bottom - (bottom - top) * k as f32 / 20.0;
        cv.line(x - 8.0, y, x + if k % 5 == 0 { 15.0 } else { 5.0 }, y, 2.0, INK);
        if k % 5 == 0 { cv.text(&format!("{}", k / 5), x + 26.0, y, 1.0, INK, Align::Center); }
    }
    let fy = bottom - (bottom - top) * clamp(a.fuel / 4000.0, 0.0, 1.0) as f32;
    cv.tri([x - 24.0, fy], [x - 10.0, fy - 7.0], [x - 10.0, fy + 7.0], LAMP_ON);
    let bingo = bottom - (bottom - top) * 0.2;
    cv.line(x - 8.0, bingo, x + 16.0, bingo, 3.0, LAMP_RED);

    // Rectangular hydraulic and oxygen block below the engine gauges.
    let r = R_HYD;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x12, 0x14, 0x14, 1.0));
    let p = if a.rpm > 55.0 { 210.0 } else { a.rpm * 3.8 };
    for (i, label) in ["O2", "HYD 1", "HYD 2"].iter().enumerate() {
        let x = r.x + 30.0 + i as f32 * 74.0;
        cv.text(label, x, r.y + 10.0, 1.0, INK, Align::Center);
        cv.rect(x - 9.0, r.y + 23.0, 18.0, 55.0, rgba(0x3c, 0x27, 0x1c, 1.0));
        cv.rect(x - 6.0, r.y + 34.0, 12.0, 30.0, rgba(0x88, 0xa0, 0x4c, 1.0));
        let value = if i == 0 { 0.80 } else { p / 300.0 };
        let y = r.y + 78.0 - value as f32 * 55.0;
        cv.line(x - 12.0, y, x + 12.0, y, 3.0, NEEDLE);
    }
    system_faces(cv, a, c);

    // Clock.
    let (x, y, rad) = gauge_face(cv, R_CLOCK);
    for k in 0..12 {
        spoke(cv, x, y, (k as f64 * 30.0 - 90.0) * DEG, rad * 0.74, rad * 0.90, 2.0, INK);
    }
    let t = 7.0 * 3600.0 + 15.0 * 60.0 + m.elapsed;
    spoke(cv, x, y, ((t / 43200.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.5, 3.0, POINTER);
    spoke(cv, x, y, ((t / 3600.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.8, 2.0, POINTER);
    spoke(cv, x, y, ((t / 60.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.85, 1.0, RED);

    // KI-13: the card turns under the lubber line, read through the bowl's
    // window: 60 degrees of it across, ticks every 5, a figure every 30.
    let (x, y, rad) = gauge_face(cv, R_COMPASS);
    cv.rect(x - rad * 0.9, y - rad * 0.36, rad * 1.8, rad * 0.72, rgba(0x1c, 0x1b, 0x16, 1.0));
    let per_deg = rad * 1.8 / 60.0;
    let base = (a.heading / 5.0).round() * 5.0;
    for k in -7..=7 {
        let h = base + k as f64 * 5.0;
        let dx = ((h - a.heading) * per_deg as f64) as f32;
        if dx.abs() > rad * 0.86 {
            continue;
        }
        let hh = (h.rem_euclid(360.0)).round() as i64;
        let len = if hh % 10 == 0 { rad * 0.16 } else { rad * 0.09 };
        cv.line(x + dx, y + rad * 0.34, x + dx, y + rad * 0.34 - len, 2.0, INK);
        if hh % 30 == 0 {
            let label = match hh {
                0 => "N".to_string(),
                90 => "E".to_string(),
                180 => "S".to_string(),
                270 => "W".to_string(),
                _ => format!("{}", hh / 10),
            };
            cv.text(&label, x + dx, y - rad * 0.06, 1.0, INK, Align::Center);
        }
    }
    cv.line(x, y - rad * 0.40, x, y + rad * 0.40, 2.0, rgba(0xf0, 0x90, 0x30, 1.0));

    let r = R_COUNTER;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x18, 0x1c, 0x1c, 1.0));
    cv.text("ЛТЦ / ДО", r.x + r.w * 0.5, r.y + 16.0, 1.0, INK, Align::Center);
    cv.rect(r.x + 14.0, r.y + 30.0, r.w - 28.0, 38.0, rgba(0x06, 0x08, 0x08, 1.0));
    cv.text(&format!("{:02} {:02}", c.loadout.flares, c.loadout.chaff), r.x + r.w * 0.5, r.y + 49.0, 2.0, INK, Align::Center);
    for x in [r.x + 30.0, r.x + r.w - 30.0] { cv.disc(x, r.y + 80.0, 6.0, INK); }

    // Warning lamp column, its legends transliterated.
    let lamps = [
        ("OSTATOK 800", a.fuel < 800.0, false),
        ("POZHAR", a.battle.fire, true),
        ("GENERATOR", a.rpm < 50.0, false),
        ("MASLO", a.oil_psi < 20.0, false),
        ("ALFA KRIT", a.alpha_deg > 22.0, true),
        ("PEREGRUZKA", a.g_load > 8.0, true),
        ("OPASN VYSOTA", ad.radar_ft < 300.0 && ad.vs_fpm < -500.0, true),
        ("SHASSI", a.gear > 0.02 && a.gear < 0.98, false),
        ("TORMOZ", a.brake > 0.1, false),
        ("ZAKRYLKI", a.flaps > 0.05, false),
    ];
    let r = R_WARN;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x10, 0x12, 0x12, 1.0));
    let h = r.h / lamps.len() as f32;
    for (i, (s, lit, red)) in lamps.iter().enumerate() {
        let y = r.y + i as f32 * h;
        let on = if *red { LAMP_RED } else { LAMP_ON };
        cv.rect(r.x + 3.0, y + 2.0, r.w - 6.0, h - 4.0, if *lit { on } else { LAMP_OFF });
        cv.text(s, r.x + r.w / 2.0, y + h / 2.0, 1.0, if *lit { rgba(0x18, 0x12, 0x04, 1.0) } else { rgba(0x7a, 0x70, 0x48, 1.0) }, Align::Center);
    }

    // Gear and flap lights.
    let r = R_GEAR;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x5c, 0x6e, 0x78, 1.0));
    let (down, moving) = (a.gear > 0.98, a.gear > 0.02 && a.gear <= 0.98);
    let col = if down { GEAR_GREEN } else if moving { LAMP_RED } else { rgba(0x1c, 0x24, 0x1e, 1.0) };
    for (x, y) in [(0.5, 0.28), (0.3, 0.62), (0.7, 0.62)] {
        cv.disc(r.x + r.w * x, r.y + r.h * y, 13.0, rgba(0x10, 0x10, 0x10, 1.0));
        cv.disc(r.x + r.w * x, r.y + r.h * y, 10.0, col);
    }
    cv.text("SHASSI", r.x + r.w / 2.0, r.y + 12.0, 1.0, rgba(0x10, 0x10, 0x10, 1.0), Align::Center);
    cv.text("ZAKR", r.x + r.w * 0.5, r.y + r.h - 14.0, 1.0, if a.flaps > 0.05 { GEAR_GREEN } else { rgba(0x10, 0x10, 0x10, 1.0) }, Align::Center);

    // IT-23 repeats the ILS-31 flight/sight picture, rather than an F-16 B-scope.
    let r = R_TV;
    let green = rgba(0x9c, 0xf0, 0x68, 1.0);
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x03, 0x09, 0x04, 1.0));
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.52);
    cv.set_clip(Some(vec![[r.x + 5.0, r.y + 5.0], [r.x + r.w - 5.0, r.y + 5.0],
        [r.x + r.w - 5.0, r.y + r.h - 5.0], [r.x + 5.0, r.y + r.h - 5.0]]));
    let (height, radio) = super::hud::mig_altitude(ad.radar_ft * 0.3048, ad.baro_ft * 0.3048);
    cv.text(&format!("{:.0}", ad.kcas * 1.852), r.x + 12.0, r.y + 20.0, 1.0, green, Align::Left);
    cv.text(&format!("{height}{}", if radio { "Р" } else { "" }), r.x + r.w - 12.0, r.y + 20.0, 1.0, green, Align::Right);
    cv.text(&format!("{:03.0}", a.heading), cx, r.y + 17.0, 1.0, green, Align::Center);
    cv.set_transform(cx, cy, (-a.bank * DEG) as f32);
    for pitch in (-30_i32..=30).step_by(10) {
        let y = ((a.pitch - pitch as f64) * 1.8) as f32;
        let span = if pitch == 0 { 65.0 } else { 24.0 };
        cv.line(-span, y, -8.0, y, 1.0, green);
        cv.line(8.0, y, span, y, 1.0, green);
        if pitch != 0 { cv.text(&format!("{}", pitch.abs()), span + 4.0, y, 1.0, green, Align::Left); }
    }
    cv.reset_transform();
    cv.circle(cx, cy, 5.0, 1.0, green);
    cv.line(cx - 28.0, cy, cx - 7.0, cy, 1.0, green);
    cv.line(cx + 7.0, cy, cx + 28.0, cy, 1.0, green);
    cv.line(cx, cy - 16.0, cx, cy - 7.0, 1.0, green);
    cv.text(&format!("М{:.2}  n{:.1}", ad.mach, a.g_smooth), r.x + 12.0, r.y + r.h - 31.0, 1.0, green, Align::Left);
    cv.text(if c.irst { "ТП" } else { "РЛ" }, r.x + 12.0, r.y + r.h - 14.0, 1.0, green, Align::Left);
    let mode = match c.mode { crate::combat::Mode::Nav => "МАРШ", crate::combat::Mode::Ccip | crate::combat::Mode::Strf => "ЗЕМЛЯ", _ => "АТК" };
    cv.text(mode, cx, r.y + r.h - 14.0, 1.0, green, Align::Center);
    if c.lock.is_some() { cv.stroke_rect(cx - 18.0, cy - 18.0, 36.0, 36.0, 1.0, green); }
    cv.set_clip(None);

    // Left switch panel: autopilot (SAU), navigation (RSBN, ARK) and the
    // altimeter setting.
    let r = R_LEFT;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x5c, 0x6e, 0x78, 1.0));
    let ink = rgba(0xf0, 0xf0, 0xe8, 1.0);
    for (i, s) in ["SAU", "STAB", "PRIVED", "OTKL"].iter().enumerate() {
        let x = r.x + 28.0 + i as f32 * 56.0;
        cv.text(s, x, r.y + 18.0, 1.0, ink, Align::Center);
        cv.disc(x, r.y + 44.0, 12.0, rgba(0x14, 0x14, 0x14, 1.0));
        cv.disc(x, r.y + 44.0, 8.0, if i == 3 { rgba(0x8c, 0x8c, 0x84, 1.0) } else { rgba(0xe8, 0xe0, 0xc8, 1.0) });
    }
    cv.line(r.x + 8.0, r.y + 70.0, r.x + r.w - 8.0, r.y + 70.0, 2.0, ink);
    for (i, s) in ["RSBN", "ARK", "KURS"].iter().enumerate() {
        let x = r.x + 38.0 + i as f32 * 74.0;
        cv.text(s, x, r.y + 88.0, 1.0, ink, Align::Center);
        cv.disc(x, r.y + 122.0, 20.0, rgba(0x14, 0x14, 0x14, 1.0));
        spoke(cv, x, r.y + 122.0, (-90.0 + 40.0 * i as f64) * DEG, 0.0, 18.0, 3.0, ink);
    }
    cv.line(r.x + 8.0, r.y + 156.0, r.x + r.w - 8.0, r.y + 156.0, 2.0, ink);
    cv.text("DAVLENIE", r.x + r.w / 2.0, r.y + 174.0, 1.0, ink, Align::Center);
    cv.rect(r.x + r.w / 2.0 - 36.0, r.y + 188.0, 72.0, 24.0, rgba(0x04, 0x04, 0x04, 1.0));
    cv.text(&format!("{}", (ad.qnh_inhg * 25.4).round() as i64), r.x + r.w / 2.0, r.y + 200.0, 2.0, ink, Align::Center);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_glass_is_in_front_of_the_full_bow_on_the_pilot_side() {
        let mut mesh = Mesh::new();
        build_static(&mut mesh);
        let r = super::super::R_MIRROR;
        let mut count = 0;
        for v in mesh.v.chunks(crate::mesh::STRIDE) {
            let (u, y) = (v[9] as f64 * 2048.0, (1.0-v[10] as f64) * 2048.0);
            if u > r.x as f64 && u < (r.x+r.w) as f64 && y > r.y as f64 && y < (r.y+r.h) as f64 {
                count += 1;
                assert!(v[2] as f64 > crate::mig29::CANOPY_Z[3] + OFFSET.z + 0.010,
                    "mirror glass is embedded in or behind the canopy bow");
            }
        }
        assert!(count > 100);
    }

    #[test]
    fn mounting_flanges_fit_the_panel_without_crossing_or_overlapping() {
        for (i, a) in DIALS.iter().enumerate() {
            let half = a.radius + 0.013;
            let top = 1.0 - 0.02 * ((a.u.abs() - HINGE) / (0.38 - HINGE)).max(0.0);
            assert!(a.u.abs() + half < 0.38);
            assert!(a.y - half * TILT.cos() > BASE && a.y + half * TILT.cos() < top);
            assert!(a.u.abs() + half < HINGE || a.u.abs() - half > HINGE,
                "dial {i} crosses the panel fold");
            for b in DIALS.iter().skip(i + 1) {
                let clear = half + b.radius + 0.013;
                assert!((a.u - b.u).abs() > clear || (pv(a.y) - pv(b.y)).abs() > clear,
                    "mounting flanges overlap");
            }
        }
        // The attitude and heading group belongs left of the HUD axis.
        assert!(DIALS[0].u < -0.08 && DIALS[1].u == DIALS[0].u);
        assert!(DIALS[0].y > DIALS[1].y);
    }

    #[test]
    fn console_panels_are_right_handed_and_follow_their_support() {
        for s in [-1.0, 1.0] {
            let f = console_at(s, 0.330, -3.0);
            assert!((f.x.len() - 1.0).abs() < 1e-12 && (f.z.len() - 1.0).abs() < 1e-12);
            assert!(f.x.dot(f.y).abs() < 1e-12 && f.x.cross(f.y).dot(f.z) > 0.999999);
            for u in [-0.055, 0.055] {
                let p = f.p(u, 0.0, 0.0);
                assert!(p.x.abs() > CONSOLE_IN && p.x.abs() < WALL_X);
                assert!((p.y - CONSOLE_Y - (p.x.abs() - CONSOLE_IN) * 0.14).abs() < 1e-12);
            }
        }
        // Seat bucket sides end at 0.261 m; consoles must leave room.
        assert!(CONSOLE_IN > 0.261);
        for rudder in [-1.0, 0.0, 1.0] {
            let mut m = Mesh::new();
            crate::cockpit::pilot::build_dynamic_mig(&mut m, &crate::cockpit::Hands { rudder, ..Default::default() });
            for v in m.v.chunks(crate::mesh::STRIDE) {
                assert!(v[0..3].iter().all(|v| v.is_finite()));
                assert!(v[1] as f64 > FLOOR, "floor cuts a pedal or boot");
            }
        }
    }

    fn blocked(eye: V3, target: V3, tri: &[f32]) -> bool {
        let pos = |i: usize| v3(tri[i] as f64, tri[i + 1] as f64, tri[i + 2] as f64);
        let (a, b, c) = (pos(0), pos(crate::mesh::STRIDE), pos(crate::mesh::STRIDE * 2));
        let (ab, ac, ray) = (b - a, c - a, target - eye);
        let h = ray.cross(ac);
        let det = ab.dot(h);
        if det.abs() < 1e-10 { return false; }
        let s = eye - a;
        let u = s.dot(h) / det;
        let q = s.cross(ab);
        let v = ray.dot(q) / det;
        let t = ac.dot(q) / det;
        u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.001 && t < 0.995
    }

    #[test]
    fn raised_controls_and_counter_clear_the_coaming() {
        let mut mesh = Mesh::new();
        build_static(&mut mesh);
        for (u, y, w, h) in [(-0.320, 1.060, 0.100, 0.065),
            (0.350, 1.025, 0.050, 0.045), (0.0, 1.035, 0.180, 0.048)] {
            let f = raised_at(u, y);
            for x in [-0.9, 0.0, 0.9] {
                for y in [-0.9, 0.0, 0.9] {
                    let target = f.p(x*w*0.5, y*h*0.5, 0.0085);
                    assert!(!mesh.v.chunks(crate::mesh::STRIDE * 3).any(|tri| blocked(crate::airframe::EYE, target, tri)),
                        "raised control {u} hidden at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn primary_flight_instruments_clear_the_pilot_through_rudder_travel() {
        let eye = crate::airframe::EYE;
        for step in -8..=8 {
            let rudder = step as f64 / 8.0;
            let mut mesh = Mesh::new();
            crate::cockpit::pilot::build_dynamic_mig(&mut mesh, &crate::cockpit::Hands { rudder, ..Default::default() });
            for (i, d) in DIALS[..4].iter().enumerate() {
                let f = panel_at(d.u, d.y);
                for (u, v) in [(0.0, 0.0), (-0.95, 0.0), (0.95, 0.0), (0.0, -0.95), (0.0, 0.95), (-0.67, -0.67), (0.67, -0.67), (-0.67, 0.67), (0.67, 0.67)] {
                    let target = f.p(u * d.radius, v * d.radius, 0.004);
                    assert!(!mesh.v.chunks(crate::mesh::STRIDE * 3).any(|tri| blocked(eye, target, tri)),
                        "pilot obscures primary dial {i} at rudder {rudder}, sample {u},{v}");
                }
            }
        }
    }

    #[test]
    fn warning_column_is_clear_of_the_canopy_rail_and_panel() {
        let mut mesh = Mesh::new();
        build_static(&mut mesh);
        let f = warning_frame();
        for u in [-0.95, 0.0, 0.95] {
            for v in [-0.95, -0.5, 0.0, 0.5, 0.95] {
                let target = f.p(u * 0.026, v * 0.1125, 0.0085);
                assert!(target.x < WALL_X - 0.01);
                assert!(!mesh.v.chunks(crate::mesh::STRIDE * 3)
                    .any(|tri| blocked(crate::airframe::EYE, target, tri)),
                    "warning column obscured at {u},{v}");
            }
        }
    }

    #[test]
    fn instrument_atlas_regions_do_not_overlap_each_other_or_the_mirrors() {
        let rects = [R_ADI, R_HSI, R_ASI, R_ALT, R_VVI, R_UUAP, R_RPM, R_EGT, R_EGT_R,
            R_FUEL, R_HYD, R_CLOCK, R_WARN, R_GEAR, R_TV, R_LEFT, R_COMPASS,
            R_WEAPON, R_EKRAN, R_SPO, R_RADALT, R_ASIMACH, R_COUNTER, crate::cockpit::R_MIRROR];
        for (i, a) in rects.iter().enumerate() {
            assert!(a.x >= 0.0 && a.y >= 1280.0 && a.x + a.w <= 2048.0 && a.y + a.h <= 2048.0);
            for b in rects.iter().skip(i + 1) {
                assert!(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y);
            }
        }
    }

    #[test]
    fn tub_fits_the_hole_cut_in_the_skin() {
        // The skin is cut away in mig29::CUTOUT (body axes); the tub, the
        // coaming and the deck behind the seat must cover that hole.
        let (half, floor, z0, z1) = crate::mig29::CUTOUT;
        assert!(WALL_X < half && FLOOR < floor + OFFSET.y);
        let hood_front = pp(0.0, pv(1.0) + 0.075, 0.04 - 0.44).z + PANEL_FORWARD;
        assert!(hood_front < z0 + OFFSET.z + 0.1 && BULKHEAD_Z < z1 + OFFSET.z);
        // The eye sits under the canopy with room for the helmet.
        let top = cp(crate::airframe::EYE.z, PI / 2.0).y;
        assert!(top - crate::airframe::EYE.y > 0.15);
    }
}
