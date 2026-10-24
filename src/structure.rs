//! Aeroelasticity and structural limits of the wing. Static: the 40-degree
//! swept wing bends up under load, and sweep turns that bending into
//! wash-out, so the outboard (aft) panels shed lift as the dynamic pressure
//! climbs: the lift slope falls and the centre of lift creeps forward. The
//! flaperons twist the wing against themselves and lose power, but half the
//! roll power is the differential stabilator, so roll never reverses. Torsion
//! alone would twist the wing nose-up until it diverges; the bending relief
//! of the sweep keeps that far beyond the flutter boundary. Dynamic: the
//! antisymmetric bending-torsion mode of the wing with its tip missile (about
//! 8 Hz) loses damping as the equivalent airspeed approaches the flutter
//! speed and grows past it, into a limit-cycle oscillation that breaks the
//! wing within seconds; turbulence and buffet excite it. Load factor beyond
//! the limit damages the structure, and at 1.5 times the limit breaks it.
//!
//! Speeds follow the clean F-16C placard and the MIL-A-8870 margins: limit
//! 800 KCAS (KEAS at sea level), flutter 15 % beyond it in equivalent
//! airspeed (920 KEAS) with a transonic dip of 14 % between Mach 0.9 and 1.1
//! and lower with heavy stores under the wings, flaperon reversal near
//! 1,100 KEAS and torsional divergence near 1,280 KEAS. The real boundaries
//! are not public; these are consistent estimates.
use crate::aero::curve;
use crate::air::{KT, RHO0};
use crate::math::*;

/// Placard and the aeroelastic speeds, KEAS.
pub const VL_KEAS: f64 = 800.0;
pub const VF_KEAS: f64 = 920.0;
pub const VR_KEAS: f64 = 1100.0;
pub const VD_KEAS: f64 = 1280.0;
/// Flutter speed against Mach: the transonic dip, then the stiffer
/// supersonic flow.
const DIP: [(f64, f64); 6] = [(0.0, 1.0), (0.8, 1.0), (0.95, 0.86), (1.08, 0.88), (1.3, 1.0), (2.0, 1.12)];
/// Underwing stores (kg) at which the flutter speed has dropped by STORE_LOSS.
const STORE_MASS: f64 = 1500.0;
const STORE_LOSS: f64 = 0.28;
/// Wing wash-out from bending: lift lost per dynamic pressure of the placard,
/// and the nose-up twist of torsion alone (share of lift at the divergence
/// pressure's x / (1 - x)).
const WASHOUT: f64 = 0.112;
const TORSION: f64 = 0.05;
/// Chordwise shift of the lost (outboard, aft) lift, fraction of MAC.
const LIFT_SHIFT: f64 = 0.15;
/// Wing sweep at quarter chord.
const SWEEP: f64 = 40.0 * DEG;
/// Antisymmetric wing mode: frequency, structural plus aerodynamic damping
/// well below the flutter speed, the amplitude-dependent aerodynamic damping
/// that caps the limit cycle, the rolling moment per metre of tip deflection.
const MODE_HZ: f64 = 8.0;
const ZETA: f64 = 0.03;
const ZETA_LCO: f64 = 1.0;
const ROLL_PER_M: f64 = 0.08;
/// Tip deflection forced by buffet and turbulence (m, static equivalent).
const EXCITE: f64 = 0.0015;
/// Oscillation the structure takes without damage, and the scale and amplitude
/// at which it fails (m of tip deflection).
const SAFE_AMPLITUDE: f64 = 0.06;
const DAMAGE_SCALE: f64 = 0.10;
const BREAK_AMPLITUDE: f64 = 0.30;
/// Tip deflection per g of wing load (m): about 14 cm at 9 g, 3 % of the
/// semispan, enough to see in a hard pull without looking rubbery.
const BEND_PER_G: f64 = 0.016;
/// Symmetric first bending mode: frequency and damping. The wing takes a
/// load change a beat late and swings a little past it.
const BEND_HZ: f64 = 3.5;
const BEND_ZETA: f64 = 0.4;
/// Ultimate load: 1.5 times the +9 / -3 g limits.
pub const N_LIMIT: (f64, f64) = (-3.0, 9.0);
const ULTIMATE: f64 = 1.5;

/// Dynamic pressure of an equivalent airspeed in knots, Pa.
pub fn q_of_keas(keas: f64) -> f64 {
    0.5 * RHO0 * (keas / KT).powi(2)
}

/// Compressibility raises the lift slope and so the twist per unit
/// dynamic pressure: the divergence pressure scales with the Prandtl-Glauert
/// factor of the Mach number normal to the sweep.
fn compressibility(mach: f64) -> f64 {
    clamp((1.0 - (mach * SWEEP.cos()).powi(2)).abs().sqrt(), 0.75, 1.0)
}

/// Wing torsional divergence speed at this Mach number, KEAS.
pub fn divergence_keas(mach: f64) -> f64 {
    VD_KEAS * compressibility(mach).sqrt()
}

/// Flutter speed for a Mach number and the stores hung under the wings, KEAS.
pub fn flutter_keas(mach: f64, wing_stores: f64) -> f64 {
    VF_KEAS * curve(&DIP, mach) * (1.0 - STORE_LOSS * smoothstep(0.0, STORE_MASS, wing_stores))
}

/// Static aeroelastic effectiveness at dynamic pressure `q` (Pa): the share of
/// the rigid wing's lift, the forward shift of the centre of lift (fraction of
/// MAC) and the share of the rigid roll power from the flaperons and
/// differential stabilator. A rigid jet passes q = 0.
pub fn effectiveness(q: f64, mach: f64) -> (f64, f64, f64) {
    if q <= 0.0 {
        return (1.0, 0.0, 1.0);
    }
    let x = (q / (q_of_keas(VD_KEAS) * compressibility(mach))).min(0.98);
    let lift = 1.0 + TORSION * x / (1.0 - x) - WASHOUT * q / q_of_keas(VL_KEAS);
    let flaperon = 1.0 - q / q_of_keas(VR_KEAS);
    (lift, LIFT_SHIFT * (1.0 - lift), 0.5 + 0.5 * flaperon)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Structure {
    /// Antisymmetric wing mode: left wingtip deflection (m, up positive; the
    /// right tip mirrors it) and its rate.
    pub flutter: f64,
    pub rate: f64,
    /// Amplitude of the oscillation, m.
    pub amplitude: f64,
    /// Tip deflection under the wing load (m, up positive) and its rate.
    pub bend: f64,
    bend_rate: f64,
    /// Equivalent airspeed over the present flutter speed.
    pub margin: f64,
    /// Flutter speed now, KEAS.
    pub vf: f64,
    /// Accumulated structural damage, 0..1, and what broke.
    pub damage: f64,
    pub failed: Option<&'static str>,
    clock: f64,
}

impl Structure {
    /// One step of the wing mode and the load checks.
    pub fn update(&mut self, dt: f64, keas: f64, mach: f64, nz: f64, excitation: f64, wing_stores: f64) {
        if dt <= 0.0 || self.failed.is_some() {
            return;
        }
        self.clock += dt;
        self.vf = flutter_keas(mach, wing_stores);
        self.margin = keas / self.vf;
        let wb = TAU * BEND_HZ;
        self.bend_rate += (wb * wb * (BEND_PER_G * nz - self.bend) - 2.0 * BEND_ZETA * wb * self.bend_rate) * dt;
        self.bend += self.bend_rate * dt;

        // Damping falls steeply towards the flutter speed and turns negative
        // past it; the oscillation's own aerodynamic damping caps the cycle.
        let wn = TAU * MODE_HZ;
        let zeta = ZETA * (1.0 - self.margin.powi(8)) + ZETA_LCO * self.amplitude * self.amplitude;
        let t = self.clock;
        let noise = 0.45 * (t * 31.0).sin() + 0.25 * (t * 63.0 + 1.3).sin() + 0.2 * (t * 77.0 + 0.4).sin() + 0.1 * (t * 50.0 + 2.1).sin();
        let force = EXCITE * excitation * noise;
        let accel = wn * wn * (force - self.flutter) - 2.0 * zeta * wn * self.rate;
        self.rate += accel * dt;
        self.flutter += self.rate * dt;
        self.amplitude = self.flutter.hypot(self.rate / wn);

        let over = ((self.amplitude - SAFE_AMPLITUDE) / DAMAGE_SCALE).max(0.0);
        let (neg, pos) = N_LIMIT;
        let overload = ((nz - pos - 0.5).max(0.0) + (neg - 0.5 - nz).max(0.0)) / (pos * (ULTIMATE - 1.0));
        self.damage = (self.damage + (over * over + 2.0 * overload) * dt).min(1.0);
        self.failed = if keas > divergence_keas(mach) {
            Some("wing divergence")
        } else if nz > pos * ULTIMATE || nz < neg * ULTIMATE {
            Some("overstress")
        } else if self.amplitude > BREAK_AMPLITUDE || self.damage >= 1.0 {
            Some(if self.amplitude > 0.5 * SAFE_AMPLITUDE { "wing flutter" } else { "overstress" })
        } else {
            None
        };
    }

    /// Rolling-moment coefficient of the antisymmetric deflection: the
    /// twisting tips lift unevenly.
    pub fn roll(&self) -> f64 {
        ROLL_PER_M * self.flutter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f64 = 1.0 / 480.0;

    fn fly(s: &mut Structure, seconds: f64, keas: f64, mach: f64, excitation: f64) -> f64 {
        let mut peak: f64 = 0.0;
        for _ in 0..(seconds / DT) as usize {
            s.update(DT, keas, mach, 1.0, excitation, 0.0);
            peak = peak.max(s.amplitude);
        }
        peak
    }

    #[test]
    fn swept_wing_sheds_lift_and_roll_power_with_dynamic_pressure() {
        let (rigid, _, ail0) = effectiveness(0.0, 0.5);
        let (low, _, _) = effectiveness(q_of_keas(350.0), 0.6);
        let (limit, shift, ail) = effectiveness(q_of_keas(VL_KEAS), 1.2);
        assert!(rigid == 1.0 && ail0 == 1.0);
        assert!(low < 1.0 && low > 0.97, "350 KEAS: {low:.3}");
        assert!(limit > 0.85 && limit < 0.95 && shift > 0.0, "800 KEAS: {limit:.3}");
        assert!(ail > 0.5 && ail < 0.85, "roll power at the placard {ail:.2}");
        // Divergence lies beyond flutter at every Mach number.
        for mach in [0.3, 0.9, 1.0, 1.2, 2.0] {
            assert!(divergence_keas(mach) > flutter_keas(mach, 0.0) * 1.08, "Mach {mach}");
        }
    }

    #[test]
    fn transonic_dip_and_stores_lower_the_flutter_speed() {
        assert!((flutter_keas(0.5, 0.0) - 920.0).abs() < 1e-9);
        assert!(flutter_keas(0.97, 0.0) < 820.0);
        assert!(flutter_keas(0.5, 1500.0) < 700.0);
    }

    /// A sudden 9 g pull bends the tips up a beat late, past the steady
    /// 36 cm, and they settle there.
    #[test]
    fn the_wing_bends_late_and_settles_under_load() {
        let mut s = Structure::default();
        for _ in 0..(1.0 / DT) as usize {
            s.update(DT, 400.0, 0.6, 1.0, 0.0, 0.0);
        }
        assert!((s.bend - BEND_PER_G).abs() < 0.002, "1 g: {:.3} m", s.bend);
        let mut peak: f64 = 0.0;
        let mut at_25ms = 0.0;
        for i in 0..(2.0 / DT) as usize {
            s.update(DT, 400.0, 0.6, 9.0, 0.0, 0.0);
            peak = peak.max(s.bend);
            if i == (0.025 / DT) as usize {
                at_25ms = s.bend;
            }
        }
        assert!(at_25ms < 0.06, "lags the pull: {at_25ms:.3} m after 25 ms");
        assert!(peak > 1.03 * 9.0 * BEND_PER_G && peak < 1.28 * 9.0 * BEND_PER_G, "swings past: peak {peak:.3} m");
        assert!((s.bend - 9.0 * BEND_PER_G).abs() < 0.005, "settles: {:.3} m", s.bend);
    }

    #[test]
    fn buffet_decays_below_flutter_and_grows_past_it() {
        let mut s = Structure::default();
        let calm = fly(&mut s, 4.0, 600.0, 0.9, 1.0);
        assert!(calm < 0.02 && s.failed.is_none(), "buffet at 600 KEAS: {calm:.3} m");
        let near = fly(&mut Structure::default(), 4.0, 0.99 * flutter_keas(1.3, 0.0), 1.3, 1.0);
        assert!(near > calm * 2.0, "lightly damped near the boundary: {near:.3}");
        // Past it, a small disturbance grows into the limit cycle and breaks the wing.
        let mut s = Structure { flutter: 0.002, ..Default::default() };
        let mut t = 0.0;
        while s.failed.is_none() && t < 20.0 {
            s.update(DT, 1.1 * VF_KEAS, 1.4, 1.0, 0.0, 0.0);
            t += DT;
        }
        assert!(s.failed == Some("wing flutter") && t > 1.0 && t < 12.0, "failed after {t:.1} s: {:?}", s.failed);
    }

    #[test]
    fn ultimate_load_and_divergence_break_the_wing() {
        let mut s = Structure::default();
        s.update(DT, 500.0, 0.8, 14.0, 0.0, 0.0);
        assert_eq!(s.failed, Some("overstress"));
        let mut s = Structure::default();
        s.update(DT, 1300.0, 1.5, 1.0, 0.0, 0.0);
        assert_eq!(s.failed, Some("wing divergence"));
        let mut s = Structure::default();
        for _ in 0..480 {
            s.update(DT, 500.0, 0.8, 9.0, 0.0, 0.0);
        }
        assert!(s.failed.is_none() && s.damage == 0.0 && s.bend > 0.06);
    }
}
