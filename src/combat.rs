//! The fight around the player: master modes and the stores on the
//! stations, the radar lock and the Sidewinder seeker, bandits flown by the
//! AI, ground targets with their guns and SAMs, every round, missile and bomb
//! in the air, damage, and the objectives of the DOGFIGHT and STRIKE
//! missions. It steps inside the fixed 120 Hz loop after the player's jet has
//! moved, and reports what happened as events for the effects and the sound.
use crate::ai::{Picture, Pilot};
use crate::math::*;
use crate::sim::{spec::STORES_MASS, trim, Aircraft, BattleDamage, Flcs, Gate, Input, Mission, Type, G0};
use crate::scenario::{self, Threats};
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
    /// Practice with no clock and no end: the gate rings, the ground targets
    /// and a pair of bandits, all coming back once cleared.
    Free,
}

impl MissionKind {
    pub const ALL: [MissionKind; 5] = [MissionKind::Gates, MissionKind::Dogfight, MissionKind::Strike, MissionKind::Airfield, MissionKind::Free];

    pub fn name(self) -> &'static str {
        match self {
            MissionKind::Gates => "GATE RUN",
            MissionKind::Dogfight => "DOGFIGHT",
            MissionKind::Strike => "STRIKE",
            MissionKind::Airfield => "AIRFIELD",
            MissionKind::Free => "FREE FLIGHT",
        }
    }

    pub fn parse(s: &str) -> Option<MissionKind> {
        match s {
            "gates" | "gate" | "run" => Some(MissionKind::Gates),
            "dogfight" | "aa" => Some(MissionKind::Dogfight),
            "strike" | "ag" => Some(MissionKind::Strike),
            "airfield" | "circuit" | "land" => Some(MissionKind::Airfield),
            "free" | "practice" => Some(MissionKind::Free),
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
            MissionKind::Free => 0.0,
        }
    }
}

/// HUD master mode, one per weapon, and NAV with the master arm safe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Nav,
    /// Gun with the EEGS funnel and LCOS pipper.
    Gun,
    /// Short-range missile, the AIM-9 or the R-73.
    Srm,
    /// Medium-range missile, the AIM-120 or the R-27R.
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

/// The F-16's stores and the MiG-29's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Store {
    /// AIM-9M.
    Aim9,
    Aim9x,
    Aim7,
    /// AIM-120C.
    Aim120,
    Aim120d,
    Mk82,
    R60m,
    R73,
    R27r,
    R27t,
    R77,
    Fab250,
    Fab500,
}

/// Weapon families, by what guides them. `arsenal` lists what each jet
/// carries in each, so a missing type shows as an empty family.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    /// Infrared dogfight missiles (Fox 2).
    ShortIr,
    /// Infrared missiles with medium-range motors (Fox 2).
    MediumIr,
    /// Semi-active radar missiles (Fox 1).
    SemiActive,
    /// Active radar missiles (Fox 3).
    ActiveRadar,
    /// Free-fall bombs.
    Bomb,
}

impl Store {
    #[cfg(test)]
    pub const ALL: [Store; 13] = [Store::Aim9, Store::Aim9x, Store::Aim7, Store::Aim120, Store::Aim120d, Store::Mk82, Store::R60m, Store::R73, Store::R27r, Store::R27t, Store::R77, Store::Fab250, Store::Fab500];

    pub fn mass(self) -> f64 {
        match self.missile() {
            Some(s) => s.mass,
            None => self.bomb().map_or(0.0, |b| b.mass),
        }
    }
    /// Zero-lift drag coefficient (on the 27.87 m^2 wing) of the store and
    /// its interference under a wing; a wingtip AIM-9 is in the clean data.
    fn drag(self) -> f64 {
        match self {
            Store::Aim9 => 0.0005,
            Store::Aim9x => 0.0004,
            Store::Aim7 => 0.0011,
            Store::Aim120 | Store::Aim120d => 0.0008,
            Store::Mk82 => 0.0013,
            Store::R60m => 0.0003,
            Store::R73 => 0.0006,
            Store::R27r | Store::R27t => 0.0011,
            Store::R77 => 0.0009,
            Store::Fab250 => 0.0013,
            Store::Fab500 => 0.0021,
        }
    }
    pub fn missile(self) -> Option<&'static Spec> {
        match self {
            Store::Aim9 => Some(&AIM9M),
            Store::Aim9x => Some(&AIM9X),
            Store::Aim7 => Some(&AIM7M),
            Store::Aim120 => Some(&AIM120C),
            Store::Aim120d => Some(&AIM120D),
            Store::R60m => Some(&R60M),
            Store::R73 => Some(&R73),
            Store::R27r => Some(&R27R),
            Store::R27t => Some(&R27T),
            Store::R77 => Some(&R77),
            _ => None,
        }
    }
    pub fn bomb(self) -> Option<&'static BombSpec> {
        match self {
            Store::Mk82 => Some(&MK82),
            Store::Fab250 => Some(&FAB250),
            Store::Fab500 => Some(&FAB500),
            _ => None,
        }
    }
    pub fn family(self) -> Family {
        match self.missile() {
            None => Family::Bomb,
            Some(s) => match s.seeker {
                Seeker::Infrared if s.max_range > 20000.0 => Family::MediumIr,
                Seeker::Infrared => Family::ShortIr,
                Seeker::SemiActive => Family::SemiActive,
                Seeker::Active => Family::ActiveRadar,
            },
        }
    }
    /// The master mode that releases it: SRM for the dogfight missiles, MRM
    /// for everything with a medium-range motor, CCIP for bombs.
    pub fn mode(self) -> Mode {
        match self.family() {
            Family::ShortIr => Mode::Srm,
            Family::MediumIr | Family::SemiActive | Family::ActiveRadar => Mode::Mrm,
            Family::Bomb => Mode::Ccip,
        }
    }
    /// Short name, for the HUD.
    pub fn name(self) -> &'static str {
        match self.missile() {
            Some(s) => s.name,
            None => self.bomb().map_or("", |b| b.name),
        }
    }
    /// Real name and, in brackets, what kind it is: the brevity code for a
    /// missile (Fox 1 semi-active, Fox 2 infrared, Fox 3 active radar).
    pub fn long_name(self) -> &'static str {
        match self {
            Store::Aim9 => "AIM-9M SIDEWINDER (FOX 2)",
            Store::Aim9x => "AIM-9X SIDEWINDER (FOX 2)",
            Store::Aim7 => "AIM-7M SPARROW (FOX 1)",
            Store::Aim120 => "AIM-120C AMRAAM (FOX 3)",
            Store::Aim120d => "AIM-120D AMRAAM (FOX 3)",
            Store::Mk82 => "MK 82 (BOMB)",
            Store::R60m => "R-60M APHID (FOX 2)",
            Store::R73 => "R-73 ARCHER (FOX 2)",
            Store::R27r => "R-27R ALAMO-A (FOX 1)",
            Store::R27t => "R-27T ALAMO-B (FOX 2)",
            Store::R77 => "R-77 ADDER (FOX 3)",
            Store::Fab250 => "FAB-250M-62 (BOMB)",
            Store::Fab500 => "FAB-500M-62 (BOMB)",
        }
    }
    /// A pair of them on the loadout screen.
    pub fn pair(self) -> &'static str {
        match self {
            Store::Aim9 => "2 X AIM-9M  IR",
            Store::Aim9x => "2 X AIM-9X  IR",
            Store::Aim7 => "2 X AIM-7M  SARH",
            Store::Aim120 => "2 X AIM-120C  RADAR",
            Store::Aim120d => "2 X AIM-120D  RADAR",
            Store::Mk82 => "2 X MK-82  500 LB",
            Store::R60m => "2 X R-60M  IR",
            Store::R73 => "2 X R-73  IR",
            Store::R27r => "2 X R-27R  SARH",
            Store::R27t => "2 X R-27T  IR",
            Store::R77 => "2 X R-77  RADAR",
            Store::Fab250 => "2 X FAB-250  250 KG",
            Store::Fab500 => "2 X FAB-500  500 KG",
        }
    }
    /// What the other airframe carries in its place: the same family, the
    /// nearest in kind (the F-16 has no infrared medium-range missile, so
    /// the R-27T's place takes the Sparrow).
    fn counterpart(self) -> Store {
        match self {
            Store::Aim9 => Store::R73,
            Store::Aim9x => Store::R73,
            Store::Aim7 => Store::R27r,
            Store::Aim120 | Store::Aim120d => Store::R77,
            Store::Mk82 => Store::Fab250,
            Store::R60m | Store::R73 => Store::Aim9,
            Store::R27r | Store::R27t => Store::Aim7,
            Store::R77 => Store::Aim120,
            Store::Fab250 | Store::Fab500 => Store::Mk82,
        }
    }
}

/// The missiles the outermost stations carry unless the loadout says
/// otherwise: the F-16's wingtip AIM-9Ms, the MiG-29's R-73s on the outer
/// pylons. The bandits always fly with these.
pub fn tip_store(jet: Type) -> Store {
    if jet == Type::Mig29 {
        Store::R73
    } else {
        Store::Aim9
    }
}

/// Choices for a station pair: 0 the outermost (the F-16's wingtips, the
/// MiG's outer pylons), 1 the next in (stations 3 and 7, the MiG's middle
/// pylons), 2 the inboard ones (stations 4 and 6, the MiG's inner pylons).
/// The F-16C Block 50: Sidewinders or AMRAAMs on the tips; missiles or bombs
/// on 3 and 7; bombs, AMRAAMs or Sparrows on 4 and 6. The MiG-29S 9.13S:
/// R-73s or R-60Ms outboard; dogfight missiles, R-77s or FAB-250s on the
/// middle pylons; the R-27s, R-77s or bombs on the inner ones.
fn pylon_choices(jet: Type, pair: u8) -> &'static [Option<Store>] {
    match (jet, pair) {
        (Type::F16, 0) => &[Some(Store::Aim9), Some(Store::Aim9x), Some(Store::Aim120), Some(Store::Aim120d)],
        (Type::F16, 1) => &[None, Some(Store::Aim9), Some(Store::Aim9x), Some(Store::Aim120), Some(Store::Aim120d), Some(Store::Aim7), Some(Store::Mk82)],
        (Type::F16, _) => &[None, Some(Store::Mk82), Some(Store::Aim120), Some(Store::Aim120d), Some(Store::Aim7)],
        (Type::Mig29, 0) => &[Some(Store::R73), Some(Store::R60m)],
        (Type::Mig29, 1) => &[None, Some(Store::R73), Some(Store::R60m), Some(Store::R77), Some(Store::Fab250)],
        (Type::Mig29, _) => &[None, Some(Store::R27r), Some(Store::R27t), Some(Store::R77), Some(Store::Fab500), Some(Store::Fab250)],
    }
}

/// Every store a jet can carry in a family, from its pylon choices; the
/// test `arsenal_by_family` spells the table out.
#[cfg(test)]
pub fn arsenal(jet: Type, family: Family) -> Vec<Store> {
    let mut v: Vec<Store> = Vec::new();
    for pair in 0..3 {
        for s in pylon_choices(jet, pair).iter().flatten() {
            if s.family() == family && !v.contains(s) {
                v.push(*s);
            }
        }
    }
    v
}

/// The OEPS-29 IRST's field (azimuth either side, and elevation below and
/// above the boresight) and its detection range against a fighter at
/// military power: some 15 km from behind, where the exhaust shows, down to
/// 8 km head on; an afterburner makes a target half as visible again.
const IRST_AZ: f64 = 30.0 * DEG;
const IRST_EL: (f64, f64) = (-15.0 * DEG, 30.0 * DEG);
const IRST_TAIL: f64 = 15000.0;
const IRST_HEAD: f64 = 8000.0;
/// Reach of the OEPS-29's laser rangefinder, m.
pub const LASER_RANGE: f64 = 6500.0;

/// The IRST sees a jet: inside its field, within its heat-detection range
/// (stretched by `margin` to keep a track) and with clear sky between.
pub fn irst_sees(a: &Aircraft, t: &Aircraft, margin: f64) -> bool {
    let r = t.pos - a.pos;
    let d = a.q.conj().rotate(r.norm());
    let (az, el) = (d.x.atan2(-d.z), d.y.asin());
    if az.abs() > IRST_AZ || el < IRST_EL.0 || el > IRST_EL.1 {
        return false;
    }
    // Aspect: 1 with the target's nose on us, 0 from behind.
    let nose_on = (t.q.axes().forward.dot(-r.norm()) + 1.0) * 0.5;
    let burner = if t.power > 50.0 { 1.5 } else { 1.0 };
    r.len() < mix(IRST_TAIL, IRST_HEAD, nose_on) * burner * margin && line_of_sight(a.pos, t.pos)
}

/// Range at which an active missile's own radar finds a fighter, m: past
/// it an AMRAAM or R-77 needs the datalink to know where to fly.
const ACTIVE_RANGE: f64 = 15000.0;

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

/// What the loadout screen sets before a sortie: the stores on the three
/// station pairs, the internal fuel and the opposition.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Config {
    /// Stations 1 and 9, the wingtips (the MiG's outer pylons).
    pub tip: Store,
    /// Stations 3 and 7.
    pub outboard: Option<Store>,
    /// Stations 4 and 6, the heavy inboard pylons.
    pub inboard: Option<Store>,
    /// Internal fuel, fraction of the 7,000 lb.
    pub fuel: f64,
    /// The player's airframe and the bandits'.
    pub jet: Type,
    pub aggressor: Type,
    pub threats: Threats,
}

impl Config {
    pub fn for_mission(kind: MissionKind) -> Config {
        let (outboard, inboard, fuel) = match kind {
            MissionKind::Gates => (None, None, 1.0),
            MissionKind::Airfield => (None, None, 0.5),
            MissionKind::Dogfight => (Some(Store::Aim120), None, 1.0),
            MissionKind::Strike => (Some(Store::Mk82), Some(Store::Mk82), 1.0),
            MissionKind::Free => (Some(Store::Aim120), Some(Store::Mk82), 1.0),
        };
        Config { tip: Store::Aim9, outboard, inboard, fuel, jet: Type::F16, aggressor: Type::Mig29, threats: Threats::for_mission(kind) }
    }

    /// The mission's stores as the given airframe carries them: the MiG-29S's
    /// air-to-air load is four R-73s and two R-77s on the inner pylons.
    pub fn for_jet(kind: MissionKind, jet: Type) -> Config {
        let mut c = Config::for_mission(kind);
        c.set_jet(jet);
        if jet == Type::Mig29 {
            (c.outboard, c.inboard) = match kind {
                MissionKind::Dogfight => (Some(Store::R73), Some(Store::R77)),
                MissionKind::Strike => (Some(Store::Fab250), Some(Store::Fab500)),
                MissionKind::Free => (Some(Store::R73), Some(Store::Fab500)),
                _ => (c.outboard, c.inboard),
            };
        }
        c
    }

    /// Next bandit fleet: F-16 aggressors, MiG-29s, then the two mixed.
    pub fn next_fleet(&mut self) {
        if self.threats.mixed {
            self.threats.mixed = false;
            self.aggressor = Type::F16;
        } else if self.aggressor == Type::Mig29 {
            self.threats.mixed = true;
        } else {
            self.aggressor = Type::Mig29;
        }
    }

    pub fn fleet_name(&self) -> &'static str {
        if self.threats.mixed {
            "F-16C + MIG-29S"
        } else {
            self.aggressor.name()
        }
    }

    /// Changes the airframe, swapping each pylon's stores for what the new
    /// one carries in their place, of the same kind where that pylon has it.
    pub fn set_jet(&mut self, jet: Type) {
        let min = self.fuel_is_min();
        self.jet = jet;
        if min {
            self.fuel = Config::fuel_choices(jet)[4];
        }
        let swap = |s: Store, pair: u8| -> Option<Store> {
            let choices = pylon_choices(jet, pair);
            let c = s.counterpart();
            choices.iter().flatten().find(|&&x| x == c).or_else(|| choices.iter().flatten().find(|x| x.family() == c.family())).or_else(|| choices.iter().flatten().find(|x| x.mode() == c.mode())).copied()
        };
        self.tip = swap(self.tip, 0).unwrap_or(tip_store(jet));
        self.outboard = self.outboard.and_then(|s| swap(s, 1));
        self.inboard = self.inboard.and_then(|s| swap(s, 2));
    }

    /// The stores hung, counted by kind, for the briefing: "4 R-73 2 R-27R".
    pub fn stores_summary(&self) -> String {
        let mut kinds: Vec<(Store, u32)> = Vec::new();
        for st in Loadout::configure(self).stations {
            match kinds.iter_mut().find(|(s, _)| *s == st.store) {
                Some(k) => k.1 += 1,
                None => kinds.push((st.store, 1)),
            }
        }
        kinds.iter().map(|(s, n)| format!("{n} {}", s.name())).collect::<Vec<_>>().join(" ")
    }

    /// Next choice for a station pair (0 tips, 1 outboard, 2 inboard).
    pub fn cycle(&mut self, pair: u8) {
        let order = pylon_choices(self.jet, pair);
        let current = match pair {
            0 => Some(self.tip),
            1 => self.outboard,
            _ => self.inboard,
        };
        let i = order.iter().position(|s| *s == current).unwrap_or(0);
        let next = order[(i + 1) % order.len()];
        match pair {
            0 => self.tip = next.unwrap_or(tip_store(self.jet)),
            1 => self.outboard = next,
            _ => self.inboard = next,
        }
    }

    /// Fuel loads on offer, fractions of the internal fuel, from full down
    /// to the jet's minimum (`Type::fuel_min`).
    pub fn fuel_choices(jet: Type) -> [f64; 5] {
        [1.0, 0.75, 0.5, 0.25, jet.fuel_min() / jet.fuel_max()]
    }

    /// Next fuel load down, back to full after the minimum.
    pub fn cycle_fuel(&mut self) {
        let choices = Config::fuel_choices(self.jet);
        let i = choices.iter().position(|&f| (f - self.fuel).abs() < 0.005).unwrap_or(0);
        self.fuel = choices[(i + 1) % choices.len()];
    }

    /// The fuel load is the minimum.
    pub fn fuel_is_min(&self) -> bool {
        (self.fuel - Config::fuel_choices(self.jet)[4]).abs() < 0.005
    }

    /// Gross weight (kg) and total drag coefficient of the stores.
    pub fn weight(&self) -> (f64, f64) {
        let l = Loadout::configure(self);
        (self.jet.empty_mass() + l.mass() + self.jet.fuel_max() * self.fuel, l.drag())
    }
}

#[derive(Clone, Debug)]
pub struct Loadout {
    /// In release order, alternating sides.
    pub stations: Vec<Station>,
    pub gun: &'static Gun,
    pub rounds: u32,
    pub chaff: u32,
    pub flares: u32,
}

/// Wingtip missile noses (stations 1 and 9; the MiG's outer pylons), on a
/// rail placed for the AIM-9M: a longer missile's nose sits further forward.
fn tips(store: Store) -> [Station; 2] {
    let t = crate::airframe::AIM9_TIP;
    let ahead = store.missile().map_or(0.0, |m| (m.length - AIM9M.length) * 0.5);
    [-1.0, 1.0].map(|s| Station { store, at: v3(s * t.x, t.y, t.z - ahead), loaded: true, tip: true })
}

impl Loadout {
    pub fn for_mission(kind: MissionKind) -> Loadout {
        Loadout::configure(&Config::for_mission(kind))
    }

    /// Stores hung as the loadout screen chose: the wingtip pair always,
    /// the outboard pair on stations 3 and 7, the inboard pair on 4 and 6.
    pub fn configure(cfg: &Config) -> Loadout {
        let mut stations = tips(cfg.tip).to_vec();
        for (pick, x) in [(cfg.outboard, 3.50), (cfg.inboard, 2.30)] {
            let Some(store) = pick else { continue };
            // Missile noses forward of the pylon, bombs centred under it.
            let (y, z) = match store {
                Store::Aim9 | Store::Aim9x | Store::R60m | Store::R73 => (-0.03, x * 0.5 - 0.83),
                Store::Aim120 | Store::Aim120d | Store::R77 => (-0.06, x * 0.5 - 1.23),
                Store::Aim7 | Store::R27r | Store::R27t => (-0.08, x * 0.5 - 1.43),
                Store::Mk82 | Store::Fab250 => (-0.08, x * 0.4 + 0.93),
                Store::Fab500 => (-0.12, x * 0.4 + 0.93),
            };
            for s in [-1.0, 1.0] {
                stations.push(Station { store, at: v3(s * x, y, z), loaded: true, tip: false });
            }
        }
        let gun = gun_of(cfg.jet);
        Loadout { stations, gun, rounds: gun.rounds, chaff: CHAFF, flares: FLARES }
    }

    pub fn count(&self, store: Store) -> u32 {
        self.stations.iter().filter(|s| s.loaded && s.store == store).count() as u32
    }

    #[cfg(test)]
    pub fn next(&self, store: Store) -> Option<usize> {
        self.stations.iter().position(|s| s.loaded && s.store == store)
    }

    /// Stores left that a master mode releases.
    pub fn count_for(&self, mode: Mode) -> u32 {
        self.stations.iter().filter(|s| s.loaded && s.store.mode() == mode).count() as u32
    }

    /// The station a master mode releases next.
    pub fn next_for(&self, mode: Mode) -> Option<usize> {
        self.stations.iter().position(|s| s.loaded && s.store.mode() == mode)
    }

    /// The store a master mode releases next, or the last one it had.
    pub fn store_for(&self, mode: Mode) -> Option<Store> {
        self.next_for(mode).or_else(|| self.stations.iter().rposition(|s| s.store.mode() == mode)).map(|i| self.stations[i].store)
    }

    /// Mass on the stations, with the spent ammunition taken off.
    pub fn mass(&self) -> f64 {
        RAILS + self.stations.iter().filter(|s| s.loaded).map(|s| s.store.mass()).sum::<f64>() - (self.gun.rounds - self.rounds) as f64 * self.gun.expended
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
    seeker: Seeker::SemiActive,
    loal: false,
    datalink: false,
    call: "SAM LAUNCH",
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
    /// Free flight: the circuit it flies instead of chasing the player.
    patrol: Option<Patrol>,
}

/// A racetrack round the gorge: eight points on a circle, flown one way or
/// the other, at a height over the terrain.
#[derive(Clone, Copy, Debug)]
struct Patrol {
    radius: f64,
    /// +1 clockwise seen from above, -1 the other way.
    turn: i32,
    over: f64,
    next: usize,
}

impl Patrol {
    fn point(&self, k: usize) -> V3 {
        let z = PATROL_CENTRE_Z;
        let ang = k as f64 * std::f64::consts::TAU / PATROL_POINTS as f64;
        let p = v3(river_x(z) + ang.sin() * self.radius, 0.0, z - ang.cos() * self.radius);
        v3(p.x, high(p, self.over), p.z)
    }

    fn after(&self, k: usize) -> usize {
        (k as i32 + self.turn).rem_euclid(PATROL_POINTS as i32) as usize
    }
}

const PATROL_POINTS: usize = 8;
const PATROL_CENTRE_Z: f64 = scenario::PATROL_CENTRE_Z;
/// A bandit turns for the next point once within this of it, m.
const PATROL_REACHED: f64 = 3000.0;

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
    /// What the hits have done to the player's systems.
    pub battle: BattleDamage,
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
    /// Radar (or ground designation) lock, or the IRST's track.
    pub lock: Option<Who>,
    /// The MiG-29's OEPS-29 optronic sight tracks in place of the radar:
    /// the IRST finds and tracks passively, so the target's RWR stays
    /// quiet, and the laser rangefinder measures range inside 6.5 km.
    pub irst: bool,
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
    /// The ground sites as deployed, for free flight to rebuild them.
    pristine: Vec<Site>,
    rnd: Lcg,
    /// Airfield sortie: legs done (takeoff, steerpoint, landing), whether the
    /// wheels are on the runway, and the vertical speed a step ago.
    pub leg: u8,
    touched: bool,
    last_vs: f64,
    /// What the loadout screen hung, for a rearm on the ground in free
    /// flight, and when the next pair of bandits and the rebuilt targets come.
    cfg: Config,
    next_wave: Option<f64>,
    rebuild_at: Option<f64>,
    rearmed: bool,
    /// Bandits and ground sites shoot at the player; in free flight they
    /// are only targets.
    hostile: bool,
}

/// A trimmed F-16 at a point, heading (radians clockwise from north) and speed.
pub fn spawn(pos: V3, heading: f64, speed: f64) -> Aircraft {
    spawn_type(Type::F16, pos, heading, speed)
}

/// Wings-level 1 g trim for any airframe: the F-16 tables at the wing
/// loading this jet has, then the power its own engines need for the drag
/// of its own wing area and of the stores.
fn trim_for(a: &Aircraft, alt: f64, speed: f64) -> crate::sim::Trim {
    let k = a.kind;
    let mut t = trim(alt, speed, a.mass * crate::aero::WING_AREA / k.area());
    let atm = crate::sim::atmosphere_at(alt);
    let mach = speed / atm.sound;
    let need = crate::aero::thrust(t.power, mach, alt) * k.area() / crate::aero::WING_AREA + 0.5 * atm.rho * speed * speed * k.area() * a.stores_drag;
    let (mut lo, mut hi) = (0.0, 100.0);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if crate::aero::thrust(mid, mach, alt) * k.thrust_scale(mid) < need {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    t.power = 0.5 * (lo + hi);
    t.throttle = (t.power / 50.0).min(0.99);
    t
}

/// A trimmed jet of either type at a point, heading and speed.
pub fn spawn_type(kind: Type, pos: V3, heading: f64, speed: f64) -> Aircraft {
    let mut a = Aircraft::new(&Mission::new());
    a.set_type(kind);
    let t = trim_for(&a, pos.y, speed);
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
    a.mass = a.kind.empty_mass() + a.stores_mass + a.fuel;
    if a.kind == Type::F16 && a.stores_drag == 0.0 && (a.stores_mass - STORES_MASS).abs() < 1e-9 && a.fuel == crate::sim::spec::FUEL_MAX {
        return;
    }
    // Trim balances the clean airframe at the new weight; the stores' drag
    // then takes the power that adds its thrust.
    let t = trim_for(a, a.pos.y, a.speed);
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

/// Callsigns in the order the bandits are put up.
const CALLSIGNS: [&str; scenario::MAX_BANDITS as usize] = ["BANDIT 1", "BANDIT 2", "BANDIT 3", "BANDIT 4", "BANDIT 5", "BANDIT 6"];

/// The heat seekers a bandit carries on its outermost stations.
fn bandit_missile(kind: Type) -> &'static Spec {
    tip_store(kind).missile().unwrap_or(&AIM9M)
}

fn bandit(jet: Aircraft, skill: f64, callsign: &'static str) -> Bandit {
    Bandit { rounds: gun_of(jet.kind).rounds, jet, pilot: Pilot::new(skill), hp: 100.0, callsign, missiles: 2, alive: true, gun_acc: 0.0, fired: 0, trail: None, programs: CHAFF, cm_clock: 0.0, patrol: None }
}

/// Height `over` the terrain, and never under 2,600 m: above the ridges.
fn high(p: V3, over: f64) -> f64 {
    (terrain::floor(p.x, p.z) + over).max(2600.0)
}

/// Two bandits `range` ahead of a point coming head-on, high over the
/// ridges and spread 3 km apart, so the merge opens out and the fight has room.
fn pair(kind: Type, from: V3, fwd: V3, range: f64) -> [Bandit; 2] {
    let centre = from + fwd * range;
    let side = fwd.cross(V3::Y).norm();
    let back = heading_of(-fwd);
    let lead = centre + side * 1400.0;
    let wing = centre - side * 1600.0 + fwd * 2500.0;
    [
        bandit(spawn_type(kind, v3(lead.x, high(lead, 1300.0), lead.z), back, 235.0), 0.55, "BANDIT 1"),
        bandit(spawn_type(kind, v3(wing.x, high(wing, 1300.0) + 300.0, wing.z), back, 235.0), 0.35, "BANDIT 2"),
    ]
}

/// Free flight's pair: the lead on a 14 km circuit flown clockwise, the
/// wing on a 9 km one the other way and 300 m higher, so their paths never
/// meet. Each starts on its circuit heading for the next point.
fn patrol_pair(kind: Type) -> [Bandit; 2] {
    let fly = |radius: f64, turn: i32, over: f64, at: usize, skill: f64, callsign: &'static str| {
        let mut p = Patrol { radius, turn, over, next: 0 };
        p.next = p.after(at);
        let from = p.point(at);
        let mut b = bandit(spawn_type(kind, from, heading_of(p.point(p.next) - from), 235.0), skill, callsign);
        b.patrol = Some(p);
        b
    };
    [fly(14000.0, 1, 1300.0, 0, 0.55, "BANDIT 1"), fly(9000.0, -1, 1600.0, 4, 0.35, "BANDIT 2")]
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
                MissionKind::Dogfight | MissionKind::Free => Mode::Srm,
                MissionKind::Strike => Mode::Ccip,
            },
            loadout: Loadout::for_mission(kind),
            hp: 100.0,
            battle: BattleDamage::default(),
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
            irst: false,
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
            pristine: Vec::new(),
            rnd: Lcg(90210),
            leg: 0,
            touched: false,
            last_vs: 0.0,
            cfg: Config::for_mission(kind),
            next_wave: None,
            rebuild_at: None,
            rearmed: true,
            hostile: kind != MissionKind::Free,
        };
        // Free flight never runs a clock; an untimed run keeps none to score.
        m.timed &= kind != MissionKind::Free;
        m.time_left = if m.timed { kind.time() } else { 0.0 };
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
                c.bandits.extend(pair(Type::F16, m.start_pos, fwd, 20000.0));
                let late = m.start_pos + fwd * 9000.0 + side * 14000.0;
                c.reserve.push(bandit(spawn(v3(late.x, high(late, 1500.0) + 600.0, late.z), heading_of(-side), 240.0), 0.8, "BANDIT 3"));
                m.gates = vec![Gate { pos: v3(centre.x, high(centre, 1300.0), centre.z), radius: 0.0, axis: fwd, passed: false }];
                m.gate_index = 0;
            }
            MissionKind::Strike => {
                c.sites = targets::place();
                m.gates = (0..targets::GROUPS.len()).filter_map(|g| targets::group_point(&c.sites, g)).map(|pos| Gate { pos, radius: 0.0, axis: fwd, passed: false }).collect();
                m.gate_index = 0;
            }
            // The gate run's start and rings, with the strike's targets along
            // the river and a pair of bandits flying circuits round the gorge.
            MissionKind::Free => {
                c.sites = targets::place();
                c.bandits.extend(patrol_pair(Type::F16));
            }
        }
        c
    }

    /// Puts up the opposition the loadout screen chose (scenario::deploy):
    /// the bandits and the ground sites, and for the fight the steerpoint at
    /// the bandits, for the strike one at each target group.
    pub fn deploy(&mut self, cfg: &Config, m: &mut Mission) {
        if !matches!(self.kind, MissionKind::Dogfight | MissionKind::Strike | MissionKind::Free) {
            return;
        }
        let d = scenario::deploy(self.kind, cfg.aggressor, &cfg.threats, m);
        self.reserve.clear();
        self.bandits = d.bandits.iter().enumerate().map(|(i, s)| {
            let callsign = CALLSIGNS[i.min(CALLSIGNS.len() - 1)];
            match s.patrol {
                Some((radius, turn, over, at)) => {
                    let mut p = Patrol { radius, turn, over, next: 0 };
                    p.next = p.after(at);
                    let from = p.point(at);
                    let mut b = bandit(spawn_type(s.kind, from, heading_of(p.point(p.next) - from), s.speed), s.skill, callsign);
                    b.patrol = Some(p);
                    b
                }
                None => bandit(spawn_type(s.kind, s.pos, s.heading, s.speed), s.skill, callsign),
            }
        }).collect();
        self.sites = d.sites;
        self.pristine = self.sites.clone();
        let fwd = m.start_vel.norm();
        match self.kind {
            MissionKind::Dogfight if !self.bandits.is_empty() => {
                let centre = self.bandits.iter().fold(V3::ZERO, |a, b| a + b.jet.pos) / self.bandits.len() as f64;
                m.gates = vec![Gate { pos: centre, radius: 0.0, axis: fwd, passed: false }];
                m.gate_index = 0;
            }
            MissionKind::Strike => {
                m.gates = (0..targets::GROUPS.len()).filter_map(|g| targets::group_point(&self.sites, g)).map(|pos| Gate { pos, radius: 0.0, axis: fwd, passed: false }).collect();
                m.gate_index = 0;
            }
            _ => {}
        }
    }

    /// Free flight goes on until the pilot quits: a new pair of bandits on
    /// their circuits half a minute after the last one falls, the targets rebuilt
    /// half a minute after the last one burns, and the jet rearmed, refuelled
    /// and repaired once it stops on the ground.
    fn practice(&mut self, a: &mut Aircraft) {
        if self.bandits.iter().all(|b| !b.alive) {
            let at = *self.next_wave.get_or_insert(self.clock + 30.0);
            if self.clock >= at {
                self.next_wave = None;
                self.bandits.extend(patrol_pair(self.cfg.aggressor));
                self.events.push(Event::Lock { text: "NEW PAIR ON PATROL".to_string() });
            }
        }
        if self.sites.iter().all(|s| s.dead) {
            let at = *self.rebuild_at.get_or_insert(self.clock + 30.0);
            if self.clock >= at {
                self.rebuild_at = None;
                self.sites = if self.pristine.is_empty() { targets::place() } else { self.pristine.clone() };
                if matches!(self.lock, Some(Who::Ground(_))) {
                    self.lock = None;
                }
                self.events.push(Event::Toast("GROUND TARGETS REBUILT".to_string()));
            }
        }
        if !a.on_ground {
            self.rearmed = false;
        } else if !self.rearmed && a.speed < 1.0 {
            self.rearmed = true;
            self.loadout = Loadout::configure(&self.cfg);
            self.hp = 100.0;
            self.battle = BattleDamage::default();
            a.battle = self.battle;
            a.fuel = a.fuel.max(a.kind.fuel_max() * self.cfg.fuel);
            a.stores_mass = self.loadout.mass();
            a.wing_stores = self.loadout.wing_mass();
            a.stores_drag = self.loadout.drag();
            a.mass = a.kind.empty_mass() + a.stores_mass + a.fuel;
            self.events.push(Event::Toast("REARMED, REFUELLED AND REPAIRED".to_string()));
        }
    }

    /// Hangs the stores the loadout screen chose, and selects a master mode
    /// they allow.
    pub fn equip(&mut self, cfg: &Config) {
        // Bandits of the chosen type, where and as fast as the briefing put them.
        for b in self.bandits.iter_mut().chain(self.reserve.iter_mut()) {
            if b.jet.kind != cfg.aggressor && !cfg.threats.mixed {
                let j = &b.jet;
                b.jet = spawn_type(cfg.aggressor, j.pos, heading_of(j.vel), j.speed);
                b.rounds = gun_of(cfg.aggressor).rounds;
            }
        }
        self.cfg = *cfg;
        self.loadout = Loadout::configure(cfg);
        if !self.modes().contains(&self.mode) {
            self.mode = *self.modes().iter().rev().find(|&&m| m != Mode::Ccip).unwrap_or(&Mode::Gun);
        }
    }

    // ------------------------------------------------------------ controls
    /// Master modes the stores on board allow, in the order X cycles them.
    /// Only the weapons: NAV is where the jet starts, and the chaff and
    /// flares have their own button, never a place in the cycle.
    fn modes(&self) -> Vec<Mode> {
        let mut v = vec![Mode::Gun];
        for m in [Mode::Srm, Mode::Mrm, Mode::Ccip] {
            if self.loadout.count_for(m) > 0 {
                v.push(m);
            }
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

    /// The selected weapon by its real name and kind, NAV with none.
    pub fn weapon_name(&self) -> String {
        match self.mode {
            Mode::Nav => "NAV".to_string(),
            Mode::Gun => format!("{} (GUN)", self.loadout.gun.name),
            m => self.loadout.store_for(m).map_or("", |s| s.long_name()).to_string(),
        }
    }

    /// The selected weapon's short name and how many of it are left: of
    /// the store that goes next, where a mode has two kinds on board.
    pub fn weapon_left(&self) -> (&'static str, u32) {
        match self.mode {
            Mode::Nav => ("", 0),
            Mode::Gun => (self.loadout.gun.calibre, self.loadout.rounds),
            m => self.loadout.store_for(m).map_or(("", 0), |s| (s.name(), self.loadout.count(s))),
        }
    }

    /// The infrared missile selected with one left: its seeker is uncaged.
    pub fn heater(&self) -> Option<&'static Spec> {
        matches!(self.mode, Mode::Srm | Mode::Mrm).then(|| self.loadout.next_for(self.mode)).flatten().and_then(|i| self.loadout.stations[i].store.missile()).filter(|s| s.infrared())
    }

    /// Selects a master mode by name, for the test switches.
    pub fn select(&mut self, mode: Mode) {
        if mode == Mode::Nav || self.modes().contains(&mode) {
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
    /// range and not behind terrain, nearest the boresight first; with the
    /// IRST, the fighters inside its field and heat-detection range.
    fn candidates(&self, a: &Aircraft, ground: bool) -> Vec<(Who, f64)> {
        if self.irst && !ground {
            let look = self.helmet.unwrap_or(a.q.axes().forward);
            let mut v: Vec<(Who, f64)> = self.bandits.iter().enumerate().filter(|(_, b)| b.alive && irst_sees(a, &b.jet, 1.0)).map(|(i, b)| {
                let r = b.jet.pos - a.pos;
                (Who::Bandit(i), look.dot(r.norm()).clamp(-1.0, 1.0).acos() + r.len() / 400000.0)
            }).collect();
            v.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
            return v;
        }
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

    /// The lock if the radar holds it: what the semi-active and active
    /// missiles need, which an IRST track cannot give them.
    pub fn radar_lock(&self) -> Option<Who> {
        self.lock.filter(|_| !self.irst)
    }

    /// The MiG's sensor switch, radar or IRST; the F-16C has no IRST. The
    /// track is dropped as the sensors change over.
    pub fn toggle_irst(&mut self, a: &Aircraft) {
        let text = if a.kind != Type::Mig29 {
            "NO IRST ON THE F-16C"
        } else {
            self.irst = !self.irst;
            self.lock = None;
            if self.irst { "IRST // OEPS-29 SEARCH" } else { "RADAR // N019M SEARCH" }
        };
        self.events.push(Event::Lock { text: text.to_string() });
    }

    /// Laser range to the IRST's target, inside the rangefinder's 6.5 km;
    /// beyond it an IRST track gives the angle only.
    pub fn laser_range(&self, a: &Aircraft) -> Option<f64> {
        self.lock.filter(|_| self.irst).and_then(|w| self.track(w, a)).map(|(p, _)| (p - a.pos).len()).filter(|&r| r <= LASER_RANGE)
    }

    /// Drops the lock or designation, if there is one.
    pub fn drop_lock(&mut self) {
        if self.lock.take().is_some() {
            self.events.push(Event::Lock { text: "LOCK DROPPED".to_string() });
        }
    }

    /// Spreads a hit over the jet's systems: a round hits one, a warhead's
    /// burst several, 25 points to each. Returns the worst system's name.
    fn wound(&mut self, battle: &mut BattleDamage, kind: Type, dmg: f64) -> &'static str {
        let mut left = dmg;
        let mut what = "";
        while left > 0.0 {
            let part = left.min(25.0);
            let (pick, side) = (self.rnd.next(), self.rnd.next());
            what = battle.hit(kind, part, pick, side);
            left -= part;
        }
        what
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
        a.battle = self.battle;
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
        self.ccip = match (self.mode, self.loadout.next_for(Mode::Ccip)) {
            (Mode::Ccip, Some(i)) => {
                let st = self.loadout.stations[i];
                st.store.bomb().and_then(|b| ccip(b, a, st.at, &terrain::floor))
            }
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
        if self.kind == MissionKind::Free && alive {
            self.practice(a);
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
                    let gun = self.loadout.gun;
                    self.gun_acc += gun.rate * dt;
                    while self.gun_acc >= 1.0 && self.loadout.rounds > 0 {
                        self.gun_acc -= 1.0;
                        self.loadout.rounds -= 1;
                        self.fired += 1;
                        let (p, v) = muzzle(a);
                        let v = v + self.dispersion(v, 0.004);
                        self.rounds.push(Round::of(gun, p, v, self.fired % TRACER_EVERY == 0, Who::Player));
                    }
                }
            }
            Mode::Srm | Mode::Mrm if pressed => self.launch(a, self.mode),
            Mode::Ccip if pressed => {
                if let Some(i) = self.loadout.next_for(Mode::Ccip) {
                    let st = self.loadout.stations[i];
                    self.loadout.stations[i].loaded = false;
                    let b = Bomb::release(st.store.bomb().unwrap_or(&MK82), a, st.at);
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

    fn launch(&mut self, a: &Aircraft, mode: Mode) {
        let Some(i) = self.loadout.next_for(mode) else { return };
        let st = self.loadout.stations[i];
        let Some(spec) = st.store.missile() else { return };
        // Launch rules (docs/WEAPONS.md). A heat seeker goes for what its
        // seeker tracks; with no tone a lock-on-before-launch one leaves
        // unguided, while the AIM-9X takes the radar's lock as its cue and
        // acquires in flight. A radar missile goes for the lock: a
        // semi-active one is not fired without it, an active one fired
        // without it searches with its own radar.
        // The IRST's track cues a heat seeker but guides no radar missile.
        let lock = self.lock.filter(|w| matches!(w, Who::Bandit(_)));
        let target = match spec.seeker {
            Seeker::Infrared => self.seeker.or(if spec.loal { lock } else { None }),
            Seeker::SemiActive | Seeker::Active => self.radar_lock().filter(|w| matches!(w, Who::Bandit(_))),
        };
        if spec.semi_active() && target.is_none() {
            self.events.push(Event::Lock { text: "NO LOCK".to_string() });
            return;
        }
        self.loadout.stations[i].loaded = false;
        let kick = if spec.drop > 0.0 { 3.0 } else { 0.0 };
        let pos = a.pos + a.q.rotate(st.at);
        let vel = a.vel - a.q.axes().up * kick;
        self.events.push(Event::Launch { pos, from: Who::Player, name: spec.call });
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
            let keep = match (self.irst, l) {
                (true, Who::Bandit(i)) => self.bandits.get(i).is_some_and(|b| b.alive && irst_sees(a, &b.jet, 1.15)),
                _ => self.track(l, a).is_some_and(|(p, _)| off(p) < 70.0 * DEG && (p - a.pos).len() < 80000.0),
            };
            if !keep {
                self.lock = None;
                self.events.push(Event::Lock { text: "LOCK LOST".to_string() });
            }
        }
        if let Some(spec) = self.heater() {
            // Slaved to the radar lock inside its gimbal (40 degrees on the
            // AIM-9M, 45 on the R-73), or keeps what it has, or searches its
            // 2.5-degree field where the helmet looks (or along the boresight).
            let gimbal = spec.gimbal;
            // An infrared seeker sees a fighter a little past the missile's
            // own reach: some 17 km for the AIM-9M, 30 for the R-27T.
            let reach = spec.max_range * 1.2;
            let usable = |w: Who, limit: f64| self.track(w, a).is_some_and(|(p, _)| off(p) < limit && (p - a.pos).len() < reach && matches!(w, Who::Bandit(_)));
            let look = self.helmet.unwrap_or(fwd);
            let near_look = |p: V3| look.dot((p - a.pos).norm()).clamp(-1.0, 1.0).acos();
            let seeker = if let Some(l) = self.lock.filter(|&l| usable(l, gimbal)) {
                Some(l)
            } else if let Some(s) = self.seeker.filter(|&s| usable(s, gimbal)) {
                Some(s)
            } else {
                self.bandits.iter().enumerate().find(|(i, b)| b.alive && near_look(b.jet.pos) < spec.acquire * 1.6 && usable(Who::Bandit(*i), gimbal)).map(|(i, _)| Who::Bandit(i))
            };
            self.seeker = seeker;
        } else {
            self.seeker = None;
        }
        self.zone_clock -= dt;
        if self.zone_clock <= 0.0 {
            self.zone_clock = 0.25;
            let spec = self.loadout.next_for(self.mode).and_then(|i| self.loadout.stations[i].store.missile());
            let shot = match (self.mode, spec) {
                (Mode::Srm | Mode::Mrm, Some(spec)) if spec.infrared() => self.seeker.map(|s| (spec, s)),
                (Mode::Mrm, Some(spec)) => self.radar_lock().filter(|w| matches!(w, Who::Bandit(_))).map(|l| (spec, l)),
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
                // Neither jet has a missile approach warner: the RWR gives a
                // radar missile away, but a heat seeker is only seen, by its
                // smoke while the motor burns or close in.
                .filter(|m| {
                    let d = (m.pos - self.bandits[i].jet.pos).len();
                    !m.spec.infrared() || d < 3000.0 || m.burning() && d < 6000.0
                })
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
            // On patrol the foe it steers for is a mark parked on its next
            // point, as the autopilot does; the player is no part of it.
            let mark = b.patrol.as_mut().map(|p| {
                let flat = |v: V3| v3(v.x, 0.0, v.z);
                if flat(p.point(p.next) - b.jet.pos).len() < PATROL_REACHED {
                    p.next = p.after(p.next);
                }
                let mut mark = b.jet.clone();
                mark.pos = p.point(p.next);
                mark.vel = V3::ZERO;
                mark
            });
            let pic = Picture { foe: mark.as_ref().unwrap_or(a), missile: threat, missiles: b.missiles, missile_spec: bandit_missile(b.jet.kind), rounds: b.rounds };
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
            if self.hostile && o.gun && b.rounds > 0 && !self.player_dead {
                b.gun_acc += gun_of(b.jet.kind).rate * dt;
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
            let gun = gun_of(self.bandits[i].jet.kind);
            for (p, v, tracer) in shots {
                let jitter = self.dispersion(v, 0.005);
                self.rounds.push(Round::of(gun, p, v + jitter, tracer, Who::Bandit(i)));
            }
            let b = &mut self.bandits[i];
            if self.hostile && o.missile && b.missiles > 0 && !self.player_dead {
                b.missiles -= 1;
                let side = if b.missiles % 2 == 0 { -1.0 } else { 1.0 };
                let t = crate::airframe::AIM9_TIP;
                let at = v3(side * t.x, t.y, t.z);
                let at = if b.jet.kind == Type::Mig29 { at + crate::mig29::store_shift(at) } else { at };
                let pos = b.jet.pos + b.jet.q.rotate(at);
                let vel = b.jet.vel;
                let spec = bandit_missile(b.jet.kind);
                self.events.push(Event::Launch { pos, from: Who::Bandit(i), name: "MISSILE LAUNCH" });
                self.warn = 4.0;
                self.fire_missile(Missile::launch(spec, pos, vel, Who::Bandit(i), Some(Who::Player)));
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
                if self.hp > 0.0 {
                    let mut battle = self.battle;
                    let what = self.wound(&mut battle, a.kind, dmg);
                    self.battle = battle;
                    self.events.push(Event::Toast(format!("HIT // {what}")));
                }
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
                    } else {
                        let (mut battle, kind) = (self.bandits[i].jet.battle, self.bandits[i].jet.kind);
                        self.wound(&mut battle, kind, dmg);
                        self.bandits[i].jet.battle = battle;
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
            let spec = self.missiles[i].spec;
            // A player's AMRAAM or R-77 rides the datalink while the radar
            // holds its target; if the lock goes before it is close enough to
            // see the target with its own radar, it flies on blind and
            // searches when it gets there.
            if spec.active() && spec.datalink && self.missiles[i].from == Who::Player && self.missiles[i].target.is_some() && self.radar_lock() != self.missiles[i].target {
                let far = self.missiles[i].target.and_then(|w| self.track(w, a)).is_none_or(|(p, _)| (p - self.missiles[i].pos).len() > ACTIVE_RANGE);
                if far {
                    self.missiles[i].target = None;
                    self.missiles[i].tracking = false;
                }
            }
            // An active missile with no target searches with its own radar;
            // an AIM-9X fired with nothing to cue it, with its seeker.
            let searching = (spec.active() || spec.infrared() && spec.loal) && self.missiles[i].from == Who::Player;
            if searching && self.missiles[i].target.is_none() && self.missiles[i].age > 1.5 {
                let (mp, mv) = (self.missiles[i].pos, self.missiles[i].vel.norm());
                let (reach, cone) = if spec.active() { (ACTIVE_RANGE, spec.acquire) } else { (8000.0, 20.0 * DEG) };
                let found = self.bandits.iter().enumerate().filter(|(_, b)| b.alive).map(|(k, b)| (k, b.jet.pos - mp)).filter(|(_, r)| r.len() < reach && mv.dot(r.norm()) > cone.cos()).min_by(|x, y| x.1.len().partial_cmp(&y.1.len()).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((k, _)) = found {
                    self.missiles[i].target = Some(Who::Bandit(k));
                    self.missiles[i].tracking = true;
                }
            }
            // An R-27R or Sparrow goes blind once the player's radar drops its target.
            if spec.semi_active() && self.missiles[i].from == Who::Player && self.radar_lock() != self.missiles[i].target {
                self.missiles[i].tracking = false;
            }
            // A semi-active SAM goes blind when its radar dies.
            if let Who::Ground(g) = self.missiles[i].from {
                let launcher = self.sites[g].pos;
                if !self.sites.iter().any(|s| s.kind == Kind::Radar && !s.dead && (s.pos - launcher).len() < 300.0) {
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
            let (armed, spec) = (self.bombs[i].armed(), self.bombs[i].spec);
            self.bombs.swap_remove(i);
            if !armed {
                self.events.push(Event::Dud { pos: p });
                continue;
            }
            self.events.push(Event::Blast { pos: p, scale: 1.25, burn: 0.0 });
            for k in 0..self.sites.len() {
                if !self.sites[k].dead {
                    let d = self.sites[k].distance(p);
                    self.damage(Who::Ground(k), blast_damage(spec.damage, spec.lethal, spec.outer, d), a);
                }
            }
            // Fragments reach a jet that drops too low.
            let d = (a.pos - p).len();
            self.damage(Who::Player, blast_damage(60.0, 30.0, 160.0, d), a);
        }
    }

    /// The ZSU-23-4s shoot at the player when he is in reach and in sight,
    /// in bursts, leading him from a lagging track; each SA-6 launcher fires
    /// while its own battery's radar sees him above the valley clutter, one
    /// round every 25 s.
    fn step_sites(&mut self, dt: f64, a: &Aircraft) {
        if !self.hostile {
            return;
        }
        for k in 0..self.sites.len() {
            if self.sites[k].dead {
                continue;
            }
            match self.sites[k].kind {
                Kind::Zsu => self.flak(k, dt, a),
                Kind::Launcher => {
                    self.sites[k].clock -= dt;
                    let s = &self.sites[k];
                    let own = |r: &&Site| r.kind == Kind::Radar && !r.dead && (r.pos - s.pos).len() < 300.0;
                    let Some(radar) = self.sites.iter().find(own) else { continue };
                    let eye = radar.eye();
                    let range = (a.pos - eye).len();
                    if s.clock <= 0.0 && s.ammo > 0 && a.agl > 300.0 && range > SA6.min_range && range < scenario::SAM_RANGE && line_of_sight(eye, a.pos) {
                        self.sites[k].clock = 25.0;
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
            MissionKind::Gates | MissionKind::Airfield | MissionKind::Free => {}
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
            // Free flight has no total: kills and ground targets destroyed.
            MissionKind::Free => (self.kills, self.destroyed),
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
                let locked = self.hostile && r < 15000.0 && b.jet.q.axes().forward.dot((a.pos - b.jet.pos).norm()) > 0.85;
                v.push((b.jet.pos, if b.jet.kind == Type::Mig29 { "29" } else { "16" }, locked));
            }
        }
        for s in self.sites.iter().filter(|s| !s.dead) {
            let r = (a.pos - s.pos).len();
            match s.kind {
                Kind::Zsu if r < 6000.0 => v.push((s.pos, "A", self.hostile && r < 2500.0)),
                Kind::Radar if r < 30000.0 => v.push((s.pos, "6", self.hostile && r < 14000.0 && a.agl > 300.0)),
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
        assert!((strike.mass() - (STORES_MASS + 3.0 * MK82_MASS - 100.0 * M61A1.expended)).abs() < 1e-9);
        let fight = Loadout::for_mission(MissionKind::Dogfight);
        assert_eq!((fight.count(Store::Aim9), fight.count(Store::Aim120)), (2, 2));
    }

    /// The MiG-29 hangs its own stores: R-73s on the outer pylons, the
    /// F-16 choices swapped for theirs, and the GSh-30-1 with 150 rounds;
    /// changing jets back gives the F-16 its own again.
    #[test]
    fn the_fulcrum_carries_its_own_stores() {
        let mut cfg = Config::for_jet(MissionKind::Strike, Type::Mig29);
        assert_eq!((cfg.outboard, cfg.inboard), (Some(Store::Fab250), Some(Store::Fab500)));
        let l = Loadout::configure(&cfg);
        assert_eq!((l.count(Store::R73), l.count(Store::Aim9), l.count_for(Mode::Ccip)), (2, 0, 4));
        assert_eq!((l.gun.name, l.rounds), ("GSH-30-1", 150));
        let dog = Loadout::configure(&Config::for_jet(MissionKind::Dogfight, Type::Mig29));
        assert_eq!((dog.count(Store::R73), dog.count(Store::R77)), (4, 2));
        for _ in 0..6 {
            cfg.cycle(2);
            assert!(cfg.inboard.is_none_or(|s| matches!(s, Store::R27r | Store::R27t | Store::R77 | Store::Fab500 | Store::Fab250)));
        }
        cfg.cycle(0);
        assert_eq!(cfg.tip, Store::R60m);
        assert_eq!(Config::for_jet(MissionKind::Dogfight, Type::Mig29).stores_summary(), "4 R-73 2 R-77");
        cfg.inboard = Some(Store::R27r);
        cfg.set_jet(Type::F16);
        assert_eq!((cfg.tip, cfg.outboard, cfg.inboard), (Store::Aim9, Some(Store::Mk82), Some(Store::Aim7)));
        assert_eq!(Loadout::configure(&cfg).gun.rounds, 511);
    }

    /// The weapon families each jet fills, and the one it does not: the
    /// F-16C has no medium-range heat seeker to answer the R-27T.
    #[test]
    fn arsenal_by_family() {
        let names = |jet, f| arsenal(jet, f).iter().map(|s| s.name()).collect::<Vec<_>>().join(" ");
        assert_eq!(names(Type::F16, Family::ShortIr), "AIM-9M AIM-9X");
        assert_eq!(names(Type::F16, Family::MediumIr), "");
        assert_eq!(names(Type::F16, Family::SemiActive), "AIM-7M");
        assert_eq!(names(Type::F16, Family::ActiveRadar), "AIM-120C AIM-120D");
        assert_eq!(names(Type::F16, Family::Bomb), "MK-82");
        assert_eq!(names(Type::Mig29, Family::ShortIr), "R-73 R-60M");
        assert_eq!(names(Type::Mig29, Family::MediumIr), "R-27T");
        assert_eq!(names(Type::Mig29, Family::SemiActive), "R-27R");
        assert_eq!(names(Type::Mig29, Family::ActiveRadar), "R-77");
        assert_eq!(names(Type::Mig29, Family::Bomb), "FAB-250 FAB-500");
        // Every store belongs to exactly one family and has a counterpart
        // on the other jet in the same family where that jet has one.
        for s in Store::ALL {
            let c = s.counterpart();
            if s.family() != Family::MediumIr {
                assert_eq!(c.family(), s.family(), "{s:?} and {c:?}");
            }
        }
    }

    /// The R-27R needs a lock to be fired and guides only while it lasts.
    #[test]
    fn the_r27r_rides_the_lock() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        let mut cfg = Config::for_jet(MissionKind::Dogfight, Type::Mig29);
        cfg.inboard = Some(Store::R27r);
        c.equip(&cfg);
        c.select(Mode::Mrm);
        let mut a = spawn_type(Type::Mig29, v3(0.0, 4000.0, 0.0), 0.0, 230.0);
        a.update(0.0, &Input::default());
        c.lock = None;
        c.launch(&a, Mode::Mrm);
        assert!(c.missiles.is_empty() && c.loadout.count(Store::R27r) == 2, "no shot without a lock");
        c.lock = Some(Who::Bandit(0));
        c.launch(&a, Mode::Mrm);
        assert_eq!((c.missiles.len(), c.missiles[0].spec.name), (1, "R-27R"));
        c.lock = None;
        c.step_missiles(crate::sim::FIXED_DT, &a);
        assert!(!c.missiles[0].tracking, "blind once the lock is gone");
    }

    /// The MiG's IRST sees a fighter from behind further than head on, and
    /// only inside its field; its track cues the R-73 but no radar missile,
    /// and the laser gives a range only inside 6.5 km. The F-16 has none.
    #[test]
    fn the_irst_tracks_passively() {
        terrain::init();
        let mut a = spawn_type(Type::Mig29, v3(0.0, 4000.0, 0.0), 0.0, 230.0);
        a.update(0.0, &Input::default());
        let ahead = |d: f64, heading: f64| spawn(a.pos + a.q.axes().forward * d, heading, 200.0);
        let toward = heading_of(-a.q.axes().forward);
        let away = heading_of(a.q.axes().forward);
        assert!(irst_sees(&a, &ahead(6000.0, toward), 1.0));
        assert!(!irst_sees(&a, &ahead(12000.0, toward), 1.0), "head on, past 8 km");
        assert!(irst_sees(&a, &ahead(12000.0, away), 1.0), "from behind, 12 km");
        let off = spawn(a.pos + Quat::axis_angle(V3::Y, 40.0 * DEG).rotate(a.q.axes().forward) * 5000.0, away, 200.0);
        assert!(!irst_sees(&a, &off, 1.0), "outside its 30 degrees");

        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        let mut cfg = Config::for_jet(MissionKind::Dogfight, Type::Mig29);
        cfg.inboard = Some(Store::R27r);
        c.equip(&cfg);
        c.bandits.truncate(1);
        c.bandits[0].jet = ahead(9000.0, away);
        c.toggle_irst(&a);
        assert!(c.irst);
        c.designate(&a);
        assert_eq!(c.lock, Some(Who::Bandit(0)));
        assert_eq!((c.radar_lock(), c.laser_range(&a)), (None, None), "angle only past the laser");
        c.select(Mode::Mrm);
        c.launch(&a, Mode::Mrm);
        assert!(c.missiles.is_empty(), "no R-27R on an IRST track");
        c.bandits[0].jet = ahead(5000.0, away);
        assert!(c.laser_range(&a).is_some_and(|r| (r - 5000.0).abs() < 1.0));

        let mut f = Combat::new(MissionKind::Dogfight, &mut Mission::new());
        f.toggle_irst(&spawn(v3(0.0, 4000.0, 0.0), 0.0, 230.0));
        assert!(!f.irst);
    }

    /// Launch rules by seeker: a Sparrow is not fired without a lock; an
    /// AMRAAM fired without one goes active and finds the bandit ahead; one
    /// fired on a lock that is then dropped far out loses the datalink and
    /// flies on blind; an AIM-9X with no tone takes the lock as its cue.
    #[test]
    fn launch_rules_follow_the_seeker() {
        terrain::init();
        let fire = |store: Store, lock: Option<Who>| {
            let mut m = Mission::new();
            let mut c = Combat::new(MissionKind::Dogfight, &mut m);
            let mut cfg = Config::for_mission(MissionKind::Dogfight);
            cfg.outboard = Some(store);
            if store.mode() == Mode::Srm {
                cfg.tip = store;
            }
            c.equip(&cfg);
            let mut a = spawn(v3(0.0, 4000.0, 0.0), 0.0, 230.0);
            a.update(0.0, &Input::default());
            // One bandit straight ahead, 12 km out and flying away.
            c.bandits.truncate(1);
            c.bandits[0].jet = spawn(a.pos + a.q.axes().forward * 12000.0, 0.0, 200.0);
            c.lock = lock;
            c.seeker = None;
            c.launch(&a, store.mode());
            (c, a)
        };
        let (c, _) = fire(Store::Aim7, None);
        assert!(c.missiles.is_empty(), "no Sparrow without a lock");
        let (mut c, a) = fire(Store::Aim120, None);
        assert_eq!(c.missiles[0].target, None);
        for _ in 0..240 {
            c.step_missiles(crate::sim::FIXED_DT, &a);
        }
        assert_eq!(c.missiles[0].target, Some(Who::Bandit(0)), "a maddog AMRAAM finds the bandit");
        let (mut c, a) = fire(Store::Aim120d, Some(Who::Bandit(0)));
        c.bandits[0].jet.pos = a.pos + a.q.axes().forward * 40000.0;
        c.lock = None;
        c.step_missiles(crate::sim::FIXED_DT, &a);
        assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking, "no datalink without the lock");
        let (c, _) = fire(Store::Aim9x, Some(Who::Bandit(0)));
        assert_eq!(c.missiles[0].target, Some(Who::Bandit(0)), "the AIM-9X locks on after launch");
        let (c, _) = fire(Store::Aim9, Some(Who::Bandit(0)));
        assert_eq!(c.missiles[0].target, None, "an AIM-9M with no tone leaves unguided");
    }

    /// The MiG-29 spawns trimmed: hands off it holds its height and speed,
    /// and its engines outpull the F110 against its own weight.
    #[test]
    fn a_fulcrum_spawns_trimmed() {
        crate::terrain::init();
        let mut a = spawn_type(Type::Mig29, v3(0.0, 3000.0, 0.0), 0.0, 230.0);
        assert_eq!(a.kind, Type::Mig29);
        for _ in 0..(20.0 / crate::sim::FIXED_DT) as usize {
            a.update(crate::sim::FIXED_DT, &Input::default());
        }
        assert!(!a.crashed && (a.pos.y - 3000.0).abs() < 150.0 && (a.speed - 230.0).abs() < 25.0, "after 20 s: {:.0} m, {:.0} m/s", a.pos.y, a.speed);
        let f16 = spawn(v3(0.0, 3000.0, 0.0), 0.0, 230.0);
        let ab = |k: Type| crate::aero::thrust(100.0, 0.0, 0.0) * k.thrust_scale(100.0) / (k.empty_mass() + 3000.0);
        assert!(ab(Type::Mig29) > ab(Type::F16), "{} vs {}", ab(Type::Mig29), ab(f16.kind));
    }

    /// The Fulcrum's manual limits: 8 g above 14,200 kg, 9 g below, and the
    /// AoA held near 25 degrees at full aft stick when slow.
    #[test]
    fn fulcrum_limits_follow_the_manual() {
        crate::terrain::init();
        let pull = Input { pitch_axis: 1.0, ..Default::default() };
        let peak = |fuel: f64, speed: f64| {
            let mut a = spawn_type(Type::Mig29, v3(0.0, 4000.0, 0.0), 0.0, speed);
            a.fuel = Type::Mig29.fuel_max() * fuel;
            a.mass = Type::Mig29.empty_mass() + a.stores_mass + a.fuel;
            let (mut g, mut alpha) = (0.0f64, 0.0f64);
            for _ in 0..(4.0 / crate::sim::FIXED_DT) as usize {
                a.update(crate::sim::FIXED_DT, &pull);
                g = g.max(a.g_load);
                alpha = alpha.max(a.alpha_deg);
            }
            (a.mass, g, alpha)
        };
        let (m, g, _) = peak(1.0, 260.0);
        assert!(m > 14200.0 && g < 8.6 && g > 7.5, "heavy: {m:.0} kg pulled {g:.2} g");
        let (m, g, _) = peak(0.3, 260.0);
        assert!(m < 14200.0 && g > 8.6 && g < 9.6, "light: {m:.0} kg pulled {g:.2} g");
        let (_, _, alpha) = peak(0.5, 120.0);
        assert!(alpha > 23.0 && alpha < 27.0, "slow pull reached {alpha:.1} deg");
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
        // Weapons only: NAV is never cycled back to, and the countermeasures
        // never come up.
        assert_eq!(seen, vec![Mode::Nav, Mode::Gun, Mode::Srm, Mode::Gun]);
        let mut f = Combat::new(MissionKind::Free, &mut Mission::new());
        let mut cfg = Config::for_jet(MissionKind::Free, Type::Mig29);
        (cfg.outboard, cfg.inboard) = (Some(Store::Fab250), Some(Store::R27r));
        f.equip(&cfg);
        let mut names = Vec::new();
        for _ in 0..4 {
            f.cycle_weapon();
            assert_ne!(f.mode, Mode::Nav);
            let line = crate::symbology::weapon_line(&f);
            assert!(!line.contains("CHAFF") && !line.contains("FLARE"), "{line}");
            names.push(line);
        }
        assert!(names.iter().any(|n| n.starts_with("GSH-30-1 (GUN) x150")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("R-73 ARCHER (FOX 2) x2")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("R-27R ALAMO-A (FOX 1) x2")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("FAB-")), "{names:?}");
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

    /// Free flight runs no clock, sends a new pair half a minute after the
    /// sky is clear, rebuilds the targets, and rearms a jet stopped on the
    /// ground; a mission with the clock switched off keeps no time either.
    #[test]
    fn free_flight_comes_back_and_rearms() {
        terrain::init();
        let mut m = Mission::new();
        m.timed = false;
        Combat::new(MissionKind::Strike, &mut m);
        assert_eq!(m.time_left, 0.0);
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Free, &mut m);
        assert!(!m.timed && m.time_left == 0.0);
        assert!(c.bandits.len() == 2 && !c.sites.is_empty() && m.gates.len() == 10);
        let mut a = spawn(v3(river_x(0.0), 4200.0, 0.0), 0.0, 240.0);
        for b in c.bandits.iter_mut() {
            b.alive = false;
        }
        for s in c.sites.iter_mut() {
            s.dead = true;
        }
        c.practice(&mut a);
        assert_eq!(c.bandits.len(), 2, "no pair before the half minute");
        c.clock += 31.0;
        c.practice(&mut a);
        assert_eq!(c.bandits.iter().filter(|b| b.alive).count(), 2);
        assert!(c.bandits[2].jet.kind == Type::Mig29 && c.bandits[2..].iter().all(|b| b.patrol.is_some()));
        assert!(c.sites.iter().all(|s| !s.dead));
        let i = c.loadout.next(Store::Aim9).unwrap();
        c.loadout.stations[i].loaded = false;
        c.hp = 30.0;
        c.practice(&mut a);
        a.on_ground = true;
        a.vel = V3::ZERO;
        a.speed = 0.0;
        c.practice(&mut a);
        assert!(c.loadout.count(Store::Aim9) == 2 && c.hp == 100.0);
    }

    /// In free flight a bandit nose-on at gun range and the guns along the
    /// river hold their fire.
    #[test]
    fn free_flight_enemies_hold_fire() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Free, &mut m);
        let zsu = c.sites.iter().find(|s| s.kind == Kind::Zsu).unwrap().pos;
        let mut a = spawn(zsu + v3(0.0, 600.0, 1200.0), 0.0, 200.0);
        let fwd = a.q.axes().forward;
        c.bandits[0].jet = spawn(a.pos + fwd * 700.0, std::f64::consts::PI, 200.0);
        for _ in 0..(8.0 / FIXED_DT_TEST) as usize {
            a.pos += a.vel * FIXED_DT_TEST;
            c.step(FIXED_DT_TEST, &mut a, false, &mut m);
        }
        assert!(c.rounds.is_empty() && c.missiles.is_empty() && c.hp == 100.0 && !c.player_dead);
        assert!(c.bandits.iter().all(|b| b.rounds == gun_of(b.jet.kind).rounds && b.missiles == 2));
        assert!(c.emitters(&a).iter().all(|e| !e.2));
    }

    /// Free flight's bandits fly their circuits round the gorge from point
    /// to point and pay no mind to a player sitting right beside them.
    #[test]
    fn free_flight_bandits_fly_their_circuits() {
        terrain::init();
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Free, &mut m);
        let centre = v3(river_x(PATROL_CENTRE_Z), 0.0, PATROL_CENTRE_Z);
        let start = c.bandits[0].jet.pos;
        let mut a = spawn(start + v3(600.0, 0.0, 600.0), 0.0, 0.0);
        let mut next = [0, 1].map(|i| c.bandits[i].patrol.unwrap().next);
        let mut legs = [0usize; 2];
        for _ in 0..(150.0 / FIXED_DT_TEST) as usize {
            c.step(FIXED_DT_TEST, &mut a, false, &mut m);
            for (i, b) in c.bandits.iter().enumerate() {
                let p = b.patrol.unwrap();
                let r = v3(b.jet.pos.x - centre.x, 0.0, b.jet.pos.z - centre.z).len();
                assert!(b.alive && (r - p.radius).abs() < 5000.0, "bandit {i} {r:.0} m from the centre of a {:.0} m circuit", p.radius);
                if p.next != next[i] {
                    next[i] = p.next;
                    legs[i] += 1;
                }
            }
        }
        assert!(legs.iter().all(|&n| n >= 2), "points reached {legs:?}");
        assert!((c.bandits[0].jet.pos - a.pos).len() > 5000.0, "the lead stayed with the player");
    }

    const FIXED_DT_TEST: f64 = crate::sim::FIXED_DT;
}
