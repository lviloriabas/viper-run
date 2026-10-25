//! Ejection: the canopy blown off, the seat fired up the rails by its
//! catapult and rocket, the pilot separated from the seat and let down under
//! the main parachute. Each jet has its own seat and sequence: the F-16's
//! ACES II picks one of three modes from speed and height, the MiG-29's
//! K-36DM fires its two stabilising booms as it leaves the cockpit.

use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
use crate::sim::{atmosphere_at, Aircraft, Type, G0};
use crate::terrain;

/// One seat's figures.
pub struct SeatSpec {
    pub name: &'static str,
    /// Seconds from the canopy going to the seat firing.
    canopy_lead: f64,
    /// Catapult stroke speed up the rails, m/s.
    catapult: f64,
    /// Rocket: seconds and acceleration (g) along the rails.
    rocket: (f64, f64),
    /// Rails leaning back from the jet's vertical, rad.
    rails: f64,
    /// Seconds from the seat firing to man-seat separation and the main
    /// parachute: slow and low, and fast.
    separate: (f64, f64),
    /// Speed (KEAS) above which the slower, drogue-first sequence runs.
    fast: f64,
    /// Height (m) above which the pilot rides the seat on its drogue until
    /// he falls through it, for the thicker air and the oxygen below.
    hold: f64,
    /// Seconds the main canopy takes to fill.
    inflate: f64,
    /// Drag areas (m^2): seat and man, with the drogue or booms out, the
    /// filled main canopy, the seat alone.
    cda: (f64, f64, f64, f64),
    /// Seat and pilot masses, kg.
    mass: (f64, f64),
    /// The parachute's gores, in turn round the canopy.
    gores: &'static [Rgb],
}

const ORANGE: Rgb = Rgb(0.92, 0.38, 0.10);
const WHITE: Rgb = Rgb(0.90, 0.90, 0.86);
const OLIVE: Rgb = Rgb(0.36, 0.40, 0.26);
const RED: Rgb = Rgb(0.80, 0.18, 0.12);

/// ACES II: canopy thrusters, then the seat 0.3 s later; mode 1 (below
/// 250 KEAS and 15,000 ft) opens the main chute at once, mode 2 steadies the
/// seat on its drogue first, mode 3 (above 15,000 ft) rides the drogue down
/// to it. C-9 canopy, 28 ft, in orange, white and olive gores.
pub const ACES2: SeatSpec = SeatSpec {
    name: "ACES II",
    canopy_lead: 0.30,
    catapult: 12.0,
    rocket: (0.25, 12.0),
    rails: 10.0 * DEG,
    separate: (0.8, 1.4),
    fast: 250.0,
    hold: 4570.0,
    inflate: 1.6,
    cda: (0.75, 1.9, 38.0, 0.45),
    mass: (75.0, 100.0),
    gores: &[ORANGE, WHITE, OLIVE, WHITE],
};

/// K-36DM: the canopy jettisoned, the seat 0.2 s later with its two
/// telescopic booms and their small chutes steadying it at any speed;
/// separation and the main canopy after about 1.6 s, held above 5,000 m.
/// Main canopy of about 60 m^2 in white and orange-red.
pub const K36DM: SeatSpec = SeatSpec {
    name: "K-36DM",
    canopy_lead: 0.20,
    catapult: 13.0,
    rocket: (0.35, 10.0),
    rails: 14.0 * DEG,
    separate: (1.2, 1.8),
    fast: 300.0,
    hold: 5000.0,
    inflate: 2.0,
    cda: (0.75, 1.5, 40.0, 0.5),
    mass: (103.0, 100.0),
    gores: &[WHITE, RED],
};

pub fn spec(kind: Type) -> &'static SeatSpec {
    match kind {
        Type::Mig29 => &K36DM,
        _ => &ACES2,
    }
}

/// A free body: position, velocity, attitude and spin (rad/s, world axes).
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub pos: V3,
    pub vel: V3,
    pub q: Quat,
    pub spin: V3,
    pub down: bool,
}

impl Body {
    fn fly(&mut self, dt: f64, mass: f64, cda: f64, rest: f64) {
        if self.down {
            return;
        }
        let rho = atmosphere_at(self.pos.y).rho;
        let speed = self.vel.len();
        self.vel = self.vel + (V3::Y * -G0 - self.vel * (0.5 * rho * speed * cda / mass)) * dt;
        self.pos = self.pos + self.vel * dt;
        if self.spin.len() > 1e-6 {
            self.q = Quat::axis_angle(self.spin.norm(), self.spin.len() * dt).mul(self.q).norm();
        }
        let floor = terrain::floor(self.pos.x, self.pos.z) + rest;
        if self.pos.y <= floor {
            self.pos.y = floor;
            self.vel = V3::ZERO;
            self.spin = V3::ZERO;
            self.down = true;
        }
    }
}

/// Phase of the sequence, for the toasts and the camera.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Canopy gone, the seat still in the cockpit.
    Canopy,
    /// The seat away, the pilot strapped in it.
    Seat,
    /// Separated: the pilot under the main canopy.
    Chute,
    /// On the ground.
    Landed,
}

pub struct Ejection {
    pub kind: Type,
    pub spec: &'static SeatSpec,
    pub age: f64,
    /// The pilot (in the seat until separation): his harness point.
    pub pilot: Body,
    /// The canopy, its body origin moving with the jet's frame.
    pub canopy: Body,
    /// The empty seat after separation.
    pub seat: Option<Body>,
    /// Main canopy filled, 0 to 1.
    pub chute: f64,
    pub phase: Phase,
    /// Seconds since the boots touched.
    pub landed_for: f64,
    /// When the seat leaves, set on firing.
    fired: Option<f64>,
    /// Separation time after firing, picked from the speed at ejection.
    separate_at: f64,
    /// The rails' push axis, fixed at the moment the seat fires.
    rail: V3,
    /// Sway phase under the canopy.
    sway: f64,
}

/// Seat pan above the jet's body origin, from where the pilot sits.
fn seat_point(a: &Aircraft) -> V3 {
    let eye = if a.kind == Type::Mig29 { crate::mig29::EYE } else { crate::airframe::EYE };
    eye - V3::Y * 0.80
}

/// Harness point above the seat pan.
const HARNESS: f64 = 0.55;

impl Ejection {
    pub fn new(a: &Aircraft) -> Ejection {
        let spec = spec(a.kind);
        let ax = a.q.axes();
        let keas = a.airspeed_kt;
        let separate_at = if keas > spec.fast { spec.separate.1 } else { spec.separate.0 };
        let pilot = Body { pos: a.pos + a.q.rotate(seat_point(a) + V3::Y * HARNESS), vel: a.vel, q: a.q, spin: V3::ZERO, down: false };
        // Thrusters throw the canopy up and back; the air takes it from there.
        let canopy = Body { pos: a.pos, vel: a.vel + ax.up * 9.0 + ax.back * 2.0, q: a.q, spin: ax.right * -1.6, down: false };
        Ejection { kind: a.kind, spec, age: 0.0, pilot, canopy, seat: None, chute: 0.0, phase: Phase::Canopy, landed_for: 0.0, fired: None, separate_at, rail: ax.up, sway: 0.0 }
    }

    /// Seconds the seat has been away from the jet, or None before it fires.
    pub fn away(&self) -> Option<f64> {
        self.fired.map(|t| self.age - t)
    }

    /// The rocket burning, for its flame.
    pub fn rocket(&self) -> bool {
        self.away().is_some_and(|t| t < self.spec.rocket.0)
    }

    /// One step. `a` is the jet the seat rides out of until it fires.
    /// Returns a toast when the sequence moves on.
    pub fn step(&mut self, dt: f64, a: &Aircraft) -> Option<&'static str> {
        let s = self.spec;
        self.age += dt;
        // The canopy flies off as a plate, tumbling, and lands where it lands.
        self.canopy.fly(dt, 60.0, 1.2, 0.3);
        let mut toast = None;
        match self.phase {
            Phase::Canopy => {
                // Still in the cockpit, riding the jet.
                self.pilot.pos = a.pos + a.q.rotate(seat_point(a) + V3::Y * HARNESS);
                self.pilot.vel = a.vel;
                self.pilot.q = a.q;
                if self.age >= s.canopy_lead {
                    let ax = a.q.axes();
                    self.rail = (ax.up * s.rails.cos() + ax.back * s.rails.sin()).norm();
                    self.pilot.vel = a.vel + self.rail * s.catapult;
                    // The seat pitches back off the rails into the wind.
                    self.pilot.spin = ax.right * -0.9;
                    self.fired = Some(self.age);
                    self.phase = Phase::Seat;
                    toast = Some(if self.kind == Type::Mig29 { "K-36DM // SEAT AWAY, BOOMS OUT" } else { "ACES II // SEAT AWAY" });
                }
            }
            Phase::Seat => {
                let t = self.away().unwrap_or(0.0);
                if t < s.rocket.0 {
                    self.pilot.vel = self.pilot.vel + self.rail * (s.rocket.1 * G0 * dt);
                }
                let (ms, mp) = s.mass;
                self.pilot.fly(dt, ms + mp, s.cda.1, HARNESS);
                // Above the hold height the pilot rides the seat down on its
                // drogue (or booms) before the main canopy can open.
                let high = self.pilot.pos.y > s.hold;
                if (t >= self.separate_at && !high) || self.pilot.down {
                    let mut seat = self.pilot;
                    seat.vel = seat.vel + self.pilot.q.axes().forward * 3.0 - V3::Y * 2.0;
                    seat.spin = v3(1.3, 0.4, 0.9);
                    self.seat = Some(seat);
                    self.pilot.spin = V3::ZERO;
                    self.phase = Phase::Chute;
                    toast = Some("SEAT SEPARATION // MAIN CHUTE");
                }
            }
            Phase::Chute | Phase::Landed => {
                if let Some(seat) = &mut self.seat {
                    seat.fly(dt, s.mass.0, s.cda.3, 0.4);
                }
                if self.phase == Phase::Landed {
                    self.landed_for += dt;
                    self.chute = (self.chute - dt / 2.5).max(0.0);
                    return None;
                }
                self.chute = (self.chute + dt / s.inflate).min(1.0);
                let fill = self.chute * self.chute;
                self.pilot.fly(dt, s.mass.1, s.cda.0 + fill * s.cda.2, 0.95);
                // The pilot hangs upright under the risers, facing where he
                // drifts, swinging a little as the canopy takes him.
                self.sway += dt;
                let course = v3(self.pilot.vel.x, 0.0, self.pilot.vel.z);
                let face = if course.len() > 0.5 { course.norm() } else { self.pilot.q.axes().forward.with_y(0.0).norm_or(V3::Z * -1.0) };
                let upright = Quat::from_forward_up(face, V3::Y);
                let swing = (self.sway * 1.7).sin() * 9.0 * DEG * (1.0 - 0.6 * fill.min(1.0)) * (-self.sway * 0.25).exp().max(0.3);
                self.pilot.q = Quat::axis_angle(upright.axes().right, swing).mul(upright);
                if self.pilot.down {
                    self.phase = Phase::Landed;
                    toast = Some("PILOT ON THE GROUND");
                }
            }
        }
        toast
    }
}

trait Flat {
    fn with_y(self, y: f64) -> V3;
    fn norm_or(self, fallback: V3) -> V3;
}

impl Flat for V3 {
    fn with_y(self, y: f64) -> V3 {
        v3(self.x, y, self.z)
    }
    fn norm_or(self, fallback: V3) -> V3 {
        if self.len() > 1e-6 { self.norm() } else { fallback }
    }
}

// ---------------------------------------------------------------- meshes
// Local axes as the jet's: x right, y up, z aft; origin at the harness.

/// The seat: bucket, back and headbox, the ejection handle and, on the
/// K-36DM, the two booms (out once it has fired).
pub fn seat_mesh(kind: Type, booms: bool) -> Mesh {
    let mut m = Mesh::new();
    let f = |o: V3| Frame { o, x: V3::X, y: V3::Y, z: V3::Z };
    let (shell, cushion) = if kind == Type::Mig29 { (Rgb(0.30, 0.36, 0.36), Rgb(0.34, 0.24, 0.16)) } else { (Rgb(0.16, 0.17, 0.17), Rgb(0.20, 0.24, 0.18)) };
    let pan = -HARNESS;
    m.block(&f(v3(0.0, pan - 0.08, -0.12)), v3(0.25, 0.08, 0.26), shell);
    m.block(&f(v3(0.0, pan + 0.01, -0.12)), v3(0.21, 0.03, 0.22), cushion);
    m.block(&f(v3(0.0, pan + 0.45, 0.17)), v3(0.25, 0.45, 0.06), shell);
    m.block(&f(v3(0.0, pan + 0.45, 0.10)), v3(0.20, 0.40, 0.02), cushion);
    m.block(&f(v3(0.0, pan + 1.02, 0.15)), v3(0.20, 0.14, 0.09), shell.scale(1.2));
    for s in [-1.0, 1.0] {
        m.block(&f(v3(s * 0.27, pan + 0.30, 0.0)), v3(0.025, 0.30, 0.20), shell.scale(0.85));
    }
    // Rocket nozzle under the pan.
    m.lathe(v3(0.0, pan - 0.16, 0.05), V3::Y * -1.0, &[(0.0, 0.0), (0.0, 0.05), (0.06, 0.07), (0.07, 0.0)], 6, Rgb(0.12, 0.12, 0.12));
    // Yellow and black handle between the knees.
    for k in 0..4 {
        let col = if k % 2 == 0 { Rgb(0.95, 0.80, 0.10) } else { Rgb(0.05, 0.05, 0.05) };
        m.block(&f(v3(-0.045 + k as f64 * 0.03, pan + 0.05, -0.36)), v3(0.015, 0.03, 0.015), col);
    }
    if kind == Type::Mig29 && booms {
        for s in [-1.0, 1.0] {
            let root = v3(s * 0.14, pan + 1.12, 0.22);
            let tip = root + v3(s * 0.20, 0.45, 1.10);
            m.capsule(root, tip, 0.02, 0.015, Rgb(0.55, 0.56, 0.55), 5);
            m.lathe(tip, (tip - root).norm(), &[(0.0, 0.0), (0.0, 0.10), (0.18, 0.13), (0.20, 0.0)], 6, WHITE);
        }
    }
    m
}

/// The pilot: seated with the knees up, or hanging with the legs down.
pub fn pilot_mesh(kind: Type, hanging: bool) -> Mesh {
    let mut m = Mesh::new();
    let (suit, helmet) = if kind == Type::Mig29 { (Rgb(0.40, 0.42, 0.36), Rgb(0.88, 0.88, 0.84)) } else { (Rgb(0.30, 0.34, 0.22), Rgb(0.78, 0.80, 0.78)) };
    let gsuit = suit.scale(0.8);
    let hip = v3(0.0, -HARNESS + 0.10, 0.0);
    m.capsule(hip, v3(0.0, 0.30, 0.04), 0.17, 0.19, suit, 8);
    m.capsule(v3(0.0, 0.42, 0.04), v3(0.0, 0.58, 0.02), 0.13, 0.13, helmet, 8);
    m.block(&Frame { o: v3(0.0, 0.48, -0.08), x: V3::X, y: V3::Y, z: V3::Z }, v3(0.09, 0.04, 0.03), Rgb(0.12, 0.12, 0.10));
    for s in [-1.0, 1.0] {
        let hp = hip + V3::X * (s * 0.10);
        let (knee, foot) = if hanging { (hp + v3(s * 0.03, -0.45, -0.06), hp + v3(s * 0.04, -0.92, 0.0)) } else { (hp + v3(0.0, 0.10, -0.45), hp + v3(0.0, -0.32, -0.55)) };
        m.capsule(hp, knee, 0.085, 0.07, gsuit, 6);
        m.capsule(knee, foot, 0.065, 0.055, gsuit, 6);
        m.capsule(foot, foot + v3(0.0, -0.03, -0.12), 0.05, 0.05, Rgb(0.08, 0.07, 0.06), 5);
        let shoulder = v3(s * 0.20, 0.24, 0.04);
        let hand = if hanging { v3(s * 0.22, 0.62, 0.0) } else { v3(s * 0.08, -0.40, -0.34) };
        m.capsule(shoulder, shoulder.lerp(hand, 0.5) + V3::X * (s * 0.06), 0.06, 0.055, suit, 6);
        m.capsule(shoulder.lerp(hand, 0.5) + V3::X * (s * 0.06), hand, 0.055, 0.045, suit, 6);
    }
    m
}

/// Main canopy filled, with its lines down to the risers at the harness:
/// gores from the vent to the skirt, the seat's own colours.
pub fn chute_mesh(spec: &SeatSpec) -> Mesh {
    let mut m = Mesh::new();
    let n = 16;
    let (radius, skirt, crown) = (3.4, 6.6, 8.6);
    let ring = |k: usize, t: f64| {
        let a = k as f64 * TAU / n as f64;
        let th = t * 80.0 * DEG;
        let r = radius * th.sin() / (80.0 * DEG).sin();
        v3(a.cos() * r, mix(crown, skirt, (1.0 - th.cos()) / (1.0 - (80.0 * DEG).cos())), a.sin() * r)
    };
    let rows = 4;
    for k in 0..n {
        let col = spec.gores[k % spec.gores.len()];
        for j in 0..rows {
            let (t0, t1) = (0.12 + j as f64 / rows as f64 * 0.88, 0.12 + (j + 1) as f64 / rows as f64 * 0.88);
            let (a, b, c, d) = (ring(k, t0), ring(k + 1, t0), ring(k + 1, t1), ring(k, t1));
            m.quad(a, b, c, d, col);
            m.quad(b, a, d, c, col.scale(0.92));
        }
        let skirt_pt = ring(k, 1.0);
        let riser = v3(if skirt_pt.x > 0.0 { 0.16 } else { -0.16 }, 0.45, 0.0);
        m.capsule(skirt_pt, riser, 0.012, 0.012, Rgb(0.82, 0.80, 0.74), 3);
    }
    m
}
