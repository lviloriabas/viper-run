//! Pilots share the player's aircraft, FLCS and engine. Flight assignments
//! coordinate an attacker and flankers; local decisions manage energy,
//! weapon solutions, missile defence, separation and terrain clearance.
use crate::math::*;
use crate::sim::{stick_for_g, stick_for_roll, Aircraft, Input, G0};
use crate::terrain;
use crate::weapons::{fly_out, gun_bore, gun_of, muzzle, Spec};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tactic { Lead, Pure, Lag, Break, Evade, Extend, Recover, Flank, Support, Reposition, Disengage }

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Role { #[default] Attack, Cover, Drag, Disengage }

#[derive(Clone, Copy, Debug)]
pub struct Tactical {
    pub role: Role,
    pub waypoint: Option<V3>,
    pub separation: V3,
    pub clear_fire: bool,
    pub shot_available: bool,
    pub radar_support: bool,
    pub radar_lock: bool,
    pub missile_ir: bool,
    pub combat: bool,
}
impl Default for Tactical {
    fn default() -> Self {
        Self { role: Role::Attack, waypoint: None, separation: V3::ZERO,
            clear_fire: true, shot_available: true, radar_support: false,
            radar_lock: false, missile_ir: false, combat: true }
    }
}
pub struct Member<'a> { pub id: usize, pub jet: &'a Aircraft, pub armed: bool, pub defensive: bool }

/// Stable assignments with immediate relief for a threatened or slow attacker.
#[derive(Default)]
pub struct Flight { attacker: Option<usize>, hold: f64 }
impl Flight {
    pub fn assign(&mut self, members: &[Member<'_>], foe: &Aircraft, dt: f64) -> Vec<Tactical> {
        self.hold -= dt;
        let fit = |m: &Member<'_>| m.armed && !m.defensive && m.jet.speed > corner_speed(m.jet) * 0.68 && m.jet.battle.engine < 0.55 && m.jet.fuel > m.jet.kind.fuel_min() * 0.15;
        let score = |m: &Member<'_>| {
            let r = foe.pos - m.jet.pos;
            2.5 * m.jet.q.axes().forward.dot(r.norm()) + (m.jet.speed / corner_speed(m.jet)).clamp(0.0, 1.4)
                - r.len() / 14000.0 - m.jet.battle.wing * 2.0
        };
        let current = members.iter().find(|m| Some(m.id) == self.attacker && fit(m));
        if current.is_none() || self.hold <= 0.0 {
            let best = members.iter().filter(|m| fit(m)).max_by(|a, b| score(a).total_cmp(&score(b)));
            if current.is_none() || best.zip(current).is_some_and(|(b, c)| score(b) > score(c) + 0.65) {
                self.attacker = best.map(|m| m.id);
            }
            self.hold = 7.0;
        }
        let axis = flat(foe.vel);
        let side = axis.cross(V3::Y);
        members.iter().enumerate().map(|(slot, m)| {
            let mut t = Tactical::default();
            t.role = if !m.armed || m.jet.battle.engine > 0.65 || m.jet.fuel <= m.jet.kind.fuel_min() * 0.15 { Role::Disengage }
                else if !fit(m) { Role::Drag }
                else if Some(m.id) == self.attacker { Role::Attack } else { Role::Cover };
            if t.role != Role::Attack {
                let sign = if m.id % 2 == 0 { 1.0 } else { -1.0 };
                let distance = (foe.pos - m.jet.pos).len();
                let width = (distance * 0.32).clamp(1100.0, 4200.0) + (slot / 2) as f64 * 500.0;
                let mut p = foe.pos + axis * 1500.0 + side * (sign * width);
                p.y = (foe.pos.y + 450.0 + (slot / 2) as f64 * 250.0).max(terrain::floor(p.x, p.z) + 650.0);
                if t.role == Role::Drag {
                    if let Some(cover) = members.iter().find(|other| Some(other.id) == self.attacker) {
                        p = cover.jet.pos + flat(m.jet.pos - foe.pos) * 1500.0;
                    }
                }
                t.waypoint = Some(p);
            }
            for other in members.iter().filter(|other| other.id != m.id) {
                let r = other.jet.pos - m.jet.pos;
                let v = other.jet.vel - m.jet.vel;
                let time = (-r.dot(v) / v.dot(v).max(1.0)).clamp(0.0, 3.0);
                let near = r + v * time;
                if near.len() < 350.0 && r.len() < 1800.0 {
                    let away = if near.len() > 1.0 { -near.norm() } else { side * if m.id < other.id { 1.0 } else { -1.0 } };
                    t.separation += away * (1.0 - near.len() / 350.0);
                }
            }
            t
        }).collect()
    }
}
/// Friendly crossing the current or predicted firing lane blocks release.
pub fn clear_lane(me: &Aircraft, foe: &Aircraft, friend: &Aircraft) -> bool {
    let r = foe.pos - me.pos;
    let length = r.len().max(1.0);
    let axis = r / length;
    let along = (friend.pos - me.pos).dot(axis);
    if along < -100.0 || along > length + 100.0 { return true; }
    let tof = along.max(0.0) / (gun_of(me.kind).velocity * 0.78);
    let near = friend.pos + (friend.vel - me.vel) * tof;
    let segment = r * ((near - me.pos).dot(axis) / length).clamp(0.0, 1.0);
    (near - me.pos - segment).len() > 140.0
}

fn corner_speed(me: &Aircraft) -> f64 {
    let density = crate::sim::atmosphere_at(me.pos.y).rho;
    let reference = me.kind.empty_mass() + me.kind.fuel_max() * 0.65;
    (205.0 * (me.mass / reference * 1.225 / density).sqrt()).clamp(195.0, 330.0)
}

#[derive(Clone, Copy)]
pub struct Weapon { pub station: usize, pub spec: &'static Spec }

/// Prefer IR in the merge and radar weapons further out, using actual fly-outs.
pub fn select_weapon(me: &Aircraft, foe: &Aircraft, weapons: &[Weapon], radar_lock: bool, skill: f64) -> Option<usize> {
    let r = foe.pos - me.pos;
    let range = r.len();
    let ata = me.q.axes().forward.dot(r.norm()).clamp(-1.0, 1.0).acos();
    let mut best: Option<(f64, usize)> = None;
    for (n, w) in weapons.iter().enumerate() {
        if weapons[..n].iter().any(|old| std::ptr::eq(old.spec, w.spec)) { continue; }
        let s = w.spec;
        let glare = (1.0 + 5.0 * (1.0 - s.ccm)) * DEG;
        if s.infrared() && r.norm().dot(crate::world::sun_dir()) > glare.cos() { continue; }
        let cone = if s.infrared() { s.gimbal.min(35.0 * DEG) } else { 50.0 * DEG };
        if ata > cone || range < s.min_range.max(450.0) || !s.infrared() && !radar_lock { continue; }
        let Some(tof) = fly_out(s, me.pos, me.vel, foe.pos, foe.vel, &terrain::floor) else { continue };
        if tof > s.life * mix(0.5, 0.7, skill) { continue; }
        let preference = if range < 4500.0 { if s.infrared() { 4.0 } else { 0.0 } }
            else if s.active() { 4.0 } else if s.semi_active() { 2.8 } else { 1.0 };
        let score = preference + s.ccm - tof * 0.035 - ata * 0.5;
        if best.is_none_or(|(old, _)| score > old) { best = Some((score, w.station)); }
    }
    best.map(|(_, station)| station)
}
pub struct Picture<'a> {
    pub foe: &'a Aircraft,
    pub missile: Option<(V3, V3)>,
    pub missiles: u32,
    pub missile_spec: &'static Spec,
    pub rounds: u32,
    pub tactical: Tactical,
}
pub struct Orders { pub input: Input, pub gun: bool, pub missile: bool, pub countermeasures: bool }
pub struct Pilot {
    pub skill: f64,
    pub tactic: Tactic,
    think: f64, burst: f64, gun_pause: f64, since_shot: f64,
    zone_clock: f64, in_zone: bool,
    escape: f64, escape_clock: f64, clear: f64,
    low_energy: bool, manoeuvre_time: f64, cm_clock: f64, defence_side: f64,
}
const FLOOR: f64 = 180.0;
impl Pilot {
    pub fn new(skill: f64) -> Pilot {
        Pilot { skill: skill.clamp(0.0, 1.0), tactic: Tactic::Pure, think: 0.0,
            burst: 0.0, gun_pause: 0.0, since_shot: 6.0, zone_clock: 0.0,
            in_zone: false, escape: 0.0, escape_clock: 0.0, clear: 0.0,
            low_energy: false, manoeuvre_time: 0.0, cm_clock: 0.0, defence_side: 1.0 }
    }
    fn g_limit(&self) -> f64 { mix(7.0, 9.0, self.skill) }
    pub fn fly(&mut self, me: &Aircraft, pic: &Picture, dt: f64) -> Orders {
        let foe = pic.foe;
        let team = pic.tactical;
        let ax = me.q.axes();
        let rel = foe.pos - me.pos;
        let range = rel.len().max(1.0);
        let los = rel / range;
        let ata = ax.forward.dot(los).clamp(-1.0, 1.0).acos();
        let foe_ata = foe.q.axes().forward.dot(-los).clamp(-1.0, 1.0).acos();
        let closure = -(foe.vel - me.vel).dot(los);
        let corner = corner_speed(me);
        self.since_shot += dt; self.think -= dt; self.cm_clock -= dt;
        self.gun_pause -= dt; self.manoeuvre_time += dt;
        self.low_energy = if self.low_energy { me.speed < corner * 0.94 }
            else { me.speed < corner * 0.68 || me.departure > 0.15 };
        let threat = pic.missile.filter(|(p, v)| {
            let r = me.pos - *p;
            let close = (*v - me.vel).dot(r.norm());
            close > 15.0 && r.len() / close < 24.0
        });
        let n_max = self.g_limit().min(me.g_avail.max(1.2));
        let gcas = recovery_clearance(me, 0.0, 6.0);
        if gcas < FLOOR {
            self.escape_clock -= dt;
            if self.tactic != Tactic::Recover || self.escape_clock <= 0.0 {
                self.escape_clock = 0.5; self.escape = escape_bank(me, n_max);
            }
            self.tactic = Tactic::Recover; self.clear = 0.0;
        } else if self.tactic == Tactic::Recover {
            self.clear = if gcas > FLOOR + 250.0 { self.clear + dt } else { 0.0 };
            if self.clear > 1.0 { self.think = 0.0; self.tactic = Tactic::Pure; }
        }
        if self.tactic != Tactic::Recover && (self.think <= 0.0 || threat.is_some() && self.tactic != Tactic::Evade) {
            self.think = mix(0.55, 0.15, self.skill);
            let next = if threat.is_some() { Tactic::Evade }
                else if team.combat && team.role == Role::Disengage { Tactic::Disengage }
                else if team.combat && foe_ata < 35.0 * DEG && range < 3000.0 && ata > 95.0 * DEG && !self.low_energy { Tactic::Break }
                else if team.combat && self.low_energy { Tactic::Extend }
                else if team.radar_support && range > 3500.0 { Tactic::Support }
                else if team.role == Role::Drag { Tactic::Extend }
                else if team.role == Role::Cover && (range > 2300.0 || ata > 45.0 * DEG) { Tactic::Flank }
                else if range < 800.0 && ata > 70.0 * DEG && foe_ata > 70.0 * DEG
                    || self.tactic == Tactic::Reposition && self.manoeuvre_time < 4.0 { Tactic::Reposition }
                else if range < 1700.0 && ata < 60.0 * DEG {
                    if closure > 55.0 && range < 1100.0 && foe_ata > 90.0 * DEG { Tactic::Lag } else { Tactic::Lead }
                } else { Tactic::Pure };
            if next != self.tactic {
                if next == Tactic::Evade {
                    let beam = (threat.unwrap().0 - me.pos).norm().cross(V3::Y).norm();
                    self.defence_side = if beam.dot(me.vel) >= 0.0 { 1.0 } else { -1.0 };
                }
                self.manoeuvre_time = 0.0;
            }
            self.tactic = next;
        }
        let bore = me.q.rotate(gun_bore());
        let lead = gun_lead(me, foe);
        let mut airbrake = false;
        let (mut dir, mut pull, mut burner) = match self.tactic {
            Tactic::Recover => {
                let v = me.vel.norm();
                if self.escape == 0.0 && v.y > RECOVERY_CLIMB.sin() {
                    ((flat(v) * RECOVERY_CLIMB.cos() + V3::Y * RECOVERY_CLIMB.sin()).norm(), n_max, true)
                } else {
                    let up = (V3::Y - v * v.y).norm();
                    let b = self.escape * DEG;
                    (up * b.cos() + v.cross(up) * b.sin(), n_max, true)
                }
            }
            Tactic::Break => (los, n_max, true),
            Tactic::Extend | Tactic::Disengage => {
                let horizontal = team.waypoint.map(|p| flat(p - me.pos)).unwrap_or_else(|| flat(me.vel));
                let direction = if self.tactic == Tactic::Disengage { flat(me.pos - foe.pos) } else { horizontal };
                ((direction - V3::Y * 0.10).norm(), if self.low_energy { 1.4 } else { 2.5 }, true)
            }
            Tactic::Reposition => ((flat(me.vel) + V3::Y * if me.speed > corner { 0.20 } else { -0.08 }).norm(), 2.0, me.speed < corner),
            Tactic::Evade => {
                let (mp, mv) = threat.unwrap_or((foe.pos, foe.vel));
                let r = mp - me.pos;
                let time = r.len() / (mv - me.vel).dot(-r.norm()).max(1.0);
                let beam = flat(r).cross(V3::Y) * self.defence_side;
                let horizontal = if time < 3.0 && !self.low_energy { (beam * 0.65 + r.norm() * 0.35).norm() }
                    else if time > 11.0 { (beam * 0.65 - flat(r) * 0.75).norm() } else { beam };
                let agl = me.pos.y - terrain::floor(me.pos.x, me.pos.z);
                let mask = masked_escape(me, mp, horizontal);
                ((mask + V3::Y * if agl > 700.0 { -0.18 } else { 0.02 }).norm(),
                    if self.low_energy { 2.0 } else if time < 3.0 { n_max } else { 4.5_f64.min(n_max) },
                    !team.missile_ir && me.speed < corner * 1.08)
            }
            Tactic::Lag => {
                airbrake = closure > 90.0 && range < 650.0 && me.speed > corner * 0.95;
                (((foe.pos - foe.q.axes().forward * 850.0) - me.pos).norm(), n_max * 0.75, false)
            }
            Tactic::Lead => {
                // Keep enough dynamic pressure for precise gun tracking.
                // Match closure with throttle once inside the firing window.
                let off = bore - me.vel.norm();
                ((lead - off).norm(), n_max, me.speed < corner)
            },
            Tactic::Flank => ((team.waypoint.unwrap_or(foe.pos) - me.pos).norm(), 4.5_f64.min(n_max), me.speed < corner),
            Tactic::Support => {
                let side = flat(los).cross(V3::Y) * self.defence_side;
                ((los * (40.0 * DEG).cos() + side * (40.0 * DEG).sin()).norm(), 3.5, false)
            }
            Tactic::Pure => {
                let intercept = (rel + foe.vel * (range / (me.speed + 250.0)).min(12.0)).norm();
                (if team.combat && range > 2500.0 { intercept } else { los }, if range > 4000.0 { 4.0 } else { n_max * 0.9 }, me.speed < corner * 1.1)
            }
        };
        if !matches!(self.tactic, Tactic::Recover | Tactic::Break | Tactic::Evade) {
            pull = pull.min(mix(1.6, n_max, ((me.speed / corner - 0.5) / 0.5).clamp(0.0, 1.0))).min((me.g_avail * 0.85).max(1.2));
            if self.low_energy && dir.y > 0.05 { dir = (flat(dir) - V3::Y * 0.06).norm(); }
            burner |= me.speed < corner * 0.9;
            if team.separation.len() > 0.01 { dir = (dir + team.separation * 1.6).norm(); pull = pull.max(3.0).min(n_max); }
        }
        if self.tactic != Tactic::Recover {
            let agl = me.pos.y - terrain::floor(me.pos.x, me.pos.z);
            let steepest = -clamp((agl - 350.0) / 1800.0, 0.0, 1.0) * 55.0 * DEG;
            if dir.y < steepest.sin() { dir = (flat(dir) * steepest.cos() + V3::Y * steepest.sin()).norm(); }
            if me.speed > corner * 1.25 { burner = false; }
        }
        if threat.is_some() && team.missile_ir && self.tactic != Tactic::Recover { burner = false; }
        if me.fuel < me.kind.fuel_min() * 0.6 && !matches!(self.tactic, Tactic::Recover | Tactic::Break | Tactic::Evade | Tactic::Extend) { burner = false; }
        let (pitch, roll) = if self.tactic == Tactic::Lead && ata < 18.0 * DEG
            && team.separation.len() < 0.01 && gcas > FLOOR + 300.0 {
            gun_steer(me, foe, lead, pull)
        } else { steer(me, dir, pull) };
        let want = if threat.is_some() && team.missile_ir && self.tactic != Tactic::Recover { 0.82 }
            else if burner { 1.0 } else if airbrake { 0.55 } else if me.speed > corner * 1.15 { 0.78 } else { 0.99 };
        let mut input = Input { pitch_axis: pitch, roll_axis: roll, airbrake, ..Default::default() };
        if burner { input.afterburner = true; }
        else if me.throttle > want + 0.005 { input.throttle_down = true; }
        else if me.throttle < want - 0.005 { input.throttle_up = true; }

        let error = bore.dot(lead).clamp(-1.0, 1.0).acos();
        let offensive = matches!(self.tactic, Tactic::Lead | Tactic::Pure | Tactic::Lag | Tactic::Flank);
        let solution = team.combat && offensive && team.clear_fire && pic.rounds > 0
            && range > 150.0 && range < 900.0 && error < mix(2.0, 0.9, self.skill) * DEG;
        if !solution { self.burst = 0.0; }
        if solution && self.burst <= 0.0 && self.gun_pause <= 0.0 {
            self.burst = mix(0.25, 0.45, self.skill);
            self.gun_pause = self.burst + mix(0.65, 0.3, self.skill);
        }
        let gun = solution && self.burst > 0.0;
        self.burst = (self.burst - dt).max(0.0);
        self.zone_clock -= dt;
        if self.zone_clock <= 0.0 {
            self.zone_clock = 0.6;
            self.in_zone = pic.missiles > 0 && fly_out(pic.missile_spec, me.pos, me.vel, foe.pos, foe.vel, &terrain::floor).is_some();
        }
        let spec = pic.missile_spec;
        let cone = if spec.infrared() { spec.gimbal.min(35.0 * DEG) } else { 50.0 * DEG };
        let aspect_ok = !spec.infrared() || foe_ata > 55.0 * DEG || self.skill > 0.7 || range < 4000.0;
        let missile = team.combat && team.clear_fire && team.shot_available && offensive
            && pic.missiles > 0 && self.in_zone && ata < cone && range > spec.min_range.max(450.0)
            && (spec.infrared() || team.radar_lock) && aspect_ok && self.since_shot > mix(15.0, 9.0, self.skill);
        if missile { self.since_shot = 0.0; self.in_zone = false; self.zone_clock = 0.0; }
        let countermeasures = team.combat && self.cm_clock <= 0.0 && threat.is_some_and(|(p, v)| {
            let r = me.pos - p;
            r.len() < 5500.0 && r.len() / (v - me.vel).dot(r.norm()).max(1.0) < 10.0
        });
        if countermeasures { self.cm_clock = mix(1.25, 0.65, self.skill); }
        Orders { input, gun, missile, countermeasures }
    }
}

/// Choose a safe descending corridor with terrain between it and the missile.
/// The predictive terrain recovery always overrides this escape direction.
fn masked_escape(me: &Aircraft, missile: V3, beam: V3) -> V3 {
    let mut best = (0.0, beam);
    for angle in [-30.0_f64, 0.0, 30.0] {
        let d = Quat::axis_angle(V3::Y, angle * DEG).rotate(beam);
        if (1..=4).any(|k| {
            let p = me.pos + d * (k as f64 * 400.0) - V3::Y * (k as f64 * 60.0);
            p.y <= terrain::floor(p.x, p.z) + 280.0
        }) { continue; }
        let end = me.pos + d * 1600.0 - V3::Y * 240.0;
        let hidden = (1..12).filter(|k| {
            let p = end.lerp(missile, *k as f64 / 12.0);
            p.y < terrain::floor(p.x, p.z) + 15.0
        }).count() as f64;
        let score = hidden * 2.0 + d.dot(beam);
        if score > best.0 { best = (score, d); }
    }
    best.1
}

fn flat(v: V3) -> V3 {
    let h = v3(v.x, 0.0, v.z);
    if h.len() > 1e-6 {
        h.norm()
    } else {
        V3::Z * -1.0
    }
}

/// Direction for the gun line that puts rounds on the foe after their time
/// of flight: they carry the shooter's velocity, the foe flies on, and
/// gravity pulls them down.
pub fn gun_lead(me: &Aircraft, foe: &Aircraft) -> V3 {
    let (gun, _) = muzzle(me);
    let r = foe.pos - gun;
    // Average speed of a round over the first 900 m.
    let tof = r.len() / (gun_of(me.kind).velocity * 0.78);
    (r + (foe.vel - me.vel) * tof + V3::Y * (0.5 * G0 * tof * tof)).norm()
}

/// Fine tracking commands the gun's angular rate, including the target's
/// line-of-sight rate. Pointing only the velocity vector leaves a persistent
/// gun error while AoA and bank change during a close pursuit.
fn gun_steer(me: &Aircraft, foe: &Aircraft, lead: V3, max_g: f64) -> (f64, f64) {
    let ax = me.q.axes();
    let bore = me.q.rotate(gun_bore());
    let r = foe.pos - me.pos;
    let los_rate = r.cross(foe.vel - me.vel) / r.dot(r).max(1.0);
    let angular = bore.cross(lead) * 1.5 + los_rate;
    let bank = (-angular.y * me.speed / G0).atan().clamp(-65.0 * DEG, 65.0 * DEG);
    let roll = stick_for_roll(((bank * RAD - me.bank) * 2.8).clamp(-120.0, 120.0));
    let n = (ax.up.y + angular.dot(ax.right) * me.speed / G0).clamp(-1.0, max_g);
    (stick_for_g(n), roll)
}

/// Stick that turns the flight path toward `dir` with at most `n_max` g:
/// roll the lift vector onto the turn and pull, holding 1 g against gravity
/// as the FLCS does. Small corrections below the horizon are pushed rather
/// than rolled inverted for.
pub fn steer(me: &Aircraft, dir: V3, n_max: f64) -> (f64, f64) {
    let ax = me.q.axes();
    let speed = me.speed.max(60.0);
    let v = if me.vel.len() > 1.0 { me.vel.norm() } else { ax.forward };
    let angle = v.dot(dir).clamp(-1.0, 1.0).acos();
    let perp = dir - v * v.dot(dir);
    let perp = if perp.len() > 1e-6 { perp.norm() } else { ax.up };
    // Turn rate that closes the angle in about 0.7 s.
    let a_turn = perp * ((angle / 0.7) * speed);
    let a_lift = a_turn + (V3::Y - v * v.y) * G0;
    let (lx, ly) = (a_lift.dot(ax.right), a_lift.dot(ax.up));
    let mag = (lx * lx + ly * ly).sqrt();
    let (phi, n) = if ly < 0.0 && mag < 3.0 * G0 && lx.abs() < -ly { ((-lx).atan2(-ly), -mag / G0) } else { (lx.atan2(ly), mag / G0) };
    // Roll first, pull once the lift vector is close.
    let align = clamp(1.0 - (phi.abs() - 15.0 * DEG) / (60.0 * DEG), 0.1, 1.0);
    let n = if n > 1.0 { 1.0 + (n.min(n_max) - 1.0) * align } else { n.max(-2.0) };
    let rate = clamp(phi * 3.0, -5.0, 5.0) * RAD;
    (stick_for_g(n), stick_for_roll(rate))
}

/// Climb angle a wings-level recovery stops pulling at.
const RECOVERY_CLIMB: f64 = 45.0 * DEG;

/// Auto-GCAS-style prediction, as on the real F-16: the lowest clearance
/// over the terrain along the recovery the jet would fly from here, a short
/// reaction and the roll to `bank` (degrees from wings level) at about
/// the protected roll rate, then the available `n` g with the lift vector there. Wings level it pulls
/// until the path climbs 45 degrees; banked it keeps turning away from the
/// ground. A point mass at constant speed, sampled every 0.2 s for 9 s.
fn recovery_clearance(me: &Aircraft, bank: f64, n: f64) -> f64 {
    // The AoA protection can reduce lift below the requested recovery g,
    // especially while slow or rolling. Leave a response margin rather than
    // predicting a pull-up the protected airframe cannot actually fly.
    let n = n.min(me.g_avail * 0.9);
    let speed = me.speed.max(80.0);
    let mut p = me.pos;
    let mut v = if me.vel.len() > 1.0 { me.vel.norm() } else { me.q.axes().forward };
    let roll_rate = me.flcs.roll_limit.clamp(20.0, 180.0);
    let roll_time = 0.3 + wrap180(me.bank - bank).abs() / roll_rate;
    let (dt, mut t) = (0.2, 0.0);
    let mut lowest = f64::INFINITY;
    while t < 9.0 {
        if t >= roll_time && (bank != 0.0 || v.y < RECOVERY_CLIMB.sin()) {
            let up = (V3::Y - v * v.y).norm();
            let right = v.cross(up);
            let lift = up * (bank * DEG).cos() + right * (bank * DEG).sin();
            let a = lift * (n * G0) - V3::Y * G0;
            v = (v + (a - v * a.dot(v)) * (dt / speed)).norm();
        }
        p += v * (speed * dt);
        t += dt;
        lowest = lowest.min(p.y - terrain::floor(p.x, p.z));
    }
    lowest
}

/// Recovery bank, degrees from wings level: straight up, a climbing turn,
/// or a hard turn away from a valley wall, whichever clears best. A turn has
/// to beat the straight pull-up by 40 m to be chosen.
fn escape_bank(me: &Aircraft, n: f64) -> f64 {
    let mut best = (recovery_clearance(me, 0.0, n), 0.0);
    for bank in [-50.0, 50.0, -75.0, 75.0] {
        let c = recovery_clearance(me, bank, n) - 40.0;
        if c > best.0 {
            best = (c, bank);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{trim, Flcs, Mission, FIXED_DT};
    use crate::terrain::{river_x, terrain_at};
    use crate::weapons::AIM9M;

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

    fn picture(foe: &Aircraft) -> Picture<'_> {
        Picture { foe, missile: None, missiles: 0, missile_spec: &AIM9M, rounds: 0, tactical: Tactical::default() }
    }

    /// The steering law turns the jet to a heading behind it and levels out.
    #[test]
    fn steering_turns_onto_a_heading() {
        let mut a = spawn(v3(0.0, 4000.0, 0.0), 0.0, 230.0);
        let want = v3(1.0, 0.0, 0.3).norm();
        for _ in 0..(20.0 / FIXED_DT) as usize {
            let (p, r) = steer(&a, want, 7.0);
            a.update(FIXED_DT, &Input { pitch_axis: p, roll_axis: r, afterburner: true, ..Default::default() });
        }
        let off = a.vel.norm().dot(want).acos() * RAD;
        assert!(off < 3.0, "{off:.1} deg off the heading");
        assert!(a.bank.abs() < 10.0 && (a.pos.y - 4000.0).abs() < 600.0, "bank {:.0}, altitude {:.0}", a.bank, a.pos.y);
        assert!(a.max_g < 9.4, "{:.1} g", a.max_g);
    }

    /// Down in the gorge, chasing a target that flies into the valley wall,
    /// the AI pilot keeps clear of the terrain for a minute and a half.
    #[test]
    fn ai_does_not_fly_into_the_ground() {
        terrain::init();
        for (z, alt, heading) in [(3000.0, 260.0, 0.0), (6000.0, 200.0, 1.2), (-2000.0, 350.0, -0.6)] {
            let x = river_x(z);
            let ground = terrain_at(x, z);
            let mut me = spawn(v3(x, ground + alt, z), heading, 220.0);
            let mut foe = spawn(v3(x + 300.0, ground + alt - 60.0, z - 1500.0), heading + 0.4, 240.0);
            let mut pilot = Pilot::new(0.5);
            for _ in 0..(90.0 / FIXED_DT) as usize {
                let o = pilot.fly(&me, &picture(&foe), FIXED_DT);
                me.update(FIXED_DT, &o.input);
                foe.update(FIXED_DT, &Input { roll_axis: 0.05, ..Default::default() });
                if foe.crashed {
                    // Keep something to chase down low.
                    foe = spawn(v3(me.pos.x, terrain::floor(me.pos.x, me.pos.z) + 150.0, me.pos.z - 1200.0), heading, 220.0);
                }
                assert!(!me.crashed, "z {z}: hit the {} at {:?}, tactic {:?}", me.collision_label, me.pos, pilot.tactic);
            }
        }
    }

    /// Two AI pilots dogfighting each other for two minutes over the gorge:
    /// neither hits the ground and neither exceeds the airframe limits.
    #[test]
    fn ai_dogfight_stays_within_limits() {
        terrain::init();
        let z = 0.0;
        let x = river_x(z);
        let mut a = spawn(v3(x, 2600.0, z), 0.0, 240.0);
        let mut b = spawn(v3(x + 200.0, 2800.0, z - 6000.0), PI, 240.0);
        let (mut pa, mut pb) = (Pilot::new(0.4), Pilot::new(0.9));
        let mut closest = f64::INFINITY;
        for _ in 0..(120.0 / FIXED_DT) as usize {
            let oa = pa.fly(&a, &picture(&b), FIXED_DT);
            let ob = pb.fly(&b, &picture(&a), FIXED_DT);
            a.update(FIXED_DT, &oa.input);
            b.update(FIXED_DT, &ob.input);
            closest = closest.min((a.pos - b.pos).len());
            assert!(!a.crashed && !b.crashed, "{} / {}", a.collision_label, b.collision_label);
        }
        assert!(a.max_g < 9.6 && b.max_g < 9.6, "{:.1} / {:.1} g", a.max_g, b.max_g);
        assert!(closest < 1500.0, "never engaged: closest {closest:.0} m");
    }

    /// An ace parked behind a straight-flying target tracks it with the gun.
    #[test]
    fn ai_tracks_for_a_gun_shot() {
        let foe = spawn(v3(0.0, 3000.0, -700.0), 0.0, 200.0);
        let mut me = spawn(v3(30.0, 2980.0, 0.0), 0.0, 210.0);
        let mut foe = foe;
        let mut pilot = Pilot::new(1.0);
        let mut fired = 0;
        let mut best_error = 180.0_f64;
        for _ in 0..(10.0 / FIXED_DT) as usize {
            let pic = Picture { foe: &foe, missile: None, missiles: 0, missile_spec: &AIM9M, rounds: 500, tactical: Tactical::default() };
            let o = pilot.fly(&me, &pic, FIXED_DT);
            fired += o.gun as usize;
            best_error = best_error.min(me.q.rotate(gun_bore()).dot(gun_lead(&me, &foe)).clamp(-1.0, 1.0).acos() * RAD);
            me.update(FIXED_DT, &o.input);
            foe.update(FIXED_DT, &Input::default());
        }
        assert!(fired > 60, "fired for {fired} steps, best error {best_error:.2}, tactic {:?}, range {:.0}", pilot.tactic, (me.pos - foe.pos).len());
    }
    #[test]
    fn flight_relies_on_cover_when_attacker_is_slow_or_threatened() {
        let foe = spawn(v3(0.0, 5000.0, -8000.0), PI, 240.0);
        let mut a = spawn(v3(0.0, 5000.0, 0.0), 0.0, 240.0);
        let b = spawn(v3(2600.0, 5400.0, 1800.0), 0.0, 240.0);
        let mut flight = Flight::default();
        let assign = flight.assign(&[
            Member { id: 0, jet: &a, armed: true, defensive: false },
            Member { id: 1, jet: &b, armed: true, defensive: false },
        ], &foe, 0.1);
        assert_eq!(assign[0].role, Role::Attack);
        assert_eq!(assign[1].role, Role::Cover);
        assert!((assign[1].waypoint.unwrap() - foe.pos).len() > 1500.0);
        a.speed = 120.0;
        let assign = flight.assign(&[
            Member { id: 0, jet: &a, armed: true, defensive: true },
            Member { id: 1, jet: &b, armed: true, defensive: false },
        ], &foe, 0.1);
        assert_eq!(assign[0].role, Role::Drag);
        assert_eq!(assign[1].role, Role::Attack);
        let assign = flight.assign(&[Member { id: 1, jet: &b, armed: false, defensive: false }], &foe, 0.1);
        assert_eq!(assign[0].role, Role::Disengage);
    }

    #[test]
    fn friendly_crossing_a_firing_lane_blocks_release() {
        let me = spawn(v3(0.0, 5000.0, 0.0), 0.0, 240.0);
        let foe = spawn(v3(0.0, 5000.0, -2000.0), 0.0, 240.0);
        let mut friend = spawn(v3(0.0, 5000.0, -800.0), 0.0, 240.0);
        assert!(!clear_lane(&me, &foe, &friend));
        friend.pos.x = 500.0;
        assert!(clear_lane(&me, &foe, &friend));
        friend.pos = v3(0.0, 5000.0, -2600.0);
        assert!(clear_lane(&me, &foe, &friend));
    }

    #[test]
    fn weapon_choice_uses_range_lock_and_remaining_stations() {
        use crate::weapons::{AIM120C, R27R};
        let me = spawn(v3(0.0, 5000.0, 0.0), 0.0, 250.0);
        let mut foe = spawn(v3(0.0, 5000.0, -2500.0), PI, 240.0);
        let weapons = [Weapon { station: 0, spec: &AIM9M }, Weapon { station: 2, spec: &AIM120C }];
        assert_eq!(select_weapon(&me, &foe, &weapons, true, 0.8), Some(0));
        foe.pos.z = -16000.0;
        assert_eq!(select_weapon(&me, &foe, &weapons, true, 0.8), Some(2));
        assert_eq!(select_weapon(&me, &foe, &weapons[1..], false, 0.8), None);
        let semi = [Weapon { station: 4, spec: &R27R }];
        assert_eq!(select_weapon(&me, &foe, &semi, true, 0.8), Some(4));
        assert_eq!(select_weapon(&me, &foe, &[], true, 0.8), None);
    }

    #[test]
    fn slow_pilots_unload_and_regain_energy_on_both_airframes() {
        for kind in [crate::sim::Type::F16, crate::sim::Type::Mig29] {
            let mut me = crate::combat::spawn_type(kind, v3(0.0, 6000.0, 0.0), 0.0, 125.0);
            let foe = spawn(v3(1000.0, 6800.0, 3500.0), PI, 230.0);
            let mut pilot = Pilot::new(0.8);
            let mut extends = 0;
            for _ in 0..(20.0 / FIXED_DT) as usize {
                let o = pilot.fly(&me, &picture(&foe), FIXED_DT);
                extends += usize::from(pilot.tactic == Tactic::Extend);
                me.update(FIXED_DT, &o.input);
                assert!(!me.crashed && !me.lost.any());
            }
            assert!(extends > 300 && me.speed > 170.0, "{kind:?}: {:.0} m/s, extends {extends}", me.speed);
        }
    }

    #[test]
    fn defence_cools_ir_tail_and_saves_countermeasures_for_closing_threats() {
        let mut me = spawn(v3(0.0, 5000.0, 0.0), 0.0, 240.0);
        me.throttle = 1.0; me.power = 100.0;
        let foe = spawn(v3(0.0, 5000.0, 5000.0), 0.0, 240.0);
        let mut pic = picture(&foe);
        pic.missile = Some((v3(0.0, 5000.0, 3000.0), v3(0.0, 0.0, -750.0)));
        pic.tactical.missile_ir = true;
        let mut pilot = Pilot::new(0.9);
        let o = pilot.fly(&me, &pic, FIXED_DT);
        assert_eq!(pilot.tactic, Tactic::Evade);
        assert!(o.countermeasures && !o.input.afterburner && o.input.throttle_down);
        assert!(!pilot.fly(&me, &pic, FIXED_DT).countermeasures);
        let mut pilot = Pilot::new(0.9);
        pic.missile = Some((v3(0.0, 5000.0, 3000.0), v3(0.0, 0.0, 750.0)));
        let o = pilot.fly(&me, &pic, FIXED_DT);
        assert!(!o.countermeasures && pilot.tactic != Tactic::Evade);
    }

    #[test]
    fn gun_burst_stops_immediately_when_the_solution_is_lost() {
        let mut me = spawn(v3(0.0, 5000.0, 0.0), 0.0, 220.0);
        let mut foe = spawn(v3(0.0, 5000.0, -650.0), 0.0, 220.0);
        let mut pilot = Pilot::new(1.0);
        let mut acquired = false;
        for _ in 0..(12.0 / FIXED_DT) as usize {
            let mut pic = picture(&foe); pic.rounds = 500;
            let o = pilot.fly(&me, &pic, FIXED_DT);
            if o.gun { acquired = true; break; }
            me.update(FIXED_DT, &o.input);
            foe.update(FIXED_DT, &Input::default());
        }
        assert!(acquired);
        foe.pos = me.pos + V3::X * 600.0;
        let mut pic = picture(&foe); pic.rounds = 500;
        assert!(!pilot.fly(&me, &pic, FIXED_DT).gun);
    }

    /// Use the protected flight model, without flares or artificial immunity.
    /// There must be an escapable shot which would hit a non-defending pilot.
    #[test]
    fn protected_airframes_can_defeat_a_missile_by_manoeuvring() {
        use crate::weapons::{Fate, Missile, Who};
        for kind in [crate::sim::Type::F16, crate::sim::Type::Mig29] {
            let mut escaped = 0;
            for range in [10000.0, 14000.0, 18000.0] {
                let run = |defend: bool| {
                    let mut me = crate::combat::spawn_type(kind, v3(0.0, 6000.0, -range), PI, 250.0);
                    let foe = spawn(v3(0.0, 6000.0, 0.0), 0.0, 250.0);
                    let mut missile = Missile::launch(&AIM9M, foe.pos, foe.vel, Who::Player, Some(Who::Bandit(0)));
                    let mut pilot = Pilot::new(0.9);
                    for _ in 0..(45.0 / FIXED_DT) as usize {
                        let input = if defend {
                            let mut pic = picture(&foe);
                            pic.missile = missile.tracking.then_some((missile.pos, missile.vel));
                            pic.tactical.missile_ir = true;
                            pilot.fly(&me, &pic, FIXED_DT).input
                        } else { Input::default() };
                        me.update(FIXED_DT, &input);
                        assert!(!me.crashed && !me.lost.any(), "{kind:?} defence hit the ground or broke up");
                        let fate = missile.step(FIXED_DT, Some((me.pos, me.vel)), &|_, _| 0.0);
                        if fate != Fate::Flying { return fate; }
                    }
                    panic!("missile exceeded its lifetime");
                };
                let baseline = run(false);
                let defence = run(true);
                if matches!(baseline, Fate::Burst(_, _)) && !matches!(defence, Fate::Burst(_, _)) { escaped += 1; }
            }
            assert!(escaped > 0, "{kind:?} could not defeat any otherwise lethal shot");
        }
    }

    #[test]
    fn gun_tracking_hits_with_both_airframes_ballistics() {
        use crate::weapons::{hits_airframe, Round, Who};
        for kind in [crate::sim::Type::F16, crate::sim::Type::Mig29] {
            let mut me = crate::combat::spawn_type(kind, v3(30.0, 4980.0, 0.0), 0.0, 210.0);
            let mut foe = spawn(v3(0.0, 5000.0, -700.0), 0.0, 200.0);
            let mut pilot = Pilot::new(1.0);
            let gun = gun_of(kind);
            let mut acc = 0.0;
            let mut rounds: Vec<Round> = Vec::new();
            let (mut fired, mut hits) = (0, 0);
            for _ in 0..(12.0 / FIXED_DT) as usize {
                let mut pic = picture(&foe); pic.rounds = gun.rounds - fired;
                let o = pilot.fly(&me, &pic, FIXED_DT);
                me.update(FIXED_DT, &o.input); foe.update(FIXED_DT, &Input::default());
                if o.gun {
                    acc += gun.rate * FIXED_DT;
                    while acc >= 1.0 && fired < gun.rounds {
                        acc -= 1.0; fired += 1;
                        let (p, v) = muzzle(&me);
                        rounds.push(Round::of(gun, p, v, false, Who::Bandit(0)));
                    }
                } else { acc = 0.0; }
                rounds.retain_mut(|r| {
                    r.step(FIXED_DT);
                    if hits_airframe(&foe, r.prev - foe.last_pos, r.pos - foe.pos).is_some() {
                        hits += 1; return false;
                    }
                    r.age < crate::weapons::ROUND_LIFE
                });
            }
            assert!(hits > 0, "{kind:?}: fired {fired}, hit {hits}");
        }
    }

}
