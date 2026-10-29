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

/// K-36DM: canopy clearance delay of 0.5 s, then the seat with its two
/// telescopic booms and their small chutes steadying it at any speed;
/// separation and the main canopy after about 1.6 s, held above 5,000 m.
/// Main canopy of about 60 m^2 in white and orange-red.
pub const K36DM: SeatSpec = SeatSpec {
    name: "K-36DM",
    canopy_lead: 0.50,
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

/// SJU-17(V)/A NACES reconstruction. The timing/drag here are gameplay
/// approximations; the Hornet does not reuse the ACES II identity.
pub const NACES:SeatSpec=SeatSpec {
    name:"SJU-17 NACES",canopy_lead:0.35,catapult:12.5,rocket:(0.30,12.0),
    rails:17.0*DEG,separate:(0.85,1.5),fast:250.0,hold:4570.0,inflate:1.7,
    cda:(0.8,2.0,38.0,0.48),mass:(92.0,100.0),gores:&[WHITE,OLIVE,WHITE,OLIVE],
};

pub fn spec(kind: Type) -> &'static SeatSpec {
    match kind {
        Type::Mig29 | Type::Su27 => &K36DM,
        Type::F18|Type::F18E|Type::EA18G => &NACES,
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
    separation_q: Quat,
}

/// Seat pan above the jet's body origin, from where the pilot sits.
fn seat_point(a: &Aircraft) -> V3 {
    let eye = crate::cockpit::design_eye(a);
    eye - V3::Y * 0.80
}

/// Harness point above the seat pan.
const HARNESS: f64 = 0.28;

impl Ejection {
    pub fn new(a: &Aircraft) -> Ejection {
        let spec = spec(a.kind);
        let ax = a.q.axes();
        let keas = a.airspeed_kt;
        let separate_at = if keas > spec.fast { spec.separate.1 } else { spec.separate.0 };
        let pilot = Body { pos: a.pos + a.q.rotate(seat_point(a) + V3::Y * HARNESS), vel: a.vel, q: a.q, spin: V3::ZERO, down: false };
        // Thrusters throw the canopy up and back; the air takes it from there.
        let canopy = Body { pos: a.pos, vel: a.vel + ax.up * 9.0 + ax.back * 2.0, q: a.q, spin: ax.right * -1.6, down: false };
        Ejection { kind: a.kind, spec, age: 0.0, pilot, canopy, seat: None, chute: 0.0, phase: Phase::Canopy, landed_for: 0.0, fired: None, separate_at, rail: ax.up, sway: 0.0, separation_q: a.q }
    }

    /// Seconds the seat has been away from the jet, or None before it fires.
    pub fn away(&self) -> Option<f64> {
        self.fired.map(|t| self.age - t)
    }

    /// Completed pull retained throughout canopy clearance and seat travel.
    pub fn handle_pull(&self) -> f64 {
        // The cockpit has completed its reach and pull before initiation.
        // Keep that pose instead of restarting the pull after canopy release.
        1.0
    }

    /// The same eased deployment index drives both first-person and external poses.
    pub fn deployment_frame(&self) -> usize {
        (smoothstep(0.0, 0.55, self.chute) * (DEPLOY_FRAMES - 1) as f64).round() as usize
    }

    /// Cached landing pose shared by the arms, loose risers and cord roots.
    pub fn landing_frame(&self) -> usize {
        (smoothstep(0.0, 0.8, self.landed_for) * (LANDING_FRAMES - 1) as f64).round() as usize
    }

    /// Cord endpoints use the pilot and canopy frames independently, so
    /// inflation and sway never pull the cords away from the risers.
    pub fn suspension(&self, gore: usize) -> (V3, V3) {
        let skirt = skirt_point(gore);
        let side = if skirt.x > 0.0 { 1.0 } else { -1.0 };
        let lower = self.landing_frame() as f64 / (LANDING_FRAMES - 1) as f64;
        let root = self.pilot.pos + self.pilot.q.rotate(riser_tip(side, lower));
        let (right, up, back, origin) = self.chute_frame();
        (root, origin + right * skirt.x + up * skirt.y + back * skirt.z)
    }

    /// Canopy and suspension lines, rooted at the harness while airborne.
    /// On touchdown the skirt drifts clear and the fabric settles beside the pilot.
    pub fn chute_frame(&self) -> (V3, V3, V3, V3) {
        let ax = self.pilot.q.axes();
        let right = ax.right.with_y(0.0).norm_or(V3::X);
        let back = right.cross(V3::Y);
        let spread = 0.12 + 0.88 * self.chute * self.chute;
        let rise = 0.35 + 0.65 * self.chute.sqrt();
        if self.phase != Phase::Landed {
            return (right * spread, V3::Y * rise, back * spread, self.pilot.pos);
        }
        let settle = smoothstep(0.0, 2.5, self.landed_for);
        let mut target = self.pilot.pos + right * 4.2 + back * 0.8;
        target.y = terrain::floor(target.x, target.z) + 0.05;
        (right * mix(spread, 1.0, settle), V3::Y * mix(rise, 0.055, settle),
            back * mix(spread, 0.65, settle), self.pilot.pos.lerp(target, settle))
    }

    fn rocket_start(&self) -> f64 {
        if self.kind.russian() { 1.06 / self.spec.catapult } else { 0.0 }
    }

    /// Rocket burning after the catapult stroke, for thrust and flame alike.
    pub fn rocket(&self) -> bool {
        self.away().is_some_and(|t| t >= self.rocket_start() && t < self.rocket_start() + self.spec.rocket.0)
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
                    toast = Some(if self.kind.russian() { "K-36DM // SEAT AWAY, BOOMS OUT" } else if self.kind==Type::F18 { "SJU-17 // SEAT AWAY" } else { "ACES II // SEAT AWAY" });
                }
            }
            Phase::Seat => {
                let t = self.away().unwrap_or(0.0);
                if self.rocket() {
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
                    self.separation_q = self.pilot.q;
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
                    // Keep the fabric on the ground after it falls to the side.
                    // Deflation is a pose change, never a disappearance.
                    return None;
                }
                self.chute = (self.chute + dt / s.inflate).min(1.0);
                let fill = self.chute * self.chute;
                self.pilot.fly(dt, s.mass.1, s.cda.0 + fill * s.cda.2, HARNESS + 0.90);
                // The pilot hangs upright under the risers, facing where he
                // drifts, swinging a little as the canopy takes him.
                self.sway += dt;
                let course = v3(self.pilot.vel.x, 0.0, self.pilot.vel.z);
                let face = if course.len() > 0.5 { course.norm() } else { self.pilot.q.axes().forward.with_y(0.0).norm_or(V3::Z * -1.0) };
                let upright = Quat::from_forward_up(face, V3::Y);
                let swing = (self.sway * 1.7).sin() * 9.0 * DEG * (1.0 - 0.6 * fill.min(1.0)) * (-self.sway * 0.25).exp().max(0.3);
                let target = Quat::axis_angle(upright.axes().right, swing).mul(upright);
                self.pilot.q = blend_attitude(self.separation_q, target, smoothstep(0.0, 0.55, self.chute));
                if self.pilot.down {
                    self.phase = Phase::Landed;
                    toast = Some("PILOT ON THE GROUND");
                }
            }
        }
        toast
    }
}

fn blend_attitude(a: Quat, b: Quat, t: f64) -> Quat {
    let sign = if a.x*b.x+a.y*b.y+a.z*b.z+a.w*b.w < 0.0 {-1.0} else {1.0};
    Quat {x:mix(a.x,b.x*sign,t), y:mix(a.y,b.y*sign,t),
        z:mix(a.z,b.z*sign,t), w:mix(a.w,b.w*sign,t)}.norm()
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
    let (shell, cushion) = if kind.russian() { (Rgb(0.30, 0.36, 0.36), Rgb(0.34, 0.24, 0.16)) } else { (Rgb(0.16, 0.17, 0.17), Rgb(0.20, 0.24, 0.18)) };
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
    if kind.russian() && booms {
        for s in [-1.0, 1.0] {
            // Deployed arm protectors and leg lift retain the limbs in the bucket.
            let guard = Frame { o: v3(s * 0.245, pan + 0.51, -0.17), ..f(V3::ZERO) };
            m.block(&guard, v3(0.014, 0.025, 0.24), shell.scale(1.1));
            m.block(&f(v3(s * 0.12, pan + 0.04, -0.32)), v3(0.038, 0.015, 0.10), shell.scale(0.8));
            let root = v3(s * 0.14, pan + 1.12, 0.22);
            let tip = root + v3(s * 0.20, 0.45, 1.10);
            m.capsule(root, tip, 0.02, 0.015, Rgb(0.55, 0.56, 0.55), 5);
            m.lathe(tip, (tip - root).norm(), &[(0.0, 0.0), (0.0, 0.10), (0.18, 0.13), (0.20, 0.0)], 6, WHITE);
        }
    }
    m
}

pub const HANDLE_FRAMES: usize = 12;

/// Shared reference between cockpit design space and the free seat's harness.
pub fn cockpit_harness_origin()->V3 {crate::airframe::EYE-V3::Y*0.52}

pub fn handle_point(kind: Type, pull: f64) -> V3 {
    let y=if kind.russian(){-0.180}else if kind==Type::F18{-0.175}else{-0.170};
    v3(0.0,y+clamp(pull,0.0,1.0)*0.10,if kind==Type::F18{-0.330}else{-0.335})
}
fn handle_curve(kind:Type,side:f64,t:f64)->V3 {
    if kind.russian() {
        // Paired K-36 loops flare outwards at the crown and meet at the base.
        v3(side*(0.034+0.026*t.cos()+0.027*t.sin()),
            0.009-0.018*t.cos()+0.039*t.sin(),0.010*t.sin())
    } else {
        let (rx,ry)=if kind==Type::F18{(0.050,0.0375)}else{(0.060,0.032)};
        let y=ry*t.sin();
        v3(rx*t.cos(),if kind==Type::F16{y.min(0.028)}else{y},0.0)
    }
}
pub fn handle_grip(kind:Type,side:f64,pull:f64)->V3 {
    let t=if kind.russian(){PI*0.38}else if side>0.0{0.927295}else{PI-0.927295};
    handle_point(kind,pull)+handle_curve(kind,side,t)
}
pub fn handle_grip_axis(kind:Type,side:f64)->V3 {
    let t=if kind.russian(){PI*0.38}else if side>0.0{0.927295}else{PI-0.927295};
    let tangent=(handle_curve(kind,side,t-0.001)-handle_curve(kind,side,t+0.001)).norm();
    if !kind.russian() && side<0.0{-tangent}else{tangent}
}
pub fn handle_mesh(kind: Type, pull: f64) -> Mesh {
    let mut m = Mesh::new();
    let centre=handle_point(kind,pull);let rest=handle_point(kind,0.0);
    let plain=|o|Frame{o,x:V3::X,y:V3::Y,z:V3::Z};
    let mount=rest-v3(0.0,0.054,-0.014);
    let dark=Rgb(0.10,0.11,0.11);
    m.block(&plain(mount),v3(if kind.russian(){0.061}else{0.038},0.012,0.026),dark);
    for side in [-1.0,1.0] {
        let x=side*(if kind.russian(){0.038}else{0.025});
        m.capsule(v3(x,-HARNESS+0.020,-0.365),mount+V3::X*x,0.009,0.009,dark,8);
        m.capsule(mount+V3::X*x,centre+v3(x,-0.032,0.0),0.003,0.003,Rgb(0.50,0.51,0.49),8);
    }
    let loops:&[f64]=if kind.russian(){&[-0.038,0.038]}else{&[0.0]};
    for &offset in loops {
        for i in 0..32 {
            let point=|t:f64|centre+handle_curve(kind,if offset<0.0{-1.0}else{1.0},t);
            let col=if kind.russian(){Rgb(0.80,0.065,0.035)}
                else if i%8<4{Rgb(0.95,0.79,0.12)}else{Rgb(0.055,0.055,0.05)};
            m.capsule(point(i as f64*TAU/32.0),point((i+1) as f64*TAU/32.0),0.007,0.007,col,8);
        }
    }
    m
}

pub const DEPLOY_FRAMES: usize = 24;
pub const LANDING_FRAMES: usize = 16;
pub const GORES: usize = 16;
const WEB: Rgb = Rgb(0.52, 0.49, 0.36);
const GLOVE: Rgb = Rgb(0.24, 0.26, 0.20);
const TORSO_RING: [(f64, f64); 8] = [(1.0, -0.45), (1.0, 0.45), (0.55, 1.0),
    (-0.55, 1.0), (-1.0, 0.45), (-1.0, -0.45), (-0.55, -1.0), (0.55, -1.0)];

fn hip_at(deploy: f64) -> V3 { v3(0.0, -HARNESS + 0.10, mix(-0.26,0.0,deploy)) }
#[cfg(test)]
fn hip_point(hanging: bool) -> V3 { hip_at(if hanging {1.0} else {0.0}) }

/// The harness samples the same cross-sections as the suit, in either pose.
/// The torso is identical in both views; only the external helmet is omitted.
fn torso_at(deploy: f64, t: f64, x: f64, front: f64) -> V3 {
    let hip = hip_at(deploy);
    let top = v3(0.0, 0.29, 0.04);
    let forward = (top - hip).norm().cross(V3::X);
    let width = mix(0.165, 0.19, t);
    let depth = mix(0.105, 0.115, t);
    hip.lerp(top, t) + V3::X * (width * x) + forward * (depth * front)
}

fn harness_anchor(hanging: bool, side: f64) -> V3 {
    torso_at(if hanging {1.0} else {0.0}, 0.94, side * 0.50, 1.0)
}

fn webbing(m: &mut Mesh, a: V3, b: V3, width: f64) {
    let y = (b - a).norm();
    let x = y.cross(V3::Z).norm();
    webbing_across(m, a, b, width, x);
}

fn webbing_across(m: &mut Mesh, a: V3, b: V3, width: f64, across: V3) {
    let y = (b - a).norm();
    let x = (across - y * y.dot(across)).norm();
    m.block(&Frame { o: (a + b) * 0.5, x, y, z: x.cross(y) },
        v3(width * 0.5, (b - a).len() * 0.5, 0.003), WEB);
}

fn riser_tip(side: f64, lower: f64) -> V3 {
    v3(side * 0.24, 1.35, -0.12).lerp(v3(side * 0.23, -0.23, -0.11), lower)
}

fn riser_grip(side: f64) -> V3 {
    harness_anchor(true, side).lerp(riser_tip(side, 0.0), 0.33)
}

fn resting_hand(side: f64) -> V3 { v3(side * 0.25, -0.30, -0.08) }

/// Closed fingers and thumb shared by cockpit, seat and parachute poses.
pub fn closed_hand(m: &mut Mesh, side: f64, centre: V3, y: V3) -> V3 {
    let x = y.cross(V3::Z).norm_or(V3::X) * side;
    let z = (V3::Z - y * y.z).norm();
    let mut glove=Mesh::new();
    let at = |a: f64, b: f64, c: f64| v3(a,b,c);
    glove.capsule(at(0.031, -0.035, 0.023), at(0.031, 0.035, 0.023), 0.021, 0.023, GLOVE, 6);
    for finger in 0..4 {
        let h = -0.027 + finger as f64 * 0.018;
        glove.capsule(at(0.029, h, 0.014), at(0.018, h, -0.021), 0.011, 0.010, GLOVE, 5);
        glove.capsule(at(0.018, h, -0.021), at(-0.022, h, -0.018), 0.010, 0.009, GLOVE, 5);
        glove.capsule(at(-0.022, h, -0.018), at(-0.025, h, 0.008), 0.009, 0.008, GLOVE, 5);
    }
    glove.capsule(at(0.030, -0.026, 0.026), at(-0.023, -0.036, 0.018), 0.014, 0.011, GLOVE.scale(0.9), 5);
    glove.capsule(at(-0.023, -0.036, 0.018), at(-0.028, -0.010, -0.003), 0.011, 0.009, GLOVE.scale(0.9), 5);
    for v in glove.v.chunks_mut(crate::mesh::STRIDE) {
        let p=centre+x*v[0] as f64+y*v[1] as f64+z*v[2] as f64;
        let n=x*v[3] as f64+y*v[4] as f64+z*v[5] as f64;
        v[..6].copy_from_slice(&[p.x as f32,p.y as f32,p.z as f32,n.x as f32,n.y as f32,n.z as f32]);
    }
    m.v.extend(glove.v);
    centre+x*0.036-y*0.052+z*0.022
}

pub fn pilot_mesh(kind: Type, hanging: bool, first_person: bool, pull: f64) -> Mesh {
    pilot_pose(kind, if hanging {1.0} else {0.0}, first_person, pull, 0.0)
}

pub fn landed_mesh(kind: Type, first_person: bool, frame: usize) -> Mesh {
    pilot_pose(kind, 1.0, first_person, 1.0, frame as f64 / (LANDING_FRAMES - 1) as f64)
}

pub fn deployment_mesh(kind: Type, first_person: bool, frame: usize) -> Mesh {
    pilot_pose(kind, frame as f64/(DEPLOY_FRAMES-1) as f64, first_person, 1.0, 0.0)
}

/// Seated, suspended or releasing the risers after touchdown.
fn pilot_pose(kind: Type, deploy: f64, first_person: bool, pull: f64, lower: f64) -> Mesh {
    let mut m = Mesh::new();
    let (suit, helmet) = if kind.russian() { (Rgb(0.40, 0.42, 0.36), Rgb(0.88, 0.88, 0.84)) }
        else { (Rgb(0.30, 0.34, 0.22), Rgb(0.78, 0.80, 0.78)) };
    let gsuit = suit.scale(0.8);
    let hip = hip_at(deploy);
    for i in 0..8 {
        let (x0, f0) = TORSO_RING[i];
        let (x1, f1) = TORSO_RING[(i + 1) % 8];
        m.quad(torso_at(deploy, 0.0, x0, f0), torso_at(deploy, 0.0, x1, f1),
            torso_at(deploy, 1.0, x1, f1), torso_at(deploy, 1.0, x0, f0), suit);
        m.tri(torso_at(deploy, 1.0, 0.0, 0.0), torso_at(deploy, 1.0, x0, f0),
            torso_at(deploy, 1.0, x1, f1), suit);
    }
    if !first_person {
        m.capsule(v3(0.0, 0.42, 0.04), v3(0.0, 0.58, 0.02), 0.13, 0.13, helmet, 8);
        m.block(&Frame { o: v3(0.0, 0.48, -0.08), x: V3::X, y: V3::Y, z: V3::Z },
            v3(0.09, 0.04, 0.03), Rgb(0.12, 0.12, 0.10));
    }
    // Webbing is centred on the suit surface: half its thickness is embedded
    // in the fabric. It follows the seated lean as well as the upright torso.
    for side in [-1.0, 1.0] {
        for pair in [0.14, 0.38, 0.66, 0.94, 1.0].windows(2) {
            let at = |t: f64| torso_at(deploy, t, side * mix(0.32, 0.51, t), 1.0);
            webbing(&mut m, at(pair[0]), at(pair[1]), 0.035);
        }
        let a = torso_at(deploy, 1.0, side * 0.51, 1.0);
        let b = torso_at(deploy, 1.0, side * 0.51, -1.0);
        webbing_across(&mut m, a, b, 0.035, V3::X);
        webbing(&mut m, b, torso_at(deploy, 0.14, side * 0.32, -1.0), 0.035);
        m.ring(&Frame { o: torso_at(deploy,0.94,side*0.50,1.0), x: V3::X, y: V3::Y, z: V3::Z },
            0.012, 0.019, 0.004, Rgb(0.55, 0.56, 0.53), 8);
        if deploy > 0.0 {
            let anchor = torso_at(deploy,0.94,side*0.50,1.0);
            // Once released, the loose riser folds down beside the hip.
            let bend = anchor.lerp(riser_tip(side, lower), 0.45) + V3::X * (side * 0.045 * lower);
            webbing(&mut m, anchor, bend, 0.032);
            webbing(&mut m, bend, riser_tip(side, lower), 0.032);
        }
    }
    for i in 0..8 {
        let (a, b) = (TORSO_RING[i], TORSO_RING[(i + 1) % 8]);
        let up = (v3(0.0, 0.29, 0.04) - hip).norm();
        webbing_across(&mut m, torso_at(deploy, 0.14, a.0, a.1),
            torso_at(deploy, 0.14, b.0, b.1), 0.040, up);
    }
    m.block(&Frame { o: torso_at(deploy, 0.14, 0.0, 1.0), x: V3::X, y: V3::Y, z: V3::Z },
        v3(0.026, 0.026, 0.008), Rgb(0.60, 0.61, 0.57));
    for side in [-1.0, 1.0] {
        let hp = hip + V3::X * (side * 0.10);
        let seated_knee=hp+v3(0.0,if kind.russian(){0.17}else{0.10},-0.45);
        let seated_foot=hp+v3(0.0,if kind.russian(){-0.18}else{-0.32},-0.55);
        let knee=seated_knee.lerp(hp+v3(side*0.03,-0.45,-0.06),deploy);
        let foot=seated_foot.lerp(hp+v3(side*0.04,-0.92,0.0),deploy);
        m.capsule(hp, knee, 0.085, 0.07, gsuit, 6);
        m.capsule(knee, foot, 0.065, 0.055, gsuit, 6);
        m.capsule(foot, foot + v3(0.0, -0.03, -0.12), 0.05, 0.05, Rgb(0.08, 0.07, 0.06), 5);
        let shoulder = v3(side * 0.20, 0.24, 0.04);
        let grip=handle_grip(kind,side,pull).lerp(riser_grip(side),deploy)
            .lerp(resting_hand(side),lower);
        let axis=handle_grip_axis(kind,side).lerp(
            (riser_tip(side,0.0)-harness_anchor(true,side)).norm(),deploy)
            .lerp(-V3::Y,lower).norm_or(V3::Y);
        let hand=closed_hand(&mut m,side,grip,axis);
        let seated_elbow=v3(side*0.22,mix(shoulder.y,hand.y,0.60),-0.30);
        let hanging_elbow=shoulder.lerp(hand,0.5)+v3(side*mix(0.09,0.025,lower),0.0,-0.06);
        let elbow=seated_elbow.lerp(hanging_elbow,deploy);
        m.capsule(shoulder, elbow, 0.06, 0.055, suit, 6);
        m.capsule(elbow, hand, 0.055, 0.035, suit, 6);

    }
    m
}

/// One reusable cord, transformed between its two world-space endpoints.
pub fn cord_mesh() -> Mesh {
    let mut m = Mesh::new();
    m.block(&Frame { o: V3::Y * 0.5, x: V3::X, y: V3::Y, z: V3::Z },
        v3(0.0018, 0.5, 0.0018), Rgb(0.82, 0.80, 0.74));
    m
}

pub fn cord_frame(a: V3, b: V3) -> Frame {
    let y = b - a;
    let axis = y.norm();
    let x = axis.cross(if axis.z.abs() < 0.9 { V3::Z } else { V3::X }).norm();
    Frame { o: a, x, y, z: x.cross(axis) }
}

fn canopy_point(k: usize, t: f64) -> V3 {
    let a = k as f64 * TAU / GORES as f64;
    let th = t * 80.0 * DEG;
    let r = 3.4 * th.sin() / (80.0 * DEG).sin();
    v3(a.cos() * r, mix(8.6, 6.6, (1.0 - th.cos()) / (1.0 - (80.0 * DEG).cos())), a.sin() * r)
}

fn skirt_point(gore: usize) -> V3 { canopy_point(gore, 1.0) }

/// Main canopy fabric, from vent to skirt in the seat's own colours.
/// The scene attaches separate suspension cords to the moving pilot.
pub fn chute_mesh(spec: &SeatSpec) -> Mesh {
    let mut m = Mesh::new();
    let n = GORES;
    let ring = canopy_point;
    let rows = 4;
    for k in 0..n {
        let col = spec.gores[k % spec.gores.len()];
        for j in 0..rows {
            let (t0, t1) = (0.12 + j as f64 / rows as f64 * 0.88, 0.12 + (j + 1) as f64 / rows as f64 * 0.88);
            let (a, b, c, d) = (ring(k, t0), ring(k + 1, t0), ring(k + 1, t1), ring(k, t1));
            m.quad(a, b, c, d, col);
            // One surface, visible from both sides. A coplanar reverse skin
            // causes z-fighting and alternates the lighting as the camera moves.
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Mission, FIXED_DT};

    #[test]
    fn k36_stabilising_booms_and_restraints_stay_stowed_in_the_cockpit() {
        for kind in [Type::Mig29,Type::Su27] {
            let extent=|m:Mesh|m.v.chunks(crate::mesh::STRIDE).map(|v|v[2])
                .fold(f32::NEG_INFINITY,f32::max);
            assert!(extent(seat_mesh(kind,false))<0.4);
            assert!(extent(seat_mesh(kind,true))>1.3);
        }
    }

    #[test]
    fn handles_have_type_specific_finishes_and_grips_touch_the_tube() {
        for kind in Type::ALL {
            let mesh=handle_mesh(kind,0.0);
            let colored:Vec<_>=mesh.v.chunks(crate::mesh::STRIDE)
                .filter(|v|v[6]>0.7).collect();
            assert!(!colored.is_empty());
            assert!(colored.iter().all(|v|if kind.russian(){v[7]<0.1}else{v[7]>0.7}));
            for side in [-1.0,1.0] {
                let grip=handle_grip(kind,side,0.0);
                let distance=colored.iter().map(|v|(v3(v[0] as f64,v[1] as f64,v[2] as f64)-grip).len())
                    .fold(f64::INFINITY,f64::min);
                // US alternating black sections may be the closest tube.
                let tube=mesh.v.chunks(crate::mesh::STRIDE).map(|v|
                    (v3(v[0] as f64,v[1] as f64,v[2] as f64)-grip).len()).fold(distance,f64::min);
                assert!(tube<0.010,"{kind:?}: grip {tube}m away from handle");
            }
        }
    }

    #[test]
    fn deployment_preserves_the_seated_pose_then_moves_limbs_continuously() {
        for kind in Type::ALL {
            let surface=|m:Mesh|m.v.chunks(crate::mesh::STRIDE)
                .filter(|v|(v[6] as f64-WEB.0).abs()>1e-6).flatten().copied().collect::<Vec<_>>();
            assert_eq!(surface(pilot_mesh(kind,false,true,1.0)),
                surface(deployment_mesh(kind,true,0)));
            assert_eq!(deployment_mesh(kind,true,DEPLOY_FRAMES-1).v,
                pilot_mesh(kind,true,true,1.0).v);
            let mut previous:Option<Vec<V3>>=None;
            for frame in 0..DEPLOY_FRAMES {
                let mesh=deployment_mesh(kind,true,frame);
                assert!(mesh.v.iter().all(|x|x.is_finite()));
                let glove:Vec<_>=mesh.v.chunks(crate::mesh::STRIDE)
                    .filter(|v|(v[6] as f64-GLOVE.0).abs()<1e-6 &&
                        (v[7] as f64-GLOVE.1).abs()<1e-6 && (v[8] as f64-GLOVE.2).abs()<1e-6)
                    .map(|v|v3(v[0] as f64,v[1] as f64,v[2] as f64)).collect();
                if let Some(old)=previous {
                    assert_eq!(old.len(),glove.len());
                    let max=old.iter().zip(&glove).map(|(a,b)|(*a-*b).len()).fold(0.0,f64::max);
                    assert!(max<0.070,"{kind:?} frame {frame}: {max}");
                }
                previous=Some(glove);
            }
        }
    }

    #[test]
    fn initiation_keeps_the_pull_and_separation_does_not_snap_the_camera() {
        for kind in Type::ALL {
            let mut a=Aircraft::new(&Mission::new());a.set_type(kind);
            a.pos.y=1000.0;a.q=Quat::axis_angle(V3::Z,35.0*DEG);
            let mut e=Ejection::new(&a);
            while e.phase!=Phase::Chute {
                assert_eq!(e.handle_pull(),1.0);
                e.step(FIXED_DT,&a);
            }
            assert_eq!(e.deployment_frame(),0);
            let before=e.pilot.q;
            e.step(FIXED_DT,&a);
            assert!((before.rotate(V3::Y)-e.pilot.q.rotate(V3::Y)).len()<0.005);
            e.chute=1.0;assert_eq!(e.deployment_frame(),DEPLOY_FRAMES-1);
        }
    }

    #[test]
    fn first_person_eye_and_handle_match_the_cockpit_at_seat_firing() {
        for kind in Type::ALL {
            let mut a=Aircraft::new(&Mission::new());a.set_type(kind);
            let e=Ejection::new(&a);
            let eye=e.pilot.pos+a.q.rotate(v3(0.0,0.52,0.0));
            assert!((eye-crate::cockpit::eye_world(&a)).len()<1e-8);
            for frame in 0..HANDLE_FRAMES {
                let pull=frame as f64/(HANDLE_FRAMES-1) as f64;
                let local=seat_point(&a)+V3::Y*HARNESS+handle_point(kind,pull);
                let cockpit=cockpit_harness_origin()+handle_point(kind,pull)
                    -crate::airframe::EYE+kind.eye()+crate::cockpit::seat_offset(kind);
                assert!((local-cockpit).len()<1e-8);
            }
        }
    }

    #[test]
    fn all_seats_clear_the_jet_separate_and_leave_the_chute_on_the_ground() {
        for kind in Type::ALL {
            let mut a = Aircraft::new(&Mission::new());
            a.kind = kind;
            a.q = Quat::axis_angle(V3::Y, 0.0);
            a.pos.y = terrain::floor(a.pos.x, a.pos.z) + 10.0;
            a.vel = V3::ZERO;
            let mut e = Ejection::new(&a);
            let start = e.pilot.pos;
            let mut saw_seat = false;
            let mut saw_chute = false;
            for _ in 0..(120.0 / FIXED_DT) as usize {
                e.step(FIXED_DT, &a);
                assert!(e.pilot.pos.x.is_finite() && e.pilot.pos.y.is_finite() && e.pilot.pos.z.is_finite());
                if e.phase == Phase::Seat && e.away().unwrap() > 0.2 {
                    saw_seat = true;
                    assert!(e.pilot.pos.y > start.y + 1.0);
                    assert!(e.seat.is_none());
                }
                saw_chute |= e.phase == Phase::Chute && e.chute > 0.9 && e.seat.is_some();
                if e.phase == Phase::Landed && e.landed_for >= 2.5 { break; }
            }
            assert!(saw_seat && saw_chute, "{kind:?}");
            assert_eq!(e.phase, Phase::Landed);
            assert!(e.chute > 0.9, "fabric disappeared");
            let (_, up, _, origin) = e.chute_frame();
            assert!((origin - e.pilot.pos).with_y(0.0).len() > 4.0);
            assert!((origin + up * 8.6).y < terrain::floor(origin.x, origin.z) + 0.6);
            let landed = e.pilot.pos;
            for _ in 0..120 { e.step(FIXED_DT, &a); }
            assert_eq!(e.pilot.pos, landed);
            assert!(e.chute > 0.9);
        }
    }

    #[test]
    fn parachute_has_one_fabric_surface_without_coplanar_reverse_faces() {
        for spec in [&ACES2, &K36DM] {
            let m = chute_mesh(spec);
            let mut triangles = std::collections::HashSet::new();
            for tri in m.v.chunks(crate::mesh::STRIDE * 3) {
                let mut points: Vec<[u32; 3]> = tri.chunks(crate::mesh::STRIDE)
                    .map(|v| [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]).collect();
                points.sort_unstable();
                assert!(triangles.insert(points), "duplicate fabric surface");
            }
        }
    }

    fn segment_distance(p: V3, a: V3, b: V3) -> f64 {
        let ab = b - a;
        (p - a.lerp(b, clamp((p - a).dot(ab) / ab.dot(ab), 0.0, 1.0))).len()
    }

    #[test]
    fn harness_webbing_follows_the_suit_in_both_views_and_poses() {
        for kind in Type::ALL {
            for hanging in [false, true] {
                let body = pilot_mesh(kind, hanging, true, 1.0);
                let external = pilot_mesh(kind, hanging, false, 1.0);
                let hip = hip_point(hanging);
                let up = (v3(0.0, 0.29, 0.04) - hip).norm();
                let len = (v3(0.0, 0.29, 0.04) - hip).len();
                let front = up.cross(V3::X);
                for v in body.v.chunks(crate::mesh::STRIDE).filter(|v| (v[6] as f64 - WEB.0).abs() < 1e-6) {
                    let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                    let t = (p - hip).dot(up) / len;
                    if hanging && t > 0.90 { continue; } // The free risers above the shoulder.
                    let tc = clamp(t, 0.0, 1.0);
                    let centre = hip.lerp(v3(0.0, 0.29, 0.04), tc);
                    let ring: Vec<_> = TORSO_RING.iter().map(|&(x, z)| v3(x * mix(0.165, 0.19, tc), 0.0, z * mix(0.105, 0.115, tc))).collect();
                    let p2 = v3(p.x, 0.0, (p - centre).dot(front));
                    let distance = (0..8).map(|i| segment_distance(p2, ring[i], ring[(i + 1) % 8]))
                        .fold(f64::INFINITY, f64::min);
                    assert!(distance < 0.014, "floating harness: {kind:?}, hanging={hanging}, {p:?}, gap={distance}");
                    assert!(external.v.chunks(crate::mesh::STRIDE).any(|e| e[..3] == v[..3] && e[6..9] == v[6..9]));
                }
            }
        }
    }

    #[test]
    fn gloves_wrap_the_risers_and_landing_lowers_both_arms() {
        for kind in Type::ALL {
            let mut previous = [f64::INFINITY; 2];
            for frame in 0..LANDING_FRAMES {
                for fp in [false, true] {
                    let body = landed_mesh(kind, fp, frame);
                    assert!(body.v.iter().all(|v| v.is_finite()));
                    for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
                        let glove: Vec<_> = body.v.chunks(crate::mesh::STRIDE)
                            .filter(|v| (v[6] as f64 - GLOVE.0).abs() < 1e-6 && (v[7] as f64 - GLOVE.1).abs() < 1e-6 && (v[8] as f64 - GLOVE.2).abs() < 1e-6 && v[0] as f64 * side > 0.0)
                            .map(|v| v3(v[0] as f64, v[1] as f64, v[2] as f64)).collect();
                        let top = glove.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                        if fp {
                            assert!(top <= previous[i] + 0.005, "hand rose after touchdown");
                            previous[i] = top;
                        }
                        if frame == 0 {
                            let grip = riser_grip(side);
                            let axis = (riser_tip(side, 0.0) - harness_anchor(true, side)).norm();
                            let x = axis.cross(V3::Z).norm() * side;
                            assert!(glove.iter().any(|p| (*p - grip).dot(x) > 0.020));
                            assert!(glove.iter().any(|p| (*p - grip).dot(x) < -0.020));
                            assert!(glove.iter().any(|p| p.z < grip.z - 0.012));
                            assert!(glove.iter().any(|p| p.z > grip.z + 0.012));
                        } else if frame == LANDING_FRAMES - 1 {
                            assert!(top < -0.23, "hand still raised on the ground");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn cords_stay_on_the_risers_during_inflation_sway_and_touchdown() {
        let a = Aircraft::new(&Mission::new());
        for fill in [0.02, 0.25, 0.65, 1.0] {
            for angle in [-9.0, 0.0, 9.0] {
                let mut e = Ejection::new(&a);
                e.phase = Phase::Chute;
                e.chute = fill;
                e.pilot.q = Quat::axis_angle(V3::Y, 0.7).mul(Quat::axis_angle(V3::X, angle * DEG));
                for landed in [false, true] {
                    e.phase = if landed { Phase::Landed } else { Phase::Chute };
                    e.landed_for = if landed { 2.5 } else { 0.0 };
                    let lower = e.landing_frame() as f64 / (LANDING_FRAMES - 1) as f64;
                    for gore in 0..GORES {
                        let (root, skirt) = e.suspension(gore);
                        let side = if skirt_point(gore).x > 0.0 { 1.0 } else { -1.0 };
                        let attachment = e.pilot.q.conj().rotate(root - e.pilot.pos);
                        assert!((attachment - riser_tip(side, lower)).len() < 1e-8);
                        assert!(root.is_finite() && skirt.is_finite() && (root - skirt).len() > 0.1);
                        let f = cord_frame(root, skirt);
                        assert!((f.p(0.0, 0.0, 0.0) - root).len() < 1e-8);
                        assert!((f.p(0.0, 1.0, 0.0) - skirt).len() < 1e-8);
                        assert!(f.x.len() > 0.99 && f.z.len() > 0.99);
                    }
                }
            }
        }
    }

    #[test]
    fn the_handle_moves_with_both_gloves_and_the_helmet_is_out_of_first_person() {
        assert!((handle_point(Type::F16, 1.0).y - handle_point(Type::F16, 0.0).y - 0.10).abs() < 1e-10);
        for kind in Type::ALL {
            for frame in 0..HANDLE_FRAMES {
                let pull = frame as f64 / (HANDLE_FRAMES - 1) as f64;
                let body = pilot_mesh(kind, false, true, pull);
                let head = pilot_mesh(kind, false, false, pull);
                assert!(body.count() < head.count());
                for v in body.v.chunks(crate::mesh::STRIDE) {
                    let p = v3(v[0] as f64, v[1] as f64, v[2] as f64);
                    assert!((p - v3(0.0, 0.52, 0.0)).len() > 0.15);
                    assert!(v.iter().all(|x| x.is_finite()));
                }
                for side in [-1.0, 1.0] {
                    let grip = handle_grip(kind,side,pull);
                    assert!(body.v.chunks(crate::mesh::STRIDE).any(|v|
                        (v3(v[0] as f64, v[1] as f64, v[2] as f64) - grip).len() < 0.025));
                }
            }
        }
    }
}
