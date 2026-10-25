//! Stores of the F-16C Block 50 and the MiG-29S 9.13S and their physics,
//! simplified but with the published figures: the M61A1 and GSh-30-1 guns and
//! their rounds, the air-to-air missiles of both jets by family (infrared,
//! semi-active and active radar) flying proportional navigation within
//! their seeker, motor, g and fuze limits, and the Mk-82, FAB-250M-62 and
//! FAB-500M-62 low-drag bombs. The sights that aim them live here too: the
//! EEGS-style funnel of rounds already in the air, the LCOS pipper on that
//! stream, the CCIP impact prediction and the missile launch-zone fly-out.
//! Everything integrates at the fixed 120 Hz step, in world metres.
use crate::math::*;
use crate::sim::{atmosphere_at, Aircraft, Type, FIXED_DT, G0};
use std::collections::VecDeque;

/// Anything that can shoot or be shot at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    Player,
    Bandit(usize),
    Ground(usize),
}

// ================================================================ guns
/// An internal gun and its ammunition.
#[derive(Debug)]
pub struct Gun {
    pub name: &'static str,
    /// Calibre, for the HUD.
    pub calibre: &'static str,
    pub rounds: u32,
    /// Rounds per second.
    pub rate: f64,
    /// Muzzle velocity, m/s.
    pub velocity: f64,
    /// Drag factor k = rho Cd A / 2m of the projectile at sea level.
    pub k: f64,
    /// Damage per hit to the system it strikes (`damage::Hull::strike`), by
    /// the round's explosive charge: 8 for the 20 mm PGU-28/B, 24 for the
    /// 30 mm OFZ with about three times the high explosive.
    pub damage: f64,
    /// Mass the jet loses per round, kg.
    pub expended: f64,
    /// Muzzle in body axes.
    pub port: V3,
}

/// M61A1 Vulcan: 511 linkless 20 mm rounds in the drum behind the cockpit,
/// 6,000 rounds per minute, PGU-28/B at 1,036 m/s from the port in the left
/// strake, abeam the seat. The 100 g projectile (Cd 0.28 supersonic) keeps
/// about 600 m/s at 1,000 m, reached in 1.3 s. The jet loses the projectile
/// and its propellant per round; the linkless feed carries the cases back.
pub const M61A1: Gun = Gun {
    name: "M61A1 VULCAN",
    calibre: "20MM",
    rounds: 511,
    rate: 100.0,
    velocity: 1036.0,
    k: 0.5 * 1.225 * 0.28 * 3.14e-4 / 0.10,
    damage: 8.0,
    expended: 0.14,
    port: v3(-0.72, 0.52, -3.40),
};

/// GSh-30-1 (9A-4071K): 150 rounds of 30 x 165 mm in the port LERX root,
/// 1,500 to 1,800 rounds per minute, the 390 g OFZ high-explosive round at
/// 860 m/s. Heavier and slower than the 20 mm round, it carries its speed
/// further, and each hit does about three times the damage.
pub const GSH301: Gun = Gun {
    name: "GSH-30-1",
    calibre: "30MM",
    rounds: 150,
    rate: 27.5,
    velocity: 860.0,
    k: 0.5 * 1.225 * 0.28 * 7.07e-4 / 0.39,
    damage: 24.0,
    expended: 0.39 + 0.16,
    port: v3(-0.63, 0.69, -3.40),
};

/// Hornet nose-mounted M61A1, 578 rounds; Su-27 starboard LERX GSh-301.
pub const M61_HORNET: Gun = Gun { rounds: 578, port: v3(0.0,0.65,-6.9), ..M61A1 };
pub const GSH_FLANKER: Gun = Gun { port: v3(1.45,0.7,-3.6), ..GSH301 };

/// The gun of an airframe.
pub fn gun_of(kind: Type) -> &'static Gun {
    match kind {
        Type::F16 => &M61A1,
        Type::Mig29 => &GSH301,
        Type::F18 => &M61_HORNET,
        Type::Su27 => &GSH_FLANKER,
    }
}

/// The gun line points through the HUD gun cross, 55 mrad above the boresight.
pub const GUN_ELEVATION: f64 = 55.0 / 650.0;
/// One round in five is a tracer.
pub const TRACER_EVERY: u32 = 5;
/// Rounds stop being tracked after this long, about 2.5 km down range.
pub const ROUND_LIFE: f64 = 3.2;

/// Gun line in body axes.
pub fn gun_bore() -> V3 {
    v3(0.0, GUN_ELEVATION.sin(), -GUN_ELEVATION.cos())
}

/// Muzzle position and velocity of a round fired now from a jet's gun.
pub fn muzzle(a: &Aircraft) -> (V3, V3) {
    let g = gun_of(a.kind);
    (a.pos + a.q.rotate(g.port), a.vel + a.q.rotate(gun_bore()) * g.velocity)
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
    /// Drag factor of the projectile.
    pub k: f64,
}

impl Round {
    pub fn new(pos: V3, vel: V3, tracer: bool, from: Who, damage: f64) -> Round {
        Round { pos, vel, prev: pos, age: 0.0, tracer, from, damage, k: M61A1.k }
    }
    /// A round from a gun, with its damage and ballistics.
    pub fn of(gun: &Gun, pos: V3, vel: V3, tracer: bool, from: Who) -> Round {
        Round { damage: gun.damage, k: gun.k, ..Round::new(pos, vel, tracer, from, 0.0) }
    }
    pub fn step(&mut self, dt: f64) {
        self.prev = self.pos;
        self.age += dt;
        ballistic(&mut self.pos, &mut self.vel, self.k, dt);
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
/// and the end of the step. The game hits the systems inside instead
/// (`damage::first_hit`); the tests aim with this coarser shape.
#[cfg(test)]
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
            ballistic(p, v, gun_of(a.kind).k, dt);
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
// The air-to-air missiles by what guides them, each family with the F-16's
// and the MiG-29's:
//
//   infrared, short range (Fox 2)   AIM-9M, AIM-9X        R-60M, R-73
//   infrared, medium range (Fox 2)  none                  R-27T
//   semi-active radar (Fox 1)       AIM-7M                R-27R
//   active radar (Fox 3)            AIM-120C, AIM-120D    R-77
//
// Which airframe carries what on which pylon is in combat.rs
// (`pylon_choices`, `arsenal`); docs/WEAPONS.md has the launch rules.

/// What a missile homes on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Seeker {
    /// The target's heat; decoyed by flares.
    Infrared,
    /// The launcher's radar energy reflected off the target, so the launcher
    /// has to keep the target locked to the end; decoyed by chaff.
    SemiActive,
    /// Its own radar, after midcourse on the launcher's data; decoyed by chaff.
    Active,
}

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
    pub seeker: Seeker,
    /// Lock-on after launch: fired with no seeker lock it can still find
    /// its target in flight (the AIM-9X Block II on the radar's cue, the
    /// AIM-120 and R-77 with their own radar). Without it, lock-on before
    /// launch only: no lock, no guidance.
    pub loal: bool,
    /// Midcourse updates from the launcher's lock over a datalink, so the
    /// missile flies to where the target is, not where it was at launch,
    /// until its seeker takes over.
    pub datalink: bool,
    /// Counter-countermeasures, the share of decoys the seeker rejects
    /// (0 none, 1 all), by generation and design: an infrared seeker's
    /// two-colour, kinematic and imaging flare rejection (IRCCM), a radar
    /// seeker's monopulse and Doppler discrimination against chaff (ECCM).
    /// Estimates: the real figures are secret.
    pub ccm: f64,
    /// Brevity call on launch.
    pub call: &'static str,
}

// ------------------------------------------- infrared, short range (Fox 2)
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
    cd0: 0.95,
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
    seeker: Seeker::Infrared,
    loal: false,
    datalink: false,
    ccm: 0.55,
    call: "FOX 2",
};

/// AIM-9X Block II Sidewinder: 85.3 kg, 3.02 m, 127 mm; the AIM-9M's motor
/// and WDU-17/B warhead in a low-drag airframe with small fins and jet-vane
/// thrust vectoring; a staring imaging-infrared seeker that looks 90 degrees
/// off the nose, cued by the JHMCS. Block II adds a datalink and lock-on
/// after launch: fired on the radar's cue with no tone, it acquires in flight.
pub const AIM9X: Spec = Spec {
    name: "AIM-9X",
    mass: 85.3,
    length: 3.02,
    diameter: 0.127,
    propellant: 25.0,
    motor: [(10100.0, 5.0), (0.0, 0.0)],
    drop: 0.0,
    cd0: 0.80,
    cna: 30.0,
    alpha_max: 35.0 * DEG,
    g_limit: 50.0,
    nav: 4.0,
    gimbal: 90.0 * DEG,
    acquire: 2.5 * DEG,
    fuze: 9.0,
    lethal: 4.0,
    outer: 14.0,
    arm: 0.5,
    life: 40.0,
    min_range: 250.0,
    max_range: 16000.0,
    seeker: Seeker::Infrared,
    loal: true,
    datalink: true,
    ccm: 0.85,
    call: "FOX 2",
};

/// R-60M (izdeliye 62M, AA-8 Aphid): 44 kg, 2.09 m, 120 mm; a small single-
/// stage motor to Mach 2.7; 3.5 kg expanding-rod warhead on an optical fuze;
/// a cooled infrared seeker that looks 20 degrees off the nose. A dogfight
/// missile for close in, lock-on before launch only.
pub const R60M: Spec = Spec {
    name: "R-60M",
    mass: 44.0,
    length: 2.09,
    diameter: 0.120,
    propellant: 10.0,
    motor: [(7500.0, 3.0), (0.0, 0.0)],
    drop: 0.0,
    cd0: 0.80,
    cna: 28.0,
    alpha_max: 25.0 * DEG,
    g_limit: 42.0,
    nav: 4.0,
    gimbal: 20.0 * DEG,
    acquire: 2.0 * DEG,
    fuze: 5.0,
    lethal: 2.5,
    outer: 8.0,
    arm: 0.4,
    life: 25.0,
    min_range: 200.0,
    max_range: 8000.0,
    seeker: Seeker::Infrared,
    loal: false,
    datalink: false,
    ccm: 0.15,
    call: "FOX 2",
};

/// R-73 (izdeliye 72, AA-11 Archer): 105 kg, 2.90 m, 170 mm; single-stage
/// motor to Mach 2.5; 7.4 kg expanding-rod warhead on an active radio fuze;
/// the cooled Mayak all-aspect infrared seeker looks 45 degrees off the nose
/// and is cued by the helmet sight, and the gas vanes in the exhaust let it
/// pull hard off the rail. Lock-on before launch only.
pub const R73: Spec = Spec {
    name: "R-73",
    mass: 105.0,
    length: 2.90,
    diameter: 0.170,
    propellant: 38.0,
    motor: [(14000.0, 6.5), (0.0, 0.0)],
    drop: 0.0,
    cd0: 0.70,
    cna: 32.0,
    alpha_max: 25.0 * DEG,
    g_limit: 40.0,
    nav: 4.0,
    gimbal: 45.0 * DEG,
    acquire: 2.5 * DEG,
    fuze: 8.0,
    lethal: 3.5,
    outer: 12.0,
    arm: 0.4,
    life: 40.0,
    min_range: 300.0,
    max_range: 20000.0,
    seeker: Seeker::Infrared,
    loal: false,
    datalink: false,
    ccm: 0.45,
    call: "FOX 2",
};

// ------------------------------------------ infrared, medium range (Fox 2)
/// R-27T (izdeliye 470T, AA-10 Alamo-B): the R-27R's 254 kg, 4.08 m, 230 mm
/// airframe, dual-thrust motor and 39 kg rod warhead with the passive
/// infrared seeker of the R-73's generation. It has to see the target
/// before it leaves the rail, cued by the radar or the IRST; the jet need
/// not keep a lock afterwards, and it gives no radar warning.
pub const R27T: Spec = Spec {
    name: "R-27T",
    mass: 254.0,
    length: 4.08,
    diameter: 0.230,
    propellant: 80.0,
    motor: [(42000.0, 2.5), (12500.0, 6.5)],
    drop: 0.0,
    cd0: 0.48,
    cna: 30.0,
    alpha_max: 20.0 * DEG,
    g_limit: 30.0,
    nav: 3.5,
    gimbal: 50.0 * DEG,
    acquire: 3.0 * DEG,
    fuze: 13.0,
    lethal: 7.0,
    outer: 24.0,
    arm: 1.0,
    life: 80.0,
    min_range: 800.0,
    max_range: 25000.0,
    seeker: Seeker::Infrared,
    loal: false,
    datalink: false,
    ccm: 0.3,
    call: "FOX 2",
};

// ----------------------------------------------- semi-active radar (Fox 1)
/// AIM-7M Sparrow: 231 kg, 3.66 m, 203 mm; Mk 58 boost-sustain motor to
/// Mach 4; 40 kg WAU-17/B annular blast-fragmentation warhead; inertial
/// midcourse, then semi-active homing on the jet's radar, which has to keep
/// the target locked to the end. Ejected from its pylon. The USAF's
/// F-16Cs never adopted it; Sparrow trials on the Block 30 and the export
/// jets that carry it put it on the same stations.
pub const AIM7M: Spec = Spec {
    name: "AIM-7M",
    mass: 231.0,
    length: 3.66,
    diameter: 0.203,
    propellant: 55.0,
    motor: [(30000.0, 2.8), (6000.0, 5.0)],
    drop: 0.3,
    cd0: 0.55,
    cna: 30.0,
    alpha_max: 20.0 * DEG,
    g_limit: 25.0,
    nav: 3.5,
    gimbal: 50.0 * DEG,
    acquire: 5.0 * DEG,
    fuze: 12.0,
    lethal: 7.0,
    outer: 22.0,
    arm: 1.0,
    life: 75.0,
    min_range: 1000.0,
    max_range: 30000.0,
    seeker: Seeker::SemiActive,
    loal: false,
    datalink: false,
    ccm: 0.45,
    call: "FOX 1",
};

/// R-27R (izdeliye 470R, AA-10 Alamo-A): 253 kg, 4.08 m, 230 mm; dual-thrust
/// motor; 39 kg expanding-rod warhead on an active radar fuze; inertial
/// midcourse, then semi-active radar homing on the jet's radar, which
/// has to keep the target locked to the end. Up to 73 km head on at height,
/// much less low down.
pub const R27R: Spec = Spec {
    name: "R-27R",
    mass: 253.0,
    length: 4.08,
    diameter: 0.230,
    propellant: 80.0,
    motor: [(42000.0, 2.5), (12500.0, 6.5)],
    drop: 0.0,
    cd0: 0.48,
    cna: 30.0,
    alpha_max: 20.0 * DEG,
    g_limit: 30.0,
    nav: 3.5,
    gimbal: 50.0 * DEG,
    acquire: 5.0 * DEG,
    fuze: 13.0,
    lethal: 7.0,
    outer: 24.0,
    arm: 1.0,
    life: 80.0,
    min_range: 800.0,
    max_range: 32000.0,
    seeker: Seeker::SemiActive,
    loal: false,
    datalink: false,
    ccm: 0.3,
    call: "FOX 1",
};

// ---------------------------------------------------- active radar (Fox 3)
/// AIM-120C AMRAAM: 161.5 kg, 3.66 m, 178 mm; boost-sustain motor to Mach 4;
/// WDU-41/B 18 kg blast-fragmentation warhead; inertial midcourse on the jet's
/// datalink, then its own active radar seeker. Fired without a lock it goes
/// active off the rail and takes the first target its radar finds.
pub const AIM120C: Spec = Spec {
    name: "AIM-120C",
    mass: 161.5,
    length: 3.66,
    diameter: 0.178,
    propellant: 52.0,
    motor: [(24000.0, 2.6), (9800.0, 6.0)],
    drop: 0.35,
    cd0: 0.5,
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
    life: 90.0,
    min_range: 1000.0,
    max_range: 35000.0,
    seeker: Seeker::Active,
    loal: true,
    datalink: true,
    ccm: 0.6,
    call: "FOX 3",
};

/// AIM-120D AMRAAM: the AIM-120C's airframe and warhead with a longer-
/// burning motor, GPS-aided inertial navigation and a two-way datalink,
/// for about half as much range again.
pub const AIM120D: Spec = Spec {
    name: "AIM-120D",
    mass: 161.5,
    length: 3.66,
    diameter: 0.178,
    propellant: 70.0,
    motor: [(24000.0, 2.6), (9800.0, 13.0)],
    drop: 0.35,
    cd0: 0.5,
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
    life: 120.0,
    min_range: 1000.0,
    max_range: 48000.0,
    seeker: Seeker::Active,
    loal: true,
    datalink: true,
    ccm: 0.75,
    call: "FOX 3",
};

/// R-77 (RVV-AE, izdeliye 170, AA-12 Adder): 175 kg, 3.60 m, 200 mm; a
/// single-stage motor; 22.5 kg rod warhead on a laser fuze; the lattice
/// tail fins hold it at high angles of attack. Inertial midcourse with
/// datalink corrections from the N019M radar, then its own active seeker;
/// close in it is fired active off the rail.
pub const R77: Spec = Spec {
    name: "R-77",
    mass: 175.0,
    length: 3.60,
    diameter: 0.200,
    propellant: 55.0,
    motor: [(26000.0, 3.0), (8000.0, 6.0)],
    drop: 0.3,
    cd0: 0.60,
    cna: 34.0,
    alpha_max: 30.0 * DEG,
    g_limit: 40.0,
    nav: 4.0,
    gimbal: 60.0 * DEG,
    acquire: 20.0 * DEG,
    fuze: 12.0,
    lethal: 6.0,
    outer: 21.0,
    arm: 1.0,
    life: 90.0,
    min_range: 600.0,
    max_range: 33000.0,
    seeker: Seeker::Active,
    loal: true,
    datalink: true,
    ccm: 0.5,
    call: "FOX 3",
};

impl Spec {
    pub fn infrared(&self) -> bool {
        self.seeker == Seeker::Infrared
    }
    pub fn semi_active(&self) -> bool {
        self.seeker == Seeker::SemiActive
    }
    pub fn active(&self) -> bool {
        self.seeker == Seeker::Active
    }

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
    visibility_clock: f64,
    notch_time: f64,
}

impl Missile {
    pub fn launch(spec: &'static Spec, pos: V3, vel: V3, from: Who, target: Option<Who>) -> Missile {
        Missile { spec, pos, vel, mass: spec.mass, age: 0.0, from, target, tracking: target.is_some(), accel: 0.0, best: f64::INFINITY, visibility_clock: 0.0, notch_time: 0.0 }
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
        // The motor figures are sea-level; with less back-pressure on the
        // nozzle exit (about the body's cross-section) thrust grows with height.
        let thrust = if thrust > 0.0 { thrust + (101_325.0 - atm.pressure).max(0.0) * 0.7 * s.area() } else { 0.0 };
        self.mass = (self.mass - flow * dt).max(s.mass - s.propellant);

        let mut lateral = V3::ZERO;
        let guided = burn >= 0.0 && self.age > 0.15;
        // A seeker cannot see through a ridge. Sample at 10 Hz, including
        // midcourse, rather than continuing to use an occluded target's truth.
        self.visibility_clock -= dt;
        if self.tracking && self.visibility_clock <= 0.0 {
            self.visibility_clock = 0.1;
            if target.is_some_and(|(tp, _)| terrain_masked(self.pos, tp, floor)) {
                self.tracking = false;
            }
        }
        let seen = if self.tracking { target } else { None };
        if let (Some((tp, tv)), true) = (seen, guided) {
            let r = tp - self.pos;
            let range = r.len().max(1.0);
            // Past the seeker's look limit a missile loses its target, unless
            // a datalink still steers it there early in its flight (a shot
            // over the shoulder, or at a target off the nose).
            let midcourse = s.datalink && self.age < 8.0;
            // Simplified look-down Doppler rejection: a sustained beam at
            // low altitude merges with clutter. Better ECCM requires a longer,
            // tighter notch; there is no instant guaranteed dodge.
            let radial = tv.dot(r / range).abs();
            let clutter = tp.y < self.pos.y + 100.0 && tp.y - floor(tp.x, tp.z) < 2500.0;
            let notching = !s.infrared() && clutter && radial < mix(45.0, 22.0, s.ccm)
                && (!s.active() || range < 15000.0);
            self.notch_time = if notching { self.notch_time + dt } else { (self.notch_time - dt * 2.0).max(0.0) };
            if r.dot(dir) / range < s.gimbal.cos() && !midcourse
                || self.notch_time > mix(0.9, 2.4, s.ccm) {
                self.tracking = false;
            } else {
                let vr = tv - self.vel;
                let omega = r.cross(vr) / (range * range);
                let closing = (-r.dot(vr) / range).max(0.0);
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
        if self.age > s.life || (burn > 2.0 && thrust == 0.0 && self.vel.len() < MIN_MACH * atm.sound) {
            return Fate::Spent(self.pos);
        }
        Fate::Flying
    }
}

/// Terrain between seeker and target, sampled separately from the missile's
/// own ground collision. Both radar and infrared require line of sight.
fn terrain_masked(a: V3, b: V3, floor: &dyn Fn(f64, f64) -> f64) -> bool {
    let n = (((b - a).len() / 180.0).ceil() as usize).clamp(4, 96);
    (1..n).any(|k| {
        let p = a.lerp(b, k as f64 / n as f64);
        p.y <= floor(p.x, p.z) + 3.0
    })
}

/// Below this Mach number after burnout a missile can no longer pull the
/// g to stay on a manoeuvring target and is written off, as launch-zone
/// tables do; it is what makes the published ranges shrink low down.
const MIN_MACH: f64 = 1.0;

/// Countermeasure load, chaff and flare cartridges: the F-16's ALE-47, and
/// the MiG-29's two 30-round BVP-30-26M dispensers in the fin-root
/// extensions with PPR-26 chaff and PPI-26 flares, the same count.
pub const CHAFF: u32 = 30;
pub const FLARES: u32 = 30;

/// The dispenser's name.
pub fn dispenser_of(kind: Type) -> &'static str {
    match kind {
        Type::F16 => "ALE-47",
        Type::Mig29 => "BVP-30-26M",
        Type::F18 => "AN/ALE-39",
        Type::Su27 => "APP-50",
    }
}

/// Chance that one countermeasure program (a flare and a chaff bundle)
/// pulls a missile's seeker off its target, before the seeker's own
/// rejection (`Spec::ccm`) throws most of that away.
///
/// Flares only fool an infrared seeker. They work best where the target is
/// faint: from ahead or abeam, with the hot tailpipe hidden, and least
/// against an afterburner's plume. Chaff only fools a radar seeker, and works
/// best with the target on the beam, where its Doppler is close to the
/// still-standing chaff cloud's. Nothing works on a missile still far out,
/// which has not yet closed on the decoy's position.
pub fn decoy_chance(spec: &Spec, missile: V3, target: V3, target_vel: V3, burner: bool) -> f64 {
    let r = target - missile;
    let range = r.len();
    if range > 6000.0 {
        return 0.0;
    }
    let near = 1.0 - smoothstep(2500.0, 6000.0, range);
    let speed = target_vel.len().max(1.0);
    // 1 flying straight away from the missile, -1 straight at it.
    let away = r.dot(target_vel) / (range.max(1.0) * speed);
    let raw = if spec.infrared() {
        let tail = away.max(0.0);
        0.8 * (1.0 - 0.4 * tail) * if burner { 0.6 } else { 1.0 }
    } else {
        let beam = 1.0 - away.abs();
        0.2 + 1.2 * beam * beam
    };
    (raw * (1.0 - spec.ccm)).min(0.9) * near
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
/// A free-fall bomb.
#[derive(Debug)]
pub struct BombSpec {
    pub name: &'static str,
    pub mass: f64,
    pub length: f64,
    pub diameter: f64,
    /// Drag factor at sea level, k = rho Cd A / 2m.
    pub k: f64,
    /// Blast and fragments against vehicles and structures: damage inside
    /// the lethal radius, falling to nothing at the outer one, m.
    pub damage: f64,
    pub lethal: f64,
    pub outer: f64,
}

/// Mk-82 LDGP, the 500 lb general-purpose bomb: 241 kg with its fuze,
/// 2.21 m, 273 mm, 87 kg of Tritonal; Cd 0.22 on the 0.0585 m^2 section.
pub const MK82: BombSpec = BombSpec {
    name: "MK-82",
    mass: 241.0,
    length: 2.21,
    diameter: 0.273,
    k: 0.5 * 1.225 * 0.22 * 0.0585 / 241.0,
    damage: BOMB_DAMAGE,
    lethal: BOMB_LETHAL,
    outer: BOMB_OUTER,
};
#[cfg(test)]
pub const MK82_MASS: f64 = MK82.mass;

/// FAB-250M-62, the 1962 low-drag 250 kg high-explosive bomb for external
/// carriage: 250 kg, 1.92 m, 300 mm, 100 kg of TNT, about the Mk-82's charge.
pub const FAB250: BombSpec = BombSpec {
    name: "FAB-250",
    mass: 250.0,
    length: 1.92,
    diameter: 0.300,
    k: 0.5 * 1.225 * 0.24 * 0.0707 / 250.0,
    damage: 400.0,
    lethal: 8.0,
    outer: 45.0,
};

/// FAB-500M-62: 497 kg, 2.47 m, 400 mm, about 200 kg of TNT. Twice the
/// charge of the FAB-250 reaches a quarter further (blast radius goes with
/// the cube root of the charge).
pub const FAB500: BombSpec = BombSpec {
    name: "FAB-500",
    mass: 497.0,
    length: 2.47,
    diameter: 0.400,
    k: 0.5 * 1.225 * 0.24 * 0.1257 / 497.0,
    damage: 500.0,
    lethal: 10.0,
    outer: 57.0,
};

/// Rack ejection, m/s downward.
pub const BOMB_EJECT: f64 = 1.5;
/// Fuze arming delay: a bomb that lands sooner is a dud.
pub const BOMB_ARM: f64 = 2.0;
/// The Mk-82's blast and fragments against vehicles and structures, m.
pub const BOMB_LETHAL: f64 = 8.0;
pub const BOMB_OUTER: f64 = 45.0;
pub const BOMB_DAMAGE: f64 = 400.0;

#[derive(Clone, Copy, Debug)]
pub struct Bomb {
    pub from: Who,
    pub spec: &'static BombSpec,
    pub pos: V3,
    pub vel: V3,
    pub age: f64,
}

impl Bomb {
    /// Released from a station (body axes) of a jet: its velocity plus the
    /// ejector's push.
    pub fn release(spec: &'static BombSpec, a: &Aircraft, station: V3) -> Bomb {
        let ax = a.q.axes();
        Bomb { from: Who::Player, spec, pos: a.pos + a.q.rotate(station), vel: a.vel - ax.up * BOMB_EJECT, age: 0.0 }
    }

    /// One fixed step; returns the impact point once it reaches the ground.
    pub fn step(&mut self, dt: f64, floor: &dyn Fn(f64, f64) -> f64) -> Option<V3> {
        let p0 = self.pos;
        self.age += dt;
        ballistic(&mut self.pos, &mut self.vel, self.spec.k, dt);
        ground_crossing(p0, self.pos, floor)
    }

    pub fn armed(&self) -> bool {
        self.age >= BOMB_ARM
    }
}

/// Continuously computed impact point: the bomb a release now would drop,
/// flown with the same step and ballistics as a real one. Returns the impact
/// point and the time of fall.
pub fn ccip(spec: &'static BombSpec, a: &Aircraft, station: V3, floor: &dyn Fn(f64, f64) -> f64) -> Option<(V3, f64)> {
    let mut b = Bomb::release(spec, a, station);
    while b.age < 60.0 {
        if let Some(p) = b.step(FIXED_DT, floor) {
            return Some((p, b.age));
        }
    }
    None
}

/// Strafe pipper: where a round fired now meets the ground, flown with the
/// gun's own ballistics, and its time of flight. Nothing past STRF_TOF,
/// beyond any useful strafing range.
pub fn gun_ccip(a: &Aircraft, floor: &dyn Fn(f64, f64) -> f64) -> Option<(V3, f64)> {
    let (mut p, mut v) = muzzle(a);
    let k = gun_of(a.kind).k;
    let dt = 0.02;
    let mut t = 0.0;
    while t < STRF_TOF {
        let p0 = p;
        ballistic(&mut p, &mut v, k, dt);
        t += dt;
        if let Some(hit) = ground_crossing(p0, p, floor) {
            return Some((hit, t));
        }
    }
    None
}

/// Longest time of flight the strafe pipper looks ahead, s.
pub const STRF_TOF: f64 = 4.0;

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
        let mut r = Round::new(v3(0.0, 1000.0, 0.0), v3(0.0, 0.0, -M61A1.velocity), false, Who::Player, 6.0);
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
        let mut high = Round::new(v3(0.0, 9000.0, 0.0), v3(0.0, 0.0, -M61A1.velocity), false, Who::Player, 6.0);
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
        for spec in [&AIM9M, &AIM9X, &R60M, &R73, &R27T, &AIM7M, &R27R, &AIM120C, &AIM120D, &R77] {
            for (offset, tv, range) in [(v3(0.0, 0.0, -1.0), v3(0.0, 0.0, -230.0), 2500.0), (v3(0.6, 0.0, -0.8), v3(-240.0, 0.0, 0.0), 4000.0), (v3(-0.3, 0.1, -1.0), v3(150.0, 0.0, 180.0), 6000.0)] {
                // The R-60M's seeker looks only 20 degrees off the nose.
                if (-offset.norm().z).acos() > spec.gimbal {
                    continue;
                }
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
    /// Low down, a tail chase of 7 km is AMRAAM range, not Sidewinder range.
    #[test]
    fn missiles_have_finite_envelopes() {
        let pos = v3(0.0, 1000.0, 0.0);
        let vel = v3(0.0, 0.0, -230.0);
        let away = v3(0.0, 0.0, -260.0);
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -3000.0), away, &flat).is_some());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -16000.0), away, &flat).is_none());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, 3000.0), v3(0.0, 0.0, 230.0), &flat).is_none());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -7000.0), away, &flat).is_none());
        assert!(fly_out(&AIM120C, pos, vel, pos + v3(0.0, 0.0, -7000.0), away, &flat).is_some());
        // Head on, low down, the Sidewinder reaches about 12 km and the
        // AMRAAM about 25 km (published launch envelopes).
        let toward = v3(0.0, 0.0, 260.0);
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -10000.0), toward, &flat).is_some());
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -15000.0), toward, &flat).is_none());
        assert!(fly_out(&AIM120C, pos, vel, pos + v3(0.0, 0.0, -20000.0), toward, &flat).is_some());
        assert!(fly_out(&AIM120C, pos, vel, pos + v3(0.0, 0.0, -32000.0), toward, &flat).is_none());
        // The R-73 outranges the Sidewinder head on; the R-27R reaches
        // AMRAAM ranges low down, but not 30 km.
        assert!(fly_out(&R73, pos, vel, pos + v3(0.0, 0.0, -12000.0), toward, &flat).is_some());
        assert!(fly_out(&R73, pos, vel, pos + v3(0.0, 0.0, -20000.0), toward, &flat).is_none());
        assert!(fly_out(&R73, pos, vel, pos + v3(0.0, 0.0, -3000.0), away, &flat).is_some());
        assert!(fly_out(&R27R, pos, vel, pos + v3(0.0, 0.0, -20000.0), toward, &flat).is_some());
        assert!(fly_out(&R27R, pos, vel, pos + v3(0.0, 0.0, -34000.0), toward, &flat).is_none());
        // Inside minimum range there is no shot.
        assert!(fly_out(&AIM9M, pos, vel, pos + v3(0.0, 0.0, -150.0), away, &flat).is_none());
    }

    /// Head on and low down, each missile reaches about as far against its
    /// rivals as the published ranges say: the AIM-9X past the AIM-9M, the
    /// R-60M short of both, the AIM-120D half again as far as the C, the
    /// R-77 and the Sparrow short of the AMRAAM.
    #[test]
    fn ranges_rank_as_published() {
        let pos = v3(0.0, 1000.0, 0.0);
        let vel = v3(0.0, 0.0, -230.0);
        let reach = |spec: &'static Spec| {
            (1..120).map(|k| k as f64 * 500.0).filter(|&r| fly_out(spec, pos, vel, pos + v3(0.0, 0.0, -r), v3(0.0, 0.0, 260.0), &flat).is_some()).fold(0.0, f64::max)
        };
        let (m9, x9, r60) = (reach(&AIM9M), reach(&AIM9X), reach(&R60M));
        assert!(x9 > m9 && r60 < m9, "AIM-9X {x9}, AIM-9M {m9}, R-60M {r60}");
        let (c, d, r77, sparrow) = (reach(&AIM120C), reach(&AIM120D), reach(&R77), reach(&AIM7M));
        assert!(d > c * 1.25 && r77 < c && sparrow < c, "AIM-120C {c}, D {d}, R-77 {r77}, AIM-7M {sparrow}");
        assert!((reach(&R27T) - reach(&R27R)).abs() <= 500.0, "the R-27s share a motor");
    }

    /// Flares only fool heat seekers and chaff only radar ones; chaff works
    /// best on the beam, and nothing works far from the missile.
    #[test]
    fn countermeasures_fool_their_own_seeker_type() {
        let m = v3(0.0, 3000.0, 2000.0);
        let t = v3(0.0, 3000.0, 0.0);
        let nose_on = v3(0.0, 0.0, 250.0);
        let beam = v3(250.0, 0.0, 0.0);
        let tail = v3(0.0, 0.0, -250.0);
        assert!(decoy_chance(&AIM9M, m, t, nose_on, false) > 0.2);
        let (head, side) = (decoy_chance(&AIM120C, m, t, nose_on, false), decoy_chance(&AIM120C, m, t, beam, false));
        assert!(side > 0.5 && head < 0.15, "beam {side:.2}, nose-on {head:.2}");
        assert_eq!(decoy_chance(&AIM9M, v3(0.0, 3000.0, 9000.0), t, beam, false), 0.0);
        // Flares do best against a faint target seen from ahead, worst
        // against an afterburner seen from behind.
        let ir = |v, burner| decoy_chance(&AIM9M, m, t, v, burner);
        assert!(ir(nose_on, false) > ir(tail, false) && ir(tail, false) > ir(tail, true));
    }

    /// Newer seekers reject more decoys: the imaging AIM-9X shrugs off most
    /// flares the old R-60M follows, and the AMRAAM sees through chaff the
    /// R-27R takes.
    #[test]
    fn counter_countermeasures_rank_by_generation() {
        let (m, t, beam) = (v3(0.0, 3000.0, 2000.0), v3(0.0, 3000.0, 0.0), v3(250.0, 0.0, 0.0));
        let p = |s: &Spec| decoy_chance(s, m, t, beam, false);
        assert!(p(&R60M) > p(&R27T) && p(&R27T) > p(&R73) && p(&R73) > p(&AIM9M) && p(&AIM9M) > p(&AIM9X));
        assert!(p(&AIM9X) < 0.15 && p(&R60M) > 0.6, "AIM-9X {:.2}, R-60M {:.2}", p(&AIM9X), p(&R60M));
        assert!(p(&R27R) > p(&R77) && p(&R77) > p(&AIM120C) && p(&AIM120C) > p(&AIM120D));
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
            let (p, tof) = ccip(&MK82, &a, station, &ground).expect("no impact");
            let mut b = Bomb::release(&MK82, &a, station);
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

    /// The 30 mm round leaves slower than the 20 mm one but, heavier for
    /// its size, keeps its speed better: the two meet near 1,500 m.
    #[test]
    fn the_30mm_round_carries_its_speed() {
        let fly = |g: &Gun, t: f64| {
            let mut r = Round::of(g, v3(0.0, 3000.0, 0.0), v3(0.0, 0.0, -g.velocity), false, Who::Player);
            for _ in 0..(t / FIXED_DT) as usize {
                r.step(FIXED_DT);
            }
            r
        };
        let (a, b) = (fly(&M61A1, 0.5), fly(&GSH301, 0.5));
        assert!(a.vel.len() > b.vel.len(), "early on the 20 mm is faster");
        let (a, b) = (fly(&M61A1, 2.5), fly(&GSH301, 2.5));
        assert!(b.vel.len() > a.vel.len(), "later the 30 mm is: {:.0} vs {:.0} m/s", b.vel.len(), a.vel.len());
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
    #[test]
    fn ridge_masks_radar_and_ir_seekers() {
        let ridge = |_x: f64, z: f64| if z.abs() < 450.0 { 2000.0 } else { 0.0 };
        for spec in [&AIM9M, &AIM120C, &R27R] {
            let mut m = Missile::launch(spec, v3(0.0, 1200.0, 4000.0), v3(0.0, 0.0, -450.0), Who::Player, Some(Who::Bandit(0)));
            let tp = v3(0.0, 1100.0, -4000.0);
            m.step(FIXED_DT, Some((tp, v3(0.0, 0.0, -230.0))), &ridge);
            assert!(!m.tracking, "{} saw through a ridge", spec.name);
        }
    }

    #[test]
    fn sustained_look_down_notch_breaks_radar_but_not_heat_seeker() {
        for spec in [&R27R, &AIM120C, &AIM9M] {
            let mut m = Missile::launch(spec, v3(0.0, 1800.0, 10000.0), v3(0.0, 0.0, -250.0), Who::Player, Some(Who::Bandit(0)));
            let tv = v3(240.0, 0.0, 0.0);
            for k in 0..(3.0 / FIXED_DT) as usize {
                let tp = v3(0.0, 1200.0, 0.0) + tv * (k as f64 * FIXED_DT);
                m.step(FIXED_DT, Some((tp, tv)), &flat);
            }
            assert_eq!(m.tracking, spec.infrared(), "{} tracking={}", spec.name, m.tracking);
        }
    }

    /// Same launch, different target decisions: near the edge of the envelope
    /// a turn away defeats a missile which reaches an undefended target.
    #[test]
    fn turning_away_can_exhaust_a_missile_that_would_hit_head_on() {
        for spec in [&AIM9M, &AIM120C, &R73, &R77] {
            let range = if spec.infrared() { 10000.0 } else if spec.name == R77.name { 14000.0 } else { 18000.0 };
            let run = |defend: bool| {
                let mut m = Missile::launch(spec, v3(0.0, 1500.0, 0.0), v3(0.0, 0.0, -250.0), Who::Player, Some(Who::Bandit(0)));
                let mut tp = v3(0.0, 1500.0, -range);
                let mut elapsed = 0.0;
                loop {
                    let angle = if defend { (elapsed / 7.0_f64).min(1.0) * PI } else { 0.0 };
                    let tv = v3(angle.sin(), 0.0, angle.cos()) * 250.0;
                    elapsed += FIXED_DT; tp += tv * FIXED_DT;
                    let fate = m.step(FIXED_DT, Some((tp, tv)), &flat);
                    if fate != Fate::Flying { break fate; }
                    assert!(elapsed < 90.0);
                }
            };
            assert!(matches!(run(false), Fate::Burst(_, _)), "{} baseline missed", spec.name);
            assert!(!matches!(run(true), Fate::Burst(_, _)), "{} defence failed", spec.name);
        }
    }

}
