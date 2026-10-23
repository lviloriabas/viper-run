//! Pilot G tolerance. In the F-16's 30-degree seat the eye sits about 28 cm
//! above the heart, so every g of normal load takes about 22 mmHg off the
//! blood pressure at eye level (blood is 1,060 kg/m3). The anti-G suit, the
//! anti-G straining manoeuvre (AGSM) and the baroreflex put some back. The
//! retina stops filling when the pressure falls towards the eye's own, the
//! brain a little later (the venous siphon helps it), and each holds a few
//! seconds of oxygen. Too little pressure greys the view, closes it into a
//! tunnel and blacks it out; the pilot then loses consciousness (G-LOC).
//! Negative g floods the head instead: the view reddens, and the reflex it
//! sets off leaves the pilot less tolerant of the pull that follows (the
//! push-pull effect).
//!
//! The figures follow the aerospace-medicine literature (Burton and Whinnery,
//! centrifuge data). Relaxed and unprotected, the view starts to close at
//! about 4.9 G on a gradual onset (0.1 G/s), blacks out near 5.5 G and the
//! pilot goes out near 6.9 G; held on a rapid-onset plateau, before the
//! reflex has acted, light loss comes 0.7 G sooner. The suit adds about
//! 1.4 G, COMBAT EDGE pressure breathing about 1.1 G and a good AGSM about
//! 3 G. The straining tires the pilot, slowly at first and then faster:
//! the centrifuge standard is 15 s at 9 G, which about one attempt in five
//! fails, so here the view starts to close after about 10 s at 9 G, narrows
//! to a tunnel over the next several, blacks out near 20 s and the pilot
//! goes out a few seconds later. How long a
//! G-LOC lasts follows the exposure: the longer and deeper the brain went
//! without blood, the longer the pilot stays out, and nobody wakes while the
//! g is still on. Centrifuge episodes average 12 s of absolute and 12 s of
//! relative incapacitation (2 to 38 s measured); a brief spike here costs
//! about 3 s and 2 s, a long grinding one up to 20 s and 12 s. Negative g
//! reddens the view from about -1.5 G and fully at the jet's -3 G limit.
//!
//! Nothing here is all or nothing. G stress accumulates: every second hard
//! past 3 G, or pushing below -1 G, adds to a dose that fades over minutes,
//! and the more of it the pilot carries, the lower the tolerance, the slower
//! the rest and the oxygen come back, and the longer a G-LOC lasts. A fresh
//! pilot holds 9 G for about 25 s before going out, one who pulled 9 G for
//! 15 s half a minute earlier only about 17 s. And the G-LOC itself comes on
//! through an almost-loss of consciousness (A-LOC): as the brain runs short
//! the hands slacken on the controls and the view dims over a few seconds,
//! then comes back through a grey haze and confusion afterwards.
use crate::math::*;
use crate::sim::{Aircraft, Input, G0};
use crate::terrain;

/// Mean arterial pressure at heart level, mmHg.
const MAP_HEART: f64 = 100.0;
/// Hydrostatic drop from heart to eye per g: 1,060 kg/m3 over 0.28 m.
const MMHG_PER_G: f64 = 1060.0 * G0 * 0.28 / 133.322;
/// Eye-level pressures (mmHg) between which the retina, and then the brain,
/// go from fully perfused to not perfused at all.
const RETINA: (f64, f64) = (0.0, 30.0);
const BRAIN: (f64, f64) = (-10.0, 4.0);
/// Seconds of oxygen each holds with no flow, and the time constant of its
/// recovery once the flow is back.
const RETINA_RESERVE: f64 = 3.0;
const RETINA_RECOVERY: f64 = 1.0;
const BRAIN_RESERVE: f64 = 5.0;
const BRAIN_RECOVERY: f64 = 2.0;
/// Protection at eye level, mmHg: an inflated CSU-13B/P suit (about 1.4 G)
/// with COMBAT EDGE pressure breathing at 9 G (about 1.1 G), and a full AGSM
/// (about 3 G).
const SUIT: f64 = 30.0;
const PBG: f64 = 25.0;
const AGSM: f64 = 65.0;
/// Seconds of full straining that exhaust the pilot, the share of the AGSM
/// lost when exhausted, and the seconds of rest that bring it back.
const ENDURANCE: f64 = 26.0;
const FATIGUE_LOSS: f64 = 1.0;
const REST: f64 = 60.0;
/// Baroreflex range (mmHg), and its time constants: sympathetic (raising the
/// pressure) is slow, vagal (lowering it) quick.
const REFLEX_MAX: f64 = 20.0;
const REFLEX_MIN: f64 = -30.0;
const REFLEX_UP: f64 = 6.0;
const REFLEX_DOWN: f64 = 2.0;
/// G-LOC: absolute, then relative incapacitation (s), from the shortest
/// episode, growing with each second of cerebral oxygen debt past a sharp
/// spike's, up to the longest.
const GLOC_ABSOLUTE: (f64, f64, f64) = (3.0, 0.5, 20.0);
const GLOC_RELATIVE: (f64, f64, f64) = (2.0, 0.3, 12.0);
/// Accumulated G stress, the "G dose": seconds weighted by how far past 3 G,
/// or below -1 G (which floods the head and sets up the push-pull effect),
/// forgotten with time constant DOSE_DECAY. DOSE_REF of it is a hard fight's
/// worth, about 20 s at 9 G. Its effect saturates: at a full dose the pilot
/// has lost DOSE_TOLERANCE mmHg at eye level (about 0.8 G, from blood pooled
/// in the legs and spent reserves), rests and refills oxygen DOSE_RECOVERY
/// times slower again, and stays out DOSE_GLOC times longer again after a
/// G-LOC.
const DOSE_DECAY: f64 = 180.0;
const DOSE_REF: f64 = 250.0;
const DOSE_TOLERANCE: f64 = 12.0;
const DOSE_RECOVERY: f64 = 1.5;
const DOSE_GLOC: f64 = 0.8;
/// Brain oxygen below which the pilot drifts into almost-loss of
/// consciousness (A-LOC): the hands slacken and the view dims steadily
/// until the G-LOC itself, instead of all at once.
const ALOC: f64 = 0.45;
/// Auto-GCAS recovery: roll rate (deg/s), pull (g) and the clearance it keeps (m).
const GCAS_ROLL: f64 = 180.0;
const GCAS_PULL: f64 = 5.0;
const GCAS_BUFFER: f64 = 60.0;
/// Stick deflection that commands 5 g through the FLCS pitch gradient (18.4 lbf).
const GCAS_STICK: f64 = 0.46;

/// What the pilot sees: each term 0 (clear) to 1.
#[derive(Clone, Copy, Debug, Default)]
pub struct Vision {
    /// Peripheral light loss closing towards the centre.
    pub tunnel: f64,
    /// Colour and brightness fading out.
    pub grey: f64,
    /// Central light loss.
    pub black: f64,
    /// Negative-g redout.
    pub red: f64,
}

#[derive(Clone, Debug)]
pub struct Pilot {
    /// Baroreflex correction at eye level, mmHg.
    pub reflex: f64,
    /// Suit inflation and AGSM effort, 0..1.
    pub suit: f64,
    pub strain: f64,
    /// AGSM fatigue, 0..1.
    pub fatigue: f64,
    /// Eye-level pressure, mmHg.
    pub eye: f64,
    /// Oxygen left in the retina and the brain, 0..1.
    pub retina: f64,
    pub brain: f64,
    pub red: f64,
    /// Cerebral oxygen debt this episode (s): the time the brain has gone
    /// short of oxygen, weighted by how short.
    pub hypoxia: f64,
    /// Seconds of incapacitation left after a G-LOC, and the length of its
    /// relative (confused) part.
    pub gloc: f64,
    pub relative: f64,
    /// Number of G-LOC episodes this sortie.
    pub glocs: u32,
    /// Auto-GCAS flying the recovery.
    pub gcas: bool,
    /// Seconds into the current AGSM breathing cycle, and the breaths taken:
    /// the L-1 manoeuvre snatches a breath every 3 s.
    pub breath: f64,
    pub breaths: u32,
    /// Accumulated G stress (see DOSE_REF).
    pub dose: f64,
}

impl Default for Pilot {
    fn default() -> Pilot {
        Pilot { reflex: 0.0, suit: 0.0, strain: 0.0, fatigue: 0.0, eye: MAP_HEART - MMHG_PER_G, retina: 1.0, brain: 1.0, red: 0.0, hypoxia: 0.0, gloc: 0.0, relative: GLOC_RELATIVE.0, glocs: 0, gcas: false, breath: 0.0, breaths: 0, dose: 0.0 }
    }
}

/// Perfusion between the pressures where it stops and where it is full.
fn perfusion(p: f64, range: (f64, f64)) -> f64 {
    clamp((p - range.0) / (range.1 - range.0), 0.0, 1.0)
}

/// Oxygen store: drains over `reserve` seconds without flow, refills with
/// time constant `recovery` when it is back.
fn oxygen(o: f64, flow: f64, reserve: f64, recovery: f64, dt: f64) -> f64 {
    clamp(o + (-(1.0 - flow) / reserve + flow * (1.0 - o) / recovery) * dt, 0.0, 1.0)
}

impl Pilot {
    /// Unconscious (absolute incapacitation).
    #[cfg(test)]
    pub fn out(&self) -> bool {
        self.gloc > self.relative
    }

    /// Share of the pilot's control inputs reaching the stick and throttle:
    /// slackening through the A-LOC as the brain runs short, none while
    /// unconscious, coming back through the confusion after it.
    pub fn authority(&self) -> f64 {
        if self.gloc <= 0.0 {
            smoothstep(0.0, ALOC, self.brain)
        } else {
            smoothstep(self.relative, 0.0, self.gloc)
        }
    }

    /// How much of a full G dose the pilot carries, 0..1, saturating.
    pub fn dosed(&self) -> f64 {
        1.0 - (-self.dose / DOSE_REF).exp()
    }

    /// Advances the physiology through one step at normal load factor `nz`.
    pub fn update(&mut self, dt: f64, a: &Aircraft) {
        let nz = a.g_load;
        let pull = |t: f64, tau: f64| clamp(dt / tau, 0.0, 1.0) * t;
        // The suit valve opens at 2 G and fills in under a second; the pilot
        // strains from about 3 G and is fully into the AGSM by 6 G, unless out,
        // and starts with the pull: the g the stick asks for, not the g felt.
        let awake = self.gloc <= 0.0;
        self.suit += pull(smoothstep(1.8, 3.0, nz) - self.suit, 0.6);
        let effort = if awake { smoothstep(3.0, 6.0, nz.max(a.flcs.n_cmd + 1.0)) } else { 0.0 };
        self.strain += pull(effort - self.strain, if effort > self.strain { 0.3 } else { 0.5 });
        // The dose builds with every second hard past 3 G or below -1 G, and
        // fades slowly; the more of it, the slower the pilot recovers.
        let stress = (nz - 3.0).max(0.0).powf(1.5) + 6.0 * (-1.0 - nz).max(0.0).powf(1.2);
        self.dose = (self.dose + (stress - self.dose / DOSE_DECAY) * dt).max(0.0);
        let k = self.dosed();
        let slow = 1.0 + DOSE_RECOVERY * k;
        self.fatigue = clamp(self.fatigue + (self.strain * self.strain / ENDURANCE - (1.0 - self.strain) / (REST * slow)) * dt, 0.0, 1.0);
        let before = self.breath;
        self.breath = if self.strain > 0.5 && awake { self.breath + dt } else { 0.0 };
        if self.breath > 0.0 && (before == 0.0 || (before / 3.0).floor() < (self.breath / 3.0).floor()) {
            self.breaths += 1;
        }

        let column = MMHG_PER_G * nz;
        // The mask pressure rises with the suit's from 4 G.
        let breathing = PBG * self.suit * smoothstep(4.0, 9.0, nz);
        let protection = SUIT * self.suit + breathing + AGSM * self.strain * (1.0 - FATIGUE_LOSS * self.fatigue * self.fatigue);
        let raw = MAP_HEART + protection - column - DOSE_TOLERANCE * k;
        // The carotid sinus reads the head-level pressure and drives it back
        // towards its 1 g value.
        let target = clamp(0.6 * (MAP_HEART - MMHG_PER_G - raw), REFLEX_MIN, REFLEX_MAX);
        self.reflex += pull(target - self.reflex, if target > self.reflex { REFLEX_UP } else { REFLEX_DOWN });
        self.eye = raw + self.reflex;

        self.retina = oxygen(self.retina, perfusion(self.eye, RETINA), RETINA_RESERVE, RETINA_RECOVERY * slow, dt);
        let flow = perfusion(self.eye, BRAIN);
        self.brain = oxygen(self.brain, flow, BRAIN_RESERVE, BRAIN_RECOVERY * slow, dt);
        let red = smoothstep(-1.5, -3.0, nz);
        self.red += pull(red - self.red, if red > self.red { 1.5 } else { 2.0 });

        // The deficit builds while the brain is short of blood and is
        // forgotten once it has refilled.
        if self.brain < 0.999 || self.gloc > 0.0 {
            self.hypoxia += (1.0 - self.brain) * dt;
        } else {
            self.hypoxia = 0.0;
        }
        let (abs, rel) = (GLOC_ABSOLUTE, GLOC_RELATIVE);
        if self.gloc > self.relative {
            // Out cold: the clock only runs with blood reaching the brain, and
            // every second more without it deepens the episode.
            self.gloc += (abs.1 * (1.0 - flow) - flow) * dt;
            self.gloc = self.gloc.min(abs.2 + self.relative);
        } else if self.gloc > 0.0 {
            self.gloc = (self.gloc - dt).max(0.0);
        } else if self.brain <= 0.0 {
            // A straight drain of the reserve leaves half of it as debt.
            // A heavy dose lengthens both parts.
            let excess = (self.hypoxia - 0.5 * BRAIN_RESERVE).max(0.0);
            let longer = 1.0 + DOSE_GLOC * k;
            self.relative = ((rel.0 + rel.1 * excess) * longer).min(rel.2);
            self.gloc = ((abs.0 + abs.1 * excess) * longer).min(abs.2) + self.relative;
            self.glocs += 1;
        }

        // Auto-GCAS only flies while the pilot cannot; it lets go once the
        // jet climbs with the recovery path clear.
        let climbing = a.vel.y > 0.05 * a.speed;
        if self.authority() < 0.6 {
            if gcas_conflict(a, GCAS_BUFFER) {
                self.gcas = true;
            } else if self.gcas && climbing && !gcas_conflict(a, 2.0 * GCAS_BUFFER) {
                self.gcas = false;
            }
        } else {
            self.gcas = false;
        }
    }

    /// The inputs that reach the controls: the pilot's, scaled by what is
    /// left of the pilot, or the Auto-GCAS recovery.
    pub fn controls(&self, a: &Aircraft, input: Input) -> Input {
        let k = self.authority();
        if k >= 1.0 && !self.gcas {
            return input;
        }
        let hands = k > 0.5;
        let mut i = Input {
            pitch_axis: input.pitch() * k,
            roll_axis: input.roll() * k,
            yaw_axis: input.yaw() * k,
            throttle_up: input.throttle_up && hands,
            throttle_down: input.throttle_down && hands,
            afterburner: input.afterburner && hands,
            airbrake: input.airbrake,
            ..Default::default()
        };
        if self.gcas {
            // Roll the short way to wings level, then pull 5 g.
            i.roll_axis = clamp(-a.bank / 25.0, -1.0, 1.0);
            i.pitch_axis = if a.bank.abs() < 60.0 { GCAS_STICK } else { 0.0 };
            i.yaw_axis = 0.0;
        }
        i
    }

    /// What the pilot sees now.
    pub fn vision(&self) -> Vision {
        let o = self.retina;
        let mut v = Vision { tunnel: 1.0 - smoothstep(0.12, 0.96, o), grey: 1.0 - smoothstep(0.35, 0.97, o), black: 1.0 - smoothstep(0.0, 0.45, o), red: self.red };
        // Through the A-LOC the whole view dims and greys as the brain runs
        // short, whatever the retina still gets.
        let fade = 1.0 - smoothstep(0.0, ALOC + 0.15, self.brain);
        v.black = v.black.max(fade);
        v.grey = v.grey.max(fade);
        v.tunnel = v.tunnel.max(fade);
        if self.gloc > 0.0 {
            // Out cold, then the view comes back through a grey haze.
            let back = smoothstep(self.relative, 0.5 * self.relative, self.gloc);
            v.black = v.black.max(1.0 - back);
            v.tunnel = v.tunnel.max(1.0 - back);
            v.grey = v.grey.max(0.6 * smoothstep(0.0, self.relative, self.gloc));
        }
        v
    }
}

/// Auto-GCAS prediction: the path of a recovery started now (half a second
/// to react, a 180 deg/s roll to wings level, then a 5 g pull until the jet
/// climbs) checked against the terrain under it, `buffer` metres of clearance.
fn gcas_conflict(a: &Aircraft, buffer: f64) -> bool {
    let speed = a.speed.max(60.0);
    let flat = v3(a.vel.x, 0.0, a.vel.z);
    let fwd = a.q.axes().forward;
    let dir = if flat.len() > 1.0 { flat.norm() } else { v3(fwd.x, 0.0, fwd.z).norm() };
    let mut gamma = clamp(a.vel.y / speed, -1.0, 1.0).asin();
    let mut p = a.pos;
    let roll = 0.5 + a.bank.abs() / GCAS_ROLL;
    let h = 0.2;
    let mut t = 0.0;
    while t < 12.0 {
        if t >= roll {
            gamma += G0 * (GCAS_PULL - gamma.cos()) / speed * h;
            if gamma > 5.0 * DEG {
                return false;
            }
        }
        p += (dir * gamma.cos() + V3::Y * gamma.sin()) * (speed * h);
        t += h;
        if p.y < terrain::floor(p.x, p.z) + buffer {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Mission;
    const DT: f64 = 1.0 / 120.0;

    /// Load-factor history through a relaxed pilot (no suit, no AGSM) or a
    /// trained one, returning the pilot and the worst vision seen.
    fn ride(p: &mut Pilot, a: &mut Aircraft, seconds: f64, g: impl Fn(f64) -> f64, relaxed: bool) -> f64 {
        let mut worst: f64 = 1.0;
        for k in 0..(seconds / DT) as usize {
            a.g_load = g(k as f64 * DT);
            p.update(DT, a);
            if relaxed {
                p.suit = 0.0;
                p.strain = 0.0;
            }
            worst = worst.min(p.retina);
        }
        worst
    }

    fn jet() -> Aircraft {
        Aircraft::new(&Mission::new())
    }

    /// Threshold g at a slow onset (0.1 G/s): first light loss, blackout and G-LOC.
    fn relaxed_thresholds(rate: f64) -> (f64, f64, f64) {
        let mut p = Pilot::default();
        let mut a = jet();
        let (mut pll, mut black, mut gloc) = (0.0, 0.0, 0.0);
        let mut t = 0.0;
        while t < 120.0 && gloc == 0.0 {
            a.g_load = 1.0 + rate * t;
            p.update(DT, &a);
            p.suit = 0.0;
            p.strain = 0.0;
            if pll == 0.0 && p.vision().tunnel > 0.05 {
                pll = a.g_load;
            }
            if black == 0.0 && p.vision().black > 0.9 {
                black = a.g_load;
            }
            if p.gloc > 0.0 {
                gloc = a.g_load;
            }
            t += DT;
        }
        (pll, black, gloc)
    }

    #[test]
    fn relaxed_tolerance_matches_centrifuge_data() {
        let (pll, black, gloc) = relaxed_thresholds(0.1);
        let pll_gradual = pll;
        println!("gradual onset: light loss {pll:.2} G, blackout {black:.2} G, G-LOC {gloc:.2} G");
        assert!(pll > 4.0 && pll < 5.2, "light loss {pll:.2}");
        assert!(black > pll + 0.5 && black < 6.2, "blackout {black:.2}");
        assert!(gloc > black + 0.4 && gloc < 7.3, "G-LOC {gloc:.2}");
        // Rapid onset: 1 G/s to a plateau held for 15 s, the centrifuge protocol.
        let plateau = (0..16)
            .map(|k| 3.0 + 0.2 * k as f64)
            .find(|&g| {
                let mut p = Pilot::default();
                ride(&mut p, &mut jet(), 15.0 + g, |t| (1.0 + t).min(g), true) < 0.85
            })
            .unwrap_or(9.0);
        println!("rapid onset plateau: light loss {plateau:.1} G");
        assert!(plateau > 3.3 && plateau < pll_gradual - 0.4, "rapid light loss {plateau:.1}");
    }

    #[test]
    fn trained_pilot_holds_a_nine_g_break_for_fifteen_seconds() {
        let mut p = Pilot::default();
        let mut a = jet();
        let worst = ride(&mut p, &mut a, 16.5, |t| 1.0 + 8.0 * (t / 1.5).min(1.0), false);
        println!("9 G break: worst vision {worst:.2}, now {:.2}, fatigue {:.2}", p.retina, p.fatigue);
        assert!(p.gloc == 0.0 && worst > 0.4, "9 G for 15 s: vision {worst:.2}, G-LOC {}", p.gloc);
        assert!(worst < 0.95, "9 G should cost some light: {worst:.2}");
        // Held long enough, the straining tires and the pilot goes out.
        ride(&mut p, &mut a, 60.0, |_| 9.0, false);
        assert!(p.glocs >= 1, "a minute more at 9 G");
    }


    /// Seconds at 9 G to a G-LOC, then seconds out cold at 1 G, after a
    /// warm-up history `before` (load factor against time) of `lead` seconds.
    fn nine_g_after(before: impl Fn(f64) -> f64, lead: f64) -> (f64, f64, f64) {
        let mut p = Pilot::default();
        let mut a = jet();
        let mut t = 0.0;
        while t < lead {
            let g = before(t);
            ride(&mut p, &mut a, DT, |_| g, false);
            t += DT;
        }
        let dose = p.dosed();
        let mut to = 0.0;
        while p.glocs == 0 && to < 60.0 {
            ride(&mut p, &mut a, DT, |_| 9.0, false);
            to += DT;
        }
        let mut out = 0.0;
        while p.gloc > 0.0 {
            ride(&mut p, &mut a, DT, |_| 1.0, false);
            out += DT;
        }
        (to, out, dose)
    }

    #[test]
    fn g_stress_builds_up_and_brings_the_next_g_loc_sooner() {
        let (fresh, fresh_out, _) = nine_g_after(|_| 1.0, 0.0);
        // A 15 s 9 G turn, then half a minute's rest at 1 G.
        let (tired, tired_out, dose) = nine_g_after(|t| if t < 15.0 { 9.0 } else { 1.0 }, 45.0);
        // Pushing at -2.5 G for 10 s, then a few seconds level.
        let (pushed, _, push_dose) = nine_g_after(|t| if t < 10.0 { -2.5 } else { 1.0 }, 13.0);
        println!("9 G to G-LOC: fresh {fresh:.1} s, after a turn {tired:.1} s (dose {dose:.2}), after a push {pushed:.1} s (dose {push_dose:.2}); out {fresh_out:.1} s fresh, {tired_out:.1} s tired");
        assert!(fresh > 20.0 && fresh < 30.0, "fresh {fresh:.1} s");
        assert!(tired < fresh - 4.0, "after a turn {tired:.1} s");
        assert!(pushed < fresh && push_dose > 0.2, "after a push {pushed:.1} s");
        assert!(tired_out > fresh_out + 1.0, "out {tired_out:.1} s tired, {fresh_out:.1} s fresh");
        // Rested long enough, the dose fades.
        let (rested, _, rest_dose) = nine_g_after(|t| if t < 15.0 { 9.0 } else { 1.0 }, 615.0);
        assert!(rest_dose < 0.1 && rested > fresh - 1.5, "after ten minutes {rested:.1} s");
    }

    #[test]
    fn the_g_loc_comes_on_through_an_a_loc() {
        let mut p = Pilot::default();
        let mut a = jet();
        let (mut slack, mut t) = (0.0, 0.0);
        while p.glocs == 0 && t < 60.0 {
            ride(&mut p, &mut a, DT, |_| 9.0, false);
            let k = p.authority();
            if k > 0.05 && k < 0.95 {
                slack += DT;
            }
            t += DT;
        }
        assert!(slack > 1.5, "the hands went slack over only {slack:.1} s");
        assert!(p.vision().black > 0.99 && p.authority() < 0.01);
    }
    #[test]
    fn a_held_nine_g_closes_the_view_gradually_before_the_g_loc() {
        let mut p = Pilot::default();
        let mut a = jet();
        let (mut t, mut narrowing, mut blind) = (0.0, 0.0, None);
        while p.glocs == 0 && t < 60.0 {
            ride(&mut p, &mut a, DT, |_| 1.0 + 8.0 * (t / 1.5_f64).min(1.0), false);
            t += DT;
            let v = p.vision();
            if t < 10.0 {
                assert!(v.tunnel < 0.01, "light loss after {t:.1} s");
            }
            if v.tunnel > 0.05 && v.black < 0.5 {
                narrowing += DT;
            }
            if v.black > 0.99 && blind.is_none() {
                blind = Some(t);
            }
        }
        println!("9 G held: view closing for {narrowing:.1} s, black at {blind:?} s, G-LOC at {t:.1} s");
        assert!(narrowing > 3.0, "the tunnel closed in {narrowing:.1} s");
        assert!(t > 18.0 && t < 30.0, "G-LOC after {t:.1} s at 9 G");
    }

    #[test]
    fn a_relaxed_pilot_goes_out_at_seven_g() {
        let mut p = Pilot::default();
        let mut a = jet();
        let mut t = 0.0;
        while p.glocs == 0 && t < 10.0 {
            ride(&mut p, &mut a, DT, |_| 7.0, true);
            t += DT;
        }
        assert!(p.glocs == 1 && p.out() && t > 3.0, "out after {t:.1} s at 7 G");
        assert!(p.vision().black > 0.99 && p.authority() == 0.0);
        // Relieved at once, a sharp spike costs only a few seconds out cold,
        // then the controls come back through the confusion.
        let mut out = 0.0;
        while p.out() {
            ride(&mut p, &mut a, DT, |_| 1.0, true);
            out += DT;
        }
        assert!(out > 2.0 && out < 6.0 && p.authority() < 0.1, "out for {out:.1} s");
        assert!(p.controls(&a, Input { pitch_up: true, ..Default::default() }).pitch().abs() < 0.1);
        let confused = p.relative + 0.1;
        ride(&mut p, &mut a, confused, |_| 1.0, true);
        assert!(p.authority() == 1.0 && p.vision().black < 0.01);
    }

    /// Time out cold after a G-LOC, with the load held for `after` seconds.
    fn episode(onset: impl Fn(f64) -> f64, after: f64) -> f64 {
        let mut p = Pilot::default();
        let mut a = jet();
        let mut t = 0.0;
        while p.glocs == 0 && t < 120.0 {
            let g = onset(t);
            ride(&mut p, &mut a, DT, |_| g, true);
            t += DT;
        }
        let held = onset(t);
        ride(&mut p, &mut a, after, |_| held, true);
        let mut out = after;
        while p.out() {
            ride(&mut p, &mut a, DT, |_| 1.0, true);
            out += DT;
        }
        out
    }

    #[test]
    fn the_time_under_g_decides_the_g_loc() {
        // Let go before the brain's reserve runs out and nothing happens.
        let mut p = Pilot::default();
        ride(&mut p, &mut jet(), 3.5, |_| 7.5, true);
        ride(&mut p, &mut jet(), 10.0, |_| 1.0, true);
        assert!(p.glocs == 0, "3.5 s at 7.5 G");
        // Past it, every second more under g keeps the pilot out longer.
        let out: Vec<f64> = [0.0, 3.0, 6.0, 30.0].iter().map(|&s| episode(|_| 7.5, s)).collect();
        println!("out cold after 0, 3, 6 and 30 s more at 7.5 G: {out:.1?} s");
        assert!(out[0] > 2.5 && out[0] < 4.0, "a spike: {:.1} s", out[0]);
        assert!(out[1] > out[0] + 3.0 && out[2] > out[1] + 3.0);
        assert!(out[3] <= 30.0 + GLOC_ABSOLUTE.2, "capped: {:.1} s", out[3]);
    }

    #[test]
    fn a_push_lowers_tolerance_to_the_next_pull() {
        let pull = |push: bool| {
            let mut p = Pilot::default();
            let mut a = jet();
            if push {
                ride(&mut p, &mut a, 3.0, |_| -2.0, false);
            }
            ride(&mut p, &mut a, 4.0, |t| 1.0 + 7.5 * (t / 1.0).min(1.0), false)
        };
        let (plain, after_push) = (pull(false), pull(true));
        assert!(after_push < plain - 0.2, "vision {plain:.2} plain, {after_push:.2} after -2 G");
    }

    #[test]
    fn negative_g_reddens_the_view() {
        let mut p = Pilot::default();
        let mut a = jet();
        ride(&mut p, &mut a, 3.0, |_| -1.0, false);
        assert!(p.vision().red < 0.05);
        ride(&mut p, &mut a, 3.0, |_| -3.0, false);
        assert!(p.vision().red > 0.8 && p.vision().black < 0.01);
    }

    #[test]
    fn hands_off_while_out_and_auto_gcas_pulls_clear() {
        let mut p = Pilot { gloc: 10.0, ..Default::default() };
        let mut a = jet();
        let input = Input { pitch_down: true, throttle_up: true, ..Default::default() };
        let i = p.controls(&a, input);
        assert!(i.pitch() == 0.0 && !i.throttle_up);
        // Diving at the ground, the recovery engages, rolls level and pulls.
        let floor = terrain::floor(a.pos.x, a.pos.z);
        a.pos.y = floor + 500.0;
        let f = a.q.axes().forward;
        a.vel = (v3(f.x, 0.0, f.z).norm() * 0.6 + v3(0.0, -0.8, 0.0)).norm() * 220.0;
        a.update(0.0, &Input::default());
        p.update(DT, &a);
        assert!(p.gcas, "a 50 degree dive at 500 m");
        assert!(p.controls(&a, input).pitch() > 0.4);
        // Level and high above the ground nothing happens.
        let mut q = Pilot { gloc: 10.0, ..Default::default() };
        let b = jet();
        q.update(DT, &b);
        assert!(!q.gcas);
    }
}
