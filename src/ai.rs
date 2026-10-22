//! AI pilot for the bandits. It flies its own sim::Aircraft through the same
//! stick, FLCS and engine as the player, so it meets the same 9 g, AoA and
//! energy limits; all it decides is where the lift vector goes, how hard to
//! pull and where the throttle sits, like a pilot. The tactics are classic
//! one-versus-one BFM: lead pursuit to track for the gun, pure pursuit to
//! close, lag pursuit to avoid an overshoot, a max-g break when the player is
//! at its six with his nose on, a beam-then-break against a missile, and
//! unloading to rebuild speed. Terrain avoidance overrides everything.
use crate::math::*;
use crate::sim::{stick_for_g, stick_for_roll, Aircraft, Input, G0};
use crate::terrain;
use crate::weapons::{fly_out, gun_bore, muzzle, Spec, MUZZLE_VELOCITY};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tactic {
    /// Gun tracking: the gun line on the lead point.
    Lead,
    /// Nose on the bandit, to close the range.
    Pure,
    /// Nose behind the bandit, to keep from overshooting.
    Lag,
    /// Max-g turn into an attacker at the six.
    Break,
    /// Missile inbound: put it on the beam, then turn into it.
    Evade,
    /// Unload and accelerate in burner.
    Extend,
    /// Terrain ahead: climb away from it.
    Recover,
}

/// What the AI sees of the fight this step.
pub struct Picture<'a> {
    pub foe: &'a Aircraft,
    /// Position and velocity of the nearest missile guiding on this jet.
    pub missile: Option<(V3, V3)>,
    pub missiles: u32,
    pub missile_spec: &'static Spec,
    pub rounds: u32,
}

pub struct Orders {
    pub input: Input,
    pub gun: bool,
    pub missile: bool,
}

pub struct Pilot {
    /// 0 a novice, 1 an ace: g tolerance, reaction time, aim and shot discipline.
    pub skill: f64,
    pub tactic: Tactic,
    think: f64,
    burst: f64,
    since_shot: f64,
    zone_clock: f64,
    in_zone: bool,
    /// Recovery bank chosen when terrain threatens, degrees, and how long
    /// the way ahead has been clear since.
    escape: f64,
    escape_clock: f64,
    clear: f64,
}

/// Clearance the AI keeps over the terrain along its predicted path, m.
const FLOOR: f64 = 180.0;

impl Pilot {
    pub fn new(skill: f64) -> Pilot {
        Pilot { skill, tactic: Tactic::Pure, think: 0.0, burst: 0.0, since_shot: 6.0, zone_clock: 0.0, in_zone: false, escape: 0.0, escape_clock: 0.0, clear: 0.0 }
    }

    fn g_limit(&self) -> f64 {
        mix(7.0, 9.0, self.skill)
    }

    pub fn fly(&mut self, me: &Aircraft, pic: &Picture, dt: f64) -> Orders {
        let foe = pic.foe;
        let ax = me.q.axes();
        let rel = foe.pos - me.pos;
        let range = rel.len().max(1.0);
        let los = rel / range;
        let ata = ax.forward.dot(los).clamp(-1.0, 1.0).acos();
        let foe_ata = foe.q.axes().forward.dot(-los).clamp(-1.0, 1.0).acos();
        let closure = -(foe.vel - me.vel).dot(los);
        self.since_shot += dt;
        self.think -= dt;

        // Terrain first: an Auto-GCAS check every step, never delayed by
        // reaction time. The recovery holds until the predicted pull-up
        // clears comfortably for a second.
        let n_max = self.g_limit();
        let gcas = recovery_clearance(me, 0.0, 6.0);
        if gcas < FLOOR {
            // The way out is re-chosen twice a second as the ground unfolds.
            self.escape_clock -= dt;
            if self.tactic != Tactic::Recover || self.escape_clock <= 0.0 {
                self.escape_clock = 0.5;
                self.escape = escape_bank(me, n_max);
            }
            self.tactic = Tactic::Recover;
            self.clear = 0.0;
        } else if self.tactic == Tactic::Recover {
            self.clear = if gcas > FLOOR + 250.0 { self.clear + dt } else { 0.0 };
            if self.clear > 1.0 {
                self.think = 0.0;
                self.tactic = Tactic::Pure;
            }
        }
        if self.tactic != Tactic::Recover && self.think <= 0.0 {
            self.think = mix(0.55, 0.15, self.skill);
            self.tactic = if pic.missile.is_some_and(|(p, v)| {
                let r = me.pos - p;
                r.len() < 6000.0 && (v - me.vel).dot(r) > 0.0
            }) {
                Tactic::Evade
            } else if foe_ata < 40.0 * DEG && range < 3500.0 && ata > 100.0 * DEG {
                Tactic::Break
            } else if (me.speed < 135.0 && ata > 60.0 * DEG) || (range < 800.0 && ata > 70.0 * DEG && foe_ata > 70.0 * DEG)
                || (self.tactic == Tactic::Extend && range < 2200.0 && foe_ata > 60.0 * DEG && me.speed > 135.0)
            {
                // Slow and out of position, or a neutral pass close aboard:
                // extend out past 2 km and come back with room to turn.
                Tactic::Extend
            } else if range < 1600.0 && ata < 60.0 * DEG {
                if closure > 70.0 && range < 900.0 && foe_ata > 90.0 * DEG {
                    Tactic::Lag
                } else {
                    Tactic::Lead
                }
            } else {
                Tactic::Pure
            };
        }

        let bore = me.q.rotate(gun_bore());
        let lead = gun_lead(me, foe);
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
            Tactic::Extend => ((flat(me.vel) - V3::Y * 0.08).norm(), 1.5, true),
            Tactic::Evade => {
                let (mp, _) = pic.missile.unwrap_or((foe.pos, foe.vel));
                let to_missile = (mp - me.pos).norm();
                if (mp - me.pos).len() < 1300.0 {
                    (to_missile, n_max, true)
                } else {
                    let beam = to_missile.cross(V3::Y).norm();
                    let beam = if beam.dot(me.vel) >= 0.0 { beam } else { -beam };
                    (beam, n_max * 0.8, false)
                }
            }
            Tactic::Lag => (((foe.pos - foe.q.axes().forward * 900.0) - me.pos).norm(), n_max, false),
            Tactic::Lead => {
                // Point the gun line, not the flight path, at the lead point.
                let off = bore - me.vel.norm();
                ((lead - off).norm(), n_max, me.speed < 190.0)
            }
            // Far out there is no hurry: a moderate pull keeps the energy.
            Tactic::Pure => (los, if range > 4000.0 { 5.0 } else { n_max * 0.9 }, me.speed < 240.0),
        };
        // Energy: below corner speed (about 230 m/s) every extra g bleeds speed
        // the jet cannot afford, so offensive turns ease off as it slows.
        if matches!(self.tactic, Tactic::Pure | Tactic::Lead | Tactic::Lag) {
            pull = pull.min(mix(2.0, n_max, clamp((me.speed - 120.0) / 110.0, 0.0, 1.0)));
            burner |= me.speed < 230.0;
        }
        // A soft deck: no dive steeper than the height above the ground allows.
        if self.tactic != Tactic::Recover {
            let agl = me.pos.y - terrain::floor(me.pos.x, me.pos.z);
            let steepest = -clamp((agl - 350.0) / 1800.0, 0.0, 1.0) * 55.0 * DEG;
            if dir.y < steepest.sin() {
                let h = flat(dir);
                dir = (h * steepest.cos() + V3::Y * steepest.sin()).norm();
            }
            if me.speed > 300.0 {
                burner = false;
                pull = pull.min(n_max);
            }
        }
        let (pitch, roll) = steer(me, dir, pull);

        // Throttle: burner when asked, else hold a corner-speed band in military power.
        let want = if burner { 1.0 } else if me.speed > 270.0 { 0.75 } else { 0.99 };
        let mut input = Input { pitch_axis: pitch, roll_axis: roll, ..Default::default() };
        if want >= 1.0 {
            input.afterburner = true;
        } else if me.throttle > want + 0.01 {
            input.throttle_down = true;
        } else if me.throttle < want - 0.01 {
            input.throttle_up = true;
        }

        // Guns: a burst when the gun line sits on the lead point inside 900 m.
        let error = bore.dot(lead).clamp(-1.0, 1.0).acos();
        let tolerance = mix(2.2, 0.9, self.skill) * DEG;
        if pic.rounds > 0 && range < 900.0 && range > 120.0 && error < tolerance {
            self.burst = self.burst.max(mix(0.5, 0.8, self.skill));
        }
        let gun = self.burst > 0.0 && pic.rounds > 0;
        self.burst = (self.burst - dt).max(0.0);

        // Missiles: inside the launch zone (a fly-out against the present
        // track), with the target near the nose. Novices only take shots from
        // behind; aces trust the all-aspect seeker.
        self.zone_clock -= dt;
        if self.zone_clock <= 0.0 {
            self.zone_clock = 0.5;
                self.in_zone = pic.missiles > 0 && ata < 25.0 * DEG && fly_out(pic.missile_spec, me.pos, me.vel, foe.pos, foe.vel, &terrain::floor).is_some();
        }
        let aspect_ok = foe_ata > 70.0 * DEG || self.skill > 0.7;
        let missile = self.in_zone && aspect_ok && self.since_shot > mix(14.0, 8.0, self.skill) && self.tactic != Tactic::Recover;
        if missile {
            self.since_shot = 0.0;
            self.in_zone = false;
        }
        Orders { input, gun, missile }
    }
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
    let tof = r.len() / (MUZZLE_VELOCITY * 0.78);
    (r + (foe.vel - me.vel) * tof + V3::Y * (0.5 * G0 * tof * tof)).norm()
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
/// 180 deg/s, then `n` g with the lift vector there. Wings level it pulls
/// until the path climbs 45 degrees; banked it keeps turning away from the
/// ground. A point mass at constant speed, sampled every 0.2 s for 9 s.
fn recovery_clearance(me: &Aircraft, bank: f64, n: f64) -> f64 {
    let speed = me.speed.max(80.0);
    let mut p = me.pos;
    let mut v = if me.vel.len() > 1.0 { me.vel.norm() } else { me.q.axes().forward };
    let roll_time = 0.3 + wrap180(me.bank - bank).abs() / 180.0;
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
        Picture { foe, missile: None, missiles: 0, missile_spec: &AIM9M, rounds: 0 }
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
        for _ in 0..(10.0 / FIXED_DT) as usize {
            let pic = Picture { foe: &foe, missile: None, missiles: 0, missile_spec: &AIM9M, rounds: 500 };
            let o = pilot.fly(&me, &pic, FIXED_DT);
            fired += o.gun as usize;
            me.update(FIXED_DT, &o.input);
            foe.update(FIXED_DT, &Input::default());
        }
        assert!(fired > 60, "fired for {fired} steps");
    }
}
