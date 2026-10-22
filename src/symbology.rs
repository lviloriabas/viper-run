//! Weapon symbology: the HUD's gun funnel and LCOS pipper, the Sidewinder
//! seeker circle, the AMRAAM steering circle, the CCIP pipper and bomb fall
//! line, the target designator box and locator line; the FCR page's
//! contacts and the RWR's threats; and the target brackets and stores line
//! of the flight overlays.
use crate::canvas::{rgba, with_alpha, Align, Canvas, Col};
use crate::combat::{Combat, Mode, Store};
use crate::math::*;
use crate::sim::Aircraft;
use crate::targets::Kind;
use crate::weapons::{muzzle, Who};
use crate::world::Camera;

const HUD: Col = rgba(0xa9, 0xff, 0xb2, 1.0);
const AMBER: Col = rgba(0xff, 0xc0, 0x5b, 1.0);
const RED: Col = rgba(0xff, 0x6d, 0x5f, 1.0);
const GREEN: Col = rgba(0x98, 0xff, 0xa3, 1.0);

/// Default gun range with no lock: 2,000 ft.
const GUN_DEFAULT: f64 = 610.0;
/// Wingspan the EEGS funnel is scaled for: 35 ft.
const FUNNEL_SPAN: f64 = 10.7;

/// How the HUD projects: a world direction from the eye, or a point in HUD
/// units (1/650 rad from the boresight, y down), to window pixels.
pub struct Pen<'a> {
    pub dir: &'a dyn Fn(V3) -> Option<[f32; 2]>,
    pub at: &'a dyn Fn(f64, f64) -> Option<[f32; 2]>,
    pub eye: V3,
    pub big: f32,
    pub small: f32,
    pub lw: f32,
}

impl Pen<'_> {
    fn point(&self, p: V3) -> Option<[f32; 2]> {
        (self.dir)((p - self.eye).norm())
    }
    fn text(&self, cv: &mut Canvas, s: &str, x: f64, y: f64, scale: f32, align: Align) {
        if let Some(p) = (self.at)(x, y) {
            cv.text(s, p[0], p[1], scale, HUD, align);
        }
    }
}

/// Stores line: master mode, the selected weapon and what is left.
pub fn stores_line(c: &Combat) -> String {
    let l = &c.loadout;
    let mut s = String::new();
    for (store, short) in [(Store::Aim9, "9M"), (Store::Aim120, "120C"), (Store::Mk82, "82")] {
        let n = l.count(store);
        let carried = l.stations.iter().any(|st| st.store == store);
        if carried {
            s.push_str(&format!("{n}{short} "));
        }
    }
    format!("{s}GUN {}  CHF {} FLR {}", l.rounds, l.chaff, l.flares)
}

/// The selected weapon, its ammunition and what is left of it, for the HUD.
fn selected(c: &Combat) -> String {
    let (n, what) = match c.mode {
        Mode::Nav => return String::new(),
        Mode::Gun => (c.loadout.rounds, "20MM"),
        Mode::Srm => (c.loadout.count(Store::Aim9), "AIM-9M"),
        Mode::Mrm => (c.loadout.count(Store::Aim120), "AIM-120C"),
        Mode::Ccip => (c.loadout.count(Store::Mk82), "MK-82"),
    };
    if n == 0 {
        format!("{what} EMPTY")
    } else {
        format!("{n} {what}")
    }
}

/// Weapon symbology over the NAV HUD (called inside its combiner clip).
pub fn hud(cv: &mut Canvas, pen: &Pen, a: &Aircraft, c: &Combat, time: f64) {
    let (big, lw) = (pen.big, pen.lw);
    let half = 70.0;
    let flash = (time * 3.0).fract() < 0.6;
    pen.text(cv, &selected(c), -half, 82.0, pen.small, Align::Left);
    if c.hp < 100.0 {
        pen.text(cv, &format!("DMG {}", (100.0 - c.hp).round() as i64), half, 82.0, pen.small, Align::Right);
    }
    if (c.warn > 0.0 || c.inbound().next().is_some()) && flash {
        pen.text(cv, "MISSILE", 0.0, 10.0, big, Align::Center);
    }
    let ax = a.q.axes();
    let lock = c.lock.and_then(|w| c.track(w, a));
    // Target designator box, or a locator line toward it from the gun cross.
    if let Some((tp, tv)) = lock {
        let range = (tp - a.pos).len();
        match pen.point(tp) {
            Some(p) => {
                let s = 5.0 * big;
                cv.stroke_rect(p[0] - s, p[1] - s, 2.0 * s, 2.0 * s, lw, HUD);
            }
            None => locator(cv, pen, a, tp),
        }
        let closure = -(tv - a.vel).dot((tp - a.pos).norm()) * 1.94384;
        let text = if range < 3700.0 { format!("{:.1} KFT", range * 3.28084 / 1000.0) } else { format!("{:.1} NM", range / 1852.0) };
        pen.text(cv, &text, half, 94.0, pen.small, Align::Right);
        if matches!(c.lock, Some(Who::Bandit(_))) {
            pen.text(cv, &format!("VC {}", closure.round() as i64), half, 106.0, pen.small, Align::Right);
        }
    }
    match c.mode {
        Mode::Nav => {}
        Mode::Gun => {
            let (gun, _) = muzzle(a);
            // EEGS funnel: the stream's two edges a wingspan apart.
            let mut left = Vec::new();
            let mut right = Vec::new();
            for &(p, _, age) in c.sight.shots.iter() {
                if age < 0.12 {
                    continue;
                }
                if let (Some(l), Some(r)) = (pen.point(p - ax.right * (FUNNEL_SPAN * 0.5)), pen.point(p + ax.right * (FUNNEL_SPAN * 0.5))) {
                    left.push(l);
                    right.push(r);
                }
            }
            if left.len() > 1 {
                cv.polyline(&left, lw, false, HUD);
                cv.polyline(&right, lw, false, HUD);
            }
            // LCOS pipper at the target's range, or 2,000 ft without a lock.
            let range = lock.map(|(p, _)| (p - gun).len()).filter(|&r| matches!(c.lock, Some(Who::Bandit(_))) && r < 1600.0).unwrap_or(GUN_DEFAULT);
            if let Some(p) = c.sight.at_range(gun, range).and_then(|p| pen.point(p)) {
                let r = 9.0 * big;
                cv.circle(p[0], p[1], r, lw, HUD);
                cv.disc(p[0], p[1], 1.2 * big, HUD);
                if lock.is_some() && range != GUN_DEFAULT {
                    // Range bar: the ring's arc shrinks as the range closes, full at 4,000 ft.
                    let f = clamp(range / 1220.0, 0.0, 1.0) as f32;
                    cv.arc(p[0], p[1], r + 2.5 * big, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU, lw * 1.5, HUD);
                }
                if let Some((tp, _)) = lock {
                    let on = pen.point(tp).is_some_and(|t| ((t[0] - p[0]).powi(2) + (t[1] - p[1]).powi(2)).sqrt() < r);
                    if on && range < 1100.0 && flash {
                        pen.text(cv, "SHOOT", 0.0, -36.0, big, Align::Center);
                    }
                }
            }
        }
        Mode::Srm => {
            // Seeker circle: on its target when tracking (solid), on the
            // boresight while searching (dashed).
            let look = c.seeker.and_then(|w| c.track(w, a)).map(|(p, _)| (p - pen.eye).norm()).unwrap_or(ax.forward);
            if let (Some(p), Some(e)) = ((pen.dir)(look), (pen.dir)((look + ax.up * (2.5 * DEG)).norm())) {
                let r = ((p[1] - e[1]).abs()).max(6.0 * big);
                if c.seeker.is_some() {
                    cv.circle(p[0], p[1], r, lw * 1.5, HUD);
                } else {
                    for k in 0..8 {
                        let a0 = k as f32 * std::f32::consts::TAU / 8.0;
                        cv.arc(p[0], p[1], r, a0, a0 + 0.45, lw, HUD);
                    }
                }
            }
            if c.shoot {
                pen.text(cv, "IN RNG", -half, 94.0, pen.small, Align::Left);
                if flash {
                    pen.text(cv, "SHOOT", 0.0, -36.0, big, Align::Center);
                }
            }
        }
        Mode::Mrm => {
            // Allowable steering error circle with the steering dot on the target.
            if let Some(cen) = (pen.at)(0.0, 0.0) {
                let edge = (pen.at)(0.0, 30.0).map(|e| e[1] - cen[1]).unwrap_or(40.0);
                cv.circle(cen[0], cen[1], edge, lw, HUD);
                if let Some((tp, _)) = lock {
                    let d = a.q.conj().rotate((tp - a.pos).norm());
                    let (x, y) = (d.x.atan2(-d.z) * 650.0, -d.y.atan2(-d.z) * 650.0);
                    let k = (30.0 / (x * x + y * y).sqrt().max(1e-6)).min(1.0);
                    if let Some(p) = (pen.at)(x * k, y * k) {
                        cv.disc(p[0], p[1], 2.2 * big, HUD);
                    }
                }
            }
            if c.shoot {
                pen.text(cv, "IN RNG", -half, 94.0, pen.small, Align::Left);
                if flash {
                    pen.text(cv, "SHOOT", 0.0, -36.0, big, Align::Center);
                }
            }
        }
        Mode::Ccip => {
            if let Some((p, tof)) = c.ccip {
                let fpm = (pen.dir)(a.vel.norm());
                if let Some(q) = pen.point(p) {
                    let r = 6.0 * big;
                    cv.circle(q[0], q[1], r, lw, HUD);
                    cv.disc(q[0], q[1], 1.2 * big, HUD);
                    // Bomb fall line from the flight-path marker down to the pipper.
                    if let Some(f) = fpm {
                        let (dx, dy) = (q[0] - f[0], q[1] - f[1]);
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        cv.line(f[0], f[1], q[0] - dx / len * r, q[1] - dy / len * r, lw, HUD);
                    }
                }
                pen.text(cv, &format!("TOF {tof:.1}"), -half, 94.0, pen.small, Align::Left);
            }
        }
    }
}

/// Target locator line: from the gun cross toward a target outside the
/// HUD's view, with the angle off the nose.
fn locator(cv: &mut Canvas, pen: &Pen, a: &Aircraft, tp: V3) {
    let d = a.q.conj().rotate((tp - a.pos).norm());
    let (x, y) = (d.x, -d.y);
    let k = (x * x + y * y).sqrt().max(1e-6);
    let off = (-d.z).clamp(-1.0, 1.0).acos() * RAD;
    let (ux, uy) = (x / k, y / k);
    if let (Some(p), Some(q)) = ((pen.at)(0.0, -55.0), (pen.at)(ux * 34.0, -55.0 + uy * 34.0)) {
        cv.line(p[0], p[1], q[0], q[1], pen.lw, HUD);
        pen.text(cv, &format!("{}", off.round() as i64), ux * 42.0, -55.0 + uy * 42.0, pen.small, Align::Center);
    }
}

/// FCR range scale in nautical miles for the present picture.
pub fn fcr_scale(a: &Aircraft, c: &Combat) -> f64 {
    let far = if c.mode.air_to_ground() {
        c.sites.iter().filter(|s| !s.dead).map(|s| (s.pos - a.pos).len()).reduce(f64::min).unwrap_or(0.0)
    } else {
        c.lock.and_then(|w| c.track(w, a)).map(|(p, _)| (p - a.pos).len()).or_else(|| c.bandits.iter().filter(|b| b.alive).map(|b| (b.jet.pos - a.pos).len()).reduce(f64::min)).unwrap_or(0.0)
    } / 1852.0;
    [10.0, 20.0, 40.0, 80.0].into_iter().find(|&s| far < s * 0.9).unwrap_or(80.0)
}

/// FCR B-scope contacts inside the frame `[left, top, right, bottom]`:
/// azimuth across, range up. Air contacts in the air-to-air and NAV modes,
/// ground targets on the ground map in CCIP. Returns false when empty.
pub fn fcr(cv: &mut Canvas, f: [f32; 4], a: &Aircraft, c: &Combat, nm: f64) -> bool {
    let [l, t, r, b] = f;
    let hd = a.heading * DEG;
    let place = |p: V3| -> Option<[f32; 2]> {
        let d = p - a.pos;
        let az = wrap180((d.x.atan2(-d.z) - hd) * RAD);
        let range = v3(d.x, 0.0, d.z).len() / 1852.0;
        (az.abs() <= 60.0 && range < nm).then(|| [l + ((az + 60.0) / 120.0) as f32 * (r - l), b - (range / nm) as f32 * (b - t)])
    };
    let mut any = false;
    if c.mode.air_to_ground() {
        cv.text("GM", r - 4.0, t + 10.0, 1.0, GREEN, Align::Right);
        for (i, s) in c.sites.iter().enumerate().filter(|(_, s)| !s.dead) {
            if let Some(p) = place(s.pos) {
                any = true;
                let locked = c.lock == Some(Who::Ground(i));
                let k = if s.kind == Kind::Bridge || s.kind == Kind::Warehouse { 4.0 } else { 2.5 };
                cv.polyline(&[[p[0], p[1] - k], [p[0] + k, p[1]], [p[0], p[1] + k], [p[0] - k, p[1]]], if locked { 2.0 } else { 1.0 }, true, if locked { AMBER } else { GREEN });
            }
        }
    } else {
        for (i, bd) in c.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
            let Some(p) = place(bd.jet.pos) else { continue };
            any = true;
            let locked = c.lock == Some(Who::Bandit(i));
            let s = if locked { 5.0 } else { 3.5 };
            cv.rect(p[0] - s, p[1] - s, 2.0 * s, 2.0 * s, if locked { AMBER } else { GREEN });
            // Aspect stick: the contact's track relative to ours.
            let th = (bd.jet.heading - a.heading) * DEG;
            cv.line(p[0], p[1], p[0] + th.sin() as f32 * 10.0, p[1] - th.cos() as f32 * 10.0, 1.0, if locked { AMBER } else { GREEN });
            if locked {
                let alt = bd.jet.pos.y * 3.28084 / 1000.0;
                cv.text(&format!("{:.0}", alt), p[0] + 8.0, p[1] + 9.0, 1.0, AMBER, Align::Left);
                let rng = (bd.jet.pos - a.pos).len() / 1852.0;
                cv.text(&format!("{} {:.1}NM {}KT", bd.callsign, rng, (bd.jet.tas_kt).round() as i64), (l + r) * 0.5, t + 10.0, 1.0, AMBER, Align::Center);
            }
        }
    }
    any
}

/// RWR threats around the scope centre: lethal ones closer in, a diamond
/// on those shooting or locked on, a flashing M for missiles in flight.
/// Returns false with nothing to show.
pub fn rwr(cv: &mut Canvas, cx: f32, cy: f32, a: &Aircraft, c: &Combat, time: f64) -> bool {
    let hd = a.heading * DEG;
    let at = |p: V3, r: f32| {
        let d = p - a.pos;
        let az = d.x.atan2(-d.z) - hd;
        [cx + az.sin() as f32 * r, cy - az.cos() as f32 * r]
    };
    let mut any = false;
    for (p, sym, hot) in c.emitters(a) {
        any = true;
        let q = at(p, if hot { 26.0 } else { 44.0 });
        let col = if hot { RED } else { GREEN };
        cv.text(sym, q[0], q[1], 1.0, col, Align::Center);
        if hot {
            cv.polyline(&[[q[0], q[1] - 8.0], [q[0] + 9.0, q[1]], [q[0], q[1] + 8.0], [q[0] - 9.0, q[1]]], 1.0, true, col);
        }
    }
    if (time * 4.0).fract() < 0.6 {
        for m in c.inbound() {
            any = true;
            let q = at(m.pos, 14.0);
            cv.text("M", q[0], q[1], 1.0, RED, Align::Center);
        }
    }
    any
}

/// Screen position of a world point for an overlay camera.
fn project(cam: &Camera, w: f32, h: f32, p: V3) -> Option<(f32, f32)> {
    let rel = p - cam.eye;
    let z = rel.dot(cam.forward);
    if z <= 0.5 {
        return None;
    }
    let tan = (cam.fov / 2.0).tan();
    let nx = rel.dot(cam.right) / (z * tan * (w / h) as f64);
    let ny = rel.dot(cam.up) / (z * tan);
    Some((((nx * 0.5 + 0.5) * w as f64) as f32, ((0.5 - ny * 0.5) * h as f64) as f32))
}

fn brackets(cv: &mut Canvas, x: f32, y: f32, s: f32, lw: f32, col: Col) {
    let k = s * 0.45;
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (cx, cy) = (x + dx * s, y + dy * s);
        cv.line(cx, cy, cx - dx * k, cy, lw, col);
        cv.line(cx, cy, cx, cy - dy * k, lw, col);
    }
}

/// Target brackets with range, the stores line and the missile warning
/// over the flight view. `chase` draws the outside view's fuller set.
#[allow(clippy::too_many_arguments)]
pub fn overlay(cv: &mut Canvas, w: f32, h: f32, s: f32, a: &Aircraft, c: &Combat, cam: &Camera, time: f64, chase: bool) {
    let sm = (s - 1.0).max(1.0);
    let mut mark = |p: V3, who: Who, col: Col, label: Option<&str>, detail: bool| {
        let d = (p - cam.eye).len();
        let Some((x, y)) = project(cam, w, h, p) else { return };
        let locked = c.lock == Some(who);
        let size = (7.0 + 600.0 / d.max(60.0) as f32 * 4.0).min(28.0) * sm;
        let alpha = if chase { 1.0 } else { 0.65 };
        if locked {
            cv.stroke_rect(x - size, y - size, 2.0 * size, 2.0 * size, 1.5, with_alpha(AMBER, alpha));
        } else {
            brackets(cv, x, y, size, 1.0, with_alpha(col, alpha * 0.85));
        }
        if (chase && detail) || locked {
            let text = if d < 1852.0 { format!("{} M", d.round() as i64) } else { format!("{:.1} NM", d / 1852.0) };
            cv.text(&text, x, y + size + 6.0 * sm, sm, with_alpha(if locked { AMBER } else { col }, alpha), Align::Center);
            if let Some(l) = label.filter(|_| chase) {
                cv.text(l, x, y - size - 6.0 * sm, sm, with_alpha(col, alpha * 0.8), Align::Center);
            }
        }
    };
    for (i, b) in c.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
        if (b.jet.pos - cam.eye).len() < 40000.0 {
            mark(b.jet.pos, Who::Bandit(i), RED, Some(b.callsign), true);
        }
    }
    for (i, st) in c.sites.iter().enumerate().filter(|(_, st)| !st.dead) {
        let d = (st.pos - cam.eye).len();
        if d < 9000.0 {
            // Names and ranges only up close, where the groups spread apart.
            mark(st.centre(), Who::Ground(i), AMBER, Some(st.kind.name()), d < 2500.0);
        }
    }
    if c.kind == crate::combat::MissionKind::Gates && c.mode == Mode::Nav {
        return;
    }
    let y = if chase { h - 54.0 * s } else { h * 0.058 + 31.0 * s };
    let (x, align) = if chase { (w * 0.5, Align::Center) } else { (w * 0.974, Align::Right) };
    cv.text(&format!("{}  {}", c.mode.label(), stores_line(c)), x, y, sm, with_alpha(HUD, 0.8), align);
    if c.hp < 100.0 {
        let col = if c.hp < 40.0 { RED } else { AMBER };
        cv.text(&format!("AIRFRAME {}%", c.hp.round() as i64), x, y + 9.0 * sm, sm, col, align);
    }
    // Outside the cockpit a small RWR scope repeats the panel's.
    if chase && c.kind != crate::combat::MissionKind::Gates {
        let (cx, cy) = (w - 70.0 * s, h * 0.47);
        cv.disc(cx, cy, 58.0, rgba(4, 10, 6, 0.55));
        for r in [20.0, 38.0, 56.0] {
            cv.circle(cx, cy, r, 1.0, with_alpha(HUD, 0.25));
        }
        cv.text("RWR", cx, cy + 66.0, sm, with_alpha(HUD, 0.6), Align::Center);
        rwr(cv, cx, cy, a, c, time);
    }
    if chase && (c.warn > 0.0 || c.inbound().next().is_some()) && (time * 3.0).fract() < 0.6 {
        cv.text("MISSILE", w * 0.5, h * 0.36, s * 2.0, RED, Align::Center);
    }
}

/// HUD field of view seen from the design eye, radians off the boresight:
/// azimuth either side, and elevation from its bottom to its top edge.
pub const HUD_FOV: (f64, f64, f64) = (7.6 * DEG, -12.6 * DEG, 7.9 * DEG);

/// True when a body-axis line of sight passes through the HUD combiner or
/// falls inside the cockpit (well below the canopy rail), where the helmet
/// blanks its own symbology, as JHMCS does with its cockpit mask.
pub fn through_hud(los_body: V3) -> bool {
    let az = los_body.x.atan2(-los_body.z);
    let el = los_body.y.atan2((los_body.x * los_body.x + los_body.z * los_body.z).sqrt());
    let hud = az.abs() < HUD_FOV.0 && el > HUD_FOV.1 && el < HUD_FOV.2;
    let cockpit = el < -20.0 * DEG && az.abs() < 110.0 * DEG;
    hud || cockpit
}

/// Helmet-mounted display, in the manner of JHMCS. Visor-fixed: the aiming
/// cross on the helmet line of sight, airspeed and altitude boxes either
/// side of it, heading above, Mach and G under the airspeed, the master
/// mode, arm state and weapon count at the bottom, and the off-boresight
/// angle. World-stabilised: the aircraft boresight cross when it is in the
/// visor, the locked target's designator box, or a locator line toward it
/// from the cross with the angle to go, and the Sidewinder seeker, which the
/// helmet cues off the boresight. It is blanked while the pilot looks
/// through the HUD (`Combat::helmet` is None then).
#[allow(clippy::too_many_arguments)]
pub fn helmet(cv: &mut Canvas, w: f32, h: f32, s: f32, a: &Aircraft, c: &Combat, cam: &Camera, time: f64) {
    let Some(look) = c.helmet else { return };
    let sm = (s - 1.0).max(1.0);
    let col = with_alpha(HUD, 0.85);
    let flash = (time * 3.0).fract() < 0.6;
    let (cx, cy) = (w * 0.5, h * 0.5);
    let r = 14.0 * s;
    // Aiming cross: a ring with four ticks and a centre dot.
    cv.circle(cx, cy, r, 1.5, col);
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        cv.line(cx + dx * r * 0.4, cy + dy * r * 0.4, cx + dx * r * 0.8, cy + dy * r * 0.8, 1.5, col);
    }
    cv.disc(cx, cy, 1.2, col);

    // Visor-fixed air data round the cross.
    let ad = crate::air::AirData::of(a);
    let (dx, dy) = (h * 0.20, h * 0.03);
    let boxed = |cv: &mut Canvas, text: &str, x: f32, y: f32| {
        let tw = Canvas::text_width(text, s) + 6.0 * s;
        let th = 9.0 * s;
        cv.stroke_rect(x - tw * 0.5, y - th * 0.5, tw, th, 1.0, col);
        cv.text(text, x, y, s, col, Align::Center);
    };
    boxed(cv, &format!("{}", ad.kcas.round() as i64), cx - dx, cy - dy);
    boxed(cv, &format!("{}", ad.baro_ft.round() as i64), cx + dx, cy - dy);
    boxed(cv, &format!("{:03}", (a.heading.round() as i64).rem_euclid(360)), cx, cy - h * 0.17);
    cv.text(&format!("M {:.2}", ad.mach), cx - dx, cy + dy + 2.0 * s, sm, col, Align::Center);
    cv.text(&format!("G {:.1}", a.g_smooth), cx - dx, cy + dy + 11.0 * s, sm, col, Align::Center);
    if a.agl < 1500.0 {
        cv.text(&format!("R {}", (a.agl * 3.28084 / 10.0).round() as i64 * 10), cx + dx, cy + dy + 2.0 * s, sm, col, Align::Center);
    }
    let oba = a.q.axes().forward.dot(look).clamp(-1.0, 1.0).acos() * RAD;
    cv.text(&format!("{}", oba.round() as i64), cx + r + 4.0 * s, cy + r, sm, col, Align::Left);
    let arm = if c.mode == Mode::Nav { "NAV".to_string() } else { format!("{} ARM", c.mode.label()) };
    cv.text(&arm, cx - dx, cy + h * 0.16, sm, col, Align::Center);
    cv.text(&selected(c), cx + h * 0.06, cy + h * 0.16, sm, col, Align::Left);
    if (c.warn > 0.0 || c.inbound().next().is_some()) && flash {
        cv.text("MISSILE", cx, cy - h * 0.11, s, col, Align::Center);
    }

    // Aircraft boresight cross, where the nose points, when the visor sees it.
    let ax = a.q.axes();
    if let Some((x, y)) = project(cam, w, h, cam.eye + ax.forward * 2000.0) {
        if (x - cx).abs() < h * 0.3 && (y - cy).abs() < h * 0.3 {
            let k = 6.0 * s;
            cv.line(x - k, y, x + k, y, 1.5, col);
            cv.line(x, y - k, x, y + k * 0.5, 1.5, col);
        }
    }

    // Locked target: designator box in the visor, or a locator line toward it.
    if let Some((p, _)) = c.lock.and_then(|l| c.track(l, a)) {
        let range = (p - a.pos).len();
        let text = if range < 3700.0 { format!("{:.1} KFT", range * 3.28084 / 1000.0) } else { format!("{:.1} NM", range / 1852.0) };
        cv.text(&text, cx + dx, cy + dy + 11.0 * s, sm, col, Align::Center);
        let seen = project(cam, w, h, p).filter(|&(x, y)| (x - cx).abs() < h * 0.32 && (y - cy).abs() < h * 0.32);
        match seen {
            Some((x, y)) => {
                let b = 9.0 * s;
                cv.stroke_rect(x - b, y - b, 2.0 * b, 2.0 * b, 1.5, col);
            }
            None => {
                let d = (p - cam.eye).norm();
                let (lx, ly) = (d.dot(cam.right), d.dot(cam.up));
                let k = (lx * lx + ly * ly).sqrt().max(1e-6);
                let (ux, uy) = ((lx / k) as f32, (-ly / k) as f32);
                let (x0, y0, x1) = (cx + ux * r, cy + uy * r, cx + ux * r * 3.2);
                cv.line(x0, y0, x1, cy + uy * r * 3.2, 1.5, col);
                let off = d.dot(cam.forward).clamp(-1.0, 1.0).acos() * RAD;
                cv.text(&format!("{}", off.round() as i64), cx + ux * r * 4.0, cy + uy * r * 4.0, sm, col, Align::Center);
            }
        }
    }
    if c.mode == Mode::Srm && c.loadout.count(Store::Aim9) > 0 {
        match c.seeker.and_then(|w| c.track(w, a)).and_then(|(p, _)| project(cam, w, h, p)) {
            Some((x, y)) => {
                cv.circle(x, y, 11.0 * s, 2.0, col);
                if c.shoot && flash {
                    cv.text("SHOOT", cx, cy - r - 10.0 * sm, s, col, Align::Center);
                }
            }
            None => {
                for k in 0..8 {
                    let a0 = k as f32 * std::f32::consts::TAU / 8.0;
                    cv.arc(cx, cy, r * 1.6, a0, a0 + 0.4, 1.0, col);
                }
            }
        }
    }
}
