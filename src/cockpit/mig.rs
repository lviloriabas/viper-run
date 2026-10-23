//! MiG-29S (9.13S) cockpit, after photos of 9.12 cockpits on Wikimedia
//! Commons, which the 9.13 kept nearly unchanged (see
//! docs/MIG29-REFERENCES.md): a narrow tub painted blue-grey,
//! an analogue panel of black-faced round instruments with the attitude
//! director (KPP) over the horizontal situation indicator (PNP) in the
//! middle, airspeed and altimeter on the left, the variometer and engine
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
const CONSOLE_IN: f64 = 0.22;
const WALL_X: f64 = 0.40;
const FLOOR: f64 = 0.22;
/// Top of the throttle quadrant housing on the left console.
pub(super) const QUADRANT_TOP: f64 = 0.655;
/// Bulkhead behind the headrest.
const BULKHEAD_Z: f64 = -2.75;

// Atlas faces, in the left half below the F-16's.
const R_ADI: R = r(0.0, 1280.0, 192.0, 192.0);
const R_HSI: R = r(192.0, 1280.0, 192.0, 192.0);
const R_ASI: R = r(384.0, 1280.0, 160.0, 160.0);
const R_ALT: R = r(544.0, 1280.0, 160.0, 160.0);
const R_VVI: R = r(704.0, 1280.0, 160.0, 160.0);
const R_UUAP: R = r(864.0, 1280.0, 160.0, 160.0);
const R_RPM: R = r(0.0, 1472.0, 160.0, 160.0);
const R_EGT: R = r(160.0, 1472.0, 160.0, 160.0);
const R_FUEL: R = r(320.0, 1472.0, 160.0, 160.0);
const R_HYD: R = r(480.0, 1472.0, 128.0, 128.0);
const R_CLOCK: R = r(608.0, 1472.0, 128.0, 128.0);
const R_WARN: R = r(736.0, 1472.0, 96.0, 320.0);
const R_GEAR: R = r(832.0, 1472.0, 192.0, 128.0);
const R_TV: R = r(0.0, 1664.0, 256.0, 208.0);
const R_LEFT: R = r(256.0, 1664.0, 224.0, 240.0);
const R_COMPASS: R = r(480.0, 1664.0, 128.0, 128.0);

// ================================================================ panel
/// Main panel: tilted back 12 degrees, its foot 0.66 m above the datum just
/// ahead of the knees; the outer parts fold 22 degrees toward the pilot.
const TILT: f64 = 12.0 * DEG;
const HINGE: f64 = 0.20;
const CANT: f64 = 22.0 * DEG;
const BASE: f64 = 0.66;

fn panel() -> Frame {
    Frame { o: v3(0.0, BASE, -3.95), x: V3::X, y: v3(0.0, TILT.cos(), -TILT.sin()), z: v3(0.0, TILT.sin(), TILT.cos()) }
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

/// Panel outline in face coordinates: the centre up to 1.0 m, the wings
/// falling to 0.90 m at their outer edges under the sills.
fn outline() -> [(f64, f64); 8] {
    let (top, side) = (pv(1.0), pv(0.90));
    [(-0.36, 0.0), (-HINGE, 0.0), (HINGE, 0.0), (0.36, 0.0), (0.36, side), (HINGE, top), (-HINGE, top), (-0.36, side)]
}

/// Canopy point in cockpit space (see mig29::canopy_point).
fn cp(z: f64, phi: f64) -> V3 {
    crate::mig29::canopy_point(z - OFFSET.z, phi) + OFFSET
}

/// Sill on side `s` at cockpit `z`.
fn sill(s: f64, z: f64) -> V3 {
    cp(z, if s > 0.0 { 0.0 } else { PI })
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
    let hood = |u: f64, v: f64, t: f64| pp(u * (1.0 - 0.10 * t), v + 0.015 + 0.06 * t, 0.04 - 0.44 * t);
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

    // Instruments. Centre: KPP over PNP, airspeed, altimeter and AoA/G on
    // the left, the variometer, tachometer and exhaust temperature on the
    // right. Right wing: fuel, hydraulics, clock. Left wing: gear lights and
    // the navigation and autopilot switches.
    gauge(m, &panel_at(0.0, 0.895), 0.056, R_ADI);
    gauge(m, &panel_at(0.0, 0.755), 0.050, R_HSI);
    gauge(m, &panel_at(-0.13, 0.915), 0.040, R_ASI);
    gauge(m, &panel_at(-0.13, 0.81), 0.040, R_ALT);
    gauge(m, &panel_at(-0.13, 0.715), 0.034, R_UUAP);
    gauge(m, &panel_at(0.13, 0.915), 0.038, R_VVI);
    gauge(m, &panel_at(0.13, 0.81), 0.040, R_RPM);
    gauge(m, &panel_at(0.13, 0.715), 0.034, R_EGT);
    gauge(m, &panel_at(0.28, 0.875), 0.036, R_FUEL);
    gauge(m, &panel_at(0.28, 0.785), 0.030, R_HYD);
    gauge(m, &panel_at(0.28, 0.705), 0.026, R_CLOCK);
    plate(m, &panel_at(-0.28, 0.885), 0.11, 0.073, R_GEAR);
    plate(m, &panel_at(-0.28, 0.755), 0.105, 0.112, R_LEFT);
    for (u, y) in [(-0.315, 0.68), (-0.245, 0.68)] {
        let f = panel_at(u, y);
        m.capsule(f.p(0.0, 0.0, 0.004), f.p(0.0, 0.0, 0.022), 0.011, 0.010, KNOB, 8);
    }

    // Warning lamp column along the right sill, facing in and aft.
    let n = v3(-0.85, 0.15, 0.50).norm();
    let y = (V3::Y - n * n.y).norm();
    plate(m, &Frame { o: v3(0.372, 0.86, -3.80), x: y.cross(n), y, z: n }, 0.045, 0.15, R_WARN);

    // Tub: floor, walls up to the sills, the canopy rails and the consoles.
    m.quad(v3(-WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, BULKHEAD_Z), v3(-WALL_X, FLOOR, BULKHEAD_Z), WALL.scale(0.5));
    m.quad(v3(-WALL_X, FLOOR, -4.45), v3(WALL_X, FLOOR, -4.45), v3(WALL_X, BASE, -3.97), v3(-WALL_X, BASE, -3.97), WALL.scale(0.45));
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
        let e = pp(s * 0.36, 0.0, 0.0);
        let et = pp(s * 0.36, pv(0.90), 0.0);
        let st = sill(s, et.z);
        m.polygon(&[e, v3(s * WALL_X, e.y, e.z), v3(s * WALL_X, st.y - 0.02, et.z), et], PANEL.scale(0.8));
        // Console: top, inboard face and front face down to the floor.
        let (zf, za) = (-3.85, BULKHEAD_Z);
        let c = |x: f64, z: f64| v3(s * x, CONSOLE_Y + (x - CONSOLE_IN) * 0.14, z);
        m.quad(c(CONSOLE_IN, zf), c(WALL_X, zf), c(WALL_X, za), c(CONSOLE_IN, za), CONSOLE);
        m.quad(c(CONSOLE_IN, zf), c(CONSOLE_IN, za), v3(s * CONSOLE_IN, FLOOR, za), v3(s * CONSOLE_IN, FLOOR, zf), WALL.scale(0.7));
        m.quad(c(CONSOLE_IN, zf), c(WALL_X, zf), v3(s * WALL_X, FLOOR, zf), v3(s * CONSOLE_IN, FLOOR, zf), WALL.scale(0.6));
        // Switch rows and knobs, in the photos' rhythm of small black
        // panels with toggles.
        for (i, z) in [-3.15, -3.0, -2.86].iter().enumerate() {
            let f = Frame { o: c(0.30, *z), x: V3::X, y: V3::Z, z: (V3::Y - V3::X * s * 0.14).norm() };
            m.block(&f.at(0.0, 0.0, 0.003), v3(0.07, 0.055, 0.003), KNOB.scale(2.0));
            for j in 0..4 {
                let p = f.p(-0.05 + j as f64 * 0.033, 0.0, 0.006);
                if (i + j) % 3 == 0 {
                    m.capsule(p, p + f.z * 0.015, 0.008, 0.008, KNOB, 6);
                } else {
                    m.capsule(p, p + f.z * 0.018 - f.y * 0.006, 0.0025, 0.002, METAL, 5);
                }
            }
        }
    }
    // Throttle quadrant on the left console, with its slot.
    let q = Frame { o: v3(-0.315, (CONSOLE_Y + QUADRANT_TOP) * 0.5 + 0.01, -3.45), x: V3::X, y: V3::Y, z: V3::Z };
    m.block(&q, v3(0.04, (QUADRANT_TOP - CONSOLE_Y) * 0.5 + 0.01, 0.30), CONSOLE.scale(0.8));
    m.quad(v3(-0.335, QUADRANT_TOP + 0.001, -3.72), v3(-0.295, QUADRANT_TOP + 0.001, -3.72), v3(-0.295, QUADRANT_TOP + 0.001, -3.30), v3(-0.335, QUADRANT_TOP + 0.001, -3.30), Rgb(0.02, 0.02, 0.022));

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
    // The mirrors are big rounded convex glasses in domed housings, the
    // centre one widest, each on a short stalk off the arch and turned to
    // the pilot.
    for d in [-0.44, 0.0, 0.44] {
        let p = cp(za, PI / 2.0 + d);
        let c = p - inward(p, 0.085) + V3::Z * 0.035;
        let n = (crate::airframe::EYE - c).norm();
        let x = V3::Y.cross(n).norm();
        let f = Frame { o: c, x, y: n.cross(x), z: n };
        let (w, h) = if d == 0.0 { (0.17, 0.075) } else { (0.14, 0.066) };
        super::convex_mirror(m, &f, w, h, -d / 0.44, FRAME.scale(0.8));
        crate::cockpit::pilot::bar(m, f.p(0.0, 0.0, -0.034), p - inward(p, 0.03), 0.007, 0.007, V3::Z, FRAME.scale(0.8));
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

    // KUS: indicated airspeed in km/h, the Mach number on its window.
    let s = Scale::new(cv, R_ASI, 120.0, 300.0);
    s.ticks(cv, 8, 1, |k| format!("{}", k * 2));
    cv.text("KM/H", s.x, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
    cv.rect(s.x - 22.0, s.y + s.rad * 0.22, 44.0, 16.0, rgba(0, 0, 0, 1.0));
    cv.text(&format!("{:.2}", ad.mach), s.x, s.y + s.rad * 0.22 + 8.0, 1.0, INK, Align::Center);
    s.needle(cv, ad.kcas * 1.852 / 1600.0, 0.86, 3.0, NEEDLE);
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

    // Twin tachometer and exhaust temperature: needles 1 and 2.
    let twin = |cv: &mut Canvas, r: R, v: f64, lo: f64, hi: f64, n: usize, step: f64, unit: &str| {
        let s = Scale::new(cv, r, 135.0, 270.0);
        s.ticks(cv, n, 1, |k| format!("{}", (lo + k as f64 * step).round() as i64));
        cv.text(unit, s.x, s.y + s.rad * 0.35, 1.0, INK, Align::Center);
        s.needle(cv, (v - lo) / (hi - lo), 0.86, 3.0, NEEDLE);
        s.needle(cv, (v * 0.996 - lo) / (hi - lo), 0.70, 3.0, rgba(0xc8, 0xc8, 0xbc, 1.0));
        cv.text("1", s.x - 10.0, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
        cv.text("2", s.x + 10.0, s.y - s.rad * 0.30, 1.0, INK, Align::Center);
        s.hub(cv);
    };
    twin(cv, R_RPM, a.rpm, 0.0, 110.0, 11, 10.0, "%");
    twin(cv, R_EGT, a.ftit / 100.0, 2.0, 10.0, 8, 1.0, "X100 C");

    // Fuel remaining, kilograms.
    let s = Scale::new(cv, R_FUEL, 135.0, 270.0);
    s.ticks(cv, 8, 1, |k| format!("{}", k * 5));
    cv.text("KG X100", s.x, s.y + s.rad * 0.35, 1.0, INK, Align::Center);
    s.needle(cv, a.fuel / 4000.0, 0.86, 3.0, NEEDLE);
    s.hub(cv);

    // Hydraulics, main and booster systems.
    let p = if a.rpm > 55.0 { 210.0 } else { a.rpm * 3.8 };
    let s = Scale::new(cv, R_HYD, 135.0, 270.0);
    s.ticks(cv, 3, 1, |k| format!("{}", k * 100));
    s.needle(cv, p / 300.0, 0.86, 3.0, NEEDLE);
    s.needle(cv, p * 0.97 / 300.0, 0.66, 2.0, rgba(0xc8, 0xc8, 0xbc, 1.0));
    s.hub(cv);

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

    // Warning lamp column, its legends transliterated.
    let lamps = [
        ("OSTATOK 800", a.fuel < 800.0, false),
        ("POZHAR", a.battle.engine >= 0.95, true),
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

    // IT-23: the radar picture as a B-scope, azimuth across and range up.
    let r = R_TV;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x04, 0x0c, 0x08, 1.0));
    let green = rgba(0x7c, 0xf0, 0x9c, 1.0);
    let dim = rgba(0x1e, 0x50, 0x34, 1.0);
    let (x0, y0, w, hh) = (r.x + 16.0, r.y + 12.0, r.w - 32.0, r.h - 28.0);
    for k in 0..=4 {
        let y = y0 + hh * k as f32 / 4.0;
        cv.line(x0, y, x0 + w, y, 1.0, dim);
        let x = x0 + w * k as f32 / 4.0;
        cv.line(x, y0, x, y0 + hh, 1.0, dim);
    }
    if c.irst {
        // IRST (TP) picture: azimuth across its 60 degrees, elevation up from
        // -15 to +30, the heat sources it sees, the track boxed, and the
        // laser range once inside 6.5 km.
        for (i, b) in c.bandits.iter().enumerate().filter(|(_, b)| b.alive && crate::combat::irst_sees(a, &b.jet, 1.0)) {
            let d = a.q.conj().rotate((b.jet.pos - a.pos).norm());
            let (az, el) = (d.x.atan2(-d.z) / DEG, d.y.asin() / DEG);
            let (x, y) = (x0 + w * ((az + 30.0) / 60.0) as f32, y0 + hh * (1.0 - ((el + 15.0) / 45.0) as f32));
            cv.disc(x, y, 3.0, green);
            if c.lock == Some(crate::weapons::Who::Bandit(i)) {
                cv.stroke_rect(x - 8.0, y - 8.0, 16.0, 16.0, 2.0, green);
            }
        }
        cv.text("TP", r.x + 3.0, y0 + 4.0, 1.0, green, Align::Left);
        let laser = c.laser_range(a).map_or("D ---".to_string(), |d| format!("D {:.1}", d / 1000.0));
        cv.text(&laser, r.x + r.w - 3.0, y0 + 4.0, 1.0, green, Align::Right);
    } else {
        let range = 40000.0;
        for b in c.bandits.iter().filter(|b| b.alive) {
            let rel = b.jet.pos - a.pos;
            let az = (bearing(rel) - a.heading + 540.0).rem_euclid(360.0) - 180.0;
            let d = v3(rel.x, 0.0, rel.z).len();
            if az.abs() < 60.0 && d < range {
                let (x, y) = (x0 + w * ((az + 60.0) / 120.0) as f32, y0 + hh * (1.0 - (d / range) as f32));
                cv.rect(x - 6.0, y - 2.0, 12.0, 4.0, green);
            }
        }
        cv.text("40", r.x + 3.0, y0 + 4.0, 1.0, green, Align::Left);
        cv.text("0", r.x + 3.0, y0 + hh - 4.0, 1.0, green, Align::Left);
    }
    cv.polyline(&[[x0 + w / 2.0 - 6.0, y0 + hh + 10.0], [x0 + w / 2.0, y0 + hh + 4.0], [x0 + w / 2.0 + 6.0, y0 + hh + 10.0]], 2.0, false, green);

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
    fn tub_fits_the_hole_cut_in_the_skin() {
        // The skin is cut away in mig29::CUTOUT (body axes); the tub, the
        // coaming and the deck behind the seat must cover that hole.
        let (half, floor, z0, z1) = crate::mig29::CUTOUT;
        assert!(WALL_X < half && FLOOR < floor + OFFSET.y);
        let hood_front = pp(0.0, pv(1.0) + 0.075, 0.04 - 0.44).z;
        assert!(hood_front < z0 + OFFSET.z + 0.1 && BULKHEAD_Z < z1 + OFFSET.z);
        // The eye sits under the canopy with room for the helmet.
        let top = cp(crate::airframe::EYE.z, PI / 2.0).y;
        assert!(top - crate::airframe::EYE.y > 0.15);
    }
}
