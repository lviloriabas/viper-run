//! Objective-driven lessons. Only unpaused simulation steps advance the checks.
use crate::combat::{Combat, MissionKind, Mode};
use crate::keymap::{key_name, Bind, Keymap};
use crate::math::*;
use crate::settings::{PadLayout, PitchSource};
use crate::sim::{Aircraft, Input, Type};
use crate::terrain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lesson {
    Controls,
    Flight,
    Weapons,
    Takeoff,
    Landing,
}
impl Lesson {
    pub const ALL: [Self; 5] = [
        Self::Controls,
        Self::Flight,
        Self::Weapons,
        Self::Takeoff,
        Self::Landing,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&l| l == self).unwrap()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Controls => "CONTROLS",
            Self::Flight => "BASIC FLIGHT",
            Self::Weapons => "WEAPONS",
            Self::Takeoff => "TAKEOFF",
            Self::Landing => "LANDING",
        }
    }
    pub fn brief(self) -> &'static str {
        match self {
        Self::Controls => "Learn pitch, roll, rudder, throttle, airbrake and views, one action at a time.",
        Self::Flight => "Hold level flight, make a controlled banked turn, climb and recover.",
        Self::Weapons => "Practice gun bursts, target acquisition, missiles, CCIP bombs and countermeasures. Training targets hold fire.",
        Self::Takeoff => "Start on runway A. Apply power, rotate, establish a climb and retract the landing gear.",
        Self::Landing => "Start on final for runway A. Configure, align, touch down on the runway and brake to a full stop.",
    }
    }
    pub fn mission(self) -> MissionKind {
        if self == Self::Takeoff {
            MissionKind::Airfield
        } else {
            MissionKind::Free
        }
    }
    pub fn steps(self) -> usize {
        match self {
            Self::Controls => 9,
            Self::Flight => 4,
            Self::Weapons => 6,
            Self::Takeoff => 4,
            Self::Landing => 4,
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "controls" => Some(Self::Controls),
            "flight" => Some(Self::Flight),
            "weapons" => Some(Self::Weapons),
            "takeoff" => Some(Self::Takeoff),
            "landing" => Some(Self::Landing),
            _ => None,
        }
    }
}
pub struct Observation<'a> {
    pub jet: &'a Aircraft,
    pub combat: &'a Combat,
    pub input: Input,
    pub looking: bool,
    pub external_view: bool,
}
pub struct Training {
    pub lesson: Lesson,
    pub stage: usize,
    pub held: f64,
    pub elapsed: f64,
    altitude: f64,
    rounds: u32,
    missiles: usize,
    bombs: usize,
    touched_on_runway: bool,
}
impl Training {
    pub fn new(lesson: Lesson, a: &Aircraft, c: &Combat) -> Self {
        Self {
            lesson,
            stage: 0,
            held: 0.0,
            elapsed: 0.0,
            altitude: a.pos.y,
            rounds: c.loadout.rounds,
            missiles: c.missiles.len(),
            bombs: c.bombs.len(),
            touched_on_runway: false,
        }
    }
    pub fn done(&self) -> bool {
        self.stage >= self.lesson.steps()
    }
    pub fn duration(&self) -> f64 {
        match (self.lesson, self.stage) {
            (Lesson::Flight, 0 | 3) => 6.0,
            (Lesson::Flight, 1) => 3.0,
            (Lesson::Landing, 1) | (Lesson::Takeoff, 1) => 1.0,
            (Lesson::Weapons, _) => 0.0,
            _ => 0.3,
        }
    }
    pub fn step(&mut self, dt: f64, o: &Observation<'_>) -> bool {
        if self.done() || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        self.elapsed += dt;
        let a = o.jet;
        if a.crashed || a.ejected || a.lost.any() {
            self.held = 0.0;
            return false;
        }
        let (u, v) = terrain::runway_local(a.pos.x, a.pos.z);
        let runway = u.abs() < terrain::RUNWAY_HALF_LENGTH && v.abs() < terrain::RUNWAY_HALF_WIDTH;
        if self.lesson == Lesson::Landing && a.on_ground {
            self.touched_on_runway |= runway;
        }
        let level =
            a.bank.abs() < 10.0 && a.vs.abs() < 8.0 && (220.0..480.0).contains(&a.airspeed_kt);
        let ok = match (self.lesson, self.stage) {
            (Lesson::Controls, 0) => o.looking,
            (Lesson::Controls, 1) => o.input.roll() < -0.25,
            (Lesson::Controls, 2) => o.input.roll() > 0.25,
            (Lesson::Controls, 3) => o.input.pitch() > 0.25,
            (Lesson::Controls, 4) => o.input.pitch() < -0.25,
            (Lesson::Controls, 5) => o.input.yaw().abs() > 0.25,
            (Lesson::Controls, 6) => o.input.throttle_up,
            (Lesson::Controls, 7) => a.brake > 0.75,
            (Lesson::Controls, 8) => o.external_view,
            (Lesson::Flight, 0 | 3) => level,
            (Lesson::Flight, 1) => {
                (20.0..55.0).contains(&a.bank.abs())
                    && (1.1..3.5).contains(&a.g_load)
                    && a.airspeed_kt > 220.0
            }
            (Lesson::Flight, 2) => a.pos.y > self.altitude + 150.0 && a.vs > 0.0,
            (Lesson::Weapons, 0) => {
                o.combat.mode == Mode::Gun
                    && o.combat.loadout.rounds < self.rounds.saturating_sub(15)
            }
            (Lesson::Weapons, 1) => o.combat.lock.is_some(),
            (Lesson::Weapons, 2) => {
                matches!(o.combat.mode, Mode::Srm | Mode::Mrm)
                    && o.combat.missiles.len() > self.missiles
            }
            (Lesson::Weapons, 3) => o.combat.mode == Mode::Ccip && o.combat.ccip.is_some(),
            (Lesson::Weapons, 4) => o.combat.bombs.len() > self.bombs,
            (Lesson::Weapons, 5) => o.combat.loadout.chaff < 30 && o.combat.loadout.flares < 30,
            (Lesson::Takeoff, 0) => a.power > 55.0 && a.airspeed_kt > 40.0,
            (Lesson::Takeoff, 1) => !a.on_ground && a.agl > 20.0 && a.vs > 0.0,
            (Lesson::Takeoff, 2) => a.gear < 0.05 && a.agl > 50.0,
            (Lesson::Takeoff, 3) => {
                a.pos.y > terrain::base_elevation() + 300.0 && a.vs > 0.0 && a.airspeed_kt > 180.0
            }
            (Lesson::Landing, 0) => a.gear > 0.98 && a.flaps > 0.5 && a.airspeed_kt < 230.0,
            (Lesson::Landing, 1) => {
                v.abs() < 55.0
                    && a.bank.abs() < 10.0
                    && a.vs < 0.0
                    && a.vs > -8.0
                    && a.airspeed_kt < 210.0
            }
            (Lesson::Landing, 2) => a.on_ground && runway,
            (Lesson::Landing, 3) => {
                self.touched_on_runway && a.on_ground && runway && a.speed < 2.0
            }
            _ => false,
        };
        self.held = if ok { self.held + dt } else { 0.0 };
        if ok && self.held >= self.duration() {
            self.stage += 1;
            self.held = 0.0;
            self.altitude = a.pos.y;
            self.missiles = o.combat.missiles.len();
            self.bombs = o.combat.bombs.len();
            return true;
        }
        false
    }
    pub fn instruction(&self) -> (&'static str, &'static str, Option<Bind>) {
        match (self.lesson, self.stage) {
            (Lesson::Controls, 0) => ("LOOK AROUND", "Hold Alt and move the mouse, or use the right stick. Keep the horizon in sight.", None),
            (Lesson::Controls, 1) => ("ROLL LEFT", "Use a small input. Banking turns the lift vector sideways.", Some(Bind::RollLeft)),
            (Lesson::Controls, 2) => ("ROLL RIGHT", "Reverse the roll, then release before the bank becomes steep.", Some(Bind::RollRight)),
            (Lesson::Controls, 3) => ("RAISE THE NOSE", "Pull gently. Watch airspeed and G.", Some(Bind::PitchUp)),
            (Lesson::Controls, 4) => ("LOWER THE NOSE", "Push gently, then center the stick. Avoid a long negative-G push.", Some(Bind::PitchDown)),
            (Lesson::Controls, 5) => ("USE THE RUDDER", "Tap either pedal. Use roll and pitch for normal turns.", Some(Bind::YawLeft)),
            (Lesson::Controls, 6) => ("ADD POWER", "Increase throttle. The engine takes a moment to spool.", Some(Bind::ThrottleUp)),
            (Lesson::Controls, 7) => ("DEPLOY AIRBRAKE", "The airbrake adds drag. Retract it after this check.", Some(Bind::Airbrake)),
            (Lesson::Controls, 8) => ("SWITCH VIEW", "Try the chase camera. The same button returns to the cockpit.", Some(Bind::View)),
            (Lesson::Flight, 0) => ("HOLD LEVEL FOR 6 SECONDS", "Bank within 10 degrees, vertical speed within 8 m/s, airspeed 220 to 480 kt.", None),
            (Lesson::Flight, 1) => ("MAKE A BANKED TURN", "Hold 20 to 55 degrees of bank, 1.1 to 3.5 G and over 220 kt for 3 seconds. Roll, then pull.", Some(Bind::RollRight)),
            (Lesson::Flight, 2) => ("CLIMB 150 METRES", "Add power and raise the nose. Preserve airspeed.", Some(Bind::ThrottleUp)),
            (Lesson::Flight, 3) => ("RECOVER LEVEL FLIGHT", "Roll wings level and stabilize the vertical speed for 6 seconds.", None),
            (Lesson::Weapons, 0) => ("FIRE A GUN BURST", "Cycle to GUN and fire at least 16 rounds. The funnel predicts the ballistic stream.", Some(Bind::Trigger)),
            (Lesson::Weapons, 1) => ("ACQUIRE A TARGET", "Select SRM or MRM, point at a training aircraft and request a lock. Wait for acquisition.", Some(Bind::Lock)),
            (Lesson::Weapons, 2) => ("LAUNCH A MISSILE", "SRM needs the seeker tone. Radar missiles use a radar track. Fire when in range.", Some(Bind::Trigger)),
            (Lesson::Weapons, 3) => ("SELECT CCIP BOMBS", "Cycle to CCIP. Descend gently until the predicted impact pipper appears.", Some(Bind::Weapon)),
            (Lesson::Weapons, 4) => ("RELEASE A BOMB", "Place the pipper on a ground target, release and pull clear. The fuze needs 2 seconds of fall.", Some(Bind::Trigger)),
            (Lesson::Weapons, 5) => ("DISPENSE COUNTERMEASURES", "Chaff opposes radar seekers; flares oppose infrared seekers. Manoeuvre as you dispense.", Some(Bind::Countermeasures)),
            (Lesson::Takeoff, 0) => ("ACCELERATE ON THE CENTRELINE", "Release wheel brakes, set full throttle and afterburner. Use small rudder corrections.", Some(Bind::Afterburner)),
            (Lesson::Takeoff, 1) => ("ROTATE AND CLIMB", "Near 150 to 170 kt, pull gently. Establish a positive climb.", Some(Bind::PitchUp)),
            (Lesson::Takeoff, 2) => ("RETRACT THE LANDING GEAR", "Above 50 m AGL and climbing. Raise the gear before exceeding its limit speed.", Some(Bind::Gear)),
            (Lesson::Takeoff, 3) => ("CLIMB CLEAR", "Climb 300 m above the airfield and accelerate past 180 kt.", None),
            (Lesson::Landing, 0) => ("CONFIGURE FOR LANDING", "Below 230 kt, lower landing gear and flaps. Use power to control descent.", Some(Bind::Gear)),
            (Lesson::Landing, 1) => ("STABILIZE THE APPROACH", "Follow runway A centreline, bank below 10 degrees, speed below 210 kt, descent below 8 m/s.", None),
            (Lesson::Landing, 2) => ("TOUCH DOWN ON RUNWAY A", "Aim just beyond the threshold. Ease the descent near the ground.", None),
            (Lesson::Landing, 3) => ("BRAKE TO A FULL STOP", "Stay on the paved runway, reduce throttle and hold wheel brakes until walking pace.", Some(Bind::WheelBrake)),
            _ => ("LESSON COMPLETE", "Completion saved. Repeat the lesson or return to flight school.", None),
        }
    }
    pub fn hint(&self, keys: &Keymap, pad: bool, layout: PadLayout, pitch: PitchSource, click_rudder: bool) -> String {
        let Some(b) = self.instruction().2 else {
            return "ESC CONFIGURATION / RESTART / MAIN MENU".into();
        };
        if pad {
            return match b {
                Bind::PitchUp | Bind::PitchDown | Bind::RollLeft | Bind::RollRight => "LEFT STICK",
                Bind::YawLeft => {
                    if layout == PadLayout::Combat {
                        if click_rudder { "HOLD X OR L3 + LEFT STICK" } else { "HOLD X + LEFT STICK" }
                    } else {
                        "LB / RB"
                    }
                }
                Bind::ThrottleUp | Bind::Afterburner => {
                    if layout == PadLayout::Combat {
                        "HOLD LB"
                    } else {
                        "HOLD RT"
                    }
                }
                Bind::Trigger => {
                    if layout == PadLayout::Combat {
                        "RT"
                    } else {
                        "A"
                    }
                }
                Bind::Weapon => {
                    if layout == PadLayout::Combat {
                        "A"
                    } else {
                        "DPAD UP"
                    }
                }
                Bind::Lock => {
                    if layout == PadLayout::Combat {
                        "RB"
                    } else {
                        "DPAD DOWN"
                    }
                }
                Bind::Gear => {
                    if layout == PadLayout::Combat {
                        "DPAD DOWN"
                    } else {
                        "X"
                    }
                }
                Bind::WheelBrake => "HOLD LT",
                Bind::View => "Y",
                Bind::Airbrake => "B",
                Bind::Countermeasures => if click_rudder { "X + LEFT STICK CLICK" } else { "LEFT STICK CLICK" },
                _ => "ESC CONFIGURATION",
            }
            .into();
        }
        if matches!(b, Bind::PitchUp | Bind::PitchDown) && pitch == PitchSource::Mouse {
            return "MOUSE PITCH (C TO CENTER)".into();
        }
        let labels: Vec<_> = keys.get(b).iter().flatten().map(|&k| key_name(k)).collect();
        if labels.is_empty() {
            "UNBOUND: ASSIGN A KEY IN ESC CONFIGURATION".into()
        } else {
            format!("{}: {}", b.label(), labels.join(" / "))
        }
    }
}
pub fn position(lesson: Lesson, kind: Type, a: &mut Aircraft) {
    if lesson == Lesson::Takeoff {
        return;
    }
    let (pos, heading, speed) = if lesson == Lesson::Landing {
        let (x, z) = terrain::runway_point(-terrain::RUNWAY_HALF_LENGTH - 3800.0, 0.0);
        let (dx, dz) = terrain::runway_dir();
        (
            v3(
                x,
                (terrain::base_elevation() + 260.0).max(terrain::floor(x, z) + 180.0),
                z,
            ),
            dx.atan2(-dz),
            95.0,
        )
    } else {
        (
            v3(
                a.pos.x,
                a.pos
                    .y
                    .max(terrain::floor(a.pos.x, a.pos.z) + 1800.0)
                    .max(3200.0),
                a.pos.z,
            ),
            a.heading * DEG,
            180.0,
        )
    };
    let fuel = a.fuel;
    *a = crate::combat::spawn_type(kind, pos, heading, speed);
    a.fuel = fuel;
    a.mass = kind.empty_mass() + a.fuel + a.stores_mass;
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Mission;
    #[test]
    fn level_check_requires_continuous_time_and_live_aircraft() {
        let mut m = Mission::new();
        let c = Combat::new(MissionKind::Free, &mut m);
        let mut a = crate::combat::spawn_type(Type::F16, v3(0.0, 4000.0, 0.0), 0.0, 180.0);
        a.airspeed_kt = 350.0;
        a.vs = 0.0;
        a.bank = 0.0;
        let mut t = Training::new(Lesson::Flight, &a, &c);
        for _ in 0..50 {
            t.step(
                0.1,
                &Observation {
                    jet: &a,
                    combat: &c,
                    input: Input::default(),
                    looking: false,
                    external_view: false,
                },
            );
        }
        assert_eq!(t.stage, 0);
        a.bank = 30.0;
        t.step(
            0.1,
            &Observation {
                jet: &a,
                combat: &c,
                input: Input::default(),
                looking: false,
                external_view: false,
            },
        );
        assert_eq!(t.held, 0.0);
        a.bank = 0.0;
        for _ in 0..61 {
            t.step(
                0.1,
                &Observation {
                    jet: &a,
                    combat: &c,
                    input: Input::default(),
                    looking: false,
                    external_view: false,
                },
            );
        }
        assert_eq!(t.stage, 1);
        a.crashed = true;
        assert!(!t.step(
            10.0,
            &Observation {
                jet: &a,
                combat: &c,
                input: Input::default(),
                looking: false,
                external_view: false
            }
        ));
        assert_eq!(t.stage, 1);
    }
    #[test]
    fn instruction_uses_live_key_bindings() {
        let mut m = Mission::new();
        let c = Combat::new(MissionKind::Free, &mut m);
        let a = Aircraft::new(&m);
        let mut t = Training::new(Lesson::Controls, &a, &c);
        t.stage = 7;
        let mut keys = Keymap::default();
        keys.set(Bind::Airbrake, 0, Some(miniquad::KeyCode::U));
        assert!(t
            .hint(&keys, false, PadLayout::Combat, PitchSource::Keys, false)
            .ends_with("U"));
        assert_eq!(
            t.hint(&keys, true, PadLayout::Combat, PitchSource::Keys, false),
            "B"
        );
    }
}
