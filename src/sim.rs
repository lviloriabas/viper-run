//! Flight model and mission. A six-degree-of-freedom rigid body after the
//! JSBSim F-16 that FlightGear flies: NASA TP-1538 aerodynamics (aero.rs), an
//! F-16 FLCS (g command with an AoA limiter, roll-rate command, aileron-rudder
//! interconnect, yaw damper) driving rate-limited surface actuators,
//! F110-GE-129 thrust with spool lag, fuel burn changing mass and inertia, ISA
//! atmosphere and compressible pitot. On top of the tables, each wing panel
//! stalls on its own, so a stall can drop a wing and start a departure.
use crate::aero::{self, Coeffs, Flow};
use crate::air::{KT, RHO0};
use crate::math::*;
use crate::structure::Structure;
use crate::terrain::{self, river_x, terrain_at};

pub const G0: f64 = 9.80665;
pub const FIXED_DT: f64 = 1.0 / 120.0;

pub const GATE_RADIUS: f64 = 47.0;
pub const MISSION_TIME: f64 = 150.0;
pub const GATE_BONUS: f64 = 8.0;
const ROUTE_Z: [f64; 10] = [7800.0, 6000.0, 4200.0, 2400.0, 500.0, -1500.0, -3500.0, -5400.0, -7200.0, -9000.0];
const ROUTE_OFFSET: [f64; 10] = [0.0, 115.0, -80.0, 160.0, -120.0, 145.0, -170.0, 90.0, -70.0, 0.0];
const ROUTE_CLEARANCE: [f64; 10] = [170.0, 135.0, 185.0, 125.0, 160.0, 140.0, 190.0, 130.0, 170.0, 150.0];

/// F-16C Block 50 mass properties (FlightGear jsb-mass-50) and FLCS limits.
pub mod spec {
    /// Empty weight plus pilot, 19,359 lb.
    pub const EMPTY_MASS: f64 = 8781.0;
    /// Two wingtip AIM-9s with launchers and the empty pylons.
    pub const STORES_MASS: f64 = 450.0;
    /// 7,000 lb of internal JP-8.
    pub const FUEL_MAX: f64 = 3175.0;
    /// Empty-jet inertia, kg m^2 (9,007 / 49,956 / 56,770 / 198 slug ft^2).
    pub const IXX: f64 = 12213.0;
    pub const IYY: f64 = 67732.0;
    pub const IZZ: f64 = 76970.0;
    pub const IXZ: f64 = 268.0;
    /// Centre of gravity, fraction of MAC: about 4 % unstable, as on the real jet.
    pub const XCG: f64 = 0.35;
    pub const G_LIMIT: f64 = 9.0;
    pub const NEG_G_LIMIT: f64 = -3.0;
    /// The limiter starts trading g for AoA here and stops near 25.5 degrees.
    pub const ALPHA_LIMITER: f64 = 22.5;
    pub const ROLL_RATE_MAX: f64 = 308.0;
    /// Surface travel in degrees and actuator rates in degrees per second.
    pub const STAB_MAX: f64 = 25.0;
    pub const STAB_RATE: f64 = 60.0;
    pub const FLAPERON_MAX: f64 = 21.5;
    /// Flaperons together with the differential stabilator they are mixed with.
    pub const FLAPERON_RATE: f64 = 80.0;
    pub const RUDDER_MAX: f64 = 30.0;
    pub const RUDDER_RATE: f64 = 120.0;
    /// Wing-panel stall: critical AoA in slow, smooth air (degrees), a little
    /// past where the limiter holds a full pull; spanwise station of each
    /// half-wing's lift (m); share of a panel's lift lost once it separates.
    pub const ALPHA_CRIT: f64 = 27.0;
    pub const PANEL_ARM: f64 = 2.3;
    pub const STALL_LIFT_LOSS: f64 = 0.3;
    /// Landing gear: seconds to cycle, zero-lift drag added fully down, and
    /// the limit speed (KCAS) past which the doors and struts take damage.
    pub const GEAR_TIME: f64 = 5.0;
    pub const GEAR_DRAG: f64 = 0.021;
    pub const GEAR_LIMIT_KT: f64 = 300.0;
    /// Trailing-edge flaps (the flaperons drooped together): full travel in
    /// degrees, the lift and drag they add fully down, seconds to travel,
    /// and the speed where they blow back up (KCAS).
    pub const FLAP_MAX: f64 = 20.0;
    pub const FLAP_CL: f64 = 0.24;
    pub const FLAP_CD: f64 = 0.028;
    pub const FLAP_TIME: f64 = 2.5;
    pub const FLAP_BLOWBACK_KT: f64 = 370.0;
}
use spec::*;

/// Integration steps per fixed simulation step: the rigid body, the FLCS and
/// the actuators run at 480 Hz.
const SUBSTEPS: usize = 4;

pub use crate::air::{atmosphere_at, calibrated_airspeed};

/// Which jet: the F-16C the tables describe, or a MiG-29S (9.13S) flown on
/// the same aerodynamic tables scaled to its size, weight and engines. The
/// MiG-29 is an approximation built from published figures, not wind-tunnel
/// data: 38.06 m^2 of wing against 27.9, 11,200 kg empty (the 9.13's manual
/// figure, 300 kg over the 9.12 for the enlarged spine and its jammer),
/// 3,820 kg of internal fuel (the 9.12's 3,630 and the 240 litres the
/// enlarged No. 1 tank in the spine adds), two RD-33s giving 22,200 lbf dry
/// and 36,600 lbf in afterburner, and about 35 % more inertia. The flight
/// manuals cited for the 9.12 and 9.13 limit the AoA to 24 degrees (Russian)
/// or 26 (German), against 26 to 28 degrees critical, and allow 9 g only
/// below 14,200 kg, 8 g above.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Type {
    #[default]
    F16,
    Mig29,
}

impl Type {
    pub fn name(self) -> &'static str {
        match self {
            Type::F16 => "F-16C BLOCK 50",
            Type::Mig29 => "MIG-29S 9.13S",
        }
    }
    /// The full name of the variant the model follows.
    pub fn full_name(self) -> &'static str {
        match self {
            Type::F16 => "F-16C BLOCK 50 FIGHTING FALCON",
            Type::Mig29 => "MIG-29S FULCRUM-C, IZDELIYE 9.13S",
        }
    }
    pub fn empty_mass(self) -> f64 {
        match self {
            Type::F16 => EMPTY_MASS,
            Type::Mig29 => 11200.0,
        }
    }
    /// Wing reference area, m^2.
    pub fn area(self) -> f64 {
        match self {
            Type::F16 => aero::WING_AREA,
            Type::Mig29 => 38.06,
        }
    }
    /// Thrust against the F110's at the same power setting.
    pub fn thrust_scale(self, power: f64) -> f64 {
        match self {
            Type::F16 => 1.0,
            Type::Mig29 => mix(22200.0 / 17000.0, 36600.0 / 29000.0, ((power - 50.0) / 50.0).clamp(0.0, 1.0)),
        }
    }
    /// Internal fuel, kg.
    pub fn fuel_max(self) -> f64 {
        match self {
            Type::F16 => FUEL_MAX,
            Type::Mig29 => 3820.0,
        }
    }
    /// Full afterburner thrust at sea level, lbf: one F110-GE-129, two RD-33s.
    pub fn max_thrust_lbf(self) -> f64 {
        match self {
            Type::F16 => 29000.0,
            Type::Mig29 => 36600.0,
        }
    }
    /// Least fuel a sortie can start with, kg: the F-16 Block 40+ minimum
    /// fuel, 1,000 lb, and for the MiG-29 an estimate of the same reserve at
    /// the RD-33 pair's higher idle flow (no published figure was found).
    pub fn fuel_min(self) -> f64 {
        match self {
            Type::F16 => 1000.0 / 2.20462,
            Type::Mig29 => 550.0,
        }
    }
    /// AoA where the limiter starts trading g for AoA, and the hard stop of
    /// the AoA protection, degrees: the F-16 caps near 25.5, the MiG-29 at
    /// the 25 degrees between its two manuals' figures.
    pub fn alpha_limits(self) -> (f64, f64) {
        match self {
            Type::F16 => (ALPHA_LIMITER, 25.0),
            Type::Mig29 => (22.0, 25.0),
        }
    }
    /// Positive load limit at a gross weight (kg).
    pub fn g_limit(self, mass: f64) -> f64 {
        match self {
            Type::Mig29 if mass > 14200.0 => 8.0,
            _ => G_LIMIT,
        }
    }
    pub fn inertia_scale(self) -> f64 {
        match self {
            Type::F16 => 1.0,
            Type::Mig29 => 1.35,
        }
    }
    pub fn next(self) -> Type {
        match self {
            Type::F16 => Type::Mig29,
            Type::Mig29 => Type::F16,
        }
    }
}

/// Battle damage in the terms the flight model uses, each from 0 (sound) to
/// 1 (gone), worked out from the hit modules (`damage::Hull::effects`):
/// thrust lost from the engines, fuel pouring from holed tanks, a wing that
/// lifts less, drags more and rolls the jet toward its side, hydraulics and
/// tail surfaces that move less far, and an airframe that takes less g.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BattleDamage {
    pub engine: f64,
    /// Fuel leaking, kg/s.
    pub leak: f64,
    pub wing: f64,
    /// The damaged wing: -1 left, 1 right.
    pub side: f64,
    pub controls: f64,
    pub airframe: f64,
    /// Thrust lost on the right engine less the left's, halved: a twin with
    /// one engine down yaws toward the dead one.
    pub asymmetry: f64,
    /// Something is burning.
    pub fire: bool,
}

impl BattleDamage {
    /// Fraction of the thrust left.
    pub fn thrust(&self) -> f64 {
        if self.engine >= 0.95 { 0.0 } else { 1.0 - self.engine }
    }

    /// Fraction of the surface travel the hydraulics still give.
    pub fn authority(&self) -> f64 {
        1.0 - 0.65 * self.controls
    }

    /// Fraction of the load limit the airframe still takes.
    pub fn strength(&self) -> f64 {
        1.0 - 0.45 * self.airframe
    }
}

/// A landing-gear leg: the tyre's lowest point at full extension (body
/// axes), the oleo's spring and damping, its stroke, and whether it brakes
/// or steers. Nose gear 3.45 m ahead of the centre of gravity, main gear
/// 0.55 m behind it on a 2.36 m track: the jet stands 5.09 m to the fin tip
/// with about 14 % of its weight on the nose wheel.
pub struct Wheel {
    pub name: &'static str,
    pub local: V3,
    k: f64,
    d: f64,
    stroke: f64,
    brakes: bool,
    steers: bool,
}

pub const WHEELS: [Wheel; 3] = [
    Wheel { name: "nose gear", local: v3(0.0, -1.55, -3.45), k: 1.6e5, d: 1.7e4, stroke: 0.30, brakes: false, steers: true },
    Wheel { name: "left main gear", local: v3(-1.18, -1.55, 0.55), k: 4.5e5, d: 4.9e4, stroke: 0.30, brakes: true, steers: false },
    Wheel { name: "right main gear", local: v3(1.18, -1.55, 0.55), k: 4.5e5, d: 4.9e4, stroke: 0.30, brakes: true, steers: false },
];

pub struct ContactPoint {
    pub name: &'static str,
    pub local: V3,
    pub margin: f64,
}

/// Body-space contact points sampled against the terrain every step, so a
/// banked wing can clip a canyon wall before the centre of mass gets there.
pub const COLLISION_POINTS: [ContactPoint; 10] = [
    ContactPoint { name: "nose", local: v3(0.0, 0.31, -7.09), margin: 0.10 },
    ContactPoint { name: "canopy", local: v3(0.0, 1.58, -3.06), margin: 0.10 },
    ContactPoint { name: "tail", local: v3(0.0, 0.55, 6.73), margin: 0.15 },
    ContactPoint { name: "left wing", local: v3(-4.86, 0.33, 2.40), margin: 0.08 },
    ContactPoint { name: "right wing", local: v3(4.86, 0.33, 2.40), margin: 0.08 },
    ContactPoint { name: "intake", local: v3(0.0, -0.43, -3.06), margin: 0.08 },
    ContactPoint { name: "belly", local: v3(0.0, -0.45, 0.40), margin: 0.10 },
    ContactPoint { name: "fin", local: v3(0.0, 3.64, 6.55), margin: 0.10 },
    ContactPoint { name: "left stabilator", local: v3(-2.79, 0.25, 6.20), margin: 0.06 },
    ContactPoint { name: "right stabilator", local: v3(2.79, 0.25, 6.20), margin: 0.06 },
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    pub yaw_left: bool,
    pub yaw_right: bool,
    pub pitch_axis: f64,
    pub roll_axis: f64,
    pub yaw_axis: f64,
    pub throttle_up: bool,
    pub throttle_down: bool,
    /// Slams the throttle to the stop, past the military detent.
    pub afterburner: bool,
    pub airbrake: bool,
    /// Landing gear handle down; flaps switch on; toe brakes pressed.
    pub gear_down: bool,
    pub flaps: bool,
    pub wheel_brake: bool,
}

impl Input {
    pub fn pitch(&self) -> f64 {
        clamp(self.pitch_axis + self.pitch_up as i32 as f64 - self.pitch_down as i32 as f64, -1.0, 1.0)
    }
    pub fn roll(&self) -> f64 {
        clamp(self.roll_axis + self.roll_right as i32 as f64 - self.roll_left as i32 as f64, -1.0, 1.0)
    }
    pub fn yaw(&self) -> f64 {
        clamp(self.yaw_axis + self.yaw_right as i32 as f64 - self.yaw_left as i32 as f64, -1.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Gate {
    pub pos: V3,
    pub radius: f64,
    pub axis: V3,
    pub passed: bool,
}

#[derive(Clone, Debug)]
pub struct Mission {
    pub gates: Vec<Gate>,
    pub gate_index: usize,
    pub time_left: f64,
    /// The clock counts down and ends the run at zero; off, it never runs
    /// and the time left stays out of the score.
    pub timed: bool,
    pub elapsed: f64,
    pub score: f64,
    pub passed: usize,
    pub start_pos: V3,
    pub start_q: Quat,
    pub start_vel: V3,
    pub start_throttle: f64,
}

impl Mission {
    pub fn new() -> Mission {
        let mut gates: Vec<Gate> = (0..ROUTE_Z.len())
            .map(|i| {
                let z = ROUTE_Z[i];
                let x = river_x(z) + ROUTE_OFFSET[i];
                let ground = terrain_at(x, z);
                Gate { pos: v3(x, ground + ROUTE_CLEARANCE[i], z), radius: GATE_RADIUS, axis: v3(0.0, 0.0, -1.0), passed: false }
            })
            .collect();
        // Ring planes face the average course direction through each gate.
        for i in 0..gates.len() {
            let a = gates[i.saturating_sub(1)].pos;
            let b = gates[(i + 1).min(gates.len() - 1)].pos;
            gates[i].axis = v3(b.x - a.x, 0.0, b.z - a.z).norm();
        }
        // Start 2.4 km short of the first gate, looking down its throat.
        let first = &gates[0];
        let back = first.axis * -2400.0;
        let mut start = v3(first.pos.x + back.x, 0.0, first.pos.z + back.z);
        start.y = (first.pos.y + 150.0).max(terrain_at(start.x, start.z) + 180.0);
        let forward = (first.pos - start).norm();
        // Trimmed for 1 g at 218 m/s: the nose sits at the trim AoA above the path.
        let speed = 218.0;
        let t = trim(start.y, speed, EMPTY_MASS + STORES_MASS + FUEL_MAX);
        let q = Quat::from_forward_up(forward, V3::Y).mul(Quat::axis_angle(V3::X, t.alpha));
        Mission {
            gates,
            gate_index: 0,
            time_left: MISSION_TIME,
            timed: true,
            elapsed: 0.0,
            score: 0.0,
            passed: 0,
            start_pos: start,
            start_q: q,
            start_vel: forward * speed,
            start_throttle: t.throttle,
        }
    }

    pub fn final_score(&self) -> f64 {
        (self.passed as f64 * 1000.0 + self.time_left.max(0.0) * 25.0).round()
    }

    pub fn current_gate(&self) -> &Gate {
        &self.gates[self.gate_index.min(self.gates.len() - 1)]
    }
}

/// Wings-level 1 g trim for a speed and altitude.
#[derive(Clone, Copy, Debug)]
pub struct Trim {
    /// Angle of attack, radians.
    pub alpha: f64,
    /// Stabilator, degrees trailing edge down.
    pub el: f64,
    /// Engine power 0..100 and the throttle that holds it.
    pub power: f64,
    pub throttle: f64,
}

pub fn trim(altitude: f64, speed: f64, mass: f64) -> Trim {
    let atm = atmosphere_at(altitude);
    let qs = 0.5 * atm.rho * speed * speed * aero::WING_AREA;
    let mach = speed / atm.sound;
    let flow = |alpha: f64, el: f64| Flow { alpha, el, speed, mach, xcg: XCG, h_over_b: 10.0, qbar: qs / aero::WING_AREA, ..Default::default() };
    // Pitching moment falls with stabilator, so bisect it to zero at each AoA.
    let el_for = |alpha: f64| {
        let (mut lo, mut hi) = (-STAB_MAX, STAB_MAX);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if aero::coefficients(&flow(alpha, mid)).cm > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    // Thrust balances drag along the path; AoA is stepped until lift carries the weight.
    let balance = |alpha: f64| {
        let el = el_for(alpha);
        let c = aero::coefficients(&flow(alpha, el));
        let (sa, ca) = (alpha.to_radians().sin(), alpha.to_radians().cos());
        let thrust = -qs * (c.cx * ca + c.cz * sa) / ca;
        let lift = qs * (-c.cz * ca + c.cx * sa) + thrust * sa;
        (lift - mass * G0, el, thrust)
    };
    let (mut a0, mut a1) = (0.0, 6.0);
    let (mut r0, mut r1) = (balance(a0).0, balance(a1).0);
    for _ in 0..30 {
        if (r1 - r0).abs() < 1e-9 {
            break;
        }
        let a2 = clamp(a1 - r1 * (a1 - a0) / (r1 - r0), -5.0, 25.0);
        a0 = a1;
        r0 = r1;
        a1 = a2;
        r1 = balance(a1).0;
    }
    let (_, el, thrust) = balance(a1);
    let (mut lo, mut hi) = (0.0, 100.0);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if aero::thrust(mid, mach, altitude) < thrust {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let power = 0.5 * (lo + hi);
    Trim { alpha: a1.to_radians(), el, power, throttle: (power / 50.0).min(1.0) }
}

/// Inertia about the body axes (roll, pitch, yaw, roll-yaw product), with
/// the wingtip missiles, the fuel in the fuselage and wing tanks and the
/// stores on the underwing pylons, about 2.9 m out on average: a bomb load
/// makes the jet slower to start and stop a roll.
fn inertia(fuel: f64, pylons: f64) -> (f64, f64, f64, f64) {
    let f = fuel / FUEL_MAX;
    let tips = 2.0 * 95.0 * 4.8 * 4.8 + pylons * 2.9 * 2.9;
    let body = f * 1157.0 * (1.31f64.powi(2) + 1.74f64.powi(2));
    let wings = f * 2.0 * 249.0 * 2.03f64.powi(2);
    (IXX + tips + wings, IYY + body, IZZ + tips + body + wings, IXZ)
}

/// First-order washout: passes changes, removes the steady part.
fn washout(state: &mut f64, x: f64, tau: f64, dt: f64) -> f64 {
    *state += (x - *state) * clamp(dt / tau, 0.0, 1.0);
    x - *state
}

/// Hydraulic actuator: 20 rad/s servo lag, rate and travel limits.
fn actuate(pos: f64, cmd: f64, max: f64, rate: f64, dt: f64) -> f64 {
    let v = clamp((clamp(cmd, -max, max) - pos) * 20.0, -rate, rate);
    clamp(pos + v * dt, -max, max)
}

/// Flight control computer and surface state, after the FlightGear F-16 FLCS:
/// the same command paths (g command with an AoA limiter, roll-rate command,
/// aileron-rudder interconnect, yaw damper), with the pitch and roll inner
/// loops closed by inverting the aerodynamic tables so the response holds
/// from 120 kt to Mach 2.
#[derive(Clone, Debug, Default)]
pub struct Flcs {
    /// Lagged command, g above 1 g.
    pub n_cmd: f64,
    /// AoA seen by the limiter, degrees.
    pub alpha_f: f64,
    /// Load-factor error integrator, g, and the load factor a step ago.
    pub n_int: f64,
    nz_last: f64,
    /// Pitch attitude held with the stick at rest, radians.
    hold: Option<f64>,
    /// Lagged roll-rate command, deg/s.
    pub p_cmd: f64,
    q_wash: f64,
    r_wash: f64,
    /// Surface positions in degrees (stabilator and flaperon trailing edge
    /// down, rudder trailing edge left positive) and airbrake opening 0..1.
    pub el: f64,
    pub ail: f64,
    pub rdr: f64,
    pub brake: f64,
}

/// Stick shaping. The real force gradients (FlightGear F-16) give almost
/// nothing over the first fifth of the travel and only 50 deg/s of roll at
/// half stick, which a mouse, keyboard or thumbstick cannot feel: they read
/// as a stick without authority. These power curves keep the same end
/// points (8 g above 1 g and -4 g pushing at the stops, 308 deg/s full
/// lateral) with a small dead band and a firm response from there on.
const STICK_DEAD: f64 = 0.03;
const PITCH_EXP: f64 = 1.3;
const ROLL_EXP: f64 = 1.5;

fn live(s: f64) -> f64 {
    ((s.abs() - STICK_DEAD) / (1.0 - STICK_DEAD)).max(0.0)
}

/// Stick (-1 push to 1 pull) to the g command above 1 g.
fn pitch_command(s: f64) -> f64 {
    let k = live(s).powf(PITCH_EXP);
    if s >= 0.0 {
        8.5 * k
    } else {
        -4.2 * k
    }
}

/// Lateral stick (-1 left to 1 right) to the roll-rate command, deg/s.
fn roll_command(s: f64) -> f64 {
    ROLL_RATE_MAX * live(s).powf(ROLL_EXP) * s.signum()
}

/// Inverse of `live`: the deflection that leaves `k` of the live travel.
fn unlive(k: f64) -> f64 {
    if k <= 0.0 {
        0.0
    } else {
        STICK_DEAD + k * (1.0 - STICK_DEAD)
    }
}
/// Lift coefficient at which shock-induced buffet starts, against Mach: a
/// typical fighter buffet boundary, lowest just below Mach 1.
const BUFFET_ONSET: [(f64, f64); 6] = [(0.5, 1.2), (0.7, 0.95), (0.85, 0.7), (0.95, 0.5), (1.05, 0.6), (1.2, 0.9)];

/// Stick deflection (-1 push to 1 pull) that commands a load factor, by
/// inverting the stick shaping: what a pilot's hand does for the AI.
pub fn stick_for_g(n: f64) -> f64 {
    let above = clamp(n - 1.0, NEG_G_LIMIT - 1.0, G_LIMIT - 1.0);
    if above >= 0.0 {
        unlive((above / 8.5).powf(1.0 / PITCH_EXP))
    } else {
        -unlive((-above / 4.2).min(1.0).powf(1.0 / PITCH_EXP))
    }
}

/// Lateral stick deflection (-1 left to 1 right) that commands a roll rate in deg/s.
pub fn stick_for_roll(rate: f64) -> f64 {
    rate.signum() * unlive((rate.abs() / ROLL_RATE_MAX).min(1.0).powf(1.0 / ROLL_EXP))
}
/// What the FLCS reads each frame: air data, rates, load factor, the
/// gravity component along the body's up axis and the mass properties.
struct Sensors {
    flow: Flow,
    nz: f64,
    qbar: f64,
    up_y: f64,
    /// Pitch attitude and bank, radians.
    theta: f64,
    phi: f64,
    inertia: (f64, f64, f64, f64),
    rotor: f64,
    /// Weight on wheels: the g integrator rests, as the FLCS ground mode does.
    ground: bool,
    /// Wing reference area of the airframe, m^2, and the mass, kg.
    area: f64,
    mass: f64,
    /// Limiter onset and hard stop (degrees), and the positive g limit.
    alpha_lim: (f64, f64),
    g_max: f64,
}

/// Surface deflection that makes a monotonically falling coefficient reach a
/// target, by bisection over the surface travel.
fn invert(max: f64, target: f64, coeff: impl Fn(f64) -> f64) -> f64 {
    let (mut lo, mut hi) = (-max, max);
    for _ in 0..18 {
        let mid = 0.5 * (lo + hi);
        if coeff(mid) > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

impl Flcs {
    fn update(&mut self, dt: f64, input: &Input, s: &Sensors) {
        let f = &s.flow;
        let (alpha, speed) = (f.alpha, f.speed.max(40.0));
        let (p, q, r) = (f.p, f.q, f.r);
        let (ixx, iyy, izz, ixz) = s.inertia;
        let qsc = s.qbar.max(50.0) * s.area * aero::CHORD;
        let qsb = s.qbar.max(50.0) * s.area * aero::SPAN;

        // Pitch. The stick commands g. The AoA limiter is a selector: past
        // the onset AoA (filtered, with pitch-rate anticipation) it lowers the
        // command to the g that puts the jet at the limit, worked out from the
        // lift slope, so it caps the slow jet near 25 degrees at full aft stick
        // without fighting the g loop. A fixed g per degree would be several
        // times stiffer than the airframe at low speed and set the nose bobbing.
        let stick = input.pitch();
        let n_cmd = clamp(pitch_command(stick), NEG_G_LIMIT - 1.0, s.g_max - 1.0);
        self.n_cmd += (n_cmd - self.n_cmd) * clamp(dt * 12.0, 0.0, 1.0);
        self.alpha_f += (alpha - self.alpha_f) * clamp(dt * 10.0, 0.0, 1.0);
        let lead = self.alpha_f + 0.1 * washout(&mut self.q_wash, q * RAD, 1.0, dt);
        let lift = |alpha: f64| -aero::coefficients(&Flow { alpha, ..*f }).cz;
        let n_alpha = (s.qbar * s.area * (lift(alpha + 1.0) - lift(alpha)) / (s.mass * G0)).max(0.05);
        let target = s.alpha_lim.1 - 1.0 * smoothstep(s.alpha_lim.0, s.alpha_lim.1, lead);
        let at_limit = s.nz - 1.0 + n_alpha * (target - lead);
        let at_floor = s.nz - 1.0 + n_alpha * (-5.0 - lead);
        let limited = lead > s.alpha_lim.0 && self.n_cmd > at_limit || lead < -4.0 && self.n_cmd < at_floor;
        let demand = if lead > s.alpha_lim.0 { self.n_cmd.min(at_limit) } else { self.n_cmd }.max(if lead < -4.0 { at_floor } else { NEG_G_LIMIT - 1.0 });
        let err = demand - (s.nz - 1.0);
        // The integrator trims out the steady g error only; it rests while the
        // limiter holds the jet or the command is still moving, and bleeds off
        // otherwise, so a release doesn't overshoot through 1 g.
        let nz_rate = if dt > 0.0 { (s.nz - self.nz_last) / dt } else { 0.0 };
        self.nz_last = s.nz;
        let settling = (n_cmd - self.n_cmd).abs() > 0.3 || err.abs() > 1.0 || nz_rate.abs() > 1.5;
        if s.ground {
            self.n_int = 0.0;
        } else if limited || settling {
            self.n_int *= 1.0 - clamp(dt * 3.0, 0.0, 1.0);
        } else {
            self.n_int = clamp(self.n_int + err * 2.5 * dt, -1.5, 1.5);
        }
        // Pitch rate that turns the flight path at the demanded load factor,
        // plus a correction that closes the g error in about a quarter second.
        let g_v = G0 / speed;
        let mut q_cmd = clamp(g_v * (demand + 1.0 - s.up_y) + 2.5 * g_v * (err + self.n_int), -1.0, 1.0);
        // Attitude hold: with the stick at rest, the wings within 60 degrees
        // of level and the nose within 70 of the horizon, the nose stays where
        // the pilot left it, as modern rate-command, attitude-hold fly-by-wire
        // does, instead of sinking back as the AoA of the last pull unloads.
        let calm = s.phi.abs() < 60.0 * DEG && s.theta.abs() < 70.0 * DEG && !s.ground;
        if stick.abs() < STICK_DEAD && calm {
            let lead = s.theta + 0.25 * (q * s.phi.cos() - r * s.phi.sin());
            // Past 2.2 g the hold first unloads, the nose easing down with the
            // excess and the held attitude following it, so a hard pull ends
            // in a quick unload and the hold settles where that leaves the nose.
            let excess = (s.nz - 2.2).max(0.0);
            let held = if excess > 0.0 { *self.hold.insert(lead) } else { *self.hold.get_or_insert(lead) };
            let q_att = (2.0 * (held - s.theta) + r * s.phi.sin()) / s.phi.cos() - 2.0 * g_v * excess;
            q_cmd = clamp(q_att, -1.0, g_v * (2.5 - s.up_y));
            self.n_int = 0.0;
        } else {
            self.hold = None;
        }
        // AoA protection: near the limits, only the pitch rate that holds the
        // AoA where it is (the path turning under the present lift) is allowed.
        let q_hold = g_v * (s.nz - s.up_y);
        let a = alpha * DEG;
        q_cmd = clamp(q_cmd, q_hold + 2.0 * ((-5.5 * DEG) - a), q_hold + 2.0 * (s.alpha_lim.1 * DEG - a));
        let qdot = 7.0 * (q_cmd - q);
        let m_req = iyy * qdot - (izz - ixx) * p * r + ixz * (p * p - r * r) + r * s.rotor;
        let el_cmd = invert(STAB_MAX, m_req / qsc, |el| aero::coefficients(&Flow { el, ..*f }).cm);

        // Roll. Rate command, reduced at high AoA; the flaperons are set for
        // the roll acceleration that closes the rate error in a tenth of a second.
        let lat = input.roll();
        let mut p_cmd = roll_command(lat);
        let p_max = ROLL_RATE_MAX * clamp(1.0 - (self.alpha_f - 15.0) / 22.0, 0.3, 1.0);
        p_cmd = clamp(p_cmd, -p_max, p_max);
        self.p_cmd += (p_cmd - self.p_cmd) * clamp(dt * 16.0, 0.0, 1.0);
        let pdot = 10.0 * (self.p_cmd * DEG - p);
        let c0 = aero::coefficients(f);
        let n_now = qsb * c0.cn;
        let gam = ixx * izz - ixz * ixz;
        let xpq = ixz * (ixx - iyy + izz);
        let xqr = izz * (izz - iyy) + ixz * ixz;
        let l_req = (gam * pdot - xpq * p * q + xqr * q * r - ixz * (n_now + q * s.rotor)) / izz;
        let ail_cmd = invert(FLAPERON_MAX, l_req / qsb, |ail| aero::coefficients(&Flow { ail, ..*f }).cl);

        // Yaw: pedals with authority shrinking at high dynamic pressure,
        // aileron-rudder interconnect for coordinated rolls at AoA, a damper on
        // the washed-out stability-axis yaw rate and sideslip feedback.
        let authority = RUDDER_MAX * clamp(12000.0 / s.qbar.max(1.0), 0.25, 1.0);
        let r_stab = (r * a.cos() - p * a.sin()) * RAD;
        let r_w = washout(&mut self.r_wash, r_stab, 1.0, dt);
        let ari = self.ail * clamp(0.02 * self.alpha_f, 0.0, 0.6);
        let damper = clamp(0.5 * 28600.0 / s.qbar.max(1.0), 0.1, 2.0);
        let rdr_cmd = -input.yaw() * authority + ari + damper * r_w - clamp(28600.0 / s.qbar.max(1.0), 0.2, 2.0) * f.beta;

        self.el = actuate(self.el, el_cmd, STAB_MAX, STAB_RATE, dt);
        self.ail = actuate(self.ail, ail_cmd, FLAPERON_MAX, FLAPERON_RATE, dt);
        self.rdr = actuate(self.rdr, rdr_cmd, RUDDER_MAX, RUDDER_RATE, dt);
        self.brake = approach(self.brake, if input.airbrake { 1.0 } else { 0.0 }, dt * 0.5);
    }
}

#[derive(Clone, Debug)]
pub struct Aircraft {
    pub pos: V3,
    pub vel: V3,
    pub q: Quat,
    /// Body angular velocity about right, up and aft (pitch up, yaw left, roll left).
    pub omega: V3,
    pub flcs: Flcs,
    pub throttle: f64,
    /// Engine power 0..100: 50 is military, above it the afterburner.
    pub power: f64,
    pub engine_spool: f64,
    pub ab_level: f64,
    pub ab: bool,
    pub brake: f64,
    pub fuel: f64,
    pub mass: f64,
    /// What hangs on the stations, kg: STORES_MASS for the two wingtip
    /// AIM-9s, set by the loadout and lightened as stores are released.
    pub stores_mass: f64,
    /// Zero-lift drag coefficient of the stores beyond that clean baseline.
    pub stores_drag: f64,
    /// Battle damage (see `BattleDamage`).
    pub battle: BattleDamage,
    pub speed: f64,
    pub airspeed_kt: f64,
    pub tas_kt: f64,
    pub eas_kt: f64,
    pub agl: f64,
    pub clearance: f64,
    pub alpha: f64,
    pub alpha_deg: f64,
    pub beta: f64,
    pub mach: f64,
    pub qbar: f64,
    pub g_load: f64,
    pub g_smooth: f64,
    pub max_g: f64,
    pub min_g: f64,
    pub g_avail: f64,
    pub vs: f64,
    pub stall: f64,
    pub buffet: f64,
    pub turbulence: f64,
    pub afterburner_visual: f64,
    pub heading: f64,
    pub pitch: f64,
    pub bank: f64,
    pub crashed: bool,
    pub impact_speed: f64,
    pub impact_water: bool,
    pub collision_label: &'static str,
    pub last_pos: V3,
    pub cl: f64,
    pub cd: f64,
    pub lift_n: f64,
    pub drag_n: f64,
    pub thrust_n: f64,
    pub fuel_flow: f64,
    pub rpm: f64,
    pub ftit: f64,
    pub nozzle: f64,
    pub oil_psi: f64,
    /// Control-surface positions normalised to their travel, for the exterior
    /// model: stabilator trailing edge up, right roll and right rudder positive.
    pub surf_pitch: f64,
    pub surf_roll: f64,
    pub surf_yaw: f64,
    /// Flow separation on the left and right wing panels, 0 attached to 1 stalled.
    pub stall_left: f64,
    pub stall_right: f64,
    /// Critical AoA for the present flight condition, degrees.
    pub alpha_crit: f64,
    /// Wing flutter, elastic bending and structural damage (structure.rs).
    pub structure: Structure,
    /// Stores hung under the wings (kg), which lower the flutter speed.
    pub wing_stores: f64,
    /// Uncommanded roll and yaw while stalled (a wing drop or spin entry), 0..1.
    pub departure: f64,
    /// Landing gear 0 up and locked to 1 down and locked, and the flaps 0..1.
    pub gear: f64,
    pub flaps: f64,
    /// Weight on wheels, the strut compression of each leg (m) and the
    /// brakes applied 0..1.
    pub on_ground: bool,
    pub compression: [f64; 3],
    pub wheel_brake: f64,
    /// Gear lowered or held down past its limit speed: the doors are bent.
    pub gear_overspeed: bool,
    /// The airframe flown.
    pub kind: Type,
}

impl Aircraft {
    pub fn new(m: &Mission) -> Aircraft {
        let mass = EMPTY_MASS + STORES_MASS + FUEL_MAX;
        let speed = m.start_vel.len();
        // A start at rest is on the runway, gear down, engine at idle.
        let parked = speed < 1.0;
        let t = if parked { Trim { alpha: 0.0, el: 0.0, power: 0.0, throttle: 0.0 } } else { trim(m.start_pos.y, speed, mass) };
        let flcs = Flcs { alpha_f: t.alpha * RAD, el: t.el, ..Default::default() };
        let mut a = Aircraft {
            pos: m.start_pos,
            vel: m.start_vel,
            q: m.start_q,
            omega: V3::ZERO,
            flcs,
            throttle: t.throttle,
            power: t.power,
            engine_spool: 0.0,
            ab_level: 0.0,
            ab: false,
            brake: 0.0,
            fuel: FUEL_MAX,
            mass,
            stores_mass: STORES_MASS,
            stores_drag: 0.0,
            battle: BattleDamage::default(),
            speed,
            airspeed_kt: 0.0,
            tas_kt: 0.0,
            eas_kt: 0.0,
            agl: 0.0,
            clearance: 0.0,
            alpha: 0.0,
            alpha_deg: 0.0,
            beta: 0.0,
            mach: 0.0,
            qbar: 0.0,
            g_load: 1.0,
            g_smooth: 1.0,
            max_g: 1.0,
            min_g: 1.0,
            g_avail: G_LIMIT,
            vs: 0.0,
            stall: 0.0,
            buffet: 0.0,
            turbulence: 0.0,
            afterburner_visual: 0.0,
            heading: 0.0,
            pitch: 0.0,
            bank: 0.0,
            crashed: false,
            impact_speed: 0.0,
            impact_water: false,
            collision_label: "",
            last_pos: m.start_pos,
            cl: 0.0,
            cd: 0.0,
            lift_n: 0.0,
            drag_n: 0.0,
            thrust_n: 0.0,
            fuel_flow: 0.0,
            rpm: 0.0,
            ftit: 0.0,
            nozzle: 0.0,
            oil_psi: 0.0,
            surf_pitch: 0.0,
            surf_roll: 0.0,
            surf_yaw: 0.0,
            stall_left: 0.0,
            stall_right: 0.0,
            alpha_crit: ALPHA_CRIT,
            departure: 0.0,
            structure: Structure::default(),
            gear: if parked { 1.0 } else { 0.0 },
            flaps: if parked { 1.0 } else { 0.0 },
            on_ground: parked,
            compression: [0.0; 3],
            wheel_brake: 0.0,
            gear_overspeed: false,
            kind: Type::F16,
            wing_stores: 0.0,
        };
        a.update(0.0, &Input::default());
        a
    }

    fn detect_terrain_collision(&mut self) {
        let mut min_clearance = f64::INFINITY;
        let mut hit: Option<(&ContactPoint, V3, f64)> = None;
        let mut hit_floor = 0.0;
        for point in COLLISION_POINTS.iter() {
            let p = self.pos + self.q.rotate(point.local);
            let floor = terrain::floor(p.x, p.z);
            let clearance = p.y - floor - point.margin;
            if clearance < min_clearance {
                min_clearance = clearance;
                hit_floor = floor;
            }
            if clearance < 0.0 && hit.is_none() {
                hit = Some((point, p, floor));
                hit_floor = floor;
            }
        }
        self.clearance = min_clearance.max(0.0);
        if let Some((point, _p, floor)) = hit {
            self.crashed = true;
            self.impact_speed = self.vel.len();
            self.impact_water = hit_floor < 0.5;
            self.collision_label = point.name;
            self.vel = self.vel * 0.08;
            self.omega = V3::ZERO;
            self.pos.y = self.pos.y.max(floor + 0.8);
        }
    }

    /// Inertia of this airframe with its fuel and pylon stores.
    fn inertia(&self) -> (f64, f64, f64, f64) {
        let (x, y, z, xz) = inertia(self.fuel / self.kind.fuel_max() * FUEL_MAX, self.wing_stores);
        let k = self.kind.inertia_scale();
        (x * k, y * k, z * k, xz * k)
    }

    /// Changes the airframe, keeping the state, and retrims for 1 g at the
    /// present speed when flying.
    pub fn set_type(&mut self, kind: Type) {
        // Same fraction of a full load in the new jet's tanks.
        self.fuel *= kind.fuel_max() / self.kind.fuel_max();
        self.kind = kind;
        self.mass = kind.empty_mass() + self.stores_mass + self.fuel;
    }

    /// Tyre and oleo forces on the legs that touch the ground, as force and
    /// moment in aircraft axes (x forward, y right, z down). Each strut is a
    /// spring and damper; the tyres roll with a little resistance, the main
    /// wheels brake, the nose wheel steers with the pedals (32 degrees at a
    /// taxi, a few at speed) and every tyre grips sideways up to its limit.
    fn wheel_forces(&mut self, input: &Input, dt: f64) -> (V3, V3) {
        let mut force = V3::ZERO;
        let mut moment = V3::ZERO;
        self.on_ground = false;
        if self.gear < 0.98 {
            self.compression = [0.0; 3];
            return (force, moment);
        }
        let w_world = self.q.rotate(self.omega);
        let ground_speed = v3(self.vel.x, 0.0, self.vel.z).len();
        let steer = -input.yaw() * mix(32.0, 4.0, smoothstep(8.0, 45.0, ground_speed)) * DEG;
        for (i, wh) in WHEELS.iter().enumerate() {
            let r = self.q.rotate(wh.local);
            let p = self.pos + r;
            let c = terrain::floor(p.x, p.z) - p.y;
            if c <= 0.0 {
                self.compression[i] = 0.0;
                continue;
            }
            self.compression[i] = c;
            if c > wh.stroke && dt > 0.0 {
                self.structure.failed = Some(wh.name);
            }
            let v = self.vel + w_world.cross(r);
            let normal = (wh.k * c - wh.d * v.y).max(0.0);
            if normal <= 0.0 {
                continue;
            }
            self.on_ground = true;
            let mut fwd = self.q.axes().forward;
            if wh.steers {
                fwd = Quat::axis_angle(self.q.axes().up, steer).rotate(fwd);
            }
            let fw = v3(fwd.x, 0.0, fwd.z).norm();
            let side = fw.cross(V3::Y).norm();
            let (v_long, v_lat) = (v.dot(fw), v.dot(side));
            let mu = 0.015 + if wh.brakes { 0.55 * self.wheel_brake } else { 0.0 };
            let f_long = -clamp(v_long * 8.0, -1.0, 1.0) * mu * normal;
            let f_lat = -clamp(v_lat * 2.0, -1.0, 1.0) * 0.75 * normal;
            let f = V3::Y * normal + fw * f_long + side * f_lat;
            let fb = self.q.conj().rotate(f);
            let mb = wh.local.cross(fb);
            force += v3(-fb.z, fb.x, -fb.y);
            moment += v3(-mb.z, mb.x, -mb.y);
        }
        (force, moment)
    }

    /// Air-data angles and rates in aircraft axes (x forward, y right, z down).
    fn air_state(&self) -> (f64, f64, f64, f64, f64, f64) {
        let lv = self.q.conj().rotate(self.vel);
        let (u, v, w) = (-lv.z, lv.x, -lv.y);
        let speed = (u * u + v * v + w * w).sqrt();
        let (alpha, beta) = if speed > 1.0 { (w.atan2(u), clamp(v / speed, -1.0, 1.0).asin()) } else { (0.0, 0.0) };
        (speed, alpha, beta, -self.omega.z, self.omega.x, -self.omega.y)
    }

    fn flow(&self, speed: f64, alpha: f64, beta: f64, p: f64, q: f64, r: f64, mach: f64, qbar: f64) -> Flow {
        let f = &self.flcs;
        let h_over_b = self.agl.max(0.0) / aero::SPAN;
        Flow { alpha: alpha * RAD, beta: beta * RAD, p, q, r, speed, mach, el: f.el, ail: f.ail, rdr: f.rdr, brake: f.brake, xcg: XCG, qbar, h_over_b }
    }

    /// Critical AoA for the present condition, degrees. Shock-induced
    /// separation lowers it towards Mach 1; thin air (a lower Reynolds
    /// number), ground effect and sideslip lower it a little; a fast pitch-up
    /// briefly delays separation (dynamic stall).
    fn critical_alpha(&self, f: &Flow) -> f64 {
        let compressibility = 1.0 - 0.22 * smoothstep(0.45, 0.95, f.mach);
        let thin_air = 1.5 * smoothstep(4000.0, 15000.0, self.pos.y);
        let ground = 2.5 * (1.0 - smoothstep(0.0, 1.0, f.h_over_b));
        let slip = 0.2 * f.beta.abs();
        let dynamic = clamp(f.q * aero::CHORD / (2.0 * f.speed.max(30.0)) * 300.0, -2.0, 4.0);
        ALPHA_CRIT * compressibility - thin_air - ground - slip + dynamic
    }

    /// Wing-panel stall, applied to the table coefficients after the FLCS
    /// has set its surfaces, so the computer meets it as a disturbance. Each
    /// half-wing separates on its own local AoA: rolling raises it on the
    /// down-going wing, sideslip on the windward one, low-level gusts on
    /// either. Weight, load factor, bank, airspeed and altitude all act
    /// through the AoA the jet needs. A separated panel loses lift and drags
    /// like a plate; an uneven stall drops that wing and yaws the nose
    /// towards it, and the lost lift lets the nose fall.
    fn separation(&mut self, f: &Flow, c: &mut Coeffs, dt: f64) {
        let crit = self.critical_alpha(f);
        let v = f.speed.max(30.0);
        let roll = f.p * PANEL_ARM / v * RAD;
        let slip = 0.25 * f.beta * clamp(f.alpha / 10.0, -1.0, 1.0);
        let span = self.q.axes().right * PANEL_ARM;
        let gusts = self.turbulence * 3.0 / v * RAD;
        let left = f.alpha - roll - slip + gust(self.pos - span) * gusts;
        let right = f.alpha + roll + slip + gust(self.pos + span) * gusts;
        self.stall_left = separate(self.stall_left, left, crit, dt);
        self.stall_right = separate(self.stall_right, right, crit, dt);
        self.alpha_crit = crit;

        let (sl, sr) = (self.stall_left, self.stall_right);
        // Per fully separated panel: lost normal force and added plate drag.
        let lost = 0.5 * STALL_LIFT_LOSS * c.cz;
        let (sa, ca) = (f.alpha.to_radians().sin(), f.alpha.to_radians().cos());
        let plate = 0.45 * sa * sa;
        c.cz -= lost * (sl + sr);
        c.cx -= plate * (sl + sr) * ca;
        c.cz -= plate * (sl + sr) * sa;
        let arm = PANEL_ARM / aero::SPAN;
        c.cl += lost * (sl - sr) * arm;
        c.cn -= plate * (sl - sr) * arm;
        c.cm -= 0.03 * 0.5 * (sl + sr) * f.alpha.signum();
    }

    /// One rigid-body step: FLCS, engine, forces and moments, integration.
    fn integrate(&mut self, dt: f64, input: &Input, want_ab: bool) {
        let atm = atmosphere_at(self.pos.y);
        let (speed, alpha, beta, p, q, r) = self.air_state();
        let qbar = 0.5 * atm.rho * speed * speed;
        let mach = speed / atm.sound;
        let sensors = Sensors {
            flow: self.flow(speed, alpha, beta, p, q, r, mach, qbar),
            nz: self.g_load,
            qbar,
            up_y: self.q.axes().up.y,
            theta: self.q.axes().forward.y.clamp(-1.0, 1.0).asin(),
            phi: (-self.q.axes().right.y).atan2(self.q.axes().up.y),
            inertia: self.inertia(),
            rotor: aero::ENGINE_MOMENTUM * self.power.min(50.0) / 50.0,
            ground: self.on_ground,
            area: self.kind.area(),
            mass: self.mass,
            alpha_lim: self.kind.alpha_limits(),
            g_max: self.kind.g_limit(self.mass) * self.battle.strength(),
        };
        self.flcs.update(dt, input, &sensors);
        // Damaged hydraulics move the surfaces only part of their travel.
        let k = self.battle.authority();
        if k < 1.0 {
            self.flcs.el = clamp(self.flcs.el, -STAB_MAX * k, STAB_MAX * k);
            self.flcs.ail = clamp(self.flcs.ail, -FLAPERON_MAX * k, FLAPERON_MAX * k);
            self.flcs.rdr = clamp(self.flcs.rdr, -RUDDER_MAX * k, RUDDER_MAX * k);
        }

        // A dead engine burns nothing and gives nothing: it spools down.
        let fueled = self.fuel > 0.0 && self.battle.thrust() > 0.0;
        let command = if !fueled { 0.0 } else if want_ab { 100.0 } else { 50.0 * self.throttle };
        self.power = clamp(self.power + aero::power_rate(self.power, command) * dt, 0.0, 100.0);
        let full = if fueled { aero::thrust(self.power, mach, self.pos.y) * self.kind.thrust_scale(self.power) } else { 0.0 };
        self.thrust_n = full * self.battle.thrust();

        let flow = self.flow(speed, alpha, beta, p, q, r, mach, qbar);
        let mut c = aero::coefficients(&flow);
        self.separation(&flow, &mut c, dt);
        // The wing's antisymmetric mode, shaken by gusts and buffet, rolls the jet.
        let keas = speed * (atm.rho / RHO0).sqrt() * KT;
        self.structure.update(dt, keas, mach, self.g_load, 0.3 * self.turbulence + self.buffet, self.wing_stores);
        c.cl += self.structure.roll();
        // Gear legs and doors drag; the flaps add lift, drag and a nose-down
        // moment the FLCS trims out.
        let fl = self.flaps;
        c.cx -= (GEAR_DRAG * self.gear + FLAP_CD * fl) * alpha.cos();
        c.cz -= (GEAR_DRAG * self.gear + FLAP_CD * fl) * alpha.sin() + FLAP_CL * fl * alpha.cos();
        c.cm -= 0.025 * fl;
        // Bombs and missiles on the pylons drag along the wind.
        c.cx -= self.stores_drag * alpha.cos();
        c.cz -= self.stores_drag * alpha.sin();
        // A holed wing: less lift, 150 drag counts at the worst, and a roll
        // and yaw toward the damaged side for the ailerons to hold off.
        let w = self.battle.wing;
        if w > 0.0 {
            c.cz *= 1.0 - 0.25 * w;
            c.cx -= 0.015 * w * alpha.cos();
            c.cl += 0.012 * w * self.battle.side;
            c.cn += 0.002 * w * self.battle.side;
        }
        let qs = qbar * self.kind.area();
        let (fx, fy, fz) = (qs * c.cx + self.thrust_n, qs * c.cy, qs * c.cz);
        let (l, m, n) = (qs * aero::SPAN * c.cl, qs * aero::CHORD * c.cm, qs * aero::SPAN * c.cn);
        // The MiG's engines sit 1.1 m either side of the centreline.
        let n = n + full * self.battle.asymmetry * 1.1;
        let (gf, gm) = self.wheel_forces(input, dt);
        let (fx, fy, fz) = (fx + gf.x, fy + gf.y, fz + gf.z);
        let (l, m, n) = (l + gm.x, m + gm.y, n + gm.z);

        // Translation in the world frame; the accelerometer reads the specific
        // force along the body's up axis.
        let accel = self.q.rotate(v3(fy, -fz, -fx)) * (1.0 / self.mass) + v3(0.0, -G0, 0.0);
        self.g_load = -fz / (self.mass * G0);

        // Rotation: body-axis moment equations with the roll-yaw product of
        // inertia and the engine rotor's gyroscopic moment.
        let (ixx, iyy, izz, ixz) = self.inertia();
        let h = aero::ENGINE_MOMENTUM * self.power.min(50.0) / 50.0;
        let gam = ixx * izz - ixz * ixz;
        let xpq = ixz * (ixx - iyy + izz);
        let xqr = izz * (izz - iyy) + ixz * ixz;
        let zpq = (ixx - iyy) * ixx + ixz * ixz;
        let pdot = (xpq * p * q - xqr * q * r + izz * l + ixz * (n + q * h)) / gam;
        let qdot = ((izz - ixx) * p * r - ixz * (p * p - r * r) + m - r * h) / iyy;
        let rdot = (zpq * p * q - xpq * q * r + ixz * l + ixx * (n + q * h)) / gam;
        let (p, q, r) = (p + pdot * dt, q + qdot * dt, r + rdot * dt);
        self.omega = v3(q, -r, -p);
        self.q = self.q.integrate(self.omega, dt);
        self.vel += accel * dt;
        self.pos += self.vel * dt;

        let wind = |cx: f64, cz: f64| (-cz * alpha.cos() + cx * alpha.sin(), -cx * alpha.cos() - cz * alpha.sin());
        let (cl, cd) = wind(c.cx, c.cz);
        self.cl = cl;
        self.cd = cd;
        self.lift_n = qs * cl;
        self.drag_n = qs * cd;
    }

    pub fn update(&mut self, dt: f64, input: &Input) {
        if self.crashed {
            return;
        }
        self.last_pos = self.pos;
        if input.throttle_up {
            self.throttle = clamp(self.throttle + dt * 0.62, 0.0, 1.0);
        }
        if input.throttle_down {
            self.throttle = clamp(self.throttle - dt * 0.62, 0.0, 1.0);
        }
        if input.afterburner {
            self.throttle = 1.0;
        }
        // The gear runs to the handle; the trailing-edge flaps droop with the
        // gear handle down or the ALT FLAPS switch, and blow back up at speed.
        self.gear = approach(self.gear, if input.gear_down { 1.0 } else { 0.0 }, dt / GEAR_TIME);
        let blown = self.airspeed_kt > FLAP_BLOWBACK_KT;
        let flaps = if (input.gear_down || input.flaps) && !blown { 1.0 } else { 0.0 };
        self.flaps = approach(self.flaps, flaps, dt / FLAP_TIME);
        self.wheel_brake = approach(self.wheel_brake, if input.wheel_brake { 1.0 } else { 0.0 }, dt * 4.0);
        if self.gear > 0.0 && self.airspeed_kt > GEAR_LIMIT_KT + 40.0 {
            self.gear_overspeed = true;
        }
        let fueled = self.fuel > 0.0 && self.battle.thrust() > 0.0;
        // Full throttle is the afterburner; anything short of the stop is dry.
        let want_ab = self.throttle >= 1.0 && fueled;
        self.ab = want_ab;
        self.agl = self.pos.y - terrain::floor(self.pos.x, self.pos.z);
        if dt > 0.0 {
            let h = dt / SUBSTEPS as f64;
            for _ in 0..SUBSTEPS {
                self.integrate(h, input, want_ab);
            }
        } else {
            // Instruments before the first step: evaluate without moving.
            let saved = self.clone();
            self.integrate(0.0, input, want_ab);
            let g = self.g_load;
            let (thrust, cl, cd, lift, drag, crit) = (self.thrust_n, self.cl, self.cd, self.lift_n, self.drag_n, self.alpha_crit);
            *self = saved;
            self.g_load = g;
            self.g_smooth = g;
            (self.thrust_n, self.cl, self.cd, self.lift_n, self.drag_n, self.alpha_crit) = (thrust, cl, cd, lift, drag, crit);
        }

        // A wing that breaks ends the flight where it is.
        if let Some(what) = self.structure.failed {
            self.crashed = true;
            self.impact_speed = self.vel.len();
            self.impact_water = false;
            self.collision_label = what;
            return;
        }

        // Fuel: specific consumption of the F110 dry and in afterburner.
        let ab = ((self.power - 50.0) / 50.0).max(0.0);
        let lbf = self.thrust_n.max(0.0) / 4.448222;
        let tsfc = mix(0.661, 1.9, ab);
        self.fuel_flow = if fueled { (lbf * tsfc / 3600.0 * 0.4536).max(0.12) } else { 0.0 };
        self.fuel = (self.fuel - (self.fuel_flow + self.battle.leak) * dt).max(0.0);
        self.mass = self.kind.empty_mass() + self.stores_mass + self.fuel;

        let core = (self.power / 50.0).min(1.0);
        self.engine_spool = core;
        self.ab_level = ab;
        // A dead engine runs down to windmilling in the airflow.
        self.rpm = if fueled { 65.0 + 35.0 * core.sqrt() } else { (99.0 * core).max((self.airspeed_kt * 0.04).min(25.0)) };
        self.ftit = if fueled { 420.0 + 480.0 * core } else { 20.0 + 400.0 * core };
        self.nozzle = 15.0 + 80.0 * ab + 60.0 * (1.0 - core * 2.0).max(0.0);
        self.oil_psi = (self.rpm * 0.65).max(0.0);

        // Derived data for instruments and effects.
        let (speed, alpha, beta, p, _, r) = self.air_state();
        let atm = atmosphere_at(self.pos.y);
        self.speed = speed;
        self.airspeed_kt = calibrated_airspeed(speed, &atm) * 1.94384;
        self.tas_kt = speed * 1.94384;
        self.eas_kt = self.tas_kt * (atm.rho / 1.225).sqrt();
        self.agl = self.pos.y - terrain::floor(self.pos.x, self.pos.z);
        self.alpha = alpha;
        self.alpha_deg = alpha * RAD;
        self.beta = beta;
        self.mach = speed / atm.sound;
        self.qbar = 0.5 * atm.rho * speed * speed;
        self.vs = self.vel.y;
        self.g_avail = (self.kind.g_limit(self.mass) * self.battle.strength()).min(self.qbar * self.kind.area() * (aero::cl_max() + FLAP_CL * self.flaps) / (self.mass * G0)).max(0.0);
        self.g_smooth += (self.g_load - self.g_smooth) * clamp(dt * 4.0, 0.0, 1.0);
        self.max_g = self.max_g.max(self.g_smooth);
        self.min_g = self.min_g.min(self.g_smooth);
        self.stall = self.stall_left.max(self.stall_right);
        // Transonic buffet: shock-induced separation once the lift coefficient
        // passes the buffet-onset boundary, which sags towards Mach 0.95, and
        // the shocks shuffling over the wing through Mach 1 at any lift.
        let onset_cl = aero::curve(&BUFFET_ONSET, self.mach);
        let shock = smoothstep(onset_cl, onset_cl + 0.4, self.cl.abs()) * (1.0 - smoothstep(1.1, 1.3, self.mach));
        let shuffle = smoothstep(0.93, 0.98, self.mach) * (1.0 - smoothstep(1.02, 1.08, self.mach));
        let mach_buffet = 0.6 * shock + 0.25 * shuffle;
        let onset = smoothstep(self.alpha_crit - 14.0, self.alpha_crit - 2.0, self.alpha_deg.abs());
        self.buffet = clamp(onset * 0.7 + self.stall + mach_buffet, 0.0, 1.0);
        let uncommanded = (p - self.flcs.p_cmd * DEG).abs() + 1.5 * r.abs();
        let departing = smoothstep(0.5, 1.5, uncommanded) * smoothstep(0.1, 0.5, self.stall);
        self.departure += (departing - self.departure) * clamp(dt * 3.0, 0.0, 1.0);
        // Low-altitude roughness is recorded for camera shake and audio only.
        self.turbulence = 1.0 - smoothstep(300.0, 1700.0, self.agl);
        self.brake = self.flcs.brake;
        self.surf_pitch = -self.flcs.el / STAB_MAX;
        self.surf_roll = -self.flcs.ail / FLAPERON_MAX;
        self.surf_yaw = -self.flcs.rdr / RUDDER_MAX;
        let now = self.q.axes();
        self.heading = wrap(now.forward.x.atan2(-now.forward.z) * RAD, 360.0);
        self.pitch = clamp(now.forward.y, -1.0, 1.0).asin() * RAD;
        self.bank = (-now.right.y).atan2(now.up.y) * RAD;
        self.afterburner_visual += (self.ab_level - self.afterburner_visual) * clamp(dt * 5.0, 0.0, 1.0);

        self.detect_terrain_collision();
    }
}

/// One panel's separation, 0 attached to 1 stalled, with hysteresis: past
/// the critical AoA the flow lets go within a fraction of a second, and it
/// only reattaches a few degrees below it. Negative AoA stalls earlier.
fn separate(sep: f64, alpha: f64, crit: f64, dt: f64) -> f64 {
    let crit = if alpha >= 0.0 { crit } else { 0.6 * crit };
    let excess = alpha.abs() - crit;
    let target = clamp(sep, smoothstep(-0.5, 5.0, excess), smoothstep(-4.5, 0.5, excess));
    let rate = if target > sep { 8.0 } else { 2.5 };
    sep + (target - sep) * clamp(dt * rate, 0.0, 1.0)
}

/// Frozen low-level turbulence, -1..1. Flying through it faster makes the
/// gusts come quicker, not stronger.
fn gust(p: V3) -> f64 {
    0.55 * (p.x * 0.021 + (p.z * 0.017).sin() * 1.3).sin() + 0.3 * (p.z * 0.033 + p.x * 0.011).sin() + 0.15 * ((p.x - p.z) * 0.067 + p.y * 0.05).sin()
}

/// True when the step from `last_pos` to `pos` crossed the gate disc.
pub fn gate_crossed(a: &Aircraft, gate: &Gate) -> bool {
    let d0 = (a.last_pos - gate.pos).dot(gate.axis);
    let d1 = (a.pos - gate.pos).dot(gate.axis);
    if !(d0 < 0.0 && d1 >= 0.0) {
        return false;
    }
    let denom = d0 - d1;
    let t = d0 / if denom == 0.0 { 1.0 } else { denom };
    let crossing = a.last_pos.lerp(a.pos, t);
    let rel = crossing - gate.pos;
    let lateral = rel - gate.axis * rel.dot(gate.axis);
    lateral.len() <= gate.radius * 0.96
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f64 = 1.0 / 120.0;

    fn aircraft(alt: f64, speed: f64) -> Aircraft {
        let mut a = Aircraft::new(&Mission::new());
        let t = trim(alt, speed, a.mass);
        a.pos = v3(0.0, alt, 0.0);
        a.vel = v3(0.0, 0.0, -speed);
        a.q = Quat::from_forward_up(v3(0.0, 0.0, -1.0), V3::Y).mul(Quat::axis_angle(V3::X, t.alpha));
        a.flcs = Flcs { alpha_f: t.alpha * RAD, el: t.el, ..Default::default() };
        a.power = t.power;
        a.throttle = t.throttle;
        a.update(0.0, &Input::default());
        a
    }
    fn run(mut a: Aircraft, seconds: f64, input: Input) -> Aircraft {
        for _ in 0..(seconds / DT).round() as usize {
            a.update(DT, &input);
        }
        a
    }
    fn full() -> Input {
        Input { roll_right: true, afterburner: true, throttle_up: true, ..Default::default() }
    }
    fn roll_rate(a: &Aircraft) -> f64 {
        -a.omega.z * RAD
    }

    #[test]
    fn initial_instruments_are_valid_before_the_first_frame() {
        let a = Aircraft::new(&Mission::new());
        assert!(a.speed > 200.0 && a.airspeed_kt > 300.0);
        assert!(a.mach > 0.5 && a.agl > 100.0);
        assert!(a.rpm > 80.0);
        assert!((a.g_load - 1.0).abs() < 0.05, "trimmed start: {:.3} g", a.g_load);
    }

    #[test]
    fn trim_holds_wings_level_flight_hands_off() {
        for (alt, speed) in [(1500.0, 220.0), (3000.0, 160.0), (9000.0, 250.0)] {
            let a0 = aircraft(alt, speed);
            let a = run(a0.clone(), 10.0, Input::default());
            assert!(a.vs.abs() < 4.0, "{alt} m {speed} m/s: climbing {:.2} m/s", a.vs);
            assert!((a.speed - speed).abs() < 4.0, "speed {:.1}", a.speed);
            assert!((a.g_load - 1.0).abs() < 0.05 && a.bank.abs() < 1.0, "{:.3} g, bank {:.2}", a.g_load, a.bank);
        }
    }

    #[test]
    fn altitude_reduces_qbar_lift_and_thrust() {
        let low = run(aircraft(3000.0, 220.0), 0.5, full());
        let high = run(aircraft(10000.0, 220.0), 0.5, full());
        assert!(high.qbar < low.qbar * 0.5);
        assert!(high.g_avail < low.g_avail);
        assert!(high.thrust_n < low.thrust_n);
    }

    #[test]
    fn airspeed_changes_attainable_g_and_roll() {
        let i = Input { roll_right: true, pitch_up: true, ..Default::default() };
        let slow = run(aircraft(3000.0, 100.0), 1.0, i);
        let fast = run(aircraft(3000.0, 250.0), 1.0, i);
        assert!(slow.g_avail < fast.g_avail);
        assert!(slow.g_load < fast.g_load);
        assert!(roll_rate(&slow) < roll_rate(&fast));
    }

    /// Battle damage costs the jet: half its thrust gone, fuel pouring out,
    /// a holed wing rolling it with the stick at rest, the surfaces short of
    /// their travel and less g to pull; a wrecked engine gives nothing.
    #[test]
    fn battle_damage_costs_performance() {
        let sound = aircraft(3000.0, 220.0);
        let mut hit = sound.clone();
        hit.battle = BattleDamage { engine: 0.5, leak: 3.0, wing: 0.6, side: 1.0, controls: 0.5, airframe: 0.6, ..Default::default() };
        let mil = Input { throttle_up: true, ..Default::default() };
        let (f0, f1) = (sound.fuel, hit.fuel);
        let sound = run(sound, 2.0, mil);
        let hit = run(hit, 2.0, mil);
        assert!(hit.thrust_n < sound.thrust_n * 0.6, "{} against {} N", hit.thrust_n, sound.thrust_n);
        assert!((f1 - hit.fuel) - (f0 - sound.fuel) > 5.0, "the leak drains fuel");
        assert!(hit.g_avail < sound.g_avail * 0.8);
        assert!(hit.flcs.ail.abs() <= FLAPERON_MAX * hit.battle.authority() + 1e-9);
        assert!(roll_rate(&hit).abs() > roll_rate(&sound).abs() + 1.0, "the wing rolls the jet");
        assert_eq!(BattleDamage { engine: 1.0, ..Default::default() }.thrust(), 0.0);
    }

    #[test]
    fn fuel_exhaustion_removes_thrust_and_flow() {
        let mut a = aircraft(3000.0, 220.0);
        a.fuel = 0.0;
        let a = run(a, 1.0, Input { afterburner: true, throttle_up: true, ..Default::default() });
        assert_eq!(a.thrust_n, 0.0);
        assert_eq!(a.fuel_flow, 0.0);
        assert!(!a.ab);
        assert_eq!(a.fuel, 0.0);
    }

    #[test]
    fn full_throttle_lights_the_afterburner_and_it_lags() {
        assert!(Mission::new().start_throttle < 1.0, "the mission starts dry");
        let mut a = aircraft(3000.0, 220.0);
        a.throttle = 0.997;
        a.power = 50.0 * a.throttle;
        a.update(DT, &Input::default());
        assert!(!a.ab);
        a.update(DT, &Input { throttle_up: true, ..Default::default() });
        assert!(a.ab, "pushing the throttle to the stop lights it");
        assert!(a.ab_level < 0.1);
        let a = run(a, 1.0, Input::default());
        assert!(a.ab && a.ab_level > 0.8, "it stays lit with the throttle at the stop");
        let a = run(a, 0.1, Input { throttle_down: true, ..Default::default() });
        assert!(!a.ab, "one notch back is military power");
        let mut b = aircraft(3000.0, 220.0);
        b.throttle = 0.4;
        b.update(DT, &Input { afterburner: true, ..Default::default() });
        assert!(b.ab && b.throttle == 1.0, "the afterburner key slams the throttle forward");
    }

    #[test]
    fn inverted_bank_reads_near_180() {
        let mut a = aircraft(3000.0, 220.0);
        a.q = a.q.mul(Quat::axis_angle(V3::Z, PI));
        let a = run(a, DT, Input::default());
        assert!(a.bank.abs() > 170.0);
    }

    /// The FLCS commands load factor: neutral stick holds 1 g, a full pull
    /// reaches 9 g in about a second and does not overshoot the limit.
    #[test]
    fn load_factor_follows_the_stick() {
        let mut a = aircraft(1000.0, 240.0);
        let pull = Input { pitch_up: true, ..Default::default() };
        let mut peak: f64 = 0.0;
        let mut t9 = None;
        for i in 0..240 {
            a.update(DT, &pull);
            peak = peak.max(a.g_load);
            if t9.is_none() && a.g_load > 8.5 {
                t9 = Some(i as f64 * DT);
            }
        }
        let t9 = t9.expect("never reached 8.5 g");
        assert!(t9 < 1.3, "8.5 g after {t9:.2} s");
        assert!(peak > 8.8 && peak < G_LIMIT + 0.4, "peak {peak:.2} g");
        // Holding 9 g bleeds speed, so the g decays as the AoA limit is reached.
        assert!(a.g_load > 6.5 && a.alpha_deg < 25.5, "{:.2} g at {:.1} deg after 2 s", a.g_load, a.alpha_deg);
        // Released, it unloads at once, and the attitude hold that takes over
        // near 1 g lets the path settle under the nose.
        let a = run(a, 1.0, Input::default());
        assert!((a.g_load - 1.0).abs() < 0.6, "1 s after release: {:.2} g", a.g_load);
        let a = run(a, 1.5, Input::default());
        assert!((a.g_load - 1.0).abs() < 0.4, "2.5 s after release: {:.2} g", a.g_load);
    }

    #[test]
    fn a_held_pull_near_the_aoa_limit_does_not_bob_the_nose() {
        let mut a = aircraft(3000.0, 130.0);
        let pull = Input { pitch_axis: 0.3, ..Default::default() };
        let mut reversals = 0;
        let mut last_q: f64 = 1.0;
        for i in 0..(8.0 / DT) as usize {
            a.update(DT, &pull);
            if i as f64 * DT > 0.5 {
                if a.omega.x * last_q < 0.0 && a.omega.x.abs() > 2.0 * DEG {
                    reversals += 1;
                }
                if a.omega.x.abs() > 2.0 * DEG {
                    last_q = a.omega.x;
                }
                assert!(a.flcs.el.abs() < 12.0, "stabilator at {:.1} deg after {:.1} s", a.flcs.el, i as f64 * DT);
            }
        }
        assert!(reversals == 0, "the nose reversed {reversals} times under a steady pull");
    }

    #[test]
    fn releasing_a_pull_settles_on_one_g_without_dropping_the_nose() {
        let a = run(aircraft(3000.0, 200.0), 4.0, Input { pitch_axis: 0.3, ..Default::default() });
        let mut a = a;
        let mut low: f64 = 9.0;
        for _ in 0..(4.0 / DT) as usize {
            a.update(DT, &Input::default());
            low = low.min(a.g_load);
        }
        assert!(low > 0.85, "dipped to {low:.2} g after the release");
        assert!((a.g_load - 1.0).abs() < 0.1, "{:.2} g 4 s after the release", a.g_load);
    }

    #[test]
    fn the_nose_stays_where_a_pitch_change_leaves_it() {
        for (stick, speed) in [(0.15, 200.0), (0.3, 200.0), (0.2, 130.0), (-0.3, 220.0)] {
            let mut a = run(aircraft(3000.0, speed), 2.0, Input { pitch_axis: stick, ..Default::default() });
            let theta = |a: &Aircraft| a.q.axes().forward.y.asin() * RAD;
            let left = theta(&a);
            let (mut lo, mut hi): (f64, f64) = (left, left);
            for _ in 0..(6.0 / DT) as usize {
                a.update(DT, &Input::default());
                lo = lo.min(theta(&a));
                hi = hi.max(theta(&a));
            }
            assert!(lo > left - 0.6 && hi < left + 1.5, "stick {stick} at {speed} m/s: left at {left:.1}, then {lo:.1} to {hi:.1} deg");
        }
    }

    #[test]
    fn aoa_limiter_holds_the_slow_jet_near_25_degrees() {
        let mut a = aircraft(3000.0, 120.0);
        let pull = Input { pitch_up: true, ..Default::default() };
        let mut peak: f64 = 0.0;
        for _ in 0..480 {
            a.update(DT, &pull);
            peak = peak.max(a.alpha_deg);
        }
        assert!(peak > 24.0 && peak < 27.0, "peak AoA {peak:.1}");
        assert!((a.alpha_deg - 25.0).abs() < 1.0, "held AoA {:.1}", a.alpha_deg);
    }

    #[test]
    fn negative_g_is_limited() {
        let mut a = aircraft(3000.0, 330.0);
        let mut low: f64 = 1.0;
        for _ in 0..240 {
            a.update(DT, &Input { pitch_down: true, ..Default::default() });
            low = low.min(a.g_load);
        }
        assert!(low < -2.5 && low > NEG_G_LIMIT - 0.4, "push {low:.2} g");
        assert!(a.alpha_deg > -7.0, "push AoA {:.1}", a.alpha_deg);
    }

    #[test]
    fn full_lateral_stick_commands_a_coordinated_300_degree_roll() {
        let mut a = aircraft(3000.0, 230.0);
        let mut beta: f64 = 0.0;
        for _ in 0..120 {
            a.update(DT, &Input { roll_right: true, ..Default::default() });
            beta = beta.max(a.beta.abs() * RAD);
        }
        let p = roll_rate(&a);
        assert!(p > 240.0 && p < ROLL_RATE_MAX + 15.0, "roll rate {p:.0} deg/s");
        assert!(beta < 4.0, "sideslip {beta:.1} deg");
        let a = run(a, 0.6, Input::default());
        assert!(roll_rate(&a).abs() < 15.0, "roll stops: {:.1} deg/s", roll_rate(&a));
    }

    #[test]
    fn airbrake_adds_drag() {
        let clean = run(aircraft(3000.0, 250.0), 3.0, Input::default());
        let braked = run(aircraft(3000.0, 250.0), 3.0, Input { airbrake: true, ..Default::default() });
        assert!(braked.drag_n > clean.drag_n);
        assert!(braked.speed < clean.speed);
    }

    #[test]
    fn engine_instruments_follow_spool_and_consumption() {
        let a0 = aircraft(3000.0, 220.0);
        let fuel = a0.fuel;
        let a = run(a0, 6.0, Input { throttle_up: true, afterburner: true, ..Default::default() });
        assert!(a.fuel < fuel);
        assert!(a.rpm > 90.0 && a.ftit > 800.0 && a.nozzle > 80.0 && a.oil_psi > 50.0);
        assert!(a.fuel_flow > 4.0, "full afterburner burns {:.1} kg/s", a.fuel_flow);
    }

    #[test]
    fn simulation_stays_finite_across_the_envelope() {
        for alt in [1500.0, 6000.0, 12000.0, 16000.0] {
            for speed in [90.0, 220.0, 400.0, 600.0] {
                let a = run(aircraft(alt, speed), 12.0, Input { pitch_axis: 0.25, roll_axis: 0.07, ..Default::default() });
                assert!(a.pos.is_finite() && a.vel.is_finite());
                assert!(a.g_load.is_finite() && a.alpha.is_finite() && a.airspeed_kt.is_finite());
                assert!((a.q.len() - 1.0).abs() < 1e-6);
                assert!(a.fuel >= 0.0 && a.g_avail <= 9.0);
            }
        }
    }

    /// Full-afterburner level-flight Mach from the trim solution: the
    /// published Block 50 figure is about Mach 2 at 40,000 ft.
    #[test]
    fn full_afterburner_level_speed_matches_reference() {
        let mass = EMPTY_MASS + STORES_MASS + FUEL_MAX * 0.5;
        let mut mach = 0.0;
        for feet in [0.0, 10000.0, 20000.0, 30000.0, 40000.0] {
            let alt = feet * 0.3048;
            let sound = atmosphere_at(alt).sound;
            let (mut low, mut high) = (0.6, 2.6);
            for _ in 0..30 {
                let mid = (low + high) / 2.0;
                if trim(alt, mid * sound, mass).power < 99.9 {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            mach = (low + high) / 2.0;
            println!("{feet} ft: level full-AB Mach {mach:.3}");
        }
        assert!(mach > 1.85 && mach < 2.3, "equilibrium Mach {mach}");
    }

    #[test]
    fn gate_counts_only_forward_crossings_inside_the_ring() {
        let m = Mission::new();
        let g = &m.gates[0];
        let mut a = Aircraft::new(&m);
        let side = g.axis.cross(V3::Y).norm();
        let mut cross = |offset: V3, forward: bool| {
            let (from, to) = if forward { (-1.0, 1.0) } else { (1.0, -1.0) };
            a.last_pos = g.pos + offset + g.axis * from;
            a.pos = g.pos + offset + g.axis * to;
            gate_crossed(&a, g)
        };
        assert!(cross(V3::ZERO, true));
        assert!(cross(side * (GATE_RADIUS * 0.9), true));
        assert!(!cross(side * (GATE_RADIUS * 1.02), true));
        assert!(!cross(V3::ZERO, false));
    }

    #[test]
    fn score_counts_gates_and_remaining_time() {
        let mut m = Mission::new();
        m.passed = 4;
        m.time_left = 37.4;
        assert_eq!(m.final_score(), (4000.0f64 + 37.4 * 25.0).round());
        m.time_left = -3.0;
        assert_eq!(m.final_score(), 4000.0);
    }

    #[test]
    fn mission_route_has_ten_gates_above_the_gorge_floor() {
        let m = Mission::new();
        assert_eq!(m.gates.len(), 10);
        for g in &m.gates {
            assert!(g.pos.y - terrain_at(g.pos.x, g.pos.z) > 100.0);
        }
        assert!(m.start_pos.y > terrain_at(m.start_pos.x, m.start_pos.z) + 150.0);
    }

    #[test]
    fn mach_altitude_and_sideslip_lower_the_critical_aoa() {
        let slow = aircraft(1000.0, 120.0);
        let fast_high = aircraft(11000.0, 260.0);
        assert!(slow.alpha_crit > 26.5, "slow and low: {:.1} deg", slow.alpha_crit);
        assert!(fast_high.alpha_crit < slow.alpha_crit - 4.0, "Mach 0.88 at 11 km: {:.1} deg", fast_high.alpha_crit);
        let mut slipping = slow.clone();
        slipping.vel = Quat::axis_angle(V3::Y, -10.0 * DEG).rotate(slipping.vel);
        slipping.update(0.0, &Input::default());
        assert!(slipping.alpha_crit < slow.alpha_crit - 1.5, "10 deg sideslip: {:.1} deg", slipping.alpha_crit);
    }

    /// In smooth air a straight full pull stops just short of the stall, but
    /// rudder at that AoA stalls the windward wing and drops it.
    #[test]
    fn rudder_at_the_aoa_limit_stalls_the_windward_wing() {
        let pull = Input { pitch_up: true, ..Default::default() };
        let mut a = run(aircraft(3000.0, 110.0), 3.0, pull);
        assert!(a.stall < 0.02, "straight pull: stall {:.2} at {:.1} deg", a.stall, a.alpha_deg);
        let g = a.g_load;
        a = run(a, 2.0, Input { yaw_right: true, ..pull });
        assert!(a.beta < -6.0 * DEG, "sideslip {:.1} deg", a.beta * RAD);
        assert!(a.stall_left > 0.4 && a.stall_right < 0.1, "left {:.2}, right {:.2}", a.stall_left, a.stall_right);
        assert!(a.g_load < g - 0.3, "lift kept: {:.2} g", a.g_load);
        let a = run(a, 1.5, Input { yaw_right: true, ..pull });
        assert!(a.bank < -5.0, "left wing did not drop: bank {:.1}", a.bank);
    }

    /// High and fast the limiter's 25 degrees lies past the critical AoA: a
    /// full pull stalls the wing into heavy buffet, and letting go recovers.
    #[test]
    fn high_altitude_pull_stalls_and_recovers() {
        let a = run(aircraft(11000.0, 250.0), 3.0, Input { pitch_up: true, ..Default::default() });
        assert!(a.stall > 0.3 && a.buffet > 0.6, "stall {:.2}, buffet {:.2} at {:.1} deg", a.stall, a.buffet, a.alpha_deg);
        let a = run(a, 4.0, Input::default());
        assert!(a.stall < 0.05, "still stalled after release: {:.2} at {:.1} deg", a.stall, a.alpha_deg);
    }

    /// Heavy stores under the wings bring the flutter speed, sagging in the
    /// transonic dip, down into reach: the clean jet shrugs off Mach 1 at
    /// 1,500 m, the loaded one flutters until the wing breaks.
    #[test]
    fn loaded_wings_flutter_through_mach_one_low_down() {
        let burner = Input { afterburner: true, throttle_up: true, ..Default::default() };
        let mut clean = aircraft(1500.0, 330.0);
        // Over the low ground west of the gorge.
        clean.pos = v3(-2000.0, 1500.0, -3000.0);
        clean.update(0.0, &Input::default());
        let mut loaded = clean.clone();
        loaded.wing_stores = 1500.0;
        for _ in 0..(20.0 / DT) as usize {
            clean.update(DT, &burner);
            loaded.update(DT, &burner);
        }
        assert!(!clean.crashed && clean.structure.amplitude < 0.02, "clean: {:.3} m, {}", clean.structure.amplitude, clean.collision_label);
        assert!(loaded.crashed && loaded.collision_label == "wing flutter", "loaded: margin {:.2}, {:.3} m", loaded.structure.margin, loaded.structure.amplitude);
    }

    #[test]
    fn separation_costs_lift_and_reattaches_with_hysteresis() {
        let mut sep = 0.0;
        for _ in 0..60 {
            sep = separate(sep, 31.0, 28.0, DT);
        }
        assert!(sep > 0.4, "3 deg past critical: {sep:.2}");
        let held = (0..120).fold(sep, |s, _| separate(s, 26.0, 28.0, DT));
        assert!(held > 0.2, "2 deg below critical lets go too early: {held:.2}");
        let gone = (0..240).fold(sep, |s, _| separate(s, 20.0, 28.0, DT));
        assert!(gone < 0.01, "8 deg below critical: {gone:.2}");
    }

    fn on_runway() -> Aircraft {
        terrain::init();
        let mut m = Mission::new();
        let _ = crate::combat::Combat::new(crate::combat::MissionKind::Airfield, &mut m);
        Aircraft::new(&m)
    }

    #[test]
    fn parked_jet_rests_on_its_struts() {
        let mut a = on_runway();
        let input = Input { gear_down: true, wheel_brake: true, ..Default::default() };
        for _ in 0..(6.0 / DT) as usize {
            a.update(DT, &input);
        }
        assert!(!a.crashed, "{}", a.collision_label);
        assert!(a.on_ground && a.speed < 0.3, "rolling at {:.2} m/s", a.speed);
        for (i, c) in a.compression.iter().enumerate() {
            assert!(*c > 0.04 && *c < 0.2, "{} compressed {c:.3} m", WHEELS[i].name);
        }
        assert!(a.pitch.abs() < 1.5 && a.bank.abs() < 0.5, "pitch {:.1} bank {:.1}", a.pitch, a.bank);
    }

    #[test]
    fn takeoff_roll_rotates_and_lifts_off() {
        let mut a = on_runway();
        let start = a.pos;
        let mut lift_off = None;
        for k in 0..(60.0 / DT) as usize {
            let rotate = a.airspeed_kt > 150.0 && a.pitch < 10.0;
            let input = Input { gear_down: true, afterburner: true, pitch_axis: if rotate { 0.35 } else { 0.0 }, ..Default::default() };
            a.update(DT, &input);
            assert!(!a.crashed, "{} at {:.0} kt", a.collision_label, a.airspeed_kt);
            if lift_off.is_none() && a.agl > 10.0 {
                lift_off = Some((k as f64 * DT, (a.pos - start).len(), a.airspeed_kt));
            }
        }
        let (t, d, kt) = lift_off.expect("never left the ground");
        assert!(d < 1500.0 && t < 25.0, "lift-off after {t:.1} s and {d:.0} m at {kt:.0} kt");
    }

    /// A stabilised approach to runway A: 3 degree path at 150 KCAS with the
    /// gear and flaps down, a flare, touchdown on the mains and a full stop
    /// on the brakes, all on the runway.
    #[test]
    fn approach_touchdown_and_stop() {
        terrain::init();
        let e = terrain::base_elevation();
        let (dx, dz) = terrain::runway_dir();
        let fwd = v3(dx, 0.0, dz);
        let (tx, tz) = terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH + 320.0, 0.0);
        let aim = v3(tx, e + 1.6, tz);
        let start = aim - fwd * 3000.0 + V3::Y * (3000.0 * (3.0 * DEG).tan());
        let mut a = crate::combat::spawn(start, dx.atan2(-dz), 78.0);
        let mut touched = false;
        let mut sink = 0.0;
        for _ in 0..(90.0 / DT) as usize {
            let flare = a.agl < 9.0;
            let (pitch, roll) = if touched {
                (0.0, 0.0)
            } else {
                let target = if flare { a.pos + fwd * 200.0 + V3::Y * (1.0 - a.agl) } else { aim + fwd * 400.0 };
                crate::ai::steer(&a, (target - a.pos).norm(), 2.0)
            };
            let throttle_up = !touched && a.airspeed_kt < 150.0;
            let input = Input { gear_down: true, pitch_axis: pitch, roll_axis: roll, throttle_up, throttle_down: !throttle_up, wheel_brake: touched, ..Default::default() };
            let vs = a.vs;
            a.update(DT, &input);
            assert!(!a.crashed, "{} at {:.0} kt, sink {:.1} m/s", a.collision_label, a.airspeed_kt, -vs);
            if a.on_ground && !touched {
                touched = true;
                sink = -vs;
            }
            if touched && a.speed < 1.0 {
                break;
            }
        }
        assert!(touched && a.speed < 1.0, "never stopped: {:.0} m/s, agl {:.1}", a.speed, a.agl);
        let (u, v) = terrain::runway_local(a.pos.x, a.pos.z);
        assert!(u.abs() < terrain::RUNWAY_HALF_LENGTH && v.abs() < terrain::RUNWAY_HALF_WIDTH, "stopped off the runway at u {u:.0} v {v:.0}");
        assert!(sink < 3.0, "touchdown sink {sink:.1} m/s");
    }
}

