//! Weapon symbology: the HUD's gun funnel and LCOS pipper, the Sidewinder
//! seeker circle, the AMRAAM steering circle, the CCIP pipper and bomb fall
//! line, the target designator box and locator line; the FCR page's
//! contacts and the RWR's threats; and the target brackets and stores line
//! of the flight overlays.
use crate::canvas::{rgba, with_alpha, Align, Canvas, Col};
use crate::combat::{Combat, Mode};
use crate::math::*;
use crate::sim::{Aircraft, Type};
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
    /// Symbology colour: the F-16's green, or the ILS-31's yellower one.
    pub col: Col,
    /// Russian lettering and metric units, as on the MiG-29's ILS-31.
    pub russian: bool,
}

impl Pen<'_> {
    fn point(&self, p: V3) -> Option<[f32; 2]> {
        (self.dir)((p - self.eye).norm())
    }
    fn text(&self, cv: &mut Canvas, s: &str, x: f64, y: f64, scale: f32, align: Align) {
        if let Some(p) = (self.at)(x, y) {
            cv.text(s, p[0], p[1], scale, self.col, align);
        }
    }
}

/// The selected weapon by its real name and kind (the brevity code for
/// missiles) and what is left of it: what the weapon switch shows.
pub fn weapon_line(c: &Combat) -> String {
    match c.weapon_left() {
        ("", _) => c.weapon_name(),
        (_, left) => format!("{} x{left}", c.weapon_name()),
    }
}

/// Stores line: the selected weapon, then the countermeasures apart.
pub fn stores_line(c: &Combat) -> String {
    let l = &c.loadout;
    format!("{}  //  CHAFF {}  FLARES {}", weapon_line(c), l.chaff, l.flares)
}

/// The selected weapon, its ammunition and what is left of it, for the HUD.
fn selected(c: &Combat, russian: bool) -> String {
    let (what, n) = match c.weapon_left() {
        ("", _) => return String::new(),
        w => w,
    };
    if russian {
        // The GSh-301 and the R- and FAB- stores in their own letters.
        let name = if what.ends_with("MM") { "ГШ-301".to_string() } else { what.replacen("R-", "Р-", 1).replacen("FAB-", "ФАБ-", 1) };
        return format!("{name} {n}");
    }
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
    let ru = pen.russian;
    pen.text(cv, &selected(c, ru), -half, 82.0, pen.small, Align::Left);
    if c.hp < 100.0 {
        pen.text(cv, &format!("{} {}", if ru { "ПОВР" } else { "DMG" }, (100.0 - c.hp).round() as i64), half, 82.0, pen.small, Align::Right);
    }
    if (c.warn > 0.0 || c.inbound().next().is_some()) && flash {
        pen.text(cv, if ru { "РАКЕТА" } else { "MISSILE" }, 0.0, 10.0, big, Align::Center);
    }
    let ax = a.q.axes();
    let lock = c.lock.and_then(|w| c.track(w, a));
    // Target designator box, or a locator line toward it from the gun cross.
    if let Some((tp, tv)) = lock {
        let range = (tp - a.pos).len();
        match pen.point(tp) {
            Some(p) => {
                let s = 5.0 * big;
                cv.stroke_rect(p[0] - s, p[1] - s, 2.0 * s, 2.0 * s, lw, pen.col);
            }
            None => locator(cv, pen, a, tp),
        }
        let closure = -(tv - a.vel).dot((tp - a.pos).norm()) * 1.94384;
        // An IRST track has a range only inside the laser's reach.
        let ranged = !c.irst || c.laser_range(a).is_some();
        // The ILS-31 gives range in kilometres, the laser's ЛД.
        let text = if !ranged {
            if ru { "ЛД ---" } else { "LASER ---" }.to_string()
        } else if ru {
            format!("Д {:.1}", range / 1000.0)
        } else if range < 3700.0 {
            format!("{:.1} KFT", range * 3.28084 / 1000.0)
        } else {
            format!("{:.1} NM", range / 1852.0)
        };
        pen.text(cv, &text, half, 94.0, pen.small, Align::Right);
        if matches!(c.lock, Some(Who::Bandit(_))) && ranged {
            let vc = if ru { format!("V {}", (closure * 1.852).round() as i64) } else { format!("VC {}", closure.round() as i64) };
            pen.text(cv, &vc, half, 106.0, pen.small, Align::Right);
        }
    }
    // The MiG's own HUD names its sensor, radar or IRST.
    if c.irst && !ru {
        pen.text(cv, "IRST", -half, 106.0, pen.small, Align::Left);
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
                cv.polyline(&left, lw, false, pen.col);
                cv.polyline(&right, lw, false, pen.col);
            }
            // LCOS pipper at the target's range, or 2,000 ft without a lock.
            let range = lock.map(|(p, _)| (p - gun).len()).filter(|&r| matches!(c.lock, Some(Who::Bandit(_))) && r < 1600.0).unwrap_or(GUN_DEFAULT);
            if let Some(p) = c.sight.at_range(gun, range).and_then(|p| pen.point(p)) {
                let r = 9.0 * big;
                cv.circle(p[0], p[1], r, lw, pen.col);
                cv.disc(p[0], p[1], 1.2 * big, pen.col);
                if lock.is_some() && range != GUN_DEFAULT {
                    // Range bar: the ring's arc shrinks as the range closes, full at 4,000 ft.
                    let f = clamp(range / 1220.0, 0.0, 1.0) as f32;
                    cv.arc(p[0], p[1], r + 2.5 * big, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + f * std::f32::consts::TAU, lw * 1.5, pen.col);
                }
                if let Some((tp, _)) = lock {
                    let on = pen.point(tp).is_some_and(|t| ((t[0] - p[0]).powi(2) + (t[1] - p[1]).powi(2)).sqrt() < r);
                    if on && range < 1100.0 && flash {
                        pen.text(cv, if ru { "ПР" } else { "SHOOT" }, 0.0, -36.0, big, Align::Center);
                    }
                }
            }
        }
        // A radar missile: the steering circle; a heat seeker shows its
        // seeker circle below, the R-27T in MRM too.
        Mode::Mrm if c.heater().is_none() => {
            // Allowable steering error circle with the steering dot on the target.
            if let Some(cen) = (pen.at)(0.0, 0.0) {
                let edge = (pen.at)(0.0, 30.0).map(|e| e[1] - cen[1]).unwrap_or(40.0);
                cv.circle(cen[0], cen[1], edge, lw, pen.col);
                if let Some((tp, _)) = lock {
                    let d = a.q.conj().rotate((tp - a.pos).norm());
                    let (x, y) = (d.x.atan2(-d.z) * 650.0, -d.y.atan2(-d.z) * 650.0);
                    let k = (30.0 / (x * x + y * y).sqrt().max(1e-6)).min(1.0);
                    if let Some(p) = (pen.at)(x * k, y * k) {
                        cv.disc(p[0], p[1], 2.2 * big, pen.col);
                    }
                }
            }
            shoot_cue(cv, pen, c, half, big, flash);
        }
        Mode::Srm | Mode::Mrm => {
            // Seeker circle: on its target when tracking (solid), on the
            // boresight while searching (dashed).
            let look = c.seeker.and_then(|w| c.track(w, a)).map(|(p, _)| (p - pen.eye).norm()).unwrap_or(ax.forward);
            if let (Some(p), Some(e)) = ((pen.dir)(look), (pen.dir)((look + ax.up * (2.5 * DEG)).norm())) {
                let r = ((p[1] - e[1]).abs()).max(6.0 * big);
                if c.seeker.is_some() {
                    cv.circle(p[0], p[1], r, lw * 1.5, pen.col);
                } else {
                    for k in 0..8 {
                        let a0 = k as f32 * std::f32::consts::TAU / 8.0;
                        cv.arc(p[0], p[1], r, a0, a0 + 0.45, lw, pen.col);
                    }
                }
            }
            shoot_cue(cv, pen, c, half, big, flash);
        }
        Mode::Ccip => {
            if let Some((p, tof)) = c.ccip {
                let fpm = (pen.dir)(a.vel.norm());
                if let Some(q) = pen.point(p) {
                    let r = 6.0 * big;
                    cv.circle(q[0], q[1], r, lw, pen.col);
                    cv.disc(q[0], q[1], 1.2 * big, pen.col);
                    // Bomb fall line from the flight-path marker down to the pipper.
                    if let Some(f) = fpm {
                        let (dx, dy) = (q[0] - f[0], q[1] - f[1]);
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        cv.line(f[0], f[1], q[0] - dx / len * r, q[1] - dy / len * r, lw, pen.col);
                    }
                }
                pen.text(cv, &format!("{} {tof:.1}", if ru { "Т" } else { "TOF" }), -half, 94.0, pen.small, Align::Left);
            }
        }
        Mode::Strf => {
            // Strafe pipper: a ring with a centre dot where the rounds would
            // hit, the slant range under it and IN RNG inside 4,000 ft.
            if let Some((p, _)) = c.ccip {
                let slant = (p - a.pos).len();
                if let Some(q) = pen.point(p) {
                    let r = 9.0 * big;
                    cv.circle(q[0], q[1], r, lw, pen.col);
                    cv.disc(q[0], q[1], 1.2 * big, pen.col);
                }
                let text = if ru { format!("Д {:.1}", slant / 1000.0) } else { format!("{:.1} RNG", slant * 3.28084 / 1000.0) };
                pen.text(cv, &text, -half, 106.0, pen.small, Align::Left);
                if slant < 1220.0 {
                    if !ru {
                        pen.text(cv, "IN RNG", -half, 94.0, pen.small, Align::Left);
                    }
                    if flash && slant > 300.0 {
                        pen.text(cv, if ru { "ПР" } else { "SHOOT" }, 0.0, -36.0, big, Align::Center);
                    }
                }
            }
        }
    }
}

/// In range: IN RNG and a flashing SHOOT, or on the ILS-31 a flashing ПР
/// (launch authorised).
fn shoot_cue(cv: &mut Canvas, pen: &Pen, c: &Combat, half: f64, big: f32, flash: bool) {
    if !c.shoot {
        return;
    }
    if !pen.russian {
        pen.text(cv, "IN RNG", -half, 94.0, pen.small, Align::Left);
    }
    if flash {
        pen.text(cv, if pen.russian { "ПР" } else { "SHOOT" }, 0.0, -36.0, big, Align::Center);
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
        c.lock.and_then(|w| c.track(w, a)).map(|(p, _)| (p - a.pos).len()).or_else(|| c.bandits.iter().enumerate().filter(|(i, _)| c.radar_contact(a, *i)).map(|(_, b)| (b.jet.pos - a.pos).len()).reduce(f64::max)).unwrap_or(0.0)
    } / 1852.0;
    [10.0, 20.0, 40.0, 80.0].into_iter().find(|&s| far < s * 0.9).unwrap_or(80.0)
}

/// FCR B-scope contacts inside the frame `[left, top, right, bottom]`:
/// azimuth across, range up. Air contacts in the air-to-air and NAV modes,
/// ground targets on the ground map in CCIP. Returns false when empty.
pub fn fcr(cv: &mut Canvas, f: [f32; 4], a: &Aircraft, c: &Combat, nm: f64) -> bool {
    let [l, t, r, b] = f;
    let place = |p: V3| -> Option<[f32; 2]> {
        let rel = p - a.pos;
        let d = a.q.conj().rotate(rel);
        let az = d.x.atan2(-d.z) * RAD;
        let range = rel.len() / 1852.0;
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
        for (i, bd) in c.bandits.iter().enumerate().filter(|(i, _)| c.radar_contact(a, *i)) {
            let Some(p) = place(bd.jet.pos) else { continue };
            any = true;
            let locked = c.lock == Some(Who::Bandit(i));
            let s = if locked { 5.0 } else { 3.5 };
            if locked {
                cv.stroke_rect(p[0] - s, p[1] - s, 2.0 * s, 2.0 * s, 2.0, AMBER);
            } else {
                cv.rect(p[0] - s, p[1] - 1.5, 2.0 * s, 3.0, GREEN);
                if c.acquiring == Some(Who::Bandit(i)) {
                    brackets(cv, p[0], p[1], 8.0, 1.0, AMBER);
                }
            }
            // Aspect stick: the contact's track relative to ours.
            let th = (bd.jet.heading - a.heading) * DEG;
            cv.line(p[0], p[1], p[0] + th.sin() as f32 * 10.0, p[1] - th.cos() as f32 * 10.0, 1.0, if locked { AMBER } else { GREEN });
            if locked {
                let alt = bd.jet.pos.y * 3.28084 / 1000.0;
                cv.text(&format!("{:.0}", alt), p[0] + 8.0, p[1] + 9.0, 1.0, AMBER, Align::Left);
                let rng = (bd.jet.pos - a.pos).len() / 1852.0;
                cv.text(&format!("{:.1}NM {}KT", rng, (bd.jet.tas_kt).round() as i64), (l + r) * 0.5, t + 10.0, 1.0, AMBER, Align::Center);
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
            mark(b.jet.pos, Who::Bandit(i), RED, None, true);
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
    cv.text(&stores_line(c), x, y, sm, with_alpha(HUD, 0.8), align);
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

/// Helmet-mounted display. The F-16C Block 50 wears JHMCS, the MiG-29S
/// (9.13S) the much simpler Shchel-3UM sight (see `shchel`). It is blanked
/// while the pilot looks through the HUD (`Combat::helmet` is None then).
#[allow(clippy::too_many_arguments)]
pub fn helmet(cv: &mut Canvas, w: f32, h: f32, s: f32, a: &Aircraft, c: &Combat, cam: &Camera, time: f64) {
    let Some(look) = c.helmet else { return };
    if a.kind == Type::Mig29 {
        shchel(cv, w, h, s, c, time);
    } else {
        jhmcs(cv, w, h, s, a, c, cam, look, time);
    }
}

/// JHMCS as on the F-16, after the HMCS chapter of the flight manual:
/// visor-fixed, the aiming cross on the helmet line of sight with the
/// airspeed and barometric altitude HUD repeaters either side and the
/// heading tape of the helmet's line of sight (not the jet's) above, the
/// whole block dropping below the cross when the pilot looks more than 30
/// degrees up; G under the airspeed, the master arm and master mode with
/// the selected store at the bottom left, the range in nautical miles to
/// the locked target at the bottom right (XXX without one), and the
/// priority RWR threat as a diamond on a circle round the cross, 12 o'clock
/// the nose, with a gap where the helmet looks. World-stabilised: the
/// aircraft boresight cross, the target designator box or the target
/// locator line with its angle (not under 10 degrees), and the AIM-9
/// missile diamond, slaved to the cross until the seeker tracks.
#[allow(clippy::too_many_arguments)]
fn jhmcs(cv: &mut Canvas, w: f32, h: f32, s: f32, a: &Aircraft, c: &Combat, cam: &Camera, look: V3, time: f64) {
    let sm = (s - 1.0).max(1.0);
    let col = with_alpha(HUD, 0.85);
    let flash = (time * 3.0).fract() < 0.6;
    let (cx, cy) = (w * 0.5, h * 0.5);
    let r = 9.0 * s;
    // Aiming cross: four arms round an open centre.
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        cv.line(cx + dx * r * 0.3, cy + dy * r * 0.3, cx + dx * r, cy + dy * r, 1.5, col);
    }

    // The data block: level with the cross, or under it looking high.
    let el = look.y.clamp(-1.0, 1.0).asin();
    let high = el > 30.0 * DEG;
    let (row, tape) = if high { (cy + h * 0.15, cy + h * 0.07) } else { (cy, cy - h * 0.17) };
    let ad = crate::air::AirData::of(a);
    let dx = h * 0.20;
    cv.text(&format!("{}", ad.kcas.round() as i64), cx - dx, row, s, col, Align::Center);
    cv.text(&format!("{}", ad.baro_ft.round() as i64), cx + dx, row, s, col, Align::Center);
    cv.text(&format!("{:.1}", a.g_smooth), cx - dx, row + 10.0 * s, sm, col, Align::Center);
    // Heading tape of the helmet's line of sight: ticks every 5 degrees,
    // figures every 10, 25 degrees either side, the reading boxed over it.
    let los = look.x.atan2(-look.z) * RAD;
    let (half, per) = (h * 0.15, h * 0.15 / 25.0);
    let base = (los / 5.0).floor() * 5.0;
    for k in -5..=6 {
        let hd = base + k as f64 * 5.0;
        let x = cx + ((hd - los) as f32) * per;
        if (x - cx).abs() > half {
            continue;
        }
        let major = (hd.rem_euclid(360.0)).round() as i64 % 10 == 0;
        cv.line(x, tape, x, tape + if major { 4.0 } else { 2.0 } * s, 1.0, col);
        if major {
            cv.text(&format!("{:02}", (hd.rem_euclid(360.0) / 10.0).round() as i64 % 36), x, tape + 9.0 * s, sm, col, Align::Center);
        }
    }
    cv.line(cx, tape - 1.0 * s, cx, tape - 4.0 * s, 1.5, col);
    let reading = format!("{:03}", (los.round() as i64).rem_euclid(360));
    let tw = Canvas::text_width(&reading, sm) + 4.0 * s;
    cv.stroke_rect(cx - tw * 0.5, tape - 13.0 * s, tw, 8.0 * s, 1.0, col);
    cv.text(&reading, cx, tape - 9.0 * s, sm, col, Align::Center);

    // Master arm and master mode, the store; range to the locked target.
    let low = cy + h * 0.24;
    cv.text(if c.mode == Mode::Nav { "OFF" } else { "ARM" }, cx - dx, low, sm, col, Align::Center);
    cv.text(c.mode.label(), cx - dx, low + 9.0 * s, sm, col, Align::Center);
    cv.text(&selected(c, false), cx - dx, low + 18.0 * s, sm, col, Align::Center);
    let lock = c.lock.and_then(|l| c.track(l, a));
    let range = lock.map(|(p, _)| format!("{:.1}", (p - a.pos).len() / 1852.0)).unwrap_or_else(|| "XXX".to_string());
    cv.text(&range, cx + dx, low, sm, col, Align::Center);

    // Priority RWR threat: a missile guiding on the jet, else whoever is
    // locked on or shooting, nearest first.
    let fwd = a.q.axes().forward;
    let bearing = |v: V3| v.x.atan2(-v.z) - fwd.x.atan2(-fwd.z);
    let threat = c.inbound().map(|m| m.pos).next().or_else(|| {
        c.emitters(a).into_iter().filter(|e| e.2).map(|e| e.0).min_by(|p, q| (*p - a.pos).len().partial_cmp(&(*q - a.pos).len()).unwrap_or(std::cmp::Ordering::Equal))
    });
    if let Some(t) = threat {
        let rr = h * 0.10;
        let gap = bearing(look) as f32;
        let n = 36;
        for k in 0..n {
            let a0 = k as f32 * std::f32::consts::TAU / n as f32;
            let mid = a0 + std::f32::consts::PI / n as f32;
            // Canvas angles run clockwise from three o'clock; 12 o'clock is the nose.
            let off = (mid - std::f32::consts::FRAC_PI_2 - gap + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            if off.abs() > 0.18 {
                cv.arc(cx, cy, rr, a0, a0 + std::f32::consts::TAU / n as f32, 1.0, col);
            }
        }
        let b = bearing(t - a.pos) as f32;
        let (px, py) = (cx + b.sin() * rr, cy - b.cos() * rr);
        let k = 4.0 * s;
        if flash || c.inbound().next().is_none() {
            cv.polyline(&[[px, py - k], [px + k, py], [px, py + k], [px - k, py]], 1.5, true, col);
        }
    }

    // Aircraft boresight cross, where the nose points, when the visor sees it.
    if let Some((x, y)) = project(cam, w, h, cam.eye + fwd * 2000.0) {
        if (x - cx).abs() < h * 0.3 && (y - cy).abs() < h * 0.3 {
            let k = 6.0 * s;
            cv.line(x - k, y, x + k, y, 1.5, col);
            cv.line(x, y - k, x, y + k * 0.5, 1.5, col);
        }
    }

    // Locked target: designator box in the visor, or a locator line toward it.
    if let Some((p, _)) = lock {
        let seen = project(cam, w, h, p).filter(|&(x, y)| (x - cx).abs() < h * 0.32 && (y - cy).abs() < h * 0.32);
        match seen {
            Some((x, y)) => {
                let b = 7.0 * s;
                cv.stroke_rect(x - b, y - b, 2.0 * b, 2.0 * b, 1.5, col);
            }
            None => {
                let d = (p - cam.eye).norm();
                let off = d.dot(cam.forward).clamp(-1.0, 1.0).acos() * RAD;
                if off >= 10.0 {
                    let (lx, ly) = (d.dot(cam.right), d.dot(cam.up));
                    let k = (lx * lx + ly * ly).sqrt().max(1e-6);
                    let (ux, uy) = ((lx / k) as f32, (-ly / k) as f32);
                    cv.line(cx + ux * r * 1.3, cy + uy * r * 1.3, cx + ux * r * 4.5, cy + uy * r * 4.5, 1.5, col);
                    cv.text(&format!("{}", off.round() as i64), cx, cy - r - 6.0 * s, sm, col, Align::Center);
                }
            }
        }
    }

    // AIM-9 missile diamond: on the target the seeker tracks, else on the
    // cross, where the helmet slaves the seeker.
    if c.heater().is_some() {
        let (x, y) = c.seeker.and_then(|w| c.track(w, a)).and_then(|(p, _)| project(cam, w, h, p)).unwrap_or((cx, cy));
        let k = 6.0 * s;
        cv.polyline(&[[x, y - k], [x + k, y], [x, y + k], [x - k, y]], 1.5, true, col);
        if c.shoot && flash {
            cv.text("SHOOT", cx, cy + r + 8.0 * sm, s, col, Align::Center);
        }
    }
}

/// Shchel-3UM, the MiG-29S's helmet sight: not a display but a collimated
/// reticle on a monocle before the right eye, which the head tracker turns
/// into a line of sight for the IRST, the radar and the missile seekers.
/// It shows the aiming ring and two lamps: target captured (ЗАХВАТ) and
/// launch permitted (ПР); no flight data.
fn shchel(cv: &mut Canvas, w: f32, h: f32, s: f32, c: &Combat, time: f64) {
    let sm = (s - 1.0).max(1.0);
    let col = rgba(0xff, 0xb0, 0x40, 0.85);
    let (cx, cy) = (w * 0.5, h * 0.5);
    let r = 7.0 * s;
    cv.circle(cx, cy, r, 1.5, col);
    cv.disc(cx, cy, 1.5, col);
    let captured = c.seeker.is_some() || c.lock.is_some();
    if captured {
        cv.text("ЗАХВАТ", cx, cy + r + 8.0 * sm, sm, col, Align::Center);
    }
    if c.shoot && (time * 3.0).fract() < 0.6 {
        cv.text("ПР", cx, cy - r - 8.0 * sm, s, col, Align::Center);
    }
}
