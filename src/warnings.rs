//! Airframe-specific warning selection, with priorities and repeat cooldowns.
use crate::{air::AirData, sim::{Aircraft, Type}, ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Cue { Fire, PullUp, OverSpeed, HighAoa, LowSpeed, Gear, OverG, Altitude }

impl Cue {
    pub fn priority(self) -> u8 { 8 - self as u8 }
}

pub struct Warnings { remaining: [f64; 8] }
impl Default for Warnings { fn default() -> Self { Self { remaining: [0.0; 8] } } }
impl Warnings {
    pub fn update(&mut self, dt: f64, a: &Aircraft) -> Option<Cue> {
        for t in &mut self.remaining { *t = (*t - dt).max(0.0); }
        if a.ejected || a.crashed { return None; }
        let flight = !a.on_ground && a.speed > 25.0;
        let ad = AirData::of(a);
        let mig = a.kind == Type::Mig29;
        let conditions = [
            (Cue::Fire, a.battle.fire, 6.0),
            (Cue::PullUp, flight && ui::pull_up(a), 3.0),
            (Cue::OverSpeed, flight && ad.overspeed(), 7.0),
            (Cue::HighAoa, flight && mig && a.alpha_deg > 24.0, 5.0),
            (Cue::LowSpeed, flight && (ui::stall(a) || !mig && a.alpha_deg > 15.0 && a.airspeed_kt < 190.0), 2.0),
            (Cue::Gear, flight && a.gear < 0.95 && a.agl < 180.0 && a.vs < -2.0 && a.airspeed_kt < 210.0, 7.0),
            (Cue::OverG, flight && mig && a.g_smooth > a.kind.g_limit(a.mass, a.mach), 6.0),
            (Cue::Altitude, flight && a.gear < 0.9 && a.agl < 90.0 && a.vs < -1.0, 6.0),
        ];
        for (cue, active, interval) in conditions {
            if active && self.remaining[cue as usize] <= 0.0 {
                self.remaining[cue as usize] = interval;
                return Some(cue);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Mission;
    #[test]
    fn warnings_prioritise_fire_and_do_not_repeat_every_frame_or_after_ejection() {
        let mut a = Aircraft::new(&Mission::new());
        a.battle.fire = true;
        let mut w = Warnings::default();
        assert_eq!(w.update(0.01, &a), Some(Cue::Fire));
        a.on_ground = true;
        assert_eq!(w.update(0.01, &a), None);
        assert_eq!(w.update(6.0, &a), Some(Cue::Fire));
        a.ejected = true;
        assert_eq!(w.update(8.0, &a), None);
    }
}
