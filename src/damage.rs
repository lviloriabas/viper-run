//! Battle damage by module, from where each round, fragment and blast
//! actually strikes the airframe. Every jet is a set of boxes in body axes,
//! each a system with its own hit points: the pilot, the engine (two on the
//! MiG-29), the fuel tanks, the wings, the stabilators and fin, the
//! hydraulics and the airframe structure. A round flies its path through
//! them and damages what it crosses, the first most, and its high explosive
//! reaches what lies close by; a warhead throws a ring of fragments round
//! its axis that hit whatever they meet, and its blast wrecks everything
//! near the burst. Incendiary hits can set a tank or an engine on fire.
//! What is left of each system becomes the flight model's `BattleDamage`,
//! and some losses end the flight at once: the pilot killed, a wing or the
//! tail shot away, the structure broken or a burning tank exploding.
use crate::math::*;
use crate::sim::{BattleDamage, Type};
use crate::weapons::hits_box;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Pilot,
    Engine,
    Fuel,
    Wing,
    Stabilators,
    Fin,
    Hydraulics,
    Structure,
}

/// One system: a box in body axes (x right, y up, z aft, from the centre
/// of gravity), its hit points, the side it is on (-1 left, 1 right, 0 on
/// the centreline) and its caution name.
pub struct Module {
    pub part: Part,
    pub at: V3,
    pub half: V3,
    pub hp: f64,
    pub side: f64,
    pub name: &'static str,
}

const fn m(part: Part, at: V3, half: V3, hp: f64, side: f64, name: &'static str) -> Module {
    Module { part, at, half, hp, side, name }
}

/// The F-16C: the cockpit under the canopy, the F-1 and A-1 fuselage tanks
/// ahead of and over the wing root, a tank in each wing, the F110 from the
/// wing root to the nozzle, the A and B hydraulic lines down the middle,
/// the wings, the stabilators and the fin, inside the fuselage structure.
const F16: [Module; 12] = [
    m(Part::Pilot, v3(0.0, 1.05, -3.25), v3(0.28, 0.40, 0.42), 5.0, 0.0, "PILOT"),
    m(Part::Fuel, v3(0.0, 0.55, -1.60), v3(0.45, 0.35, 0.75), 22.0, 0.0, "FWD FUEL TANK"),
    m(Part::Fuel, v3(0.0, 0.55, 0.30), v3(0.45, 0.35, 0.75), 22.0, 0.0, "AFT FUEL TANK"),
    m(Part::Fuel, v3(-2.0, 0.28, 0.90), v3(0.85, 0.09, 0.85), 18.0, -1.0, "LEFT WING TANK"),
    m(Part::Fuel, v3(2.0, 0.28, 0.90), v3(0.85, 0.09, 0.85), 18.0, 1.0, "RIGHT WING TANK"),
    m(Part::Engine, v3(0.0, 0.25, 3.40), v3(0.50, 0.50, 2.30), 30.0, 0.0, "ENGINE"),
    m(Part::Hydraulics, v3(0.0, 0.0, 1.50), v3(0.25, 0.25, 3.00), 18.0, 0.0, "HYDRAULICS"),
    m(Part::Wing, v3(-3.0, 0.28, 1.20), v3(1.90, 0.12, 1.50), 45.0, -1.0, "LEFT WING"),
    m(Part::Wing, v3(3.0, 0.28, 1.20), v3(1.90, 0.12, 1.50), 45.0, 1.0, "RIGHT WING"),
    m(Part::Stabilators, v3(0.0, 0.40, 5.60), v3(2.70, 0.15, 1.00), 35.0, 0.0, "STABILATORS"),
    m(Part::Fin, v3(0.0, 2.10, 5.20), v3(0.12, 1.30, 1.20), 20.0, 0.0, "FIN"),
    m(Part::Structure, v3(0.0, 0.45, -0.20), v3(0.65, 0.65, 6.70), 45.0, 0.0, "AIRFRAME"),
];

/// The MiG-29S: the cockpit, the enlarged No. 1 tank in the spine, the
/// tanks between the nacelles and in the wings, two RD-33s in their
/// nacelles, the hydraulics, the wings, the stabilators and the twin fins.
const MIG29: [Module; 14] = [
    m(Part::Pilot, v3(0.0, 1.15, -3.55), v3(0.28, 0.40, 0.42), 6.0, 0.0, "PILOT"),
    m(Part::Fuel, v3(0.0, 0.95, -1.20), v3(0.40, 0.35, 0.90), 22.0, 0.0, "SPINE FUEL TANK"),
    m(Part::Fuel, v3(0.0, 0.55, 0.90), v3(0.55, 0.30, 1.00), 22.0, 0.0, "CENTRE FUEL TANK"),
    m(Part::Fuel, v3(-2.6, 0.45, 1.60), v3(1.00, 0.09, 0.90), 18.0, -1.0, "LEFT WING TANK"),
    m(Part::Fuel, v3(2.6, 0.45, 1.60), v3(1.00, 0.09, 0.90), 18.0, 1.0, "RIGHT WING TANK"),
    m(Part::Engine, v3(-1.1, 0.15, 3.60), v3(0.45, 0.45, 2.60), 24.0, -1.0, "LEFT ENGINE"),
    m(Part::Engine, v3(1.1, 0.15, 3.60), v3(0.45, 0.45, 2.60), 24.0, 1.0, "RIGHT ENGINE"),
    m(Part::Hydraulics, v3(0.0, 0.20, 1.50), v3(0.30, 0.25, 3.20), 20.0, 0.0, "HYDRAULICS"),
    m(Part::Wing, v3(-3.6, 0.45, 2.00), v3(2.00, 0.12, 1.60), 50.0, -1.0, "LEFT WING"),
    m(Part::Wing, v3(3.6, 0.45, 2.00), v3(2.00, 0.12, 1.60), 50.0, 1.0, "RIGHT WING"),
    m(Part::Stabilators, v3(0.0, 0.36, 6.40), v3(2.60, 0.15, 0.90), 38.0, 0.0, "STABILATORS"),
    m(Part::Fin, v3(-1.6, 1.60, 5.40), v3(0.12, 1.20, 1.30), 18.0, -1.0, "LEFT FIN"),
    m(Part::Fin, v3(1.6, 1.60, 5.40), v3(0.12, 1.20, 1.30), 18.0, 1.0, "RIGHT FIN"),
    m(Part::Structure, v3(0.0, 0.50, -0.30), v3(0.75, 0.60, 7.00), 55.0, 0.0, "AIRFRAME"),
];

pub fn layout(kind: Type) -> &'static [Module] {
    match kind {
        Type::F16 => &F16,
        Type::Mig29 => &MIG29,
    }
}

/// Share of a round's damage each module along its path takes: it loses
/// energy in everything it goes through.
const PENETRATION: [f64; 4] = [1.0, 0.6, 0.35, 0.2];
/// Burn rate of a fire as it starts, hit points a second, in the burning
/// system and in the structure round it (it grows as it burns), and the
/// chance a second that it goes out. A badly damaged engine loses its oil
/// and seizes: below half its hit points it loses `SEIZE` a second.
const BURN: f64 = 2.0;
const BURN_STRUCTURE: f64 = 1.0;
const FIRE_OUT: f64 = 0.06;
const SEIZE: f64 = 0.5;

/// A jet's damage state: the hit points left in each module, a fire and
/// what destroyed the jet, if anything has.
#[derive(Clone, Debug)]
pub struct Hull {
    pub kind: Type,
    pub hp: Vec<f64>,
    /// The burning module and how long it has burnt.
    pub fire: Option<(usize, f64)>,
    /// The MiG-29's engine fire extinguisher, good for one shot; the
    /// F-16C has none.
    extinguisher: bool,
    pub dead: Option<&'static str>,
}

impl Hull {
    pub fn new(kind: Type) -> Hull {
        Hull { kind, hp: layout(kind).iter().map(|m| m.hp).collect(), fire: None, extinguisher: kind == Type::Mig29, dead: None }
    }

    fn left(&self, i: usize) -> f64 {
        (self.hp[i] / layout(self.kind)[i].hp).clamp(0.0, 1.0)
    }

    /// The modules a segment crosses (body axes), in order, with the
    /// distance along it where it enters each.
    fn crossed(&self, p0: V3, p1: V3) -> Vec<(usize, f64)> {
        let len = (p1 - p0).len();
        let mut v: Vec<(usize, f64)> = layout(self.kind).iter().enumerate().filter_map(|(i, m)| hits_box(p0 - m.at, p1 - m.at, m.half).map(|t| (i, t * len))).collect();
        v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    /// A round's path through the airframe, in body axes: `None` if it
    /// passes clear of every system (between the wing and the tail, say).
    /// It hits what it crosses, the first hardest, and its explosive
    /// charge reaches the systems within `splash` metres of where it
    /// struck. Returns the systems hit, by name.
    pub fn strike(&mut self, p0: V3, p1: V3, damage: f64, splash: f64, rnd: &mut Lcg) -> Option<Vec<&'static str>> {
        let path = self.crossed(p0, p1);
        path.first()?;
        let mut hit = Vec::new();
        // The structure is the skin and frames round everything else: the
        // round holes it on the way in without losing much, and spends
        // itself in the systems inside.
        let (shell, inner): (Vec<_>, Vec<_>) = path.into_iter().partition(|&(i, _)| layout(self.kind)[i].part == Part::Structure);
        for &(i, _) in &shell {
            self.harm(i, damage * 0.5, 0.0, rnd);
            hit.push(i);
        }
        let Some(&(first, at)) = inner.first() else { return Some(self.names(hit)) };
        let impact = p0 + (p1 - p0).norm() * at;
        for (k, &(i, _)) in inner.iter().take(PENETRATION.len()).enumerate() {
            self.harm(i, damage * PENETRATION[k], 0.12 * damage / 6.0, rnd);
            hit.push(i);
        }
        if splash > 0.0 {
            for (i, m) in layout(self.kind).iter().enumerate() {
                if i != first && box_distance(impact - m.at, m.half) < splash {
                    self.harm(i, damage * 0.5, 0.0, rnd);
                    hit.push(i);
                }
            }
        }
        Some(self.names(hit))
    }

    /// A warhead bursting at `at` (body axes) while flying along `axis`: a
    /// ring of `fragments` thrown out 55 to 125 degrees off its axis, each
    /// hitting what it meets inside `outer` metres with less force the
    /// further it flies, and a blast that wrecks everything within
    /// `lethal` metres and reaches half as far again. Returns the systems hit.
    pub fn burst(&mut self, at: V3, axis: V3, lethal: f64, outer: f64, fragments: usize, rnd: &mut Lcg) -> Vec<&'static str> {
        let mut hit = Vec::new();
        let axis = if axis.len() > 1e-6 { axis.norm() } else { V3::Z };
        let e1 = axis.cross(if axis.y.abs() < 0.9 { V3::Y } else { V3::X }).norm();
        let e2 = axis.cross(e1);
        let roll = rnd.next() * TAU;
        for k in 0..fragments {
            // Spread evenly through the ring, with a little scatter.
            let polar = (55.0 + 70.0 * (k as f64 + rnd.next()) / fragments as f64) * DEG;
            let az = roll + k as f64 * 2.399963 + (rnd.next() - 0.5) * 0.2;
            let d = axis * polar.cos() + (e1 * az.cos() + e2 * az.sin()) * polar.sin();
            if let Some(&(i, dist)) = self.crossed(at, at + d * outer).first() {
                self.harm(i, 11.0 * (1.0 - dist / outer), 0.05, rnd);
                hit.push(i);
            }
        }
        // Blast: 18 points per metre of lethal radius at the burst, falling
        // to nothing at one and a half times that radius. The cockpit keeps
        // it off the pilot, whom only fragments reach.
        let reach = lethal * 1.5;
        for (i, m) in layout(self.kind).iter().enumerate().filter(|(_, m)| m.part != Part::Pilot) {
            let d = box_distance(at - m.at, m.half);
            if d < reach {
                self.harm(i, 18.0 * lethal * (1.0 - d / reach), 0.0, rnd);
                hit.push(i);
            }
        }
        self.names(hit)
    }

    /// Damage to one module, with a chance of setting a tank or an engine
    /// on fire, and the losses that end the flight.
    fn harm(&mut self, i: usize, damage: f64, ignite: f64, rnd: &mut Lcg) {
        if damage <= 0.0 || self.dead.is_some() {
            return;
        }
        let m = &layout(self.kind)[i];
        let was = self.hp[i];
        self.hp[i] = (self.hp[i] - damage).max(0.0);
        if matches!(m.part, Part::Fuel | Part::Engine) && self.fire.is_none() && rnd.next() < ignite.min(0.35) {
            self.fire = Some((i, 0.0));
        }
        if was > 0.0 && self.hp[i] == 0.0 {
            self.dead = match m.part {
                Part::Pilot => Some("PILOT KILLED"),
                Part::Wing => Some(if m.side < 0.0 { "LEFT WING SHOT OFF" } else { "RIGHT WING SHOT OFF" }),
                Part::Stabilators => Some("TAIL SHOT AWAY"),
                Part::Structure => Some("AIRFRAME BROKE UP"),
                Part::Fuel if self.fire.is_some_and(|(f, _)| f == i) => Some("FUEL TANK EXPLODED"),
                _ => None,
            };
        }
    }

    /// A second of fire: the burning system and the structure round it
    /// burn, and the fire may go out, or the MiG's extinguisher puts out
    /// an engine fire after two seconds. A burning tank that burns through
    /// explodes.
    pub fn step(&mut self, dt: f64, rnd: &mut Lcg) {
        for i in 0..self.hp.len() {
            let f = self.left(i);
            if layout(self.kind)[i].part == Part::Engine && f > 0.0 && f < 0.5 {
                self.hp[i] = (self.hp[i] - SEIZE * dt).max(0.0);
            }
        }
        let Some((i, t)) = self.fire else { return };
        let part = layout(self.kind)[i].part;
        if part == Part::Engine && self.extinguisher && t > 2.0 {
            self.extinguisher = false;
            self.fire = None;
            return;
        }
        if rnd.next() < FIRE_OUT * dt {
            self.fire = None;
            return;
        }
        self.fire = Some((i, t + dt));
        let grow = 1.0 + t / 8.0;
        self.harm(i, BURN * grow * dt, 0.0, rnd);
        if let Some(s) = layout(self.kind).iter().position(|m| m.part == Part::Structure) {
            self.harm(s, BURN_STRUCTURE * grow * dt, 0.0, rnd);
        }
    }

    /// What the damage does to the jet's flight.
    pub fn effects(&self) -> BattleDamage {
        let mut b = BattleDamage::default();
        let (mut wing_l, mut wing_r, mut engines, mut asymmetry) = (0.0f64, 0.0f64, 0.0, 0.0);
        for (i, m) in layout(self.kind).iter().enumerate() {
            let lost = 1.0 - self.left(i);
            match m.part {
                Part::Engine => {
                    // A damaged engine gives less; a seized one nothing.
                    let share = if self.kind == Type::Mig29 { 0.5 } else { 1.0 };
                    let loss = if lost >= 1.0 { 1.0 } else { lost * 0.7 };
                    engines += share * loss;
                    asymmetry += 0.5 * loss * m.side;
                }
                Part::Fuel => b.leak += lost * 3.0,
                Part::Wing if m.side < 0.0 => wing_l = lost,
                Part::Wing => wing_r = lost,
                Part::Stabilators | Part::Hydraulics => b.controls = b.controls.max(lost),
                Part::Fin => b.controls = b.controls.max(lost * 0.5),
                Part::Structure => b.airframe = lost,
                Part::Pilot => {}
            }
        }
        b.engine = engines.min(1.0);
        b.asymmetry = asymmetry;
        b.fire = self.fire.is_some();
        b.wing = wing_l.max(wing_r);
        b.side = if wing_r > wing_l { 1.0 } else { -1.0 };
        b
    }

    /// What is left of the jet, 0 to 100: the hit points left over all its
    /// systems, weighted by their size.
    pub fn integrity(&self) -> f64 {
        let (left, all): (f64, f64) = layout(self.kind).iter().zip(&self.hp).fold((0.0, 0.0), |(l, a), (m, hp)| (l + hp, a + m.hp));
        if self.dead.is_some() {
            0.0
        } else {
            100.0 * left / all
        }
    }

    fn names(&self, mut hit: Vec<usize>) -> Vec<&'static str> {
        hit.sort_unstable();
        hit.dedup();
        hit.into_iter().map(|i| layout(self.kind)[i].name).collect()
    }
}

/// Where a segment (body axes) first enters any system of a jet, as a
/// distance along it: `None` if it misses them all.
pub fn first_hit(kind: Type, p0: V3, p1: V3) -> Option<f64> {
    let len = (p1 - p0).len();
    layout(kind).iter().filter_map(|m| hits_box(p0 - m.at, p1 - m.at, m.half)).map(|t| t * len).reduce(f64::min)
}

/// Distance from a point to a box centred on the origin, 0 inside it.
fn box_distance(p: V3, half: V3) -> f64 {
    v3((p.x.abs() - half.x).max(0.0), (p.y.abs() - half.y).max(0.0), (p.z.abs() - half.z).max(0.0)).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(h: &Hull, name: &str) -> usize {
        layout(h.kind).iter().position(|m| m.name == name).unwrap()
    }

    /// Every module sits inside the airframe's bounding box and the pilot
    /// inside the canopy, round the design eye.
    #[test]
    fn modules_sit_inside_the_airframe() {
        for (kind, eye, span) in [(Type::F16, crate::airframe::EYE, 5.0), (Type::Mig29, crate::mig29::EYE, 5.7)] {
            for m in layout(kind) {
                assert!(m.at.x.abs() + m.half.x <= span && m.at.z.abs() + m.half.z <= 7.6, "{:?} {}", kind, m.name);
            }
            let pilot = &layout(kind)[0];
            assert!(box_distance(eye - pilot.at, pilot.half) < 0.25, "{:?} pilot round the eye", kind);
        }
    }

    /// A round through the canopy kills the pilot; one through the wingtip
    /// only holes the wing; one between the wing and the tail misses.
    #[test]
    fn rounds_hit_what_they_cross() {
        let mut rnd = Lcg(7);
        let mut h = Hull::new(Type::F16);
        let names = h.strike(v3(-5.0, 1.05, -3.25), v3(5.0, 1.05, -3.25), 6.0, 0.3, &mut rnd).unwrap();
        assert!(names.contains(&"PILOT"));
        assert_eq!(h.dead, Some("PILOT KILLED"));
        let mut h = Hull::new(Type::F16);
        let names = h.strike(v3(-4.5, 3.0, 1.2), v3(-4.5, -3.0, 1.2), 6.0, 0.3, &mut rnd).unwrap();
        assert_eq!(names, vec!["LEFT WING"]);
        assert!(h.dead.is_none() && h.effects().wing > 0.0 && h.effects().side < 0.0);
        let mut h = Hull::new(Type::F16);
        assert!(h.strike(v3(-4.5, 3.0, 3.6), v3(-4.5, -3.0, 3.6), 6.0, 0.3, &mut rnd).is_none(), "clear between wing and stabilator");
    }

    /// A Sidewinder bursting a metre and a half off the jet destroys it;
    /// one bursting 12 m away only damages it; the MiG loses one engine to
    /// a burst on its side and keeps the other.
    #[test]
    fn warheads_kill_close_and_damage_further_out() {
        let s = &crate::weapons::AIM9M;
        let fragments = (s.outer * 8.0) as usize;
        let mut rnd = Lcg(3);
        let mut h = Hull::new(Type::F16);
        h.burst(v3(0.0, 2.0, 1.0), V3::Z, s.lethal, s.outer, fragments, &mut rnd);
        assert!(h.dead.is_some(), "a close burst kills");
        let mut killed = 0;
        for seed in 0..40 {
            let mut rnd = Lcg(seed);
            let mut h = Hull::new(Type::F16);
            h.burst(v3(12.0, 0.0, 0.0), V3::Z, s.lethal, s.outer, fragments, &mut rnd);
            assert!(h.integrity() < 100.0, "fragments reach 12 m");
            killed += h.dead.is_some() as usize;
        }
        assert!(killed < 20, "most survive a 12 m burst: {killed} of 40 killed");
        let mut h = Hull::new(Type::Mig29);
        let right = index(&h, "RIGHT ENGINE");
        h.hp[right] = 0.0;
        let e = h.effects();
        assert!((e.engine - 0.5).abs() < 1e-9 && e.thrust() == 0.5);
    }

    /// A burning tank burns through and explodes unless the fire goes out;
    /// the MiG's extinguisher puts out an engine fire.
    #[test]
    fn fires_burn_through_or_go_out() {
        let mut rnd = Lcg(11);
        let mut h = Hull::new(Type::F16);
        let tank = index(&h, "AFT FUEL TANK");
        h.fire = Some((tank, 0.0));
        let mut t = 0.0;
        while h.fire.is_some() && h.dead.is_none() && t < 60.0 {
            h.step(0.1, &mut rnd);
            t += 0.1;
        }
        assert!(h.dead == Some("FUEL TANK EXPLODED") || h.fire.is_none());
        let mut h = Hull::new(Type::Mig29);
        let engine = index(&h, "LEFT ENGINE");
        h.fire = Some((engine, 0.0));
        for _ in 0..30 {
            h.step(0.1, &mut Lcg(99));
        }
        assert!(h.fire.is_none() && h.dead.is_none(), "extinguished");
    }
}
