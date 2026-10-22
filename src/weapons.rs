//! Stores of the F-16C Block 50 and their physics, simplified but with the
//! published figures: the M61A1 gun and its 20 mm rounds, AIM-9M and AIM-120C
//! missiles flying proportional navigation within their seeker, motor, g and
//! fuze limits, and Mk-82 low-drag bombs. The sights that aim them live here
//! too: the EEGS-style funnel of rounds already in the air, the LCOS pipper on
//! that stream, the CCIP impact prediction and the missile launch-zone
//! fly-out. Everything integrates at the fixed 120 Hz step, in world metres.
use crate::math::*;
use crate::sim::{atmosphere_at, Aircraft, FIXED_DT, G0};
use std::collections::VecDeque;

/// Anything that can shoot or be shot at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    Player,
    Bandit(usize),
    Ground(usize),
}

// ================================================================ M61A1 gun
/// 511 linkless 20 mm rounds in the drum behind the cockpit.
pub const GUN_ROUNDS: u32 = 511;
/// 6,000 rounds per minute.
pub const GUN_RATE: f64 = 100.0;
/// PGU-28/B muzzle velocity, m/s.
pub const MUZZLE_VELOCITY: f64 = 1036.0;
/// Gun muzzle in body axes: the port in the left strake, abeam the seat.
pub const GUN_PORT: V3 = v3(-0.72, 0.52, -3.40);
/// The gun line points through the HUD gun cross, 55 mrad above the boresight.
pub const GUN_ELEVATION: f64 = 55.0 / 650.0;
/// Mass the jet loses per round: the 100 g projectile and its propellant; the
/// linkless feed carries the empty cases back into the drum.
pub const ROUND_EXPENDED: f64 = 0.14;
/// One round in five is a tracer.
pub const TRACER_EVERY: u32 = 5;
/// Drag factor k = rho Cd A / 2m at sea level for the 100 g, 20 mm projectile
/// (Cd 0.28 supersonic): about 600 m/s are left at 1,000 m, reached in 1.3 s.
const ROUND_K: f64 = 0.5 * 1.225 * 0.28 * 3.14e-4 / 0.10;
/// Rounds stop being tracked after this long, about 2.5 km down range.
pub const ROUND_LIFE: f64 = 3.2;

/// Gun line in body axes.
pub fn gun_bore() -> V3 {
    v3(0.0, GUN_ELEVATION.sin(), -GUN_ELEVATION.cos())
}

/// Muzzle position and velocity of a round fired now from a jet.
pub fn muzzle(a: &Aircraft) -> (V3, V3) {
    (a.pos + a.q.rotate(GUN_PORT), a.vel + a.q.rotate(gun_bore()) * MUZZLE_VELOCITY)
}

/// Velocity-squared drag scaled by air density, and gravity: the motion of
/// rounds and bombs alike. Semi-implicit Euler at the fixed step.
fn ballistic(pos: &mut V3, vel: &mut V3, k_sea: f64, dt: f64) {
    let k = k_sea * atmosphere_at(pos.y).rho / 1.225;
    let v = vel.len();
    *vel += (*vel * (-k * v) - V3::Y * G0) * dt;
    *pos += *vel * dt;
}

#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub pos: V3,
    pub vel: V3,
    /// Position at the start of the last step, for swept hit tests.
    pub prev: V3,
    pub age: f64,
    pub tracer: bool,
    pub from: Who,
    /// Damage to an aircraft (hit points of 100) on a hit.
    pub damage: f64,
}

impl Round {
    pub fn new(pos: V3, vel: V3, tracer: bool, from: Who, damage: f64) -> Round {
        Round { pos, vel, prev: pos, age: 0.0, tracer, from, damage }
    }
    pub fn step(&mut self, dt: f64) {
        self.prev = self.pos;
        self.age += dt;
        ballistic(&mut self.pos, &mut self.vel, ROUND_K, dt);
    }
}

/// Where a moving segment comes closest to a moving point over one step:
/// relative positions at the start and the end, linear in between. Returns
/// the fraction of the step and the miss distance.
pub fn closest(r0: V3, r1: V3) -> (f64, f64) {
    let d = r1 - r0;
    let t = if d.dot(d) > 1e-12 { clamp(-r0.dot(d) / d.dot(d), 0.0, 1.0) } else { 0.0 };
    (t, (r0 + d * t).len())
}

/// Hit test of a swept round against an aircraft-sized ellipsoid (4.5 m half
/// span, 1.4 m half height, 7 m half length) in the target's body axes.
/// `p0`, `p1` are the round's positions relative to the target at the start
/// and the end of the step.
pub fn hits_airframe(a: &Aircraft, p0: V3, p1: V3) -> Option<f64> {
    let inv = a.q.conj();
    let scale = |p: V3| {
        let b = inv.rotate(p);
        v3(b.x / 4.5, b.y / 1.4, b.z / 7.0)
    };
    let (t, d) = closest(scale(p0), scale(p1));
    (d < 1.0).then_some(t)
}

/// Segment against an axis-aligned box centred on the origin with half extents `h`.
pub fn hits_box(p0: V3, p1: V3, h: V3) -> Option<f64> {
    let d = p1 - p0;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (o, dd, e) in [(p0.x, d.x, h.x), (p0.y, d.y, h.y), (p0.z, d.z, h.z)] {
        if dd.abs() < 1e-12 {
            if o.abs() > e {
                return None;
            }
            continue;
        }
        let (a, b) = ((-e - o) / dd, (e - o) / dd);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
        if t0 > t1 {
            return None;
        }
    }
    Some(t0)
}

/// Point where a step from `p0` to `p1` went below the ground, interpolated
/// on the height above it.
pub fn ground_crossing(p0: V3, p1: V3, floor: &dyn Fn(f64, f64) -> f64) -> Option<V3> {
    let h1 = p1.y - floor(p1.x, p1.z);
    if h1 >= 0.0 {
        return None;
    }
    let h0 = (p0.y - floor(p0.x, p0.z)).max(0.0);
    Some(p0.lerp(p1, h0 / (h0 - h1)))
}

/// Damage of a blast at distance `d`: full inside `inner`, falling linearly
/// to nothing at `outer`.
pub fn blast_damage(full: f64, inner: f64, outer: f64, d: f64) -> f64 {
    full * clamp(1.0 - (d - inner) / (outer - inner), 0.0, 1.0)
}

/// Gun sight: virtual rounds fired from the gun every 1/30 s and flown with
/// the real ballistics. Their present positions, relative to the jet, are the
/// EEGS funnel; the one at the target's range is the LCOS pipper, exact for a
/// shooter who keeps tracking the way he has for the last time of flight.
#[derive(Default)]
pub struct GunSight {
    pub shots: VecDeque<(V3, V3, f64)>,
    clock: f64,
}

/// History kept for the funnel, s: rounds out to about 1,500 m.
const SIGHT_SPAN: f64 = 1.6;

impl GunSight {
    pub fn step(&mut self, dt: f64, a: &Aircraft) {
        for (p, v, age) in self.shots.iter_mut() {
            ballistic(p, v, ROUND_K, dt);
            *age += dt;
        }
        while self.shots.front().is_some_and(|s| s.2 > SIGHT_SPAN) {
            self.shots.pop_front();
        }
        self.clock -= dt;
        if self.clock <= 0.0 {
            self.clock += 1.0 / 30.0;
            let (p, v) = muzzle(a);
            self.shots.push_back((p, v, 0.0));
        }
    }

    /// Where the stream is at a slant range from the gun, if it reaches it.
    pub fn at_range(&self, gun: V3, range: f64) -> Option<V3> {
        let mut prev: Option<(V3, f64)> = None;
        for &(p, _, _) in self.shots.iter().rev() {
            let r = (p - gun).len();
            if let Some((q, rq)) = prev {
                if rq <= range && r >= range {
                    return Some(q.lerp(p, (range - rq) / (r - rq).max(1e-6)));
                }
            }
            prev = Some((p, r));
        }
        None
    }
}

// ================================================================ missiles
/// Missile data. Masses, sizes, warheads and limits are the published
/// figures; motor impulse and drag are fitted to the published speeds.
#[derive(Debug)]
pub struct Spec {
    pub name: &'static str,
    pub mass: f64,
    pub length: f64,
    pub diameter: f64,
    /// Propellant burnt by the end of the last motor stage, kg.
    pub propellant: f64,
    /// Motor stages: thrust (N) and duration (s).
    pub motor: [(f64, f64); 2],
    /// Seconds between release and ignition (ejected from a pylon), 0 off a rail.
    pub drop: f64,
    /// Zero-lift drag coefficient on the body cross-section, subsonic.
    pub cd0: f64,
    /// Normal-force slope per radian of body angle of attack, on the cross-section.
    pub cna: f64,
    /// Body angle of attack the airframe can hold, rad.
    pub alpha_max: f64,
    pub g_limit: f64,
    /// Proportional-navigation constant.
    pub nav: f64,
    /// Seeker look limit off the missile's flight path, rad.
    pub gimbal: f64,
    /// Seeker field that acquires a target before launch, rad.
    pub acquire: f64,
    /// Proximity-fuze trigger distance and the blast's lethal and outer radii, m.
    pub fuze: f64,
    pub lethal: f64,
    pub outer: f64,
    /// Fuze arming delay and self-destruct time, s.
    pub arm: f64,
    pub life: f64,
    /// Minimum launch range, m, and the range the HUD and AI screen shots to.
    pub min_range: f64,
    pub max_range: f64,
    /// Active radar seeker that searches by itself (AIM-120); otherwise it
    /// needs a lock at launch (AIM-9).
    pub active: bool,
}

/// AIM-9M Sidewinder: 85.5 kg, 2.87 m, 127 mm; Mk 36 Mod 11 reduced-smoke motor
/// (about 5 s), Mach 2.5; WDU-17/B 9.4 kg annular blast-fragmentation warhead
/// on a DSU-15 active optical fuze; all-aspect seeker gimballed to 40 degrees.
pub const AIM9M: Spec = Spec {
    name: "AIM-9M",
    mass: 85.5,
    length: 2.87,
    diameter: 0.127,
    propellant: 25.0,
    motor: [(10100.0, 5.0), (0.0, 0.0)],
    drop: 0.0,
    cd0: 0.42,
    cna: 32.0,
    alpha_max: 22.0 * DEG,
    g_limit: 35.0,
    nav: 4.0,
    gimbal: 40.0 * DEG,
    acquire: 2.5 * DEG,
    fuze: 9.0,
    lethal: 4.0,
    outer: 14.0,
    arm: 0.5,
    life: 40.0,
    min_range: 300.0,
    max_range: 14000.0,
    active: false,
};

/// AIM-120C AMRAAM: 161.5 kg, 3.66 m, 178 mm; boost-sustain motor to Mach 4;
/// WDU-41/B 18 kg blast-fragmentation warhead; inertial midcourse on the jet's
/// track, then its own active radar seeker.
pub const AIM120C: Spec = Spec {
    name: "AIM-120C",
    mass: 161.5,
    length: 3.66,
    diameter: 0.178,
    propellant: 52.0,
    motor: [(24000.0, 2.6), (9800.0, 6.0)],
    drop: 0.35,
    cd0: 0.38,
    cna: 30.0,
    alpha_max: 20.0 * DEG,
    g_limit: 40.0,
    nav: 4.0,
    gimbal: 60.0 * DEG,
    acquire: 20.0 * DEG,
    fuze: 12.0,
    lethal: 6.0,
    outer: 20.0,
    arm: 1.0,
    life: 70.0,
    min_range: 1000.0,
    max_range: 35000.0,
    active: true,
};

impl Spec {
    fn area(&self) -> f64 {
        PI * 0.25 * self.diameter * self.diameter
    }

    /// Thrust and mass flow at a time since ignition.
    fn thrust(&self, t: f64) -> (f64, f64) {
        let total: f64 = self.motor.iter().map(|&(f, s)| f * s).sum();
        let mut t0 = 0.0;
        for &(f, s) in &self.motor {
            if t >= t0 && t < t0 + s {
                return (f, self.propellant * f / total);
            }
            t0 += s;
        }
        (0.0, 0.0)
    }

    /// Wave drag: the subsonic coefficient rises through Mach 1 and falls slowly beyond.
    fn cd(&self, mach: f64) -> f64 {
        let rise = smoothstep(0.85, 1.15, mach) * 0.55;
        let fall = smoothstep(1.2, 3.5, mach) * 0.35;
        self.cd0 * (1.0 + rise - fall)
    }

    /// Damage to a 100-point airframe from a burst at a miss distance.
    pub fn damage(&self, miss: f64) -> f64 {
        blast_damage(140.0, self.lethal, self.outer, miss)
    }
}

/// What happened to a missile in one step.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fate {
    Flying,
    /// The proximity fuze fired at this point and miss distance.
    Burst(V3, f64),
    Ground(V3),
    /// Self-destruct, or too slow to fly on.
    Spent(V3),
}

#[derive(Clone, Debug)]
pub struct Missile {
    pub spec: &'static Spec,
    pub pos: V3,
    pub vel: V3,
    pub mass: f64,
    pub age: f64,
    pub from: Who,
    pub target: Option<Who>,
    /// The seeker still sees its target.
    pub tracking: bool,
    /// Lateral acceleration flown in the last step, m/s^2.
    pub accel: f64,
    /// Closest the missile has been to its target, m.
    pub best: f64,
}

impl Missile {
    pub fn launch(spec: &'static Spec, pos: V3, vel: V3, from: Who, target: Option<Who>) -> Missile {
        Missile { spec, pos, vel, mass: spec.mass, age: 0.0, from, target, tracking: target.is_some(), accel: 0.0, best: f64::INFINITY }
    }

    /// Motor burning (for the smoke trail and the seeker's heat).
    pub fn burning(&self) -> bool {
        self.spec.thrust(self.age - self.spec.drop).0 > 0.0
    }

    /// One step against a target's present position and velocity.
    /// Proportional navigation turns the flight path at N times the line-of-
    /// sight rate times the closing speed, plus what holds it up against
    /// gravity; the airframe's angle-of-attack and structural limits cap it,
    /// and the induced drag of pulling bleeds speed like on the real missile.
    pub fn step(&mut self, dt: f64, target: Option<(V3, V3)>, floor: &dyn Fn(f64, f64) -> f64) -> Fate {
        let s = self.spec;
        let p0 = self.pos;
        self.age += dt;
        let atm = atmosphere_at(self.pos.y);
        let speed = self.vel.len().max(1.0);
        let dir = self.vel / speed;
        let q = 0.5 * atm.rho * speed * speed;
        let qs = q * s.area();
        let burn = self.age - s.drop;
        let (thrust, flow) = if burn >= 0.0 { s.thrust(burn) } else { (0.0, 0.0) };
        self.mass = (self.mass - flow * dt).max(s.mass - s.propellant);

        let mut lateral = V3::ZERO;
        let guided = burn >= 0.0 && self.age > 0.15;
        let seen = if self.tracking { target } else { None };
        if let (Some((tp, tv)), true) = (seen, guided) {
            let r = tp - self.pos;
            let range = r.len().max(1.0);
            if r.dot(dir) / range < s.gimbal.cos() {
                self.tracking = false;
            } else {
                let vr = tv - self.vel;
                let omega = r.cross(vr) / (range * range);
                let closing = (-r.dot(vr) / range).max(50.0);
                lateral = omega.cross(r / range) * (s.nav * closing);
            }
        }
        if guided {
            // Lift that keeps the flight path from sagging under gravity.
            lateral += V3::Y * G0 - dir * (dir.y * G0);
        }
        lateral -= dir * lateral.dot(dir);
        let limit = (s.g_limit * G0).min(qs * s.cna * s.alpha_max / self.mass);
        if lateral.len() > limit {
            lateral = lateral.norm() * limit;
        }
        self.accel = lateral.len();
        // Normal force N needs an angle of attack N / (q S CNa), which tilts it
        // back by the same angle: induced drag N alpha.
        let normal = self.mass * self.accel;
        let induced = if qs > 1.0 { normal * normal / (qs * s.cna) } else { 0.0 };
        let drag = qs * s.cd(speed / atm.sound) + induced;
        let accel = dir * ((thrust - drag) / self.mass) + lateral - V3::Y * G0;
        self.vel += accel * dt;
        self.pos += self.vel * dt;

        if let Some((tp, tv)) = target.filter(|_| self.age >= s.arm) {
            // The proximity fuze works whether or not the seeker still sees:
            // closest approach during the step, relative to the moving target.
            let (t, miss) = closest(p0 - (tp - tv * dt), self.pos - tp);
            self.best = self.best.min(miss);
            // It fires at the closest point, once the range starts opening.
            if miss < s.fuze && (t < 1.0 || miss < 1.0) {
                return Fate::Burst(p0.lerp(self.pos, t), miss);
            }
        }
        if let Some(p) = ground_crossing(p0, self.pos, floor) {
            return Fate::Ground(p);
        }
        if self.age > s.life || (burn > 2.0 && thrust == 0.0 && self.vel.len() < 160.0) {
            return Fate::Spent(self.pos);
        }
        Fate::Flying
    }
}

/// Launch-zone check: flies a missile out against the target's straight-line
/// path at a coarse step and says whether the fuze would fire (and when).
/// The HUD's IN RNG cue and the AI's shot decision both use it.
pub fn fly_out(spec: &'static Spec, pos: V3, vel: V3, tp: V3, tv: V3, floor: &dyn Fn(f64, f64) -> f64) -> Option<f64> {
    let range = (tp - pos).len();
    if range < spec.min_range || range > spec.max_range * 1.2 {
        return None;
    }
    let mut m = Missile::launch(spec, pos, vel, Who::Player, Some(Who::Bandit(0)));
    let dt = 0.05;
    let mut t = 0.0;
    while t < spec.life {
        t += dt;
        let target = (tp + tv * t, tv);
        match m.step(dt, Some(target), floor) {
            Fate::Flying => {}
            Fate::Burst(_, miss) => return (miss < spec.lethal * 1.5).then_some(t),
            _ => return None,
        }
        if !m.tracking {
            return None;
        }
    }
    None
}

// ================================================================ bombs
/// Mk-82 LDGP, the 500 lb general-purpose bomb: 241 kg with its fuze,
/// 2.21 m, 273 mm, 87 kg of Tritonal.
pub const MK82_MASS: f64 = 241.0;
/// Drag factor at sea level, Cd 0.22 on the 0.0585 m^2 cross-section.
const MK82_K: f64 = 0.5 * 1.225 * 0.22 * 0.0585 / MK82_MASS;
/// MAU-12 rack ejection, m/s downward.
pub const BOMB_EJECT: f64 = 1.5;
/// Fuze arming delay: a bomb that lands sooner is a dud.
pub const BOMB_ARM: f64 = 2.0;
/// Blast and fragments against vehicles and structures, m.
pub const BOMB_LETHAL: f64 = 8.0;
pub const BOMB_OUTER: f64 = 45.0;
pub const BOMB_DAMAGE: f64 = 400.0;

#[derive(Clone, Copy, Debug)]
pub struct Bomb {
    pub pos: V3,
    pub vel: V3,
    pub age: f64,
}

impl Bomb {
    /// Released from a station (body axes) of a jet: its velocity plus the
    /// ejector's push.
    pub fn release(a: &Aircraft, station: V3) -> Bomb {
        let ax = a.q.axes();
        Bomb { pos: a.pos + a.q.rotate(station), vel: a.vel - ax.up * BOMB_EJECT, age: 0.0 }
    }

    /// One fixed step; returns the impact point once it reaches the ground.
    pub fn step(&mut self, dt: f64, floor: &dyn Fn(f64, f64) -> f64) -> Option<V3> {
        let p0 = self.pos;
        self.age += dt;
        ballistic(&mut self.pos, &mut self.vel, MK82_K, dt);
        ground_crossing(p0, self.pos, floor)
    }

    pub fn armed(&self) -> bool {
        self.age >= BOMB_ARM
    }
}

/// Continuously computed impact point: the bomb a release now would drop,
/// flown with the same step and ballistics as a real one. Returns the impact
/// point and the time of fall.
pub fn ccip(a: &Aircraft, station: V3, floor: &dyn Fn(f64, f64) -> f64) -> Option<(V3, f64)> {
    let mut b = Bomb::release(a, station);
    while b.age < 60.0 {
        if let Some(p) = b.step(FIXED_DT, floor) {
            return Some((p, b.age));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{trim, Flcs, Input, Mission};

    fn flat(_x: f64, _z: f64) -> f64 {
        0.0
    }

    fn jet(alt: f64, speed: f64) -> Aircraft {
        let mut a = Aircraft::new(&Mission::new());
        let t = trim(alt, speed, a.mass);
        a.pos = v3(0.0, alt, 0.0);
        a.vel = v3(0.0, 0.0, -speed);
        a.q = Quat::axis_angle(V3::X, t.alpha);
        let mut f = Flcs::default();
        f.alpha_f = t.alpha * RAD;
        f.el = t.el;
        a.flcs = f;
        a.power = t.power;
        a.throttle = t.throttle;
        a.update(0.0, &Input::default());
        a
    }

    /// 20 mm ballistics at sea level: about 1.3 s to 1,000 m with roughly
    /// 600 m/s left, and the round falls under gravity on the way.
    #[test]
    fn rounds_slow_down_and_drop() {
        let mut r = Round::new(v3(0.0, 1000.0, 0.0), v3(0.0, 0.0, -MUZZLE_VELOCITY), false, Who::Player, 6.0);
        let mut t = 0.0;
        while -r.pos.z < 1000.0 {
            r.step(FIXED_DT);
            t += FIXED_DT;
        }
        assert!(t > 1.15 && t < 1.45, "time of flight {t:.2} s");
        let v = r.vel.len();
        assert!(v > 520.0 && v < 700.0, "retained {v:.0} m/s");
        let drop = 1000.0 - r.pos.y;
        assert!(drop > 5.0 && drop < 0.5 * G0 * t * t + 0.5, "drop {drop:.1} m");
        // Thin air at 9 km lets the round keep more of its speed.
        let mut high = Round::new(v3(0.0, 9000.0, 0.0), v3(0.0, 0.0, -MUZZLE_VELOCITY), false, Who::Player, 6.0);
        for _ in 0..(t / FIXED_DT) as usize {
            high.step(FIXED_DT);
        }
        assert!(high.vel.len() > v + 150.0);
    }

    #[test]
    fn swept_hit_tests() {
        let a = jet(3000.0, 220.0);
        // Through the fuselage, nose to tail, from 20 m ahead to 20 m behind.
        assert!(hits_airframe(&a, v3(0.0, 0.0, -20.0), v3(0.0, 0.0, 20.0)).is_some());
        // Passing 6 m above misses.
        assert!(hits_airframe(&a, v3(0.0, 6.0, -20.0), v3(0.0, 6.0, 20.0)).is_none());
        // Box: enters at a third of the way.
        let t = hits_box(v3(-15.0, 0.0, 0.0), v3(15.0, 0.0, 0.0), v3(5.0, 2.0, 3.0)).unwrap();
        assert!((t - 1.0 / 3.0).abs() < 1e-9);
        assert!(hits_box(v3(-15.0, 3.0, 0.0), v3(15.0, 3.0, 0.0), v3(5.0, 2.0, 3.0)).is_none());
    }

    #[test]
    fn blast_damage_falls_off_with_distance() {
        assert_eq!(blast_damage(400.0, 8.0, 45.0, 3.0), 400.0);
        assert!((blast_damage(400.0, 8.0, 45.0, 26.5) - 200.0).abs() < 1e-9);
        assert_eq!(blast_damage(400.0, 8.0, 45.0, 60.0), 0.0);
        assert!(AIM9M.damage(2.0) >= 100.0, "a close burst kills");
        assert!(AIM9M.damage(10.0) > 0.0 && AIM9M.damage(10.0) < 60.0, "a near miss damages");
        assert_eq!(AIM120C.damage(30.0), 0.0);
    }

    /// Both missiles hit a target that flies straight and level, from behind
    /// and across, and the motor and drag give the published speeds.
    #[test]
    fn guidance_hits_a_non_manoeuvring_target() {
        for spec in [&AIM9M, &AIM120C] {
            for (offset, tv, range) in [(v3(0.0, 0.0, -1.0), v3(0.0, 0.0, -230.0), 2500.0), (v3(0.6, 0.0, -0.8), v3(-240.0, 0.0, 0.0), 4000.0), (v3(-0.3, 0.1, -1.0), v3(150.0, 0.0, 180.0), 6000.0)] {
                let tp0 = v3(0.0, 5000.0, 0.0) + offset.norm() * range;
                let mut m = Missile::launch(spec, v3(0.0, 5000.0, 0.0), v3(0.0, 0.0, -250.0), Who::Player, Some(Who::Bandit(0)));
                let mut t = 0.0;
                let mut peak: f64 = 0.0;
                let fate = loop {
                    t += FIXED_DT;
                    let f = m.step(FIXED_DT, Some((tp0 + tv * t, tv)), &flat);
                    peak = peak.max(m.vel.len());
                    if f != Fate::Flying || t > 60.0 {
                        break f;
                    }
                };
                match fate {
                    Fate::Burst(_, miss) => assert!(miss < spec.lethal, "{} {range} m: miss {miss:.1} m", spec.name),
                    other => panic!("{} {range} m: {other:?} after {t:.1} s", spec.name),
                }
                let mach = peak / atmosphere_at(5000.0).sound;
                assert!(mach > 2.0 && mach < 4.6, "{} peak Mach {mach:.2}", spec.name);
            }
        }
    }

    /// A Sidewinder fired at a target far beyond its kinematic range runs out
    /// of energy, and one fired with the target behind the seeker loses it.
    /// Low down, a tail chase of 10 km is AMRAAM range, not Sidewinder range.
    #[test]
    fn missiles_have_finite_envelopes() {
        let pos = v3(0.0, 1000.0, 0.0);
        let vel = v3(0.0, 0.0, -230.0);
        let away = v3(0.0, 0.0, -260.0);
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -3000.0), away, &flat).is_some());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -16000.0), away, &flat).is_none());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, 3000.0), v3(0.0, 0.0, 230.0), &flat).is_none());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -10000.0), away, &flat).is_none());
        assert!(fly_out(&AIM120C, pos, vel, pos + v3(0.0, 0.0, -10000.0), away, &flat).is_some());
        // Inside minimum range there is no shot.
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -150.0), away, &flat).is_none());
    }

    /// The CCIP pipper is the real bomb's impact point: a bomb released at
    /// that moment and flown by the game lands on it.
    #[test]
    fn ccip_matches_the_simulated_impact() {
        let ground = |x: f64, z: f64| 300.0 + 20.0 * (x * 0.003).sin() + 0.01 * z;
        for (alt, speed, dive) in [(1500.0, 230.0, 0.0), (1200.0, 250.0, -20.0), (2500.0, 200.0, -35.0)] {
            let mut a = jet(alt, speed);
            let d = dive * DEG;
            a.vel = v3(0.0, d.sin(), -d.cos()) * speed;
            a.q = Quat::axis_angle(V3::X, d + a.alpha);
            let station = v3(3.5, -0.2, 2.3);
            let (p, tof) = ccip(&a, station, &ground).expect("no impact");
            let mut b = Bomb::release(&a, station);
            let hit = loop {
                if let Some(h) = b.step(FIXED_DT, &ground) {
                    break h;
                }
            };
            assert!((hit - p).len() < 0.01, "{alt} m {dive} deg: {:.2} m apart", (hit - p).len());
            assert!(tof > 3.0 && tof < 25.0 && b.armed());
            // Drag makes it fall short of a vacuum trajectory.
            let vac = (a.vel.y + (a.vel.y * a.vel.y + 2.0 * G0 * (alt - p.y)).sqrt()) / G0;
            assert!(-p.z < -a.vel.z * vac, "{alt} m: impact {:.0} m ahead", -p.z);
        }
    }

    /// The LCOS pipper on the virtual stream: rounds fired at a target held on
    /// it, flying straight with the shooter, hit.
    #[test]
    fn rounds_fired_at_the_pipper_hit() {
        let mut a = jet(3000.0, 230.0);
        let mut sight = GunSight::default();
        for _ in 0..240 {
            a.update(FIXED_DT, &Input::default());
            sight.step(FIXED_DT, &a);
        }
        let (gun, _) = muzzle(&a);
        let range = 600.0;
        let pipper = sight.at_range(gun, range).expect("stream reaches 600 m");
        let mut target = a.clone();
        target.pos = pipper;
        let mut rounds: Vec<Round> = (0..20).map(|_| Round::new(gun, muzzle(&a).1, false, Who::Player, 6.0)).collect();
        let mut hits = 0;
        for _ in 0..240 {
            target.pos += target.vel * FIXED_DT;
            rounds.retain_mut(|r| {
                r.step(FIXED_DT);
                let t0 = target.pos - target.vel * FIXED_DT;
                if hits_airframe(&target, r.prev - t0, r.pos - target.pos).is_some() {
                    hits += 1;
                    return false;
                }
                true
            });
        }
        assert_eq!(hits, 20);
    }
}
