//! The pilot, the ACES II seat, the sidestick and the throttle. Limbs are
//! lofts through chamfered cross-sections that change size along the limb,
//! so the flight suit, G-suit, boots and gloves read as shapes instead of
//! joined tubes. The controls sit far enough forward on the side consoles
//! that both gloved hands show in the lower corners of the forward view.
use super::{screen, Hands, METALC, R_KNEE, SEAT};
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};

const SUIT: Rgb = Rgb(0.31, 0.33, 0.235);
const GSUIT: Rgb = Rgb(0.245, 0.265, 0.185);
const POCKET: Rgb = Rgb(0.355, 0.365, 0.26);
const VEST: Rgb = Rgb(0.19, 0.205, 0.15);
const WEBBING: Rgb = Rgb(0.42, 0.40, 0.30);
const GLOVE: Rgb = Rgb(0.42, 0.49, 0.35);
const GLOVE_DARK: Rgb = Rgb(0.32, 0.38, 0.27);
const BOOT: Rgb = Rgb(0.085, 0.08, 0.075);
const SOLE: Rgb = Rgb(0.035, 0.035, 0.035);
const RUBBER: Rgb = Rgb(0.055, 0.06, 0.065);
const GRIP: Rgb = Rgb(0.115, 0.12, 0.125);
const RED: Rgb = Rgb(0.62, 0.12, 0.08);

/// Sidestick pivot on its raised mount at the front of the right console.
const STICK: V3 = v3(0.365, 0.668, -3.81);
const SHOULDER: V3 = v3(0.19, 1.00, -3.26);
/// Upper arm and forearm (elbow to wrist) lengths.
const ARM: (f64, f64) = (0.30, 0.31);

/// Unit outlines for loft sections: x across the limb, y toward its front.
const OCT: [(f64, f64); 8] = [(1.0, -0.45), (1.0, 0.45), (0.55, 1.0), (-0.55, 1.0), (-1.0, 0.45), (-1.0, -0.45), (-0.55, -1.0), (0.55, -1.0)];
/// Hexagon with a ridge on top, so a limb seen from above shows two lit faces.
const HEX: [(f64, f64); 6] = [(0.0, 1.0), (1.0, 0.5), (1.0, -0.5), (0.0, -1.0), (-1.0, -0.5), (-1.0, 0.5)];

/// Loft station: centre, half width along the side axis, half height, and the
/// colour of the band that starts here.
#[derive(Clone, Copy)]
struct St(V3, f64, f64, Rgb);

/// Sweeps `shape` through the stations with capped ends. Each section lies
/// across the path, bisecting the bend at joints, with its x axis as close to
/// `side` as the path allows.
fn loft(m: &mut Mesh, st: &[St], side: V3, shape: &[(f64, f64)]) {
    let n = st.len();
    let rings: Vec<Vec<V3>> = (0..n)
        .map(|i| {
            let into = if i > 0 { (st[i].0 - st[i - 1].0).norm() } else { V3::ZERO };
            let out = if i + 1 < n { (st[i + 1].0 - st[i].0).norm() } else { V3::ZERO };
            let t = (into + out).norm();
            let a = (side - t * side.dot(t)).norm();
            let b = a.cross(t);
            shape.iter().map(|&(x, y)| st[i].0 + a * (x * st[i].1) + b * (y * st[i].2)).collect()
        })
        .collect();
    let k = shape.len();
    for i in 0..n - 1 {
        for j in 0..k {
            let j2 = (j + 1) % k;
            m.quad(rings[i][j], rings[i][j2], rings[i + 1][j2], rings[i + 1][j], st[i].3);
        }
    }
    m.polygon(&rings[0], st[0].3);
    m.polygon(&rings[n - 1], st[n - 2].3);
}

/// Box from `a` to `b`, half width `w` across and half height `h` along `up`.
pub(super) fn bar(m: &mut Mesh, a: V3, b: V3, w: f64, h: f64, up: V3, col: Rgb) {
    let z = (b - a).norm();
    let x = up.cross(z).norm();
    let y = z.cross(x);
    m.block(&Frame { o: (a + b) * 0.5, x, y, z }, v3(w, h, (b - a).len() * 0.5), col);
}

/// Bars through a polyline, each overlapping the next so bends stay closed.
pub(super) fn strap(m: &mut Mesh, pts: &[V3], w: f64, h: f64, up: V3, col: Rgb) {
    for p in pts.windows(2) {
        let d = (p[1] - p[0]).norm() * h.max(w * 0.3);
        bar(m, p[0] - d, p[1] + d, w, h, up, col);
    }
}

/// Grip axes: `g` up the grip, `fwd` where the fingers wrap, `out` toward
/// the back of the hand (outboard).
#[derive(Clone, Copy)]
struct Grip {
    o: V3,
    g: V3,
    fwd: V3,
    out: V3,
}

impl Grip {
    fn new(o: V3, g: V3, side: f64) -> Grip {
        let fwd = (-V3::Z - g * -g.z).norm();
        Grip { o, g, fwd, out: fwd.cross(g) * side }
    }
    fn p(&self, f: f64, u: f64, s: f64) -> V3 {
        self.o + self.fwd * f + self.g * u + self.out * s
    }
    fn frame(&self, f: f64, u: f64, s: f64) -> Frame {
        Frame { o: self.p(f, u, s), x: self.fwd, y: self.g, z: self.out }
    }
}

/// Gloved hand closed round a grip of half extents `rx` across and `rf` fore
/// and aft, centred on `k.o`: one mitten piece from the wrist along the
/// outboard side and round the front, the palm heel behind the grip, and the
/// thumb through the points `thumb`. Returns the wrist.
fn fist(m: &mut Mesh, k: &Grip, rx: f64, rf: f64, thumb: [V3; 3]) -> V3 {
    let wrist = k.p(-0.072, -0.004, rx + 0.009);
    let ring = |deg: f64, pad: f64| {
        let t = deg * DEG;
        k.p(t.cos() * (rf + pad), 0.0, t.sin() * (rx + pad))
    };
    loft(m, &[St(wrist, 0.030, 0.018, GLOVE), St(k.p(-0.036, 0.0, rx + 0.016), 0.043, 0.016, GLOVE), St(k.p(0.004, 0.0, rx + 0.015), 0.046, 0.015, GLOVE), St(ring(35.0, 0.015), 0.046, 0.015, GLOVE), St(ring(-15.0, 0.014), 0.045, 0.014, GLOVE), St(ring(-65.0, 0.013), 0.043, 0.013, GLOVE), St(ring(-100.0, 0.012), 0.040, 0.012, GLOVE)], k.g, &OCT);
    m.block(&k.frame(-rf - 0.010, -0.006, 0.004), v3(0.012, 0.034, rx * 0.9), GLOVE);
    thumb_loft(m, thumb, k.out);
    wrist
}

/// Tapering gloved thumb through three points.
fn thumb_loft(m: &mut Mesh, t: [V3; 3], side: V3) {
    loft(m, &[St(t[0], 0.013, 0.012, GLOVE), St(t[1], 0.011, 0.010, GLOVE), St(t[2], 0.0085, 0.008, GLOVE)], side, &OCT);
}

/// Elbow of a two-bone arm from `shoulder` to `wrist`, bent toward `pole`.
fn elbow(shoulder: V3, wrist: V3, pole: V3) -> V3 {
    let (a, b) = ARM;
    let d = wrist - shoulder;
    let l = clamp(d.len(), b - a + 0.01, a + b - 0.002);
    let dir = d.norm();
    let x = (a * a - b * b + l * l) / (2.0 * l);
    let h = (a * a - x * x).max(0.0).sqrt();
    shoulder + dir * x + (pole - dir * pole.dot(dir)).norm() * h
}

/// Sleeve from shoulder through the elbow to the wrist, under the gauntlet.
/// `cuff` is the half width (across) and half height of the glove's opening
/// at the wrist: taller than wide for a hand on the stick, flatter for the
/// palm-down hand on the throttle.
fn arm(m: &mut Mesh, shoulder: V3, wrist: V3, pole: V3, cuff: (f64, f64)) {
    let e = elbow(shoulder, wrist, pole);
    let back = (e - wrist).norm();
    let (cw, ch) = cuff;
    loft(
        m,
        &[
            St(shoulder, 0.068, 0.066, SUIT),
            St(shoulder.lerp(e, 0.5), 0.064, 0.062, SUIT),
            St(e, 0.060, 0.056, SUIT),
            St(e.lerp(wrist, 0.5), 0.050, 0.050, SUIT),
            St(wrist + back * 0.07, cw + 0.010, ch + 0.006, SUIT),
        ],
        V3::X,
        &HEX,
    );
    // Pen pocket on the outside of the upper arm, and the glove's gauntlet
    // over the cuff.
    let ua = (e - shoulder).norm();
    let x = V3::X * shoulder.x.signum();
    let side = (x - ua * x.dot(ua)).norm();
    let mid = shoulder.lerp(e, 0.35) + side * 0.064;
    bar(m, mid, mid + ua * 0.09, 0.026, 0.007, side, POCKET);
    loft(m, &[St(wrist + back * 0.08, cw + 0.012, ch + 0.010, GLOVE_DARK), St(wrist + back * 0.03, cw + 0.008, ch + 0.006, GLOVE_DARK), St(wrist - back * 0.008, cw, ch, GLOVE)], V3::X, &HEX);
}

fn leg(m: &mut Mesh, s: f64, rudder: f64) {
    let hip = v3(s * 0.10, 0.60, -3.48);
    let knee = v3(s * 0.15, 0.675, -3.96);
    let ankle = v3(s * 0.165, 0.31, -4.25 - s * rudder * 0.04);
    let thigh = (knee - hip).norm();
    let shin = (ankle - knee).norm();
    let front = V3::X.cross(shin).norm();
    loft(
        m,
        &[
            St(hip - thigh * 0.05, 0.088, 0.078, GSUIT),
            St(hip.lerp(knee, 0.4) + V3::Y * 0.006, 0.093, 0.081, GSUIT),
            St(hip.lerp(knee, 0.8), 0.075, 0.068, SUIT),
            St(knee, 0.068, 0.072, SUIT),
            St(knee + shin * 0.09 + front * 0.006, 0.062, 0.064, GSUIT),
            St(knee.lerp(ankle, 0.42) - front * 0.014, 0.064, 0.068, GSUIT),
            St(knee.lerp(ankle, 0.72), 0.052, 0.053, BOOT),
            St(ankle, 0.049, 0.050, BOOT),
        ],
        V3::X,
        &OCT,
    );
    // Kneecap, and the big G-suit pocket on the outside of the thigh.
    let kt = (thigh + shin).norm();
    let kf = V3::X.cross(kt).norm();
    m.block(&Frame { o: knee + kf * 0.06, x: V3::X, y: kf, z: kt }, v3(0.046, 0.016, 0.04), SUIT.scale(1.08));
    let n = (V3::X * s + V3::Y * 0.9).norm();
    let mid = hip.lerp(knee, 0.48);
    let y = thigh;
    let x = y.cross(n).norm();
    m.block(&Frame { o: mid + n * 0.092, x, y, z: n }, v3(0.036, 0.075, 0.009), POCKET);
    m.block(&Frame { o: mid + n * 0.101 + y * -0.06, x, y, z: n }, v3(0.038, 0.016, 0.004), POCKET.scale(0.85));
    // Boot on the rudder pedal.
    let foot = |dy: f64, dz: f64| ankle + v3(0.0, dy, dz);
    loft(m, &[St(foot(0.03, 0.0), 0.050, 0.052, BOOT), St(foot(-0.045, -0.05), 0.052, 0.048, BOOT), St(foot(-0.06, -0.15), 0.048, 0.034, BOOT), St(foot(-0.055, -0.205), 0.036, 0.022, BOOT)], V3::X, &OCT);
    m.block(&Frame { o: foot(-0.07, 0.03), x: V3::X, y: V3::Y, z: V3::Z }, v3(0.044, 0.034, 0.034), BOOT);
    bar(m, foot(-0.104, 0.06), foot(-0.094, -0.215), 0.054, 0.009, V3::Y, SOLE);
    m.block(&Frame { o: foot(-0.125, -0.10), x: V3::X, y: V3::Y, z: V3::Z }, v3(0.06, 0.012, 0.10), Rgb(0.24, 0.26, 0.26));
}

/// Seat axes: reclined 30 degrees, `up` along the back, `fwd` out of it.
fn seat_axes() -> (V3, V3, V3) {
    let (c, s) = ((30.0 * DEG).cos(), (30.0 * DEG).sin());
    (v3(0.0, 0.58, -3.50), v3(0.0, c, s), v3(0.0, s, -c))
}

/// Seat, torso, harness, kneeboard and the control mounts: everything that
/// does not move. The caller facets the mesh.
pub fn build_static(m: &mut Mesh) {
    let (hip, up, fwd) = seat_axes();
    let seat = |o: V3| Frame { o, x: V3::X, y: up, z: -fwd };
    let at = |u: f64, f: f64| hip + up * u + fwd * f;

    // ACES II: pan, bucket sides, back, headrest and the catapult rails.
    m.block(&Frame { o: v3(0.0, 0.47, -3.40), x: V3::X, y: V3::Y, z: V3::Z }, v3(0.23, 0.06, 0.24), SEAT.scale(0.8));
    for s in [-1.0, 1.0] {
        let side = Frame { o: v3(s * 0.245, 0.56, -3.36), x: V3::X, y: V3::Y, z: V3::Z };
        m.block(&side, v3(0.016, 0.10, 0.25), SEAT.scale(0.9));
        bar(m, at(0.0, -0.21) + V3::X * s * 0.20, at(1.0, -0.21) + V3::X * s * 0.20, 0.024, 0.03, -fwd, SEAT.scale(0.7));
    }
    m.block(&seat(at(0.36, -0.17)), v3(0.23, 0.40, 0.05), SEAT);
    m.block(&seat(at(0.82, -0.21)), v3(0.155, 0.13, 0.07), SEAT.scale(1.1));
    // Yellow and black ejection handle on the front of the seat pan.
    for i in 0..8 {
        let (t0, t1) = (i as f64 * PI / 8.0, (i + 1) as f64 * PI / 8.0);
        let p = |t: f64| v3(0.06 * t.cos(), 0.535 + 0.045 * t.sin(), -3.655);
        let col = if i % 2 == 0 { Rgb(0.85, 0.68, 0.16) } else { Rgb(0.08, 0.08, 0.06) };
        bar(m, p(t0), p(t1), 0.0085, 0.0085, V3::Z, col);
    }

    // Torso in the flight suit, survival vest and harness. The head sits at
    // the eye and is left out.
    loft(m, &[St(at(0.0, 0.0), 0.165, 0.105, SUIT), St(at(0.14, 0.0), 0.15, 0.098, VEST), St(at(0.30, 0.012), 0.178, 0.118, VEST), St(at(0.45, 0.0), 0.188, 0.112, SUIT), St(at(0.535, -0.012), 0.15, 0.085, SUIT)], V3::X, &OCT);
    let chest = |u: f64, f: f64, x: f64| Frame { o: at(u, f) + V3::X * x, x: V3::X, y: up, z: fwd };
    for s in [-1.0, 1.0] {
        m.block(&chest(0.24, 0.10, s * 0.12), v3(0.026, 0.04, 0.012), VEST.scale(1.3));
        let x = |k: f64| V3::X * s * k;
        strap(m, &[at(0.50, 0.096) + x(0.10), at(0.44, 0.118) + x(0.095), at(0.30, 0.134) + x(0.085), at(0.14, 0.103) + x(0.06), at(0.07, 0.10) + x(0.04)], 0.016, 0.004, fwd, WEBBING);
        strap(m, &[at(0.04, 0.02) + x(0.17), at(0.05, 0.095) + x(0.12), at(0.07, 0.10) + x(0.03)], 0.018, 0.004, up, WEBBING);
    }
    m.block(&chest(0.07, 0.106, 0.0), v3(0.036, 0.03, 0.009), METALC);

    // Kneeboard strapped to the left thigh.
    let kb = Frame { o: v3(-0.165, 0.745, -3.76), x: V3::X, y: v3(0.0, 0.17, -0.985), z: v3(0.0, 0.985, 0.17) };
    m.block(&kb.at(0.0, 0.0, -0.006), v3(0.068, 0.085, 0.005), Rgb(0.22, 0.24, 0.20));
    screen(m, &kb, 0.12, 0.15, R_KNEE);

    // Raised sidestick mount, the right arm rest under the neutral elbow, and
    // the throttle slot along the outboard edge of the left console.
    m.block(&Frame { o: STICK - V3::Y * 0.014, x: V3::X, y: V3::Y, z: V3::Z }, v3(0.058, 0.014, 0.06), Rgb(0.10, 0.11, 0.11));
    let e = elbow(SHOULDER, stick_grip(&Hands::default()).1, pole(1.0));
    bar(m, v3(0.36, e.y - 0.075, e.z + 0.09), v3(0.36, e.y - 0.08, e.z - 0.20), 0.045, 0.022, V3::Y, Rgb(0.12, 0.12, 0.11));
    bar(m, v3(0.36, 0.64, e.z), v3(0.36, e.y - 0.08, e.z - 0.04), 0.03, 0.02, V3::Z, Rgb(0.09, 0.10, 0.10));
    m.block(&Frame { o: v3(-0.445, 0.645, -3.74), x: V3::X, y: V3::Y, z: V3::Z }, v3(0.017, 0.006, 0.13), Rgb(0.06, 0.065, 0.07));
}

fn pole(side: f64) -> V3 {
    v3(side * 0.55, -1.0, 0.55)
}

/// Sidestick axes for the current deflection, and the right wrist.
fn stick_grip(h: &Hands) -> (Grip, V3) {
    // The grip leans aft 7 degrees. The real stick barely moves; here full
    // deflection tilts it about 11 degrees so the hand visibly follows input.
    let g = v3(h.roll * 0.20, 1.0, 0.13 + h.pitch * 0.20).norm();
    let k = Grip::new(STICK + g * 0.105, g, 1.0);
    (k, k.p(-0.072, -0.004, 0.019 + 0.009))
}

fn sidestick(m: &mut Mesh, k: &Grip) {
    let base = STICK;
    loft(m, &[St(base, 0.042, 0.042, RUBBER), St(base + k.g * 0.02, 0.032, 0.034, RUBBER), St(base + k.g * 0.04, 0.017, 0.021, GRIP)], V3::X, &OCT);
    // Grip, then the broad head standing above the hand, leaning forward.
    let p = |f: f64, u: f64| k.p(f, u - 0.105, 0.0);
    loft(m, &[St(p(0.0, 0.035), 0.016, 0.020, GRIP), St(p(0.0, 0.07), 0.019, 0.025, GRIP), St(p(0.002, 0.13), 0.019, 0.025, GRIP), St(p(0.006, 0.15), 0.023, 0.031, GRIP), St(p(0.008, 0.172), 0.024, 0.032, GRIP), St(p(0.006, 0.186), 0.018, 0.025, GRIP), St(p(0.006, 0.192), 0.010, 0.014, GRIP)], V3::X, &OCT);
    // Palm rest flaring outboard at the foot of the grip, the trigger under
    // the head, and on top the red weapon release, the trim hat and the
    // target management switch.
    m.block(&k.frame(-0.008, -0.048, 0.027), v3(0.036, 0.007, 0.02), GRIP.scale(1.25));
    m.block(&k.frame(0.033, 0.04, 0.0), v3(0.006, 0.01, 0.007), GRIP.scale(0.7));
    let top = |f: f64, s: f64| k.p(f, 0.082, s);
    knob(m, top(0.016, -0.010), k.g, 0.0065, 0.008, RED);
    knob(m, top(0.0, 0.003), k.g, 0.006, 0.009, METALC.scale(0.6));
    m.block(&k.frame(0.0, 0.09, 0.003), v3(0.009, 0.002, 0.0025), METALC.scale(0.6));
    m.block(&k.frame(0.0, 0.09, 0.003), v3(0.0025, 0.002, 0.009), METALC.scale(0.6));
    knob(m, top(-0.012, 0.012), k.g, 0.006, 0.007, METALC.scale(0.45));
}

/// Short octagonal cylinder from `c` along `axis`: knobs and thumbwheels.
fn knob(m: &mut Mesh, c: V3, axis: V3, r: f64, len: f64, col: Rgb) {
    let side = if axis.y.abs() < 0.9 { V3::Y } else { V3::X };
    loft(m, &[St(c, r, r, col), St(c + axis * len, r, r, col)], side, &OCT);
}

/// F-16 throttle (TQS) after the grip diagram, cockpit photos and the 1:1
/// Viper TQS replica: a lever on the outboard edge of the left console
/// carries a grip that runs inboard, rising about 15 degrees, from a fat
/// root bulb at the lever through a waist to the switch head at its inboard
/// end. The left hand lies level on top of it: back of the hand up, the
/// mitten falling over the front, thumb along the aft face to the
/// cursor control and speed brake. Returns the wrist.
fn throttle(m: &mut Mesh, h: &Hands) -> V3 {
    let z = -3.64 - h.throttle * 0.20;
    let a = v3(0.95, 0.26, -0.10).norm();
    // Grip frame: `g` along the grip toward the head, `fwd` forward, `out` up.
    let k = Grip::new(v3(-0.36, 0.79, z - 0.01), a, -1.0);

    // Lever from the slot to a pivot under the root, and the cutoff release
    // trigger on its outboard side.
    let foot = v3(-0.445, 0.64, z + 0.02);
    bar(m, foot, k.p(0.0, -0.088, -0.022), 0.012, 0.022, V3::Z, Rgb(0.20, 0.22, 0.22));
    strap(m, &[foot + v3(-0.012, 0.03, 0.0), foot + v3(-0.02, 0.06, -0.01), foot + v3(-0.016, 0.075, -0.035)], 0.004, 0.006, V3::X, METALC);

    // Grip body: root bulb, waist under the hand, head at the inboard end.
    let st = |u: f64, w: f64, h: f64| St(k.p(0.0, u, 0.0), w, h, GRIP);
    loft(m, &[st(-0.098, 0.022, 0.026), st(-0.085, 0.032, 0.036), st(-0.050, 0.030, 0.033), st(-0.010, 0.023, 0.026), st(0.030, 0.024, 0.027), st(0.058, 0.028, 0.031), st(0.080, 0.025, 0.028), st(0.088, 0.014, 0.016)], k.out, &OCT);
    // MAN RNG/UNCAGE knob on top of the head, ANT ELEV thumbwheel on its end,
    // DOG FIGHT switch and comm switch on top behind them, and on the aft
    // face the speed brake slider and the radar cursor control.
    let (white, grey) = (Rgb(0.74, 0.74, 0.71), Rgb(0.50, 0.51, 0.50));
    knob(m, k.p(0.004, 0.062, 0.024), k.out, 0.011, 0.013, white);
    knob(m, k.p(0.004, 0.084, 0.0), k.g, 0.014, 0.009, white);
    knob(m, k.p(-0.014, 0.070, 0.020), (k.out - k.fwd).norm(), 0.007, 0.010, grey);
    m.block(&k.frame(-0.006, 0.046, 0.026), v3(0.005, 0.005, 0.004), GRIP.scale(0.6));
    m.block(&k.frame(-0.028, 0.040, 0.010), v3(0.003, 0.011, 0.004), grey);
    knob(m, k.p(-0.026, 0.056, -0.010), -k.fwd, 0.008, 0.010, grey);

    // The hand lies level on top of the grip, in line with the forearm
    // (forward and a little inboard), one mitten piece from the wrist over
    // the root bulb and down in front; the thumb runs along the aft face.
    let hf = v3(0.45, 0.0, -0.89).norm();
    let hs = V3::Y.cross(hf);
    let c = k.p(-0.004, -0.024, 0.0) + V3::Y * 0.040;
    let at = |s: f64, y: f64| c + hf * s + V3::Y * y;
    let wrist = at(-0.075, -0.010);
    loft(m, &[St(wrist, 0.034, 0.017, GLOVE), St(at(-0.040, 0.0), 0.044, 0.016, GLOVE), St(at(0.0, 0.002), 0.047, 0.015, GLOVE), St(at(0.028, -0.006), 0.046, 0.013, GLOVE), St(at(0.044, -0.026), 0.044, 0.012, GLOVE), St(at(0.048, -0.048), 0.040, 0.011, GLOVE)], hs, &OCT);
    thumb_loft(m, [k.p(-0.034, -0.044, 0.008), k.p(-0.037, -0.008, 0.002), k.p(-0.033, 0.026, 0.004)], k.out);
    wrist
}

/// Legs on the pedals, both arms and the two controls under the hands.
pub fn build_dynamic(m: &mut Mesh, h: &Hands) {
    for s in [-1.0, 1.0] {
        leg(m, s, h.rudder);
    }
    let (k, _) = stick_grip(h);
    sidestick(m, &k);
    let (rx, rf) = (0.019, 0.025);
    let wrist = fist(m, &k, rx, rf, [k.p(-rf - 0.006, 0.026, -rx * 0.5), k.p(-0.027, 0.058, -rx - 0.003), k.p(-0.02, 0.078, -0.015)]);
    arm(m, SHOULDER, wrist, pole(1.0), (0.026, 0.036));
    let wrist = throttle(m, h);
    arm(m, v3(-SHOULDER.x, SHOULDER.y, SHOULDER.z), wrist, pole(-1.0), (0.034, 0.018));
    m.facet();
}
