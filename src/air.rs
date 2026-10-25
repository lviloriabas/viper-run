//! Air data: the ICAO standard atmosphere on a non-standard day (ISA + dT over
//! a sea-level QNH), the altitudes an air-data computer derives from static
//! pressure and density, pitot-static airspeeds, and the readings the HUD and
//! the panel gauges show. The flight model breathes the same air.
use crate::aero::curve;
use crate::math::*;
use crate::sim::{Aircraft, G0};

pub const R_AIR: f64 = 287.05287;
pub const GAMMA: f64 = 1.4;
pub const P0: f64 = 101325.0;
pub const T0: f64 = 288.15;
pub const RHO0: f64 = 1.225;
const LAPSE: f64 = 0.0065;
const T11: f64 = 216.65;
const P11: f64 = 22632.06;
const P20: f64 = 5474.889;
const LAPSE20: f64 = 0.001;
/// Pressure-law exponents g0 / (R L) of the 0-11 km and 20-32 km layers.
const N_TROP: f64 = G0 / (R_AIR * LAPSE);
const N_STRAT: f64 = G0 / (R_AIR * LAPSE20);
/// Scale height of the isothermal layer from 11 to 20 km.
const H_ISO: f64 = R_AIR * T11 / G0;
/// Earth radius used by the ISA for geopotential altitude.
const EARTH_RADIUS: f64 = 6356766.0;

pub const FT: f64 = 3.28084;
pub const KT: f64 = 1.943844;
pub const PA_PER_INHG: f64 = 3386.389;
/// Published F-16 limits: 800 KCAS or Mach 2.05, whichever comes first.
pub const VMAX_KCAS: f64 = 800.0;
pub const MACH_MAX: f64 = 2.05;
/// Static-source position error of the nose-boom probe, as a fraction of
/// the static pressure against Mach: the bow shock sweeping back over the
/// static ports through Mach 1 makes the altimeter jump (the "Mach jump").
const STATIC_ERROR: [(f64, f64); 6] = [(0.0, 0.0), (0.85, 0.0), (0.96, -0.006), (1.0, 0.003), (1.06, 0.001), (1.2, 0.0)];

/// Day conditions: sea-level pressure (QNH) and a uniform deviation from the
/// ISA temperature, the "ISA + dT" model of performance manuals.
#[derive(Clone, Copy, Debug)]
pub struct Weather {
    pub qnh: f64,
    pub isa_dev: f64,
}

/// Cold River: a cool autumn day, 10 C below standard under a 1018 hPa high.
pub const DAY: Weather = Weather { qnh: 101800.0, isa_dev: -10.0 };

#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// Static (outside) air temperature, K.
    pub temp: f64,
    /// Static pressure, Pa.
    pub pressure: f64,
    /// Density, kg/m3.
    pub rho: f64,
    /// Speed of sound, m/s.
    pub sound: f64,
    /// Standard-atmosphere altitude of the static pressure, m.
    pub pressure_alt: f64,
    /// Standard-atmosphere altitude of the density, m.
    pub density_alt: f64,
}

fn isa_temp(h: f64) -> f64 {
    if h <= 11000.0 {
        T0 - LAPSE * h
    } else if h <= 20000.0 {
        T11
    } else {
        T11 + LAPSE20 * (h - 20000.0)
    }
}

fn isa_pressure(h: f64) -> f64 {
    if h <= 11000.0 {
        P0 * (isa_temp(h) / T0).powf(N_TROP)
    } else if h <= 20000.0 {
        P11 * (-(h - 11000.0) / H_ISO).exp()
    } else {
        P20 * (isa_temp(h) / T11).powf(-N_STRAT)
    }
}

/// Standard-atmosphere altitude at which the pressure is `p`: what an
/// altimeter set to 1013.25 hPa reads.
pub fn pressure_altitude(p: f64) -> f64 {
    if p >= P11 {
        T0 / LAPSE * (1.0 - (p / P0).powf(1.0 / N_TROP))
    } else if p >= P20 {
        11000.0 + H_ISO * (P11 / p).ln()
    } else {
        20000.0 + T11 / LAPSE20 * ((p / P20).powf(-1.0 / N_STRAT) - 1.0)
    }
}

/// Standard-atmosphere altitude at which the density is `rho`.
pub fn density_altitude(rho: f64) -> f64 {
    let rho11 = P11 / (R_AIR * T11);
    let rho20 = P20 / (R_AIR * T11);
    if rho >= rho11 {
        T0 / LAPSE * (1.0 - (rho / RHO0).powf(1.0 / (N_TROP - 1.0)))
    } else if rho >= rho20 {
        11000.0 + H_ISO * (rho11 / rho).ln()
    } else {
        20000.0 + T11 / LAPSE20 * ((rho / rho20).powf(-1.0 / (N_STRAT + 1.0)) - 1.0)
    }
}

/// Integral of 1 / T_isa from 0 to `h`. With it the hydrostatic equation gives
/// the true thickness between two pressure levels on a non-standard day.
fn inv_temp_integral(h: f64) -> f64 {
    let f11 = (T0 / T11).ln() / LAPSE;
    if h <= 11000.0 {
        (T0 / isa_temp(h)).ln() / LAPSE
    } else if h <= 20000.0 {
        f11 + (h - 11000.0) / T11
    } else {
        f11 + 9000.0 / T11 + (isa_temp(h) / T11).ln() / LAPSE20
    }
}

impl Weather {
    #[allow(dead_code)]
    pub const STANDARD: Weather = Weather { qnh: P0, isa_dev: 0.0 };

    /// Static air at a geometric altitude above mean sea level. The pressure
    /// levels are the standard ones anchored at QNH on the sea surface, each
    /// layer stretched or squeezed by its temperature (dh = T / T_isa dHp),
    /// and the temperature on each level is ISA + dT.
    pub fn at(&self, altitude_m: f64) -> Atmosphere {
        let z = clamp(altitude_m, -500.0, 30000.0);
        let h = EARTH_RADIUS * z / (EARTH_RADIUS + z);
        let hp0 = pressure_altitude(self.qnh);
        let base = inv_temp_integral(hp0);
        let mut hp = h + hp0;
        for _ in 0..4 {
            let height = hp - hp0 + self.isa_dev * (inv_temp_integral(hp) - base);
            hp -= (height - h) / (1.0 + self.isa_dev / isa_temp(hp));
        }
        let temp = isa_temp(hp) + self.isa_dev;
        let pressure = isa_pressure(hp);
        let rho = pressure / (R_AIR * temp);
        Atmosphere { temp, pressure, rho, sound: (GAMMA * R_AIR * temp).sqrt(), pressure_alt: hp, density_alt: density_altitude(rho) }
    }

    /// Barometric altimeter reading with the subscale set to this day's QNH.
    pub fn altimeter(&self, atm: &Atmosphere) -> f64 {
        atm.pressure_alt - pressure_altitude(self.qnh)
    }
}

/// Standard-day air, for reference figures quoted at ISA (validation tests).
#[allow(dead_code)]
pub fn isa(altitude_m: f64) -> Atmosphere {
    Weather::STANDARD.at(altitude_m)
}

/// The air the jet flies in today.
pub fn atmosphere_at(altitude_m: f64) -> Atmosphere {
    DAY.at(altitude_m)
}

/// Compressible pitot impact pressure over static pressure, with a normal
/// shock ahead of the probe above Mach 1 (Rayleigh pitot formula).
fn impact_ratio(mach: f64) -> f64 {
    if mach <= 1.0 {
        (1.0 + 0.2 * mach * mach).powf(3.5) - 1.0
    } else {
        (1.2 * mach * mach).powf(3.5) * (2.4 / (2.8 * mach * mach - 0.4)).powf(2.5) - 1.0
    }
}

fn mach_from_impact_ratio(ratio: f64) -> f64 {
    if ratio <= impact_ratio(1.0) {
        return (5.0 * ((ratio.max(0.0) + 1.0).powf(1.0 / 3.5) - 1.0)).sqrt();
    }
    let (mut low, mut high) = (1.0, 5.0);
    for _ in 0..40 {
        let mid = (low + high) / 2.0;
        if impact_ratio(mid) < ratio {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

/// Calibrated airspeed (m/s) of a Mach number at a static pressure: the speed
/// that gives the same pitot impact pressure at sea-level standard.
pub fn cas_for_mach(mach: f64, pressure: f64) -> f64 {
    (GAMMA * R_AIR * T0).sqrt() * mach_from_impact_ratio(pressure * impact_ratio(mach) / P0)
}

/// Mach number of a calibrated airspeed (m/s) at a static pressure.
pub fn mach_for_cas(cas: f64, pressure: f64) -> f64 {
    mach_from_impact_ratio(P0 * impact_ratio(cas / (GAMMA * R_AIR * T0).sqrt()) / pressure)
}

pub fn calibrated_airspeed(speed: f64, atm: &Atmosphere) -> f64 {
    cas_for_mach(speed / atm.sound, atm.pressure)
}

/// What the air-data computer, radar altimeter and accelerometer hand the
/// displays, in cockpit units.
#[derive(Clone, Copy, Debug)]
pub struct AirData {
    pub atm: Atmosphere,
    /// Barometric altitude, altimeter set to QNH.
    pub baro_ft: f64,
    pub pressure_alt_ft: f64,
    pub density_alt_ft: f64,
    pub radar_ft: f64,
    pub qnh_inhg: f64,
    pub oat_c: f64,
    /// Total (stagnation) temperature at the probe.
    pub tat_c: f64,
    pub mach: f64,
    pub kcas: f64,
    pub keas: f64,
    pub ktas: f64,
    pub gs_kt: f64,
    pub vs_fpm: f64,
    /// Dynamic pressure, Pa.
    pub qbar: f64,
    /// Density relative to sea-level standard.
    pub sigma: f64,
    /// Maximum allowable airspeed at this pressure level, and its Mach number.
    pub vmax_kcas: f64,
    pub vmax_mach: f64,
    pub g: f64,
    pub g_max: f64,
    pub g_min: f64,
}

impl AirData {
    pub fn of(a: &Aircraft) -> AirData {
        let atm = atmosphere_at(a.pos.y);
        let tas = a.vel.len();
        let mach = tas / atm.sound;
        // The static ports read a little off the free stream near Mach 1.
        let ports = Atmosphere { pressure_alt: pressure_altitude(atm.pressure * (1.0 + curve(&STATIC_ERROR, mach))), ..atm };
        AirData {
            atm,
            baro_ft: DAY.altimeter(&ports) * FT,
            pressure_alt_ft: ports.pressure_alt * FT,
            density_alt_ft: atm.density_alt * FT,
            radar_ft: a.agl * FT,
            qnh_inhg: DAY.qnh / PA_PER_INHG,
            oat_c: atm.temp - 273.15,
            tat_c: atm.temp * (1.0 + 0.2 * mach * mach) - 273.15,
            mach,
            kcas: cas_for_mach(mach, atm.pressure) * KT,
            keas: tas * (atm.rho / RHO0).sqrt() * KT,
            ktas: tas * KT,
            gs_kt: a.vel.x.hypot(a.vel.z) * KT,
            vs_fpm: a.vel.y * FT * 60.0,
            qbar: 0.5 * atm.rho * tas * tas,
            sigma: atm.rho / RHO0,
            vmax_kcas: a.kind.vmax_kcas().min(cas_for_mach(a.kind.max_mach(), atm.pressure) * KT),
            vmax_mach: a.kind.max_mach().min(mach_for_cas(a.kind.vmax_kcas() / KT, atm.pressure)),
            g: a.g_smooth,
            g_max: a.max_g,
            g_min: a.min_g,
        }
    }

    pub fn overspeed(&self) -> bool {
        self.kcas > self.vmax_kcas || self.mach > self.vmax_mach
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const COLD: Weather = Weather { qnh: 101800.0, isa_dev: -10.0 };

    #[test]
    fn isa_sea_level_and_tropopause() {
        let sea = isa(0.0);
        assert!((sea.rho - 1.225).abs() < 0.001);
        assert!((sea.sound - 340.294).abs() < 0.01);
        assert!((sea.temp - 288.15).abs() < 1e-9);
        // The tropopause is 11 km geopotential, 11,019 m geometric.
        let trop = isa(11019.0);
        assert!((trop.pressure - 22632.0).abs() < 5.0, "{}", trop.pressure);
        assert!((trop.temp - 216.65).abs() < 0.01);
        assert!((trop.sound - 295.07).abs() < 0.05);
        let high = isa(20063.0);
        assert!((high.pressure - 5474.9).abs() < 5.0, "{}", high.pressure);
    }

    #[test]
    fn standard_day_altitudes_agree() {
        for z in [0.0, 1500.0, 6000.0, 11019.0, 15000.0, 25000.0] {
            let atm = isa(z);
            assert!((atm.pressure_alt - atm.density_alt).abs() < 0.05, "{z}");
            assert!((Weather::STANDARD.altimeter(&atm) - atm.pressure_alt).abs() < 1e-6);
            let geopotential = EARTH_RADIUS * z / (EARTH_RADIUS + z);
            assert!((atm.pressure_alt - geopotential).abs() < 0.05, "{z}");
        }
    }

    #[test]
    fn inverse_altitudes_round_trip() {
        for h in [-300.0, 0.0, 4000.0, 11000.0, 14000.0, 20000.0, 27000.0] {
            assert!((pressure_altitude(isa_pressure(h)) - h).abs() < 0.01, "{h}");
            assert!((density_altitude(isa_pressure(h) / (R_AIR * isa_temp(h))) - h).abs() < 0.01, "{h}");
        }
    }

    /// The day's air must satisfy dp/dz = -rho g at every height, with gravity
    /// falling off as the inverse square of the distance from the Earth's centre.
    #[test]
    fn non_standard_day_is_hydrostatic() {
        let mut p = COLD.at(0.0).pressure;
        assert!((p - COLD.qnh).abs() < 0.01);
        let dz = 2.0;
        let mut z = 0.0;
        while z < 9000.0 {
            let g = G0 * (EARTH_RADIUS / (EARTH_RADIUS + z + dz / 2.0)).powi(2);
            p -= COLD.at(z + dz / 2.0).rho * g * dz;
            z += dz;
        }
        let atm = COLD.at(9000.0);
        assert!((p / atm.pressure - 1.0).abs() < 2e-4, "{p} vs {}", atm.pressure);
        assert!((atm.temp - (isa_temp(atm.pressure_alt) - 10.0)).abs() < 1e-9);
    }

    #[test]
    fn cold_day_altimeter_and_density_altitude() {
        // Field elevation reads zero with QNH set.
        assert!(COLD.altimeter(&COLD.at(0.0)).abs() < 0.01);
        // 10,000 ft true on an ISA-10 day: the altimeter over-reads by about
        // 4 % (the "cold temperature error"), density altitude is lower.
        let atm = COLD.at(3048.0);
        let ind = COLD.altimeter(&atm);
        assert!(ind > 3048.0 * 1.03 && ind < 3048.0 * 1.05, "{ind}");
        assert!(atm.density_alt < atm.pressure_alt - 300.0);
        assert!((atm.temp - 273.15 - (15.0 - 6.5 * atm.pressure_alt / 1000.0 - 10.0)).abs() < 0.01);
    }

    #[test]
    fn compressible_calibrated_airspeed() {
        let sea = isa(0.0);
        for speed in [50.0, 200.0, 340.0, 500.0, 680.0] {
            assert!((calibrated_airspeed(speed, &sea) - speed).abs() < 0.001);
        }
        assert!((cas_for_mach(1.0, P0) - sea.sound).abs() < 1e-6);
        // Mach 1 at 30,000 ft is about 390 KCAS, Mach 2 at 40,000 ft about 651.
        let fl300 = isa(9144.0);
        assert!((cas_for_mach(1.0, fl300.pressure) * KT - 390.0).abs() < 2.0);
        let fl400 = isa(12192.0);
        assert!((cas_for_mach(2.0, fl400.pressure) * KT - 651.0).abs() < 3.0);
        assert!(calibrated_airspeed(300.0, &isa(10000.0)) < 200.0);
        for (mach, z) in [(0.4, 0.0), (0.95, 6000.0), (1.6, 9000.0), (2.05, 13000.0)] {
            let p = COLD.at(z).pressure;
            assert!((mach_for_cas(cas_for_mach(mach, p), p) - mach).abs() < 1e-6);
        }
        // 800 KCAS is about Mach 1.21 on a standard sea-level day.
        assert!((mach_for_cas(800.0 / KT, P0) - 1.21).abs() < 0.01);
    }
}
