//! The fight around the player: master modes and the stores on the
//! stations, the radar lock and the Sidewinder seeker, bandits flown by the
//! AI, ground targets with their guns and SAMs, every round, missile and bomb
//! in the air, damage, and the objectives of the DOGFIGHT and STRIKE
//! missions. It steps inside the fixed 120 Hz loop after the player's jet has
//! moved, and reports what happened as events for the effects and the sound.
use crate::ai::{Picture, Pilot};
use crate::math::*;
use crate::sim::{spec::STORES_MASS, trim, Aircraft, Flcs, Gate, Input, Mission, G0};
use crate::targets::{self, Kind, Site};
use crate::terrain::{self, river_x};
use crate::weapons::*;
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MissionKind {
    Gates,
    Dogfight,
    Strike,
    /// Take off from the airbase, fly out to a steerpoint, come back and land.
    Airfield,
}

impl MissionKind {
    pub const ALL: [MissionKind; 4] = [MissionKind::Gates, MissionKind::Dogfight, MissionKind::Strike, MissionKind::Airfield];

    pub fn name(self) -> &'static str {
        match self {
            MissionKind::Gates => "GATE RUN",
            MissionKind::Dogfight => "DOGFIGHT",
            MissionKind::Strike => "STRIKE",
            MissionKind::Airfield => "AIRFIELD",
        }
    }

    pub fn parse(s: &str) -> Option<MissionKind> {
        match s {
            "gates" | "gate" | "run" => Some(MissionKind::Gates),
            "dogfight" | "aa" => Some(MissionKind::Dogfight),
            "strike" | "ag" => Some(MissionKind::Strike),
            "airfield" | "circuit" | "land" => Some(MissionKind::Airfield),
            _ => None,
        }
    }

    /// Mission clock, seconds.
    pub fn time(self) -> f64 {
        match self {
            MissionKind::Gates => crate::sim::MISSION_TIME,
            MissionKind::Dogfight => 360.0,
            MissionKind::Strike => 420.0,
            MissionKind::Airfield => 600.0,
        }
    }
}

/// HUD master mode, one per weapon, and NAV with the master arm safe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Nav,
    /// Gun with the EEGS funnel and LCOS pipper.
    Gun,
    /// Short-range missile, the AIM-9.
    Srm,
    /// Medium-range missile, the AIM-120.
    Mrm,
    /// Bombs on the continuously computed impact point.
    Ccip,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Nav => "NAV",
            Mode::Gun => "GUN",
            Mode::Srm => "SRM",
            Mode::Mrm => "MRM",
            Mode::Ccip => "CCIP",
        }
    }
    pub fn air_to_ground(self) -> bool {
        self == Mode::Ccip
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Store {
    Aim9,
    Aim120,
    Mk82,
}

impl Store {
    pub fn mass(self) -> f64 {
        match self {
            Store::Aim9 => AIM9M.mass,
            Store::Aim120 => AIM120C.mass,
            Store::Mk82 => MK82_MASS,
        }
    }
    /// Zero-lift drag coefficient (on the 27.87 m^2 wing) of the store and
    /// its interference under a wing; a wingtip AIM-9 is in the clean data.
    fn drag(self) -> f64 {
        match self {
            Store::Aim9 => 0.0005,
            Store::Aim120 => 0.0008,
            Store::Mk82 => 0.0013,
        }
    }
}

/// Launch rails, pylons and racks, kg: STORES_MASS less its two AIM-9s.
const RAILS: f64 = STORES_MASS - 2.0 * 85.5;

/// One carriage position: a store hung at a body-axis point (a missile's
/// nose, a bomb's centre). Wingtip rails carry their store in the clean drag.
#[derive(Clone, Copy, Debug)]
pub struct Station {
    pub store: Store,
    pub at: V3,
    pub loaded: bool,
    pub tip: bool,
}

/// What the loadout screen sets before a sortie: the stores on the two
/// pylon pairs and the internal fuel.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Config {
    /// Stations 3 and 7.
    pub outboard: Option<Store>,
    /// Stations 4 and 6, the heavy inboard pylons.
    pub inboard: Option<Store>,
    /// Internal fuel, fraction of the 7,000 lb.
    pub fuel: f64,
}

impl Config {
    pub fn for_mission(kind: MissionKind) -> Config {
        match kind {
            MissionKind::Gates => Config { outboard: None, inboard: None, fuel: 1.0 },
            MissionKind::Airfield => Config { outboard: None, inboard: None, fuel: 0.6 },
            MissionKind::Dogfight => Config { outboard: Some(Store::Aim120), inboard: None, fuel: 1.0 },
            MissionKind::Strike => Config { outboard: Some(Store::Mk82), inboard: Some(Store::Mk82), fuel: 1.0 },
        }
    }

    /// Next choice for a pylon pair: missiles and bombs outboard, bombs inboard.
    pub fn cycle(&mut self, inboard: bool) {
        let order: &[Option<Store>] = if inboard { &[None, Some(Store::Mk82), Some(Store::Aim120)] } else { &[None, Some(Store::Aim9), Some(Store::Aim120), Some(Store::Mk82)] };
        let slot = if inboard { &mut self.inboard } else { &mut self.outboard };
        let i = order.iter().position(|s| s == slot).unwrap_or(0);
        *slot = order[(i + 1) % order.len()];
    }

    pub fn cycle_fuel(&mut self) {
        self.fuel = if self.fuel >= 0.99 { 0.5 } else if self.fuel < 0.6 { 0.75 } else { 1.0 };
    }

    /// Gross weight (kg) and total drag coefficient of the stores.
    pub fn weight(&self) -> (f64, f64) {
        let l = Loadout::configure(self);
        (crate::sim::spec::EMPTY_MASS + l.mass() + crate::sim::spec::FUEL_MAX * self.fuel, l.drag())
    }
}

#[derive(Clone, Debug)]
pub struct Loadout {
    /// In release order, alternating sides.
    pub stations: Vec<Station>,
    pub rounds: u32,
    pub chaff: u32,
    pub flares: u32,
}

/// Wingtip AIM-9 noses (stations 1 and 9).
fn tips() -> [Station; 2] {
    let t = crate::airframe::AIM9_TIP;
    [-1.0, 1.0].map(|s| Station { store: Store::Aim9, at: v3(s * t.x, t.y, t.z), loaded: true, tip: true })
}

impl Loadout {
    pub fn for_mission(kind: MissionKind) -> Loadout {
        Loadout::configure(&Config::for_mission(kind))
    }

    /// Stores hung as the loadout screen chose: the wingtip AIM-9s always,
    /// the outboard pair on stations 3 and 7, the inboard pair on 4 and 6.
    pub fn configure(cfg: &Config) -> Loadout {
        let mut stations = tips().to_vec();
        for (pick, x) in [(cfg.outboard, 3.50), (cfg.inboard, 2.30)] {
            let Some(store) = pick else { continue };
            // Missile noses forward of the pylon, bombs centred under it.
            let (y, z) = match store {
                Store::Aim9 => (-0.03, x * 0.5 - 0.83),
                Store::Aim120 => (-0.06, x * 0.5 - 1.23),
                Store::Mk82 => (-0.08, x * 0.4 + 0.93),
            };
            for s in [-1.0, 1.0] {
                stations.push(Station { store, at: v3(s * x, y, z), loaded: true, tip: false });
            }
        }
        Loadout { stations, rounds: GUN_ROUNDS, chaff: CHAFF, flares: FLARES }
    }

    pub fn count(&self, store: Store) -> u32 {
        self.stations.iter().filter(|s| s.loaded && s.store == store).count() as u32
    }

    pub fn next(&self, store: Store) -> Option<usize> {
        self.stations.iter().position(|s| s.loaded && s.store == store)
    }

    /// Mass on the stations, with the spent ammunition taken off.
    pub fn mass(&self) -> f64 {
        RAILS + self.stations.iter().filter(|s| s.loaded).map(|s| s.store.mass()).sum::<f64>() - (GUN_ROUNDS - self.rounds) as f64 * ROUND_EXPENDED
    }

    /// Mass hung on the underwing pylons, which lowers the flutter speed.
    pub fn wing_mass(&self) -> f64 {
        self.stations.iter().filter(|s| s.loaded && !s.tip).map(|s| s.store.mass()).sum()
    }

    pub fn drag(&self) -> f64 {
        self.stations.iter().filter(|s| s.loaded && !s.tip).map(|s| s.store.drag()).sum()
    }
}

/// SA-6 3M9 missile: 599 kg, 5.8 m, 335 mm, solid boost then an integral
/// ramjet to Mach 2.8; 57 kg warhead; semi-active, so it guides only while
/// its radar lives. Range 3 to 22 km.
pub const SA6: Spec = Spec {
    name: "3M9",
    mass: 599.0,
    length: 5.8,
    diameter: 0.335,
    propellant: 240.0,
    motor: [(85000.0, 4.0), (14000.0, 18.0)],
    drop: 0.0,
    cd0: 0.30,
    cna: 25.0,
    alpha_max: 15.0 * DEG,
    g_limit: 20.0,
    nav: 3.5,
    gimbal: 45.0 * DEG,
    acquire: 5.0 * DEG,
    fuze: 14.0,
    lethal: 6.0,
    outer: 22.0,
    arm: 1.5,
    life: 35.0,
    min_range: 3000.0,
    max_range: 22000.0,
    active: false,
    infrared: false,
};

/// Smoke left in the air: a missile motor's exhaust, a damaged jet's trail
/// or a burning wreck's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Smoke {
    Motor,
    Damage,
    Fire,
}

impl Smoke {
    /// Seconds a puff lasts, and between puffs.
    pub fn life(self) -> f64 {
        match self {
            Smoke::Motor => 3.5,
            Smoke::Damage => 3.0,
            Smoke::Fire => 5.0,
        }
    }
    fn spacing(self) -> f64 {
        match self {
            Smoke::Motor => 0.04,
            Smoke::Damage => 0.08,
            Smoke::Fire => 0.06,
        }
    }
}

pub struct Trail {
    pub kind: Smoke,
    /// Points and the time they were laid.
    pub points: VecDeque<(V3, f64)>,
    pub open: bool,
    last: f64,
}

impl Trail {
    fn new(kind: Smoke) -> Trail {
        Trail { kind, points: VecDeque::new(), open: true, last: f64::NEG_INFINITY }
    }
}

pub struct Bandit {
    pub jet: Aircraft,
    pub pilot: Pilot,
    pub hp: f64,
    pub callsign: &'static str,
    pub rounds: u32,
    pub missiles: u32,
    pub alive: bool,
    gun_acc: f64,
    fired: u32,
    trail: Option<usize>,
    /// Countermeasure programs left and seconds to the next.
    programs: u32,
    cm_clock: f64,
}

/// A killed jet falling out of the sky, tumbling and burning.
pub struct Wreck {
    pub jet: Aircraft,
    spin: V3,
    trail: usize,
}

/// What happened in a step, for the effects, the sound and the messages.
#[derive(Clone, Debug)]
pub enum Event {
    Launch { pos: V3, from: Who, name: &'static str },
    Release { pos: V3 },
    /// A round struck something solid.
    Hit { pos: V3 },
    Dirt { pos: V3 },
    Splash { pos: V3 },
    Airburst { pos: V3, vel: V3, scale: f64 },
    Blast { pos: V3, scale: f64, burn: f64 },
    Dud { pos: V3 },
    Kill { text: String },
    PlayerHit { damage: f64 },
    Lock { text: String },
    /// A countermeasure program: a flare and a chaff bundle thrown out.
    Decoy { pos: V3, vel: V3 },
    /// A line for the pilot, such as an airfield sortie's next leg.
    Toast(String),
}

pub struct Combat {
    pub kind: MissionKind,
    pub mode: Mode,
    pub loadout: Loadout,
    /// The player's airframe hit points, of 100.
    pub hp: f64,
    pub bandits: Vec<Bandit>,
    /// Bandits that join once the first one is down.
    reserve: Vec<Bandit>,
    pub wrecks: Vec<Wreck>,
    pub sites: Vec<Site>,
    pub rounds: Vec<Round>,
    pub missiles: Vec<Missile>,
    missile_trails: Vec<usize>,
    pub bombs: Vec<Bomb>,
    pub trails: Vec<Trail>,
    pub sight: GunSight,
    /// Radar (or ground designation) lock.
    pub lock: Option<Who>,
    /// What the Sidewinder's seeker is tracking before launch.
    pub seeker: Option<Who>,
    /// CCIP impact point and time of fall.
    pub ccip: Option<(V3, f64)>,
    /// The selected missile would hit its target if fired now.
    pub shoot: bool,
    /// Where the pilot's helmet looks (world direction), in the cockpit view:
    /// it cues the Sidewinder seeker and the radar lock off the boresight.
    pub helmet: Option<V3>,
    zone_clock: f64,
    pub clock: f64,
    pub kills: usize,
    pub destroyed: usize,
    pub points: f64,
    pub player_dead: bool,
    done_at: Option<f64>,
    pub events: Vec<Event>,
    /// Seconds left of the missile-launch warning.
    pub warn: f64,
    /// The gun fired this step.
    pub firing: bool,
    trigger_was: bool,
    gun_acc: f64,
    fired: u32,
    player_trail: Option<usize>,
    sam_clock: f64,
    rnd: Lcg,
    /// Airfield sortie: legs done (takeoff, steerpoint, landing), whether the
    /// wheels are on the runway, and the vertical speed a step ago.
    pub leg: u8,
    touched: bool,
    last_vs: f64,
}

/// A trimmed jet at a point, heading (radians clockwise from north) and speed.
pub fn spawn(pos: V3, heading: f64, speed: f64) -> Aircraft {
    let mut a = Aircraft::new(&Mission::new());
    let t = trim(pos.y, speed, a.mass);
    let fwd = v3(heading.sin(), 0.0, -heading.cos());
    a.pos = pos;
    a.last_pos = pos;
    a.vel = fwd * speed;
    a.q = Quat::from_forward_up(fwd, V3::Y).mul(Quat::axis_angle(V3::X, t.alpha));
    let mut f = Flcs::default();
    f.alpha_f = t.alpha * RAD;
    f.el = t.el;
    a.flcs = f;
    a.power = t.power;
    a.throttle = t.throttle;
    a.update(0.0, &Input::default());
    a
}

/// Hangs a loadout on a jet and trims it again for 1 g at the new weight
/// and drag, so a loaded start flies hands-off like the clean one.
pub fn load(a: &mut Aircraft, l: &Loadout) {
    a.stores_mass = l.mass();
    a.wing_stores = l.wing_mass();
    a.stores_drag = l.drag();
    a.mass = crate::sim::spec::EMPTY_MASS + a.stores_mass + a.fuel;
    if a.stores_drag == 0.0 && (a.stores_mass - STORES_MASS).abs() < 1e-9 && a.fuel == crate::sim::spec::FUEL_MAX {
        return;
    }
    // Trim balances the clean airframe at the new weight; the stores' drag
    // then takes the power that adds its thrust.
    let mut t = trim(a.pos.y, a.speed, a.mass);
    let atm = crate::sim::atmosphere_at(a.pos.y);
    let mach = a.speed / atm.sound;
    let need = crate::aero::thrust(t.power, mach, a.pos.y) + 0.5 * atm.rho * a.speed * a.speed * crate::aero::WING_AREA * a.stores_drag;
    let (mut lo, mut hi) = (0.0, 100.0);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if crate::aero::thrust(mid, mach, a.pos.y) < need {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    t.power = 0.5 * (lo + hi);
    t.throttle = (t.power / 50.0).min(0.99);
    let fwd = a.vel.norm();
    a.q = Quat::from_forward_up(fwd, V3::Y).mul(Quat::axis_angle(V3::X, t.alpha));
    let mut f = Flcs::default();
    f.alpha_f = t.alpha * RAD;
    f.el = t.el;
    a.flcs = f;
    a.power = t.power;
    a.throttle = t.throttle;
    a.update(0.0, &Input::default());
}

fn bandit(jet: Aircraft, skill: f64, callsign: &'static str) -> Bandit {
    Bandit { jet, pilot: Pilot::new(skill), hp: 100.0, callsign, rounds: GUN_ROUNDS, missiles: 2, alive: true, gun_acc: 0.0, fired: 0, trail: None, programs: CHAFF, cm_clock: 0.0 }
}

fn heading_of(v: V3) -> f64 {
    v.x.atan2(-v.z)
}

/// True when nothing solid stands between two points: the terrain sampled
/// along the line every 150 m or so.
pub fn line_of_sight(a: V3, b: V3) -> bool {
    let d = (b - a).len();
    let n = ((d / 150.0) as usize).clamp(4, 80);
    (1..n).all(|k| {
        let p = a.lerp(b, k as f64 / n as f64);
        p.y > terrain::floor(p.x, p.z) + 5.0
    })
}

fn in_river(p: V3) -> bool {
    (p.x - river_x(p.z)).abs() < 22.0 && p.y < 1.0 + terrain::floor(p.x, p.z)
}

impl Combat {
    /// Sets up a mission: the loadout, the bandits or the ground targets,
    /// and the steerpoints and clock in the mission record.
    pub fn new(kind: MissionKind, m: &mut Mission) -> Combat {
        let mut c = Combat {
            kind,
            mode: match kind {
                MissionKind::Gates | MissionKind::Airfield => Mode::Nav,
                MissionKind::Dogfight => Mode::Srm,
                MissionKind::Strike => Mode::Ccip,
            },
            loadout: Loadout::for_mission(kind),
            hp: 100.0,
            bandits: Vec::new(),
            reserve: Vec::new(),
            wrecks: Vec::new(),
            sites: Vec::new(),
            rounds: Vec::new(),
            missiles: Vec::new(),
            missile_trails: Vec::new(),
            bombs: Vec::new(),
            trails: Vec::new(),
            sight: GunSight::default(),
            lock: None,
            seeker: None,
            ccip: None,
            shoot: false,
            helmet: None,
            zone_clock: 0.0,
            clock: 0.0,
            kills: 0,
            destroyed: 0,
            points: 0.0,
            player_dead: false,
            done_at: None,
            events: Vec::new(),
            warn: 0.0,
            firing: false,
            trigger_was: false,
            gun_acc: 0.0,
            fired: 0,
            player_trail: None,
            sam_clock: 25.0,
            rnd: Lcg(90210),
            leg: 0,
            touched: false,
            last_vs: 0.0,
        };
        m.time_left = kind.time();
        let fwd = m.start_vel.norm();
        match kind {
            MissionKind::Gates => {}
            MissionKind::Airfield => {
                // Lined up on runway A, 150 m in from the threshold, brakes off;
                // the steerpoint is 9 km past the far end at 3,000 ft above
                // the field, then the touchdown zone of runway A again.
                let (dx, dz) = terrain::runway_dir();
                let fwd = v3(dx, 0.0, dz);
                let e = terrain::base_elevation();
                let (sx, sz) = terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH + 150.0, 0.0);
                m.start_pos = v3(sx, e + crate::sim::WHEELS[0].local.y.abs() - 0.1, sz);
                m.start_vel = V3::ZERO;
                m.start_q = Quat::from_forward_up(fwd, V3::Y);
                m.start_throttle = 0.0;
                let (wx, wz) = terrain::runway_point(terrain::RUNWAY_HALF_LENGTH + 9000.0, 0.0);
                let (tx, tz) = terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH + 320.0, 0.0);
                m.gates = vec![Gate { pos: v3(wx, e + 900.0, wz), radius: 0.0, axis: fwd, passed: false }, Gate { pos: v3(tx, e, tz), radius: 0.0, axis: fwd, passed: false }];
                m.gate_index = 0;
            }
            MissionKind::Dogfight => {
                // A pair 20 km down the gorge coming head-on, high over the
                // ridges and spread 3 km apart, so the merge opens out and the
                // fight has room; a third jet joins from the flank after the
                // first kill.
                // The player starts above the ridges too, not down by the river.
                m.start_pos.y = m.start_pos.y.max(2700.0);
                let level = flat_of(fwd);
                m.start_vel = level * m.start_vel.len();
                m.start_q = Quat::from_forward_up(level, V3::Y);
                let centre = m.start_pos + fwd * 20000.0;
                let side = fwd.cross(V3::Y).norm();
                let alt = |p: V3, over: f64| (terrain::floor(p.x, p.z) + over).max(2600.0);
                let back = heading_of(-fwd);
                let lead = centre + side * 1400.0;
                let wing = centre - side * 1600.0 + fwd * 2500.0;
                c.bandits.push(bandit(spawn(v3(lead.x, alt(lead, 1300.0), lead.z), back, 235.0), 0.55, "BANDIT 1"));
                c.bandits.push(bandit(spawn(v3(wing.x, alt(wing, 1300.0) + 300.0, wing.z), back, 235.0), 0.35, "BANDIT 2"));
                let late = m.start_pos + fwd * 9000.0 + side * 14000.0;
                c.reserve.push(bandit(spawn(v3(late.x, alt(late, 1500.0) + 600.0, late.z), heading_of(-side), 240.0), 0.8, "BANDIT 3"));
                m.gates = vec![Gate { pos: v3(centre.x, alt(centre, 1300.0), centre.z), radius: 0.0, axis: fwd, passed: false }];
                m.gate_index = 0;
            }
            MissionKind::Strike => {
                c.sites = targets::place();
                m.gates = (0..targets::GROUPS.len()).filter_map(|g| targets::group_point(&c.sites, g)).map(|pos| Gate { pos, radius: 0.0, axis: fwd, passed: false }).collect();
                m.gate_index = 0;
            }
        }
        c
    }

    /// Hangs the stores the loadout screen chose, and selects a master mode
    /// they allow.
    pub fn equip(&mut self, cfg: &Config) {
        self.loadout = Loadout::configure(cfg);
        if !self.modes().contains(&self.mode) {
            self.mode = *self.modes().iter().rev().find(|&&m| m != Mode::Ccip).unwrap_or(&Mode::Gun);
        }
    }

    // ------------------------------------------------------------ controls
    /// Master modes the stores on board allow, in the order X cycles them.
    fn modes(&self) -> Vec<Mode> {
        let mut v = vec![Mode::Nav, Mode::Gun];
        if self.loadout.count(Store::Aim9) > 0 {
            v.push(Mode::Srm);
        }
        if self.loadout.count(Store::Aim120) > 0 {
            v.push(Mode::Mrm);
        }
        if self.loadout.count(Store::Mk82) > 0 {
            v.push(Mode::Ccip);
        }
        v
    }

    pub fn cycle_weapon(&mut self) {
        let modes = self.modes();
        let i = modes.iter().position(|&m| m == self.mode).map(|i| (i + 1) % modes.len()).unwrap_or(0);
        self.mode = modes[i];
        self.seeker = None;
        self.shoot = false;
    }

    /// Selects a master mode by name, for the test switches.
    pub fn select(&mut self, mode: Mode) {
        if self.modes().contains(&mode) {
            self.mode = mode;
        }
    }

    /// Position and velocity of anything in the fight that is still alive.
    pub fn track(&self, who: Who, a: &Aircraft) -> Option<(V3, V3)> {
        match who {
            Who::Player => (!self.player_dead).then_some((a.pos, a.vel)),
            Who::Bandit(i) => self.bandits.get(i).filter(|b| b.alive).map(|b| (b.jet.pos, b.jet.vel)),
            Who::Ground(i) => self.sites.get(i).filter(|s| !s.dead).map(|s| (s.centre(), V3::ZERO)),
        }
    }

    /// Lock candidates the radar can see: within 60 degrees of the nose, in
    /// range and not behind terrain, nearest the boresight first.
    fn candidates(&self, a: &Aircraft, ground: bool) -> Vec<(Who, f64)> {
        let fwd = a.q.axes().forward;
        // The helmet picks among what the radar can see: nearest its line of sight.
        let look = self.helmet.unwrap_or(fwd);
        let mut v: Vec<(Who, f64)> = Vec::new();
        let mut consider = |who: Who, p: V3, max: f64| {
            let r = p - a.pos;
            let off = fwd.dot(r.norm()).clamp(-1.0, 1.0).acos();
            if off < 60.0 * DEG && r.len() < max && line_of_sight(a.pos, p + V3::Y * 3.0) {
                v.push((who, look.dot(r.norm()).clamp(-1.0, 1.0).acos() + r.len() / 400000.0));
            }
        };
        if ground {
            for (i, s) in self.sites.iter().enumerate().filter(|(_, s)| !s.dead) {
                consider(Who::Ground(i), s.centre(), 15000.0);
            }
        } else {
            for (i, b) in self.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
                consider(Who::Bandit(i), b.jet.pos, 74000.0);
            }
        }
        v.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    /// T: lock the target nearest the boresight, or step to the next one.
    pub fn designate(&mut self, a: &Aircraft) {
        let ground = self.mode.air_to_ground();
        let mut list = self.candidates(a, ground);
        if list.is_empty() {
            list = self.candidates(a, !ground);
        }
        let next = match self.lock.and_then(|l| list.iter().position(|(w, _)| *w == l)) {
            Some(i) => list.get(i + 1).or(list.first()).map(|x| x.0),
            None => list.first().map(|x| x.0),
        };
        self.lock = next;
        let text = match next {
            Some(Who::Bandit(i)) => format!("LOCK {}", self.bandits[i].callsign),
            Some(Who::Ground(i)) => format!("DESIGNATED {}", self.sites[i].kind.name()),
            _ => "NO TARGET".to_string(),
        };
        self.events.push(Event::Lock { text });
    }

    /// One countermeasure program from a jet: a flare and a chaff bundle.
    /// Each missile guiding on it may lose its target, by its seeker type.
    fn decoy(&mut self, who: Who, pos: V3, vel: V3) {
        self.events.push(Event::Decoy { pos, vel });
        for k in 0..self.missiles.len() {
            let m = &self.missiles[k];
            if m.target != Some(who) || !m.tracking {
                continue;
            }
            let p = decoy_chance(m.spec, m.pos, pos, vel);
            if self.rnd.next() < p {
                self.missiles[k].tracking = false;
                if who == Who::Player {
                    self.events.push(Event::Lock { text: "MISSILE DECOYED".to_string() });
                }
            }
        }
    }

    /// The countermeasures button: one program, while cartridges last.
    pub fn countermeasures(&mut self, a: &Aircraft) {
        if self.player_dead || (self.loadout.chaff == 0 && self.loadout.flares == 0) {
            return;
        }
        self.loadout.chaff = self.loadout.chaff.saturating_sub(1);
        self.loadout.flares = self.loadout.flares.saturating_sub(1);
        self.decoy(Who::Player, a.pos, a.vel);
    }

    // ------------------------------------------------------------ step
    /// One fixed step of the fight, after the player's jet has moved.
    pub fn step(&mut self, dt: f64, a: &mut Aircraft, trigger: bool, m: &mut Mission) {
        self.clock += dt;
        self.warn = (self.warn - dt).max(0.0);
        let pressed = trigger && !self.trigger_was;
        self.trigger_was = trigger;
        let alive = !a.crashed && !self.player_dead;
        self.firing = false;
        if alive {
            self.player_weapons(dt, a, trigger, pressed);
        }
        a.stores_mass = self.loadout.mass();
        a.wing_stores = self.loadout.wing_mass();
        a.stores_drag = self.loadout.drag();
        self.sight.step(dt, a);
        self.ccip = match (self.mode, self.loadout.next(Store::Mk82)) {
            (Mode::Ccip, Some(i)) => ccip(a, self.loadout.stations[i].at, &terrain::floor),
            _ => None,
        };
        self.sensors(dt, a);
        self.step_bandits(dt, a);
        self.step_wrecks(dt);
        self.step_rounds(dt, a);
        self.step_missiles(dt, a);
        self.step_bombs(dt, a);
        if alive {
            self.step_sites(dt, a);
        }
        self.damage_smoke(a);
        self.objectives(m);
        if self.kind == MissionKind::Airfield && alive {
            self.circuit(a, m);
        }
    }

    /// The airfield sortie: airborne and climbing through 500 ft, then the
    /// steerpoint within 1.5 km, then a landing on runway A brought to a full
    /// stop on the runway. Touchdown is scored on the sink rate and on how far
    /// from the aim point the wheels meet the runway.
    fn circuit(&mut self, a: &Aircraft, m: &mut Mission) {
        let e = terrain::base_elevation();
        match self.leg {
            0 if !a.on_ground && a.pos.y - e > 150.0 => {
                self.leg = 1;
                self.points += 500.0;
                self.events.push(Event::Toast("AIRBORNE // GEAR UP, FLY TO STPT 1".to_string()));
            }
            1 if m.gates.first().is_some_and(|g| (v3(g.pos.x, 0.0, g.pos.z) - v3(a.pos.x, 0.0, a.pos.z)).len() < 1500.0) => {
                self.leg = 2;
                m.gate_index = 1;
                self.points += 500.0;
                self.events.push(Event::Toast("STPT 1 // RETURN AND LAND RUNWAY A".to_string()));
            }
            2 => {
                let (u, v) = terrain::runway_local(a.pos.x, a.pos.z);
                if a.on_ground && !self.touched {
                    self.touched = true;
                    let aim = (u - (-terrain::RUNWAY_HALF_LENGTH + 320.0)).abs();
                    let sink = -self.last_vs;
                    let firm = if sink < 1.0 { "GREASED" } else if sink < 2.0 { "FIRM" } else { "HARD" };
                    self.points += (1000.0 - sink * 250.0).max(0.0) + (500.0 - aim).max(0.0);
                    self.events.push(Event::Toast(format!("TOUCHDOWN {firm} // {} FPM // {} FT FROM AIM POINT", (sink * 196.85).round() as i64, (aim * 3.28084).round() as i64)));
                }
                if !a.on_ground {
                    self.touched = false;
                }
                let on_runway = u.abs() < terrain::RUNWAY_HALF_LENGTH + 60.0 && v.abs() < terrain::RUNWAY_HALF_WIDTH;
                if a.on_ground && a.speed < 3.0 && self.done_at.is_none() {
                    if on_runway {
                        self.leg = 3;
                        self.points += 1000.0;
                        self.done_at = Some(self.clock);
                    } else {
                        self.events.push(Event::Toast("OFF THE RUNWAY // TAXI BACK ONTO IT".to_string()));
                    }
                }
            }
            _ => {}
        }
        self.last_vs = a.vs;
    }

    fn player_weapons(&mut self, dt: f64, a: &Aircraft, trigger: bool, pressed: bool) {
        match self.mode {
            Mode::Gun => {
                if pressed {
                    // The barrels spin up and the first round leaves at once.
                    self.gun_acc = 1.0;
                }
                if trigger && self.loadout.rounds > 0 {
                    self.firing = true;
                    self.gun_acc += GUN_RATE * dt;
                    while self.gun_acc >= 1.0 && self.loadout.rounds > 0 {
                        self.gun_acc -= 1.0;
                        self.loadout.rounds -= 1;
                        self.fired += 1;
                        let (p, v) = muzzle(a);
                        let v = v + self.dispersion(v, 0.004);
                        self.rounds.push(Round::new(p, v, self.fired % TRACER_EVERY == 0, Who::Player, 6.0));
                    }
                }
            }
            Mode::Srm if pressed => self.launch(a, Store::Aim9),
            Mode::Mrm if pressed => self.launch(a, Store::Aim120),
            Mode::Ccip if pressed => {
                if let Some(i) = self.loadout.next(Store::Mk82) {
                    self.loadout.stations[i].loaded = false;
                    let b = Bomb::release(a, self.loadout.stations[i].at);
                    self.events.push(Event::Release { pos: b.pos });
                    self.bombs.push(b);
                }
            }
            _ => {}
        }
    }

    /// Random spread around a velocity, standard deviation `sigma` radians.
    fn dispersion(&mut self, v: V3, sigma: f64) -> V3 {
        let g = |r: &mut Lcg| (r.next() + r.next() + r.next() - 1.5) * 2.0;
        v3(g(&mut self.rnd), g(&mut self.rnd), g(&mut self.rnd)) * (v.len() * sigma)
    }

    fn open_trail(&mut self, kind: Smoke) -> usize {
        if let Some(i) = self.trails.iter().position(|t| !t.open && t.points.is_empty()) {
            self.trails[i] = Trail::new(kind);
            return i;
        }
        self.trails.push(Trail::new(kind));
        self.trails.len() - 1
    }

    fn lay(&mut self, trail: usize, p: V3) {
        let now = self.clock;
        let t = &mut self.trails[trail];
        if now - t.last >= t.kind.spacing() {
            t.last = now;
            t.points.push_back((p, now));
        }
    }

    fn launch(&mut self, a: &Aircraft, store: Store) {
        let Some(i) = self.loadout.next(store) else { return };
        let st = self.loadout.stations[i];
        self.loadout.stations[i].loaded = false;
        let (spec, target, kick) = match store {
            Store::Aim9 => (&AIM9M, self.seeker, 0.0),
            _ => (&AIM120C, self.lock.filter(|w| matches!(w, Who::Bandit(_))), 3.0),
        };
        let pos = a.pos + a.q.rotate(st.at);
        let vel = a.vel - a.q.axes().up * kick;
        self.events.push(Event::Launch { pos, from: Who::Player, name: if store == Store::Aim9 { "FOX 2" } else { "FOX 3" } });
        self.fire_missile(Missile::launch(spec, pos, vel, Who::Player, target));
        self.seeker = None;
    }

    fn fire_missile(&mut self, m: Missile) {
        let t = self.open_trail(Smoke::Motor);
        self.missiles.push(m);
        self.missile_trails.push(t);
    }

    /// Lock upkeep, the Sidewinder seeker and the launch-zone cue.
    fn sensors(&mut self, dt: f64, a: &Aircraft) {
        let fwd = a.q.axes().forward;
        let off = |p: V3| fwd.dot((p - a.pos).norm()).clamp(-1.0, 1.0).acos();
        if let Some(l) = self.lock {
            let keep = self.track(l, a).is_some_and(|(p, _)| off(p) < 70.0 * DEG && (p - a.pos).len() < 80000.0);
            if !keep {
                self.lock = None;
                self.events.push(Event::Lock { text: "LOCK LOST".to_string() });
            }
        }
        if self.mode == Mode::Srm && self.loadout.count(Store::Aim9) > 0 {
            // Slaved to the radar lock inside its 40-degree gimbal, or keeps
            // what it has, or searches its 2.5-degree field where the helmet
            // looks (or along the boresight).
            let gimbal = AIM9M.gimbal;
            let usable = |w: Who, limit: f64| self.track(w, a).is_some_and(|(p, _)| off(p) < limit && (p - a.pos).len() < 18000.0 && matches!(w, Who::Bandit(_)));
            let look = self.helmet.unwrap_or(fwd);
            let near_look = |p: V3| look.dot((p - a.pos).norm()).clamp(-1.0, 1.0).acos();
            let seeker = if let Some(l) = self.lock.filter(|&l| usable(l, gimbal)) {
                Some(l)
            } else if let Some(s) = self.seeker.filter(|&s| usable(s, gimbal)) {
                Some(s)
            } else {
                self.bandits.iter().enumerate().find(|(i, b)| b.alive && near_look(b.jet.pos) < AIM9M.acquire * 1.6 && usable(Who::Bandit(*i), gimbal)).map(|(i, _)| Who::Bandit(i))
            };
            self.seeker = seeker;
        } else {
            self.seeker = None;
        }
        self.zone_clock -= dt;
        if self.zone_clock <= 0.0 {
            self.zone_clock = 0.25;
            let shot = match self.mode {
                Mode::Srm => self.seeker.map(|s| (&AIM9M, s)),
                Mode::Mrm if self.loadout.count(Store::Aim120) > 0 => self.lock.filter(|w| matches!(w, Who::Bandit(_))).map(|l| (&AIM120C, l)),
                _ => None,
            };
            self.shoot = shot.and_then(|(spec, w)| self.track(w, a).map(|t| (spec, t))).is_some_and(|(spec, (p, v))| fly_out(spec, a.pos, a.vel, p, v, &terrain::floor).is_some());
        }
    }

    fn step_bandits(&mut self, dt: f64, a: &Aircraft) {
        // The flanker arrives once the first bandit is down.
        if self.kills >= 1 && !self.reserve.is_empty() {
            let mut b = self.reserve.remove(0);
            b.jet.pos = a.pos + (b.jet.pos - a.pos).norm() * 9000.0;
            b.jet.pos.y = (terrain::floor(b.jet.pos.x, b.jet.pos.z) + 1200.0).max(a.pos.y + 600.0);
            b.jet.vel = (a.pos - b.jet.pos).norm() * 240.0;
            let h = heading_of(b.jet.vel);
            b.jet = spawn(b.jet.pos, h, 240.0);
            self.events.push(Event::Lock { text: format!("{} INBOUND", b.callsign) });
            self.bandits.push(b);
        }
        for i in 0..self.bandits.len() {
            if !self.bandits[i].alive {
                continue;
            }
            let threat = self
                .missiles
                .iter()
                .filter(|m| m.target == Some(Who::Bandit(i)) && m.tracking)
                .map(|m| (m.pos, m.vel))
                .min_by(|x, y| (x.0 - self.bandits[i].jet.pos).len().partial_cmp(&(y.0 - self.bandits[i].jet.pos).len()).unwrap_or(std::cmp::Ordering::Equal));
            // A missile closing inside 4 km gets a program every second or so.
            let b = &mut self.bandits[i];
            b.cm_clock -= dt;
            if threat.is_some_and(|(p, _)| (p - b.jet.pos).len() < 4000.0) && b.cm_clock <= 0.0 && b.programs > 0 {
                b.cm_clock = 1.1;
                b.programs -= 1;
                let (p, v) = (b.jet.pos, b.jet.vel);
                self.decoy(Who::Bandit(i), p, v);
            }
            let b = &mut self.bandits[i];
            let pic = Picture { foe: a, missile: threat, missiles: b.missiles, missile_spec: &AIM9M, rounds: b.rounds };
            let o = b.pilot.fly(&b.jet, &pic, dt);
            b.jet.update(dt, &o.input);
            if b.jet.crashed {
                let (p, name) = (b.jet.pos, b.callsign);
                b.alive = false;
                if let Some(t) = b.trail.take() {
                    self.trails[t].open = false;
                }
                self.events.push(Event::Blast { pos: p, scale: 1.3, burn: 20.0 });
                self.scored_kill(Who::Bandit(i), format!("{name} CRASHED"));
                continue;
            }
            let mut shots = Vec::new();
            if o.gun && b.rounds > 0 && !self.player_dead {
                b.gun_acc += GUN_RATE * dt;
                while b.gun_acc >= 1.0 && b.rounds > 0 {
                    b.gun_acc -= 1.0;
                    b.rounds -= 1;
                    b.fired += 1;
                    let (p, v) = muzzle(&b.jet);
                    shots.push((p, v, b.fired % 4 == 0));
                }
            } else {
                b.gun_acc = 0.0;
            }
            for (p, v, tracer) in shots {
                let jitter = self.dispersion(v, 0.005);
                self.rounds.push(Round::new(p, v + jitter, tracer, Who::Bandit(i), 6.0));
            }
            let b = &mut self.bandits[i];
            if o.missile && b.missiles > 0 && !self.player_dead {
                b.missiles -= 1;
                let side = if b.missiles % 2 == 0 { -1.0 } else { 1.0 };
                let t = crate::airframe::AIM9_TIP;
                let pos = b.jet.pos + b.jet.q.rotate(v3(side * t.x, t.y, t.z));
                let vel = b.jet.vel;
                self.events.push(Event::Launch { pos, from: Who::Bandit(i), name: "MISSILE LAUNCH" });
                self.warn = 4.0;
                self.fire_missile(Missile::launch(&AIM9M, pos, vel, Who::Bandit(i), Some(Who::Player)));
            }
        }
    }

    fn scored_kill(&mut self, who: Who, text: String) {
        if self.lock == Some(who) {
            self.lock = None;
        }
        if self.seeker == Some(who) {
            self.seeker = None;
        }
        self.kills += 1;
        self.points += 1500.0;
        self.events.push(Event::Kill { text });
    }

    fn kill_bandit(&mut self, i: usize) {
        let b = &mut self.bandits[i];
        if !b.alive {
            return;
        }
        b.alive = false;
        let mut jet = b.jet.clone();
        let name = b.callsign;
        if let Some(t) = b.trail.take() {
            self.trails[t].open = false;
        }
        jet.vel = jet.vel * 0.8 + V3::Y * 15.0;
        let spin = v3(self.rnd.next() - 0.5, self.rnd.next() - 0.5, (self.rnd.next() - 0.5) * 6.0);
        let trail = self.open_trail(Smoke::Fire);
        self.events.push(Event::Airburst { pos: jet.pos, vel: jet.vel, scale: 1.0 });
        self.wrecks.push(Wreck { jet, spin, trail });
        self.scored_kill(Who::Bandit(i), format!("SPLASH {name}"));
    }

    fn step_wrecks(&mut self, dt: f64) {
        let mut landed = Vec::new();
        for (i, w) in self.wrecks.iter_mut().enumerate() {
            // A tumbling airframe: about 3 m^2 of drag area on 9 t.
            let v = w.jet.vel.len();
            w.jet.vel += (w.jet.vel * (-2.0e-4 * v) - V3::Y * G0) * dt;
            w.jet.pos += w.jet.vel * dt;
            w.jet.q = w.jet.q.integrate(w.spin, dt);
            if w.jet.pos.y < terrain::floor(w.jet.pos.x, w.jet.pos.z) + 1.0 {
                landed.push(i);
            }
        }
        for i in 0..self.wrecks.len() {
            let (p, t) = (self.wrecks[i].jet.pos, self.wrecks[i].trail);
            self.lay(t, p);
        }
        for &i in landed.iter().rev() {
            let w = self.wrecks.remove(i);
            self.trails[w.trail].open = false;
            let water = in_river(w.jet.pos) || w.jet.pos.y < 2.0;
            self.events.push(if water { Event::Splash { pos: w.jet.pos } } else { Event::Blast { pos: w.jet.pos, scale: 1.3, burn: 18.0 } });
        }
    }

    fn damage(&mut self, who: Who, dmg: f64, a: &Aircraft) {
        if dmg <= 0.0 {
            return;
        }
        match who {
            Who::Player => {
                if self.player_dead {
                    return;
                }
                self.hp -= dmg;
                self.events.push(Event::PlayerHit { damage: dmg });
                if self.hp <= 0.0 {
                    self.hp = 0.0;
                    self.player_dead = true;
                    self.events.push(Event::Airburst { pos: a.pos, vel: a.vel, scale: 1.0 });
                }
            }
            Who::Bandit(i) => {
                if let Some(b) = self.bandits.get_mut(i).filter(|b| b.alive) {
                    b.hp -= dmg;
                    if b.hp <= 0.0 {
                        self.kill_bandit(i);
                    }
                }
            }
            Who::Ground(i) => {
                let Some(s) = self.sites.get_mut(i).filter(|s| !s.dead) else { return };
                s.hp -= dmg;
                if s.hp <= 0.0 {
                    s.dead = true;
                    let (scale, burn) = s.kind.blast();
                    let (pos, name, points) = (s.pos, s.kind.name(), s.kind.points());
                    self.destroyed += 1;
                    self.points += points;
                    if self.lock == Some(who) {
                        self.lock = None;
                    }
                    self.events.push(Event::Blast { pos, scale, burn });
                    self.events.push(Event::Kill { text: format!("{name} DESTROYED") });
                }
            }
        }
    }

    fn step_rounds(&mut self, dt: f64, a: &Aircraft) {
        let mut hits: Vec<(Who, f64)> = Vec::new();
        let mut i = 0;
        while i < self.rounds.len() {
            let mut r = self.rounds[i];
            r.step(dt);
            let mut gone = r.age > ROUND_LIFE;
            let mut hit = |who: Who, at: V3, dmg: f64, events: &mut Vec<Event>| {
                hits.push((who, dmg));
                events.push(Event::Hit { pos: at });
            };
            if !gone && r.from != Who::Player && !self.player_dead && (r.pos - a.pos).len() < 40.0 {
                if let Some(t) = hits_airframe(a, r.prev - (a.pos - a.vel * dt), r.pos - a.pos) {
                    hit(Who::Player, r.prev.lerp(r.pos, t), r.damage, &mut self.events);
                    gone = true;
                }
            }
            if !gone && r.from == Who::Player {
                for (k, b) in self.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
                    if (r.pos - b.jet.pos).len() < 40.0 {
                        if let Some(t) = hits_airframe(&b.jet, r.prev - (b.jet.pos - b.jet.vel * dt), r.pos - b.jet.pos) {
                            hit(Who::Bandit(k), r.prev.lerp(r.pos, t), r.damage, &mut self.events);
                            gone = true;
                            break;
                        }
                    }
                }
                if !gone {
                    for (k, s) in self.sites.iter().enumerate().filter(|(_, s)| !s.dead) {
                        if (r.pos - s.centre()).len() < 80.0 {
                            if let Some(t) = hits_box(s.local(r.prev), s.local(r.pos), s.kind.half()) {
                                hit(Who::Ground(k), r.prev.lerp(r.pos, t), s.kind.round_damage(), &mut self.events);
                                gone = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !gone {
                if let Some(p) = ground_crossing(r.prev, r.pos, &terrain::floor) {
                    // Every other round raises a visible puff.
                    if r.tracer || self.rnd.next() < 0.4 {
                        self.events.push(if in_river(p) { Event::Splash { pos: p } } else { Event::Dirt { pos: p } });
                    }
                    gone = true;
                }
            }
            if gone {
                self.rounds.swap_remove(i);
            } else {
                self.rounds[i] = r;
                i += 1;
            }
        }
        for (who, dmg) in hits {
            self.damage(who, dmg, a);
        }
    }

    fn step_missiles(&mut self, dt: f64, a: &Aircraft) {
        let mut i = 0;
        while i < self.missiles.len() {
            // An AMRAAM fired without a lock searches with its own radar.
            if self.missiles[i].spec.active && self.missiles[i].target.is_none() && self.missiles[i].age > 1.5 {
                let (mp, mv) = (self.missiles[i].pos, self.missiles[i].vel.norm());
                let found = self.bandits.iter().enumerate().filter(|(_, b)| b.alive).map(|(k, b)| (k, b.jet.pos - mp)).filter(|(_, r)| r.len() < 15000.0 && mv.dot(r.norm()) > (20.0f64 * DEG).cos()).min_by(|x, y| x.1.len().partial_cmp(&y.1.len()).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((k, _)) = found {
                    self.missiles[i].target = Some(Who::Bandit(k));
                    self.missiles[i].tracking = true;
                }
            }
            // A semi-active SAM goes blind when its radar dies.
            if let Who::Ground(_) = self.missiles[i].from {
                if !self.sites.iter().any(|s| s.kind == Kind::Radar && !s.dead) {
                    self.missiles[i].tracking = false;
                }
            }
            let target = self.missiles[i].target.and_then(|w| self.track(w, a));
            let fate = self.missiles[i].step(dt, target, &terrain::floor);
            let (pos, burning, trail) = (self.missiles[i].pos, self.missiles[i].burning(), self.missile_trails[i]);
            if burning {
                self.lay(trail, pos);
            }
            match fate {
                Fate::Flying => {
                    i += 1;
                    continue;
                }
                Fate::Burst(p, miss) => {
                    let m = &self.missiles[i];
                    let dmg = m.spec.damage(miss);
                    let (who, vel) = (m.target, m.vel);
                    self.events.push(Event::Airburst { pos: p, vel: vel * 0.1, scale: 0.55 });
                    if let Some(w) = who {
                        self.damage(w, dmg, a);
                    }
                }
                Fate::Ground(p) => self.events.push(Event::Blast { pos: p, scale: 0.5, burn: 0.0 }),
                Fate::Spent(p) => self.events.push(Event::Airburst { pos: p, vel: V3::ZERO, scale: 0.3 }),
            }
            self.trails[trail].open = false;
            self.missiles.swap_remove(i);
            self.missile_trails.swap_remove(i);
        }
    }

    fn step_bombs(&mut self, dt: f64, a: &Aircraft) {
        let mut i = 0;
        while i < self.bombs.len() {
            let Some(p) = self.bombs[i].step(dt, &terrain::floor) else {
                i += 1;
                continue;
            };
            let armed = self.bombs[i].armed();
            self.bombs.swap_remove(i);
            if !armed {
                self.events.push(Event::Dud { pos: p });
                continue;
            }
            self.events.push(Event::Blast { pos: p, scale: 1.25, burn: 0.0 });
            for k in 0..self.sites.len() {
                if !self.sites[k].dead {
                    let d = self.sites[k].distance(p);
                    self.damage(Who::Ground(k), blast_damage(BOMB_DAMAGE, BOMB_LETHAL, BOMB_OUTER, d), a);
                }
            }
            // Fragments reach a jet that drops too low.
            let d = (a.pos - p).len();
            self.damage(Who::Player, blast_damage(60.0, 30.0, 160.0, d), a);
        }
    }

    /// The ZSU-23-4s shoot at the player when he is in reach and in sight,
    /// in bursts, leading him from a lagging track; the SA-6 battery fires
    /// while its radar sees him above the valley clutter.
    fn step_sites(&mut self, dt: f64, a: &Aircraft) {
        self.sam_clock -= dt;
        let radar = self.sites.iter().position(|s| s.kind == Kind::Radar && !s.dead);
        for k in 0..self.sites.len() {
            if self.sites[k].dead {
                continue;
            }
            match self.sites[k].kind {
                Kind::Zsu => self.flak(k, dt, a),
                Kind::Launcher => {
                    let Some(r) = radar else { continue };
                    let s = &self.sites[k];
                    let eye = self.sites[r].eye();
                    let range = (a.pos - eye).len();
                    if self.sam_clock <= 0.0 && s.ammo > 0 && a.agl > 300.0 && range > SA6.min_range && range < 14000.0 && line_of_sight(eye, a.pos) {
                        self.sam_clock = 25.0;
                        let s = &mut self.sites[k];
                        s.ammo -= 1;
                        let pos = s.eye();
                        let r = a.pos + a.vel * 6.0 - pos;
                        let dir = (flat_of(r) * (30.0 * DEG).cos() + V3::Y * (30.0 * DEG).sin()).norm();
                        self.events.push(Event::Launch { pos, from: Who::Ground(k), name: "SAM LAUNCH" });
                        self.warn = 5.0;
                        self.fire_missile(Missile::launch(&SA6, pos, dir * 45.0, Who::Ground(k), Some(Who::Player)));
                    }
                }
                _ => {}
            }
        }
    }

    fn flak(&mut self, k: usize, dt: f64, a: &Aircraft) {
        let s = &self.sites[k];
        let gun = s.eye();
        let r = a.pos - gun;
        let range = r.len();
        // 23 mm: 970 m/s, about 2.5 km of slant range; it leads the jet by the
        // time of flight and holds up for the drop.
        let tof = range / 820.0;
        let lead = (r + a.vel * tof + V3::Y * (0.5 * G0 * tof * tof)).norm();
        let engaged = range < 2500.0 && range > 150.0 && r.y > range * 0.05 && line_of_sight(gun, a.pos);
        let s = &mut self.sites[k];
        if !engaged {
            s.clock = 0.0;
            s.aim = lead;
            return;
        }
        // The turret follows with a lag of most of a second, so a jinking jet
        // stays ahead of it.
        s.aim = (s.aim + (lead - s.aim) * clamp(dt / 0.8, 0.0, 1.0)).norm();
        s.clock += dt;
        // Bursts of a second and a half, then three seconds to re-lay.
        if s.clock % 4.5 > 1.5 {
            return;
        }
        let aim = s.aim;
        // Four barrels, 3,400 rounds per minute together.
        if self.rnd.next() < 57.0 * dt {
            let v = aim * 970.0;
            let v = v + self.dispersion(v, 0.010);
            let tracer = self.rnd.next() < 0.33;
            self.rounds.push(Round::new(gun, v, tracer, Who::Ground(k), 3.0));
        }
    }

    /// Damaged jets trail smoke: grey once hurt, black once badly hurt.
    fn damage_smoke(&mut self, a: &Aircraft) {
        if self.hp < 60.0 && !a.crashed {
            let t = match self.player_trail {
                Some(t) => t,
                None => {
                    let t = self.open_trail(Smoke::Damage);
                    self.player_trail = Some(t);
                    t
                }
            };
            self.lay(t, a.pos + a.q.axes().back * 6.0);
        }
        for i in 0..self.bandits.len() {
            let b = &self.bandits[i];
            if !b.alive || b.hp >= 60.0 {
                continue;
            }
            let t = match b.trail {
                Some(t) => t,
                None => {
                    let t = self.open_trail(if b.hp < 30.0 { Smoke::Fire } else { Smoke::Damage });
                    self.bandits[i].trail = Some(t);
                    t
                }
            };
            let p = self.bandits[i].jet.pos + self.bandits[i].jet.q.axes().back * 6.0;
            self.lay(t, p);
        }
        let now = self.clock;
        for t in self.trails.iter_mut() {
            while t.points.front().is_some_and(|p| now - p.1 > t.kind.life()) {
                t.points.pop_front();
            }
        }
    }

    fn objectives(&mut self, m: &mut Mission) {
        match self.kind {
            MissionKind::Gates | MissionKind::Airfield => {}
            MissionKind::Dogfight => {
                if self.done_at.is_none() && self.reserve.is_empty() && self.bandits.iter().all(|b| !b.alive) {
                    self.done_at = Some(self.clock);
                }
            }
            MissionKind::Strike => {
                // Steer to the first group with anything left standing.
                if let Some(g) = (0..targets::GROUPS.len()).find(|&g| self.sites.iter().any(|s| s.group == g && !s.dead)) {
                    m.gate_index = g.min(m.gates.len().saturating_sub(1));
                }
                if self.done_at.is_none() && self.sites.iter().filter(|s| s.primary).all(|s| s.dead) {
                    self.done_at = Some(self.clock);
                }
            }
        }
    }

    /// The objective is met (after a moment to watch the last explosion).
    pub fn won(&self) -> bool {
        self.done_at.is_some_and(|t| self.clock - t > 1.5)
    }

    /// Score so far: targets and kills, plus the clock left once complete.
    pub fn score(&self, m: &Mission) -> f64 {
        let bonus = if self.done_at.is_some() { m.time_left.max(0.0) * 10.0 + self.hp * 5.0 } else { 0.0 };
        (self.points + bonus).round()
    }

    /// Objective progress for the overlays: done and total.
    pub fn tally(&self) -> (usize, usize) {
        match self.kind {
            MissionKind::Gates => (0, 0),
            MissionKind::Airfield => (self.leg as usize, 3),
            MissionKind::Dogfight => (self.kills, self.bandits.len() + self.reserve.len()),
            MissionKind::Strike => (self.sites.iter().filter(|s| s.primary && s.dead).count(), self.sites.iter().filter(|s| s.primary).count()),
        }
    }

    /// Threats for the RWR: bearing source, kind, and whether it is shooting.
    pub fn emitters(&self, a: &Aircraft) -> Vec<(V3, &'static str, bool)> {
        let mut v = Vec::new();
        for b in self.bandits.iter().filter(|b| b.alive) {
            let r = (a.pos - b.jet.pos).len();
            if r < 60000.0 {
                let locked = r < 15000.0 && b.jet.q.axes().forward.dot((a.pos - b.jet.pos).norm()) > 0.85;
                v.push((b.jet.pos, "16", locked));
            }
        }
        for s in self.sites.iter().filter(|s| !s.dead) {
            let r = (a.pos - s.pos).len();
            match s.kind {
                Kind::Zsu if r < 6000.0 => v.push((s.pos, "A", r < 2500.0)),
                Kind::Radar if r < 30000.0 => v.push((s.pos, "6", r < 14000.0 && a.agl > 300.0)),
                _ => {}
            }
        }
        v
    }

    /// Missiles in the air guiding on the player.
    pub fn inbound(&self) -> impl Iterator<Item = &Missile> {
        self.missiles.iter().filter(|m| m.target == Some(Who::Player) && m.tracking)
    }
}

fn flat_of(v: V3) -> V3 {
    v3(v.x, 0.0, v.z).norm()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate run carries exactly the clean jet's stores, so its flight
    /// is unchanged; bombs and missiles add mass and drag, and releasing
    /// them takes it off again.
    #[test]
    fn loadouts_feed_mass_and_drag() {
        let gates = Loadout::for_mission(MissionKind::Gates);
        assert!((gates.mass() - STORES_MASS).abs() < 1e-9);
        assert_eq!(gates.drag(), 0.0);
        let mut strike = Loadout::for_mission(MissionKind::Strike);
        assert_eq!(strike.count(Store::Mk82), 4);
        assert!((strike.mass() - STORES_MASS - 4.0 * MK82_MASS).abs() < 1e-9);
        assert!(strike.drag() > 0.004 && strike.drag() < 0.007);
        let i = strike.next(Store::Mk82).unwrap();
        strike.stations[i].loaded = false;
        strike.rounds -= 100;
        assert!((strike.mass() - (STORES_MASS + 3.0 * MK82_MASS - 100.0 * ROUND_EXPENDED)).abs() < 1e-9);
        let fight = Loadout::for_mission(MissionKind::Dogfight);
        assert_eq!((fight.count(Store::Aim9), fight.count(Store::Aim120)), (2, 2));
    }

    /// The same jet at the same throttle: four Mk-82s, their weight and drag,
    /// bleed speed in a sustained turn and slow the roll.
    #[test]
    fn stores_cost_turn_and_roll_performance() {
        crate::terrain::init();
        let fly = |l: &Loadout, input: Input, secs: f64| {
            let mut a = spawn(v3(0.0, 5000.0, 0.0), 0.0, 220.0);
            load(&mut a, l);
            a.throttle = 0.95;
            for _ in 0..(secs / crate::sim::FIXED_DT) as usize {
                a.update(crate::sim::FIXED_DT, &input);
            }
            a
        };
        let clean = Loadout::for_mission(MissionKind::Gates);
        let bombs = Loadout::for_mission(MissionKind::Strike);
        let pull = Input { pitch_axis: 0.3, ..Default::default() };
        let (c, b) = (fly(&clean, pull, 10.0), fly(&bombs, pull, 10.0));
        assert!(c.speed > b.speed + 5.0, "after a 10 s turn: clean {:.0} m/s, bombed {:.0} m/s", c.speed, b.speed);
        let roll = Input { roll_axis: 1.0, ..Default::default() };
        let (c, b) = (fly(&clean, roll, 0.25), fly(&bombs, roll, 0.25));
        assert!(c.bank.abs() > b.bank.abs() + 1.0, "bank after 0.25 s: clean {:.1}, bombed {:.1}", c.bank, b.bank);
    }

    #[test]
    fn modes_cycle_through_the_stores_on_board() {
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Gates, &mut m);
        assert_eq!(c.mode, Mode::Nav);
        let mut seen = vec![c.mode];
        for _ in 0..3 {
            c.cycle_weapon();
            seen.push(c.mode);
        }
        assert_eq!(seen, vec![Mode::Nav, Mode::Gun, Mode::Srm, Mode::Nav]);
        assert_eq!(m.time_left, crate::sim::MISSION_TIME);
        assert_eq!(m.gates.len(), 10, "the gate run keeps its gates");
    }

    /// The lock takes the bandit nearest the boresight, and T steps to the next.
    #[test]
    fn lock_picks_the_target_nearest_the_boresight() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        let mut a = spawn(v3(0.0, 4000.0, 0.0), 0.0, 230.0);
        a.update(0.0, &Input::default());
        let fwd = a.q.axes().forward;
        let right = a.q.axes().right;
        c.bandits[0].jet = spawn(a.pos + fwd * 8000.0 + right * 2500.0, PI, 230.0);
        c.bandits[1].jet = spawn(a.pos + fwd * 12000.0 + right * 300.0, PI, 230.0);
        c.designate(&a);
        assert_eq!(c.lock, Some(Who::Bandit(1)));
        c.designate(&a);
        assert_eq!(c.lock, Some(Who::Bandit(0)));
        // A bandit behind the jet cannot be locked.
        c.bandits[0].jet.pos = a.pos - fwd * 3000.0;
        c.bandits[1].jet.pos = a.pos - fwd * 5000.0;
        c.lock = None;
        c.designate(&a);
        assert_eq!(c.lock, None);
    }

    /// Rounds and a missile from the player bring a bandit down; the kill
    /// leaves a falling wreck and counts toward the objective.
    #[test]
    fn damage_kills_and_scores() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        let a = spawn(v3(0.0, 4000.0, 0.0), 0.0, 230.0);
        for _ in 0..10 {
            c.damage(Who::Bandit(0), 6.0, &a);
        }
        assert!((c.bandits[0].hp - 40.0).abs() < 1e-9 && c.bandits[0].alive);
        c.damage(Who::Bandit(0), AIM9M.damage(3.0), &a);
        assert!(!c.bandits[0].alive && c.kills == 1 && c.wrecks.len() == 1);
        assert!(c.events.iter().any(|e| matches!(e, Event::Kill { .. })));
        assert_eq!(c.tally(), (1, 3));
        // The player survives a near miss, not a direct hit.
        c.damage(Who::Player, AIM9M.damage(12.0), &a);
        assert!(!c.player_dead && c.hp < 100.0);
        c.damage(Who::Player, AIM9M.damage(1.0), &a);
        assert!(c.player_dead);
    }

    /// A Sidewinder fired at a locked bandit ahead flies to it and kills it.
    #[test]
    fn a_locked_sidewinder_kills_a_bandit_ahead() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        let mut a = spawn(v3(river_x(0.0), 4200.0, 0.0), 0.0, 240.0);
        let fwd = a.q.axes().forward;
        c.bandits[0].jet = spawn(a.pos + fwd * 2500.0, 0.0, 220.0);
        // Out of flares, so the shot is down to the guidance.
        c.bandits[0].programs = 0;
        c.bandits[1].jet = spawn(a.pos + fwd * 30000.0 + V3::Y * 2000.0, 0.0, 220.0);
        c.reserve.clear();
        let mut m2 = m.clone();
        c.step(FIXED_DT_TEST, &mut a, false, &mut m2);
        assert_eq!(c.seeker, Some(Who::Bandit(0)), "seeker on the bandit in the boresight");
        c.step(FIXED_DT_TEST, &mut a, true, &mut m2);
        assert_eq!(c.missiles.len(), 1);
        assert_eq!(c.loadout.count(Store::Aim9), 1);
        for _ in 0..(12.0 / FIXED_DT_TEST) as usize {
            a.update(FIXED_DT_TEST, &Input::default());
            c.step(FIXED_DT_TEST, &mut a, false, &mut m2);
        }
        assert!(c.kills >= 1 || c.bandits[0].hp < 100.0, "bandit untouched, hp {}", c.bandits[0].hp);
    }

    /// A Mk-82 dropped on the CCIP pipper over the bridge brings it down.
    #[test]
    fn a_bomb_on_the_pipper_drops_the_bridge() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Strike, &mut m);
        let bridge = c.sites.iter().position(|s| s.kind == Kind::Bridge).unwrap();
        let target = c.sites[bridge].pos;
        // The guns are quiet for the test. A steady 34-degree dive from the
        // south (moved kinematically, clear of the valley walls) toward a
        // point beyond the bridge; release when the pipper reaches it.
        for s in c.sites.iter_mut().filter(|s| s.kind == Kind::Zsu) {
            s.dead = true;
        }
        let start = target + v3(0.0, 1100.0, 1500.0);
        let vel = (target + v3(0.0, 0.0, -400.0) - start).norm() * 220.0;
        let mut a = spawn(start, 0.0, 220.0);
        a.q = Quat::from_forward_up(vel, V3::Y);
        a.vel = vel;
        let mut released = false;
        for _ in 0..(25.0 / FIXED_DT_TEST) as usize {
            if a.pos.y > target.y + 150.0 {
                a.pos += a.vel * FIXED_DT_TEST;
            }
            let pipper_on = c.ccip.is_some_and(|(p, _)| p.z <= target.z + 2.0);
            c.step(FIXED_DT_TEST, &mut a, pipper_on && !released, &mut m);
            released |= pipper_on;
        }
        assert!(released, "pipper never reached the bridge");
        assert!(c.sites[bridge].dead, "bridge hp {}", c.sites[bridge].hp);
        assert!(c.destroyed >= 1 && c.points >= Kind::Bridge.points());
    }

    const FIXED_DT_TEST: f64 = crate::sim::FIXED_DT;
}
