//! Condensation and shock waves. Wherever the flow speeds up, its pressure
//! and temperature drop along an adiabat; once the cooled air holds more
//! water vapour than saturation allows, it fogs. The F-16 shows it in three
//! places: the LERX vortex streaming over each wing at high AoA, the core of
//! the wingtip vortex in a hard pull, and the supersonic pocket over the jet
//! near Mach 1, whose fog ends at the terminating shock (the vapor cone).
//! High and cold, the exhaust leaves a contrail (Schmidt-Appleman: below
//! about -40 C). Past Mach 1 the bow and tail shocks trail back at the Mach
//! angle; they are drawn as faint shimmering cones.
//!
//! Each region's pressure comes from a pressure coefficient on the free
//! stream: the LERX vortex core about -12 sin^2(alpha) (Polhamus-style vortex
//! suction), the tip vortex a Rankine core of radius 0.6 m carrying the
//! circulation L / (rho V b') of an elliptic wing, and the transonic pocket
//! about -0.45. Saturation follows the Magnus formula in a moist boundary
//! layer (60 to 70 % relative humidity under the cumulus) and drier air
//! above; how thick the fog looks follows the water that condenses, so the
//! thin, cold air high up hardly shows it.
use crate::aero;
use crate::airframe;
use crate::math::*;
use crate::sim::{atmosphere_at, Aircraft};
use crate::world::sun_dir;
use std::collections::VecDeque;

/// Relative humidity against altitude: a moist boundary layer under the
/// cumulus base near 1,500 m, drier air above it.
const HUMIDITY: [(f64, f64); 6] = [(0.0, 0.6), (1000.0, 0.68), (2500.0, 0.7), (4000.0, 0.45), (7000.0, 0.3), (12000.0, 0.25)];
/// Effective span of the trailing vortices, pi/4 of the 9.14 m span, and
/// the tip vortex core radius (m).
const VORTEX_SPAN: f64 = 0.785 * 9.144;
const TIP_CORE: f64 = 0.6;
/// Body positions (x right, y up, z aft) where the trails start: the wingtip
/// rail's trailing edge and a little behind the nozzle.
const TIP: V3 = v3(4.75, 0.33, 3.0);
const CONTRAIL_GAP: f64 = 14.0;
/// Seconds a trail point lasts: tip vapour re-evaporates as the core
/// pressure recovers; a contrail lingers.
const TIP_LIFE: f64 = 1.1;
const CONTRAIL_LIFE: f64 = 22.0;
const MAX_POINTS: usize = 360;

/// Saturation vapour pressure over water, hPa, at `t` degrees C (Magnus).
fn magnus(t: f64) -> f64 {
    6.112 * (17.62 * t / (243.12 + t)).exp()
}

/// Vapour pressure of the day's air at an altitude, hPa.
fn vapour(altitude: f64, temp: f64) -> f64 {
    aero::curve(&HUMIDITY, altitude) * magnus(temp - 273.15)
}

/// Fog density where the local pressure coefficient is `cp`: the water that
/// condenses (g per kg of air) as the flow expands, 0 dry to 1 dense.
fn fog(altitude: f64, qbar: f64, cp: f64) -> f64 {
    let atm = atmosphere_at(altitude);
    let ratio = clamp(1.0 + cp * qbar / atm.pressure, 0.2, 1.0);
    let local = atm.pressure * ratio / 100.0;
    let excess = vapour(altitude, atm.temp) * ratio - magnus(atm.temp * ratio.powf(0.2857) - 273.15);
    smoothstep(0.02, 0.35, 622.0 * excess / local)
}

/// How much condenses where, for one aircraft state, 0..1 each.
#[derive(Clone, Copy, Debug, Default)]
pub struct Condensation {
    pub lerx: f64,
    pub tips: f64,
    pub cone: f64,
    pub contrail: f64,
}

pub fn condensation(a: &Aircraft) -> Condensation {
    let alpha = a.alpha.max(0.0);
    let lerx = fog(a.pos.y, a.qbar, -12.0 * alpha.sin().powi(2)) * smoothstep(6.0, 11.0, a.alpha_deg);
    let rho = atmosphere_at(a.pos.y).rho;
    let gamma = a.lift_n.abs() / (rho * a.speed.max(30.0) * VORTEX_SPAN);
    let swirl = gamma / (TAU * TIP_CORE) / a.speed.max(30.0);
    let tips = fog(a.pos.y, a.qbar, -2.0 * swirl * swirl);
    let pocket = smoothstep(0.86, 0.96, a.mach) * (1.0 - smoothstep(1.0, 1.04, a.mach));
    let cone = fog(a.pos.y, a.qbar, -0.45 * pocket) * pocket;
    let celsius = atmosphere_at(a.pos.y).temp - 273.15;
    let contrail = smoothstep(-38.0, -46.0, celsius) * smoothstep(5.0, 20.0, a.power);
    Condensation { lerx, tips, cone, contrail }
}

#[derive(Clone, Copy)]
struct Point {
    pos: V3,
    age: f64,
    alpha: f64,
}

#[derive(Default)]
struct Trail {
    pts: VecDeque<Point>,
}

impl Trail {
    /// Adds a point once the source has moved a couple of metres; a gap in
    /// the fog leaves a zero-alpha point that breaks the ribbon.
    fn feed(&mut self, pos: V3, alpha: f64) {
        let last = self.pts.back().copied();
        if alpha < 0.02 && last.is_none_or(|p| p.alpha < 0.02) {
            return;
        }
        if last.is_some_and(|p| (p.pos - pos).len() < 2.0 && p.age < 0.05) {
            return;
        }
        self.pts.push_back(Point { pos, age: 0.0, alpha });
        if self.pts.len() > MAX_POINTS {
            self.pts.pop_front();
        }
    }

    fn age(&mut self, dt: f64, life: f64) {
        for p in self.pts.iter_mut() {
            p.age += dt;
        }
        while self.pts.front().is_some_and(|p| p.age > life) {
            self.pts.pop_front();
        }
    }
}

/// Vapour trails left in the air, and the fog attached to the jet.
#[derive(Default)]
pub struct Vapor {
    tips: [Trail; 2],
    contrail: Trail,
    /// Condensation of the last update, for the attached fog.
    now: Condensation,
}

impl Vapor {
    pub fn clear(&mut self) {
        *self = Vapor::default();
    }

    /// Ages the trails and lets the jet feed them.
    pub fn update(&mut self, dt: f64, aircraft: Option<&Aircraft>) {
        if dt <= 0.0 {
            return;
        }
        for t in self.tips.iter_mut() {
            t.age(dt, TIP_LIFE);
        }
        self.contrail.age(dt, CONTRAIL_LIFE);
        let Some(a) = aircraft.filter(|a| !a.crashed) else {
            self.now = Condensation::default();
            return;
        };
        let c = condensation(a);
        self.now = c;
        let ax = a.q.axes();
        let body = |p: V3| a.pos + ax.right * p.x + ax.up * p.y + ax.back * p.z;
        for (i, side) in [-1.0, 1.0].iter().enumerate() {
            self.tips[i].feed(body(v3(side * TIP.x, TIP.y + a.structure.bend, TIP.z)), c.tips);
        }
        let nozzle = airframe::NOZZLE_EXIT;
        self.contrail.feed(body(v3(0.0, nozzle.y, nozzle.z + CONTRAIL_GAP)), c.contrail);
    }

    /// Eye-relative alpha-blended facets: the trails, then the fog and the
    /// shocks riding on the jet.
    pub fn emit(&self, data: &mut Vec<f32>, aircraft: Option<&Aircraft>, eye: V3, time: f64) {
        let sun = sun_dir();
        ribbon(data, &self.contrail, eye, CONTRAIL_LIFE, (1.2, 9.0), 0.55);
        for t in &self.tips {
            ribbon(data, t, eye, TIP_LIFE, (0.18, 0.7), 0.7);
        }
        let Some(a) = aircraft.filter(|a| !a.crashed) else { return };
        let ax = a.q.axes();
        let body = |p: V3| a.pos - eye + ax.right * p.x + ax.up * p.y + ax.back * p.z;
        let c = self.now;
        if c.lerx > 0.02 {
            lerx_vortex(data, a, &body, c.lerx, sun, time);
        }
        if c.cone > 0.02 {
            vapor_cone(data, a, &body, c.cone, sun, time);
        }
        if a.mach > 1.0 {
            shocks(data, a, eye, time);
        }
    }
}

fn vert(data: &mut Vec<f32>, p: V3, c: [f64; 3], a: f64) {
    data.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32, c[0] as f32, c[1] as f32, c[2] as f32, a.clamp(0.0, 1.0) as f32]);
}

/// Sun-lit white of a fog facet with normal `n`.
fn shade(n: V3, sun: V3) -> [f64; 3] {
    let k = 0.72 + 0.22 * n.dot(sun).abs() + 0.06 * n.y;
    [0.93 * k, 0.95 * k, 0.98 * k]
}

/// A camera-facing strip through a trail, widening and fading with age.
fn ribbon(data: &mut Vec<f32>, t: &Trail, eye: V3, life: f64, width: (f64, f64), alpha: f64) {
    let pts: Vec<&Point> = t.pts.iter().collect();
    for w in pts.windows(2) {
        let (p0, p1) = (w[0], w[1]);
        if p0.alpha < 0.02 || p1.alpha < 0.02 {
            continue;
        }
        let (c0, c1) = (p0.pos - eye, p1.pos - eye);
        let along = c1 - c0;
        let side = along.cross(c0).norm();
        let fade = |p: &Point| p.alpha * alpha * (1.0 - smoothstep(0.3 * life, life, p.age)) * smoothstep(0.0, 0.08, p.age);
        let half = |p: &Point| mix(width.0, width.1, smoothstep(0.0, life, p.age).sqrt());
        let (s0, s1) = (side * half(p0), side * half(p1));
        let col = [0.9, 0.92, 0.95];
        let (a0, a1) = (fade(p0), fade(p1));
        for (p, a) in [(c0 - s0, a0), (c0 + s0, a0), (c1 + s1, a1), (c0 - s0, a0), (c1 + s1, a1), (c1 - s1, a1)] {
            vert(data, p, col, a);
        }
    }
}

/// Faceted tube through rings of (centre, radius, alpha) with `sides` sides,
/// alternate panels a shade darker so the fog reads as low-poly facets.
fn tube(data: &mut Vec<f32>, rings: &[(V3, f64, f64)], sides: usize, axis_up: V3, sun: V3, twist: f64) {
    let step = TAU / sides as f64;
    for w in rings.windows(2) {
        let (ca, ra, aa) = w[0];
        let (cb, rb, ab) = w[1];
        let dir = (cb - ca).norm();
        let u = (axis_up - dir * axis_up.dot(dir)).norm();
        let v = dir.cross(u);
        let at = |c: V3, r: f64, k: f64| {
            let t = k * step + twist;
            c + u * (t.cos() * r) + v * (t.sin() * r)
        };
        for k in 0..sides {
            let (k0, k1) = (k as f64, (k + 1) as f64);
            let tm = (k0 + 0.5) * step + twist;
            let n = u * tm.cos() + v * tm.sin();
            let col = shade(n, sun);
            let col = if k % 2 == 0 { col } else { [col[0] * 0.93, col[1] * 0.93, col[2] * 0.93] };
            let (a0, a1, b0, b1) = (at(ca, ra, k0), at(ca, ra, k1), at(cb, rb, k0), at(cb, rb, k1));
            for (p, a) in [(a0, aa), (a1, aa), (b1, ab), (a0, aa), (b1, ab), (b0, ab)] {
                vert(data, p, col, a);
            }
        }
    }
}

/// The LERX vortices: a fog tube from each strake root over the wing, tight
/// while the vortex holds, swelling into a diffuse cloud past its burst
/// point, which moves forward over the wing from about 25 degrees of AoA.
fn lerx_vortex(data: &mut Vec<f32>, a: &Aircraft, body: &dyn Fn(V3) -> V3, k: f64, sun: V3, time: f64) {
    let burst = 1.0 - 0.75 * smoothstep(24.0, 34.0, a.alpha_deg);
    let reach = mix(0.55, 1.0, smoothstep(8.0, 18.0, a.alpha_deg));
    let up = a.q.axes().up;
    for side in [-1.0, 1.0] {
        let rings: Vec<(V3, f64, f64)> = (0..=9)
            .map(|i| {
                let s = i as f64 / 9.0 * reach;
                let p = v3(side * mix(0.62, 2.1, s.powf(0.8)), mix(0.5, 1.05, s), mix(-4.15, 3.6, s));
                let past = smoothstep(burst - 0.05, burst + 0.2, s);
                let wobble = 1.0 + 0.08 * (time * 23.0 + s * 9.0 + side).sin();
                let r = mix(0.07, 0.5, s) * (1.0 + 2.2 * past) * wobble;
                let alpha = k * 0.9 * smoothstep(0.0, 0.12, s) * (1.0 - 0.6 * past) * (1.0 - smoothstep(0.75 * reach, reach, s));
                (body(p), r, alpha)
            })
            .collect();
        tube(data, &rings, 6, up, sun, time * 2.0 * side);
        // The sheet of fog winding into the core from the strake edge.
        let sheath: Vec<(V3, f64, f64)> = rings.iter().map(|&(c, r, al)| (c, r * 1.9, al * 0.3)).collect();
        tube(data, &sheath, 6, up, sun, -time * 1.3 * side);
    }
}

/// The vapor cone: fog filling the supersonic pocket around the jet aft of
/// the canopy and ending sharply at the terminating shock, which slides aft
/// to the trailing edge as the jet reaches Mach 1.
fn vapor_cone(data: &mut Vec<f32>, a: &Aircraft, body: &dyn Fn(V3) -> V3, k: f64, sun: V3, time: f64) {
    let shock = mix(0.2, 3.6, smoothstep(0.9, 1.01, a.mach));
    let front = -3.4;
    let ax = a.q.axes();
    let rings: Vec<(V3, f64, f64)> = (0..=6)
        .map(|i| {
            let s = i as f64 / 6.0;
            let z = mix(front, shock, s);
            let r = mix(0.9, 3.4, s.powf(0.7)) * (1.0 + 0.03 * (time * 31.0 + s * 5.0).sin());
            let alpha = k * 0.42 * smoothstep(0.0, 0.7, s);
            (body(v3(0.0, 0.55, z)), r, alpha)
        })
        .collect();
    tube(data, &rings, 10, ax.up, sun, 0.4);
    // The shock face: a disc closing the cone, densest at its rim.
    let (c, r, alpha) = rings[rings.len() - 1];
    let col = shade(ax.back, sun);
    for k in 0..10 {
        let t0 = k as f64 * TAU / 10.0 + 0.4;
        let t1 = (k + 1) as f64 * TAU / 10.0 + 0.4;
        let p = |t: f64| c + ax.right * (t.cos() * r) + ax.up * (t.sin() * r);
        for (q, al) in [(c, alpha * 0.15), (p(t0), alpha * 0.6), (p(t1), alpha * 0.6)] {
            vert(data, q, col, al);
        }
    }
}

/// Bow and tail shocks past Mach 1: faint cones at the Mach angle around
/// the flight path, fading out a few dozen metres back.
fn shocks(data: &mut Vec<f32>, a: &Aircraft, eye: V3, time: f64) {
    let mu = (1.0 / a.mach).asin();
    let dir = a.vel.norm();
    let ax = a.q.axes();
    let u = (ax.up - dir * ax.up.dot(dir)).norm();
    let v = dir.cross(u);
    let strength = smoothstep(1.0, 1.15, a.mach) * 0.13;
    for (apex, length) in [(-7.1, 28.0), (6.7, 22.0)] {
        let tip = a.pos - eye + ax.back * apex;
        let rings = [(0.0, 0.0), (0.35, strength), (1.0, 0.0)];
        for w in rings.windows(2) {
            let (s0, al0) = w[0];
            let (s1, al1) = w[1];
            for k in 0..12 {
                let t0 = k as f64 * TAU / 12.0;
                let t1 = (k + 1) as f64 * TAU / 12.0;
                let flicker = 0.7 + 0.3 * (time * 37.0 + k as f64 * 1.7).sin();
                let ring = |s: f64, t: f64| {
                    let d = length * s;
                    tip - dir * d + (u * t.cos() + v * t.sin()) * (d * mu.tan())
                };
                let col = [0.86, 0.9, 0.95];
                let (a0, a1) = (al0 * flicker, al1 * flicker);
                for (p, al) in [(ring(s0, t0), a0), (ring(s0, t1), a0), (ring(s1, t1), a1), (ring(s0, t0), a0), (ring(s1, t1), a1), (ring(s1, t0), a1)] {
                    vert(data, p, col, al);
                }
            }
        }
    }
}

/// Whether a listener at `at` is inside the Mach cone trailing the jet,
/// where its boom has already arrived.
pub fn inside_mach_cone(a: &Aircraft, at: V3) -> bool {
    if a.mach <= 1.0 || a.crashed {
        return false;
    }
    let dir = a.vel.norm();
    let rel = at - a.pos;
    let behind = -rel.dot(dir);
    let off = (rel + dir * behind).len();
    behind > 0.0 && off.atan2(behind) < (1.0 / a.mach).asin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{trim, Input, Mission};

    fn jet(alt: f64, speed: f64, alpha_deg: f64, g: f64) -> Aircraft {
        let mut a = Aircraft::new(&Mission::new());
        let t = trim(alt, speed, a.mass);
        a.pos = v3(-2000.0, alt, -3000.0);
        a.vel = v3(0.0, 0.0, -speed);
        a.q = Quat::from_forward_up(v3(0.0, 0.0, -1.0), V3::Y).mul(Quat::axis_angle(V3::X, alpha_deg * DEG));
        a.flcs.alpha_f = alpha_deg;
        a.flcs.el = t.el;
        a.update(0.0, &Input::default());
        a.lift_n = g * a.mass * 9.80665;
        a
    }

    #[test]
    fn humid_low_air_fogs_in_a_hard_pull_and_not_cruising() {
        let cruise = condensation(&jet(1500.0, 220.0, 2.0, 1.0));
        assert!(cruise.lerx < 0.01 && cruise.tips < 0.01 && cruise.cone < 0.01, "{cruise:?}");
        let pull = condensation(&jet(1500.0, 220.0, 16.0, 8.0));
        assert!(pull.lerx > 0.5 && pull.tips > 0.2, "{pull:?}");
        // The same pull high in the dry air leaves the wing clean.
        let high = condensation(&jet(9000.0, 220.0, 16.0, 8.0));
        assert!(high.lerx < pull.lerx * 0.5, "{high:?}");
    }

    #[test]
    fn vapor_cone_only_near_mach_one_and_contrails_only_high() {
        let sound = atmosphere_at(1000.0).sound;
        assert!(condensation(&jet(1000.0, 0.97 * sound, 1.0, 1.0)).cone > 0.3);
        assert!(condensation(&jet(1000.0, 0.8 * sound, 1.0, 1.0)).cone < 0.01);
        assert!(condensation(&jet(1000.0, 1.2 * sound, 1.0, 1.0)).cone < 0.01);
        assert!(condensation(&jet(1000.0, 220.0, 2.0, 1.0)).contrail < 0.01);
        assert!(condensation(&jet(10000.0, 250.0, 3.0, 1.0)).contrail > 0.9);
    }

    #[test]
    fn the_boom_arrives_behind_the_jet_inside_the_mach_cone() {
        let sound = atmosphere_at(3000.0).sound;
        let a = jet(3000.0, 1.4 * sound, 2.0, 1.0);
        let fwd = a.vel.norm();
        assert!(inside_mach_cone(&a, a.pos - fwd * 50.0 + V3::Y * 10.0));
        assert!(!inside_mach_cone(&a, a.pos + fwd * 50.0));
        assert!(!inside_mach_cone(&a, a.pos - fwd * 20.0 + V3::Y * 40.0));
        let slow = jet(3000.0, 0.9 * sound, 2.0, 1.0);
        assert!(!inside_mach_cone(&slow, slow.pos - fwd * 50.0));
    }
}
