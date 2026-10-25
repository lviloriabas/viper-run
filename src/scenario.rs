//! The opposition for the fighting missions (DOGFIGHT, STRIKE and FREE
//! FLIGHT): how many bandits and of which type, how many SA-6 batteries and
//! ZSU-23-4 guns, and where one of the prepared layouts puts them in the
//! Cold River valley. The loadout screen's tactical map and the mission
//! itself read the same deployment, so the map shows what the sortie meets.
use crate::combat::MissionKind;
use crate::math::*;
use crate::sim::{Mission, Type};
use crate::targets::{self, Site, NO_GROUP};
use crate::terrain::{self, river_x};

/// Most bandits, SA-6 batteries and ZSU-23-4s a layout can field.
pub const MAX_BANDITS: u8 = 6;
pub const MAX_SAMS: u8 = 3;
pub const MAX_GUNS: u8 = 6;
/// SA-6 and ZSU-23-4 engagement ranges, m, for the map's rings.
pub const SAM_RANGE: f64 = 14000.0;
pub const GUN_RANGE: f64 = 2500.0;
/// Station of the centre the patrol circuits ring, halfway down the gate route.
pub const PATROL_CENTRE_Z: f64 = -600.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    /// Pairs coming head-on down the gorge, the defences spread along it.
    HeadOn,
    /// Pairs closing from both flanks, guns massed on the route.
    Ambush,
    /// Everyone starts inside 6 km: a merged turning fight from the off.
    Furball,
    /// Fighters on combat air patrol behind a belt of SAM batteries.
    SamAlley,
}

impl Layout {
    pub const ALL: [Layout; 4] = [Layout::HeadOn, Layout::Ambush, Layout::Furball, Layout::SamAlley];

    pub fn name(self) -> &'static str {
        match self {
            Layout::HeadOn => "HEAD-ON",
            Layout::Ambush => "AMBUSH",
            Layout::Furball => "FURBALL",
            Layout::SamAlley => "SAM ALLEY",
        }
    }

    pub fn brief(self) -> &'static str {
        match self {
            Layout::HeadOn => "Pairs come straight up the gorge from 20 km. Guns and SAMs spread along the river.",
            Layout::Ambush => "Pairs cut in from both flanks over the ridges. Guns massed along the route.",
            Layout::Furball => "Everyone inside 6 km at the start. No time for long shots.",
            Layout::SamAlley => "Fighters orbit behind a belt of SA-6 batteries. Stay low or go around.",
        }
    }

    pub fn next(self) -> Layout {
        let i = Layout::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Layout::ALL[(i + 1) % Layout::ALL.len()]
    }

    /// The bandits, SA-6 batteries and guns the layout comes with.
    pub fn counts(self, kind: MissionKind) -> (u8, u8, u8) {
        match (self, kind) {
            (Layout::HeadOn, MissionKind::Strike) => (0, 1, 3),
            (Layout::HeadOn, _) => (3, 0, 0),
            (Layout::Ambush, MissionKind::Strike) => (2, 1, 5),
            (Layout::Ambush, _) => (4, 0, 3),
            (Layout::Furball, _) => (4, 0, 2),
            (Layout::SamAlley, _) => (2, 3, 4),
        }
    }

    /// SA-6 batteries by preference: river station and offset east, m.
    fn sams(self) -> [(f64, f64); MAX_SAMS as usize] {
        match self {
            Layout::HeadOn => [(1250.0, -165.0), (-4600.0, 190.0), (-9500.0, -170.0)],
            Layout::Ambush => [(1250.0, -165.0), (3600.0, 260.0), (-1800.0, 240.0)],
            Layout::Furball => [(-6500.0, -180.0), (-2400.0, 200.0), (-10500.0, 160.0)],
            Layout::SamAlley => [(4600.0, -170.0), (1250.0, -165.0), (-2600.0, 185.0)],
        }
    }

    /// ZSU-23-4s by preference: river station and offset east, m.
    fn guns(self) -> [(f64, f64); MAX_GUNS as usize] {
        match self {
            Layout::HeadOn => [(3440.0, 75.0), (1170.0, -120.0), (-770.0, 110.0), (-2700.0, 150.0), (5200.0, -90.0), (-6000.0, 120.0)],
            Layout::Ambush => [(3000.0, 60.0), (2600.0, -70.0), (2200.0, 90.0), (1800.0, -60.0), (1400.0, 70.0), (1000.0, -80.0)],
            Layout::Furball => [(5600.0, 80.0), (5200.0, -90.0), (-3000.0, 100.0), (-3400.0, -100.0), (500.0, 90.0), (100.0, -90.0)],
            Layout::SamAlley => [(4500.0, -120.0), (1170.0, -120.0), (-2500.0, 140.0), (-770.0, 110.0), (3000.0, 80.0), (-4500.0, -100.0)],
        }
    }
}

/// The opposition chosen on the loadout screen.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Threats {
    pub layout: Layout,
    pub bandits: u8,
    pub sams: u8,
    pub guns: u8,
    /// F-16 aggressors and MiG-29s alternating, instead of one type.
    pub mixed: bool,
}

impl Threats {
    /// What each mission starts with: free flight keeps its pair on patrol
    /// over the classic SA-6 battery and guns.
    pub fn for_mission(kind: MissionKind) -> Threats {
        match kind {
            MissionKind::Free => Threats { layout: Layout::SamAlley, bandits: 2, sams: 1, guns: 3, mixed: false },
            _ => Threats::with_layout(Layout::HeadOn, kind, false),
        }
    }

    pub fn with_layout(layout: Layout, kind: MissionKind, mixed: bool) -> Threats {
        let (bandits, sams, guns) = layout.counts(kind);
        Threats { layout, bandits, sams, guns, mixed }
    }
}

/// A bandit to put in the air: airframe, where, which way and how fast,
/// how good its pilot is, and the circuit it flies if it patrols (radius,
/// turn direction, height over the ground, first point).
#[derive(Clone, Copy, Debug)]
pub struct BanditSpec {
    pub kind: Type,
    pub pos: V3,
    pub heading: f64,
    pub speed: f64,
    pub skill: f64,
    pub patrol: Option<(f64, i32, f64, usize)>,
}

pub struct Deployment {
    pub bandits: Vec<BanditSpec>,
    pub sites: Vec<Site>,
}

/// Height `over` the terrain, and never under 2,600 m: above the ridges.
fn high(p: V3, over: f64) -> f64 {
    (terrain::floor(p.x, p.z) + over).max(2600.0)
}

fn heading_of(v: V3) -> f64 {
    v.x.atan2(-v.z)
}

/// Where everything starts for a mission flown from `m`'s start.
pub fn deploy(kind: MissionKind, aggressor: Type, t: &Threats, m: &Mission) -> Deployment {
    let start = v3(m.start_pos.x, 0.0, m.start_pos.z);
    let fwd = {
        let f = v3(m.start_vel.x, 0.0, m.start_vel.z);
        if f.len() > 1.0 { f.norm() } else { v3(0.0, 0.0, -1.0) }
    };
    let right = fwd.cross(V3::Y).norm();
    let n = t.bandits.min(MAX_BANDITS) as usize;
    let other = match aggressor { Type::F16=>Type::Mig29, Type::Mig29=>Type::F16, Type::F18=>Type::Su27, Type::Su27=>Type::F18 };
    let kind_of = |i: usize| if t.mixed && i % 2 == 1 { other } else { aggressor };
    // Leads fly better than their wingmen.
    let skill = |i: usize| if i % 2 == 0 { 0.55 } else { 0.35 };
    let at = |p: V3, over: f64| v3(p.x, high(p, over), p.z);
    let mut bandits = Vec::new();
    for i in 0..n {
        let (pair, wing) = ((i / 2) as f64, i % 2 == 1);
        let spec = |pos: V3, toward: V3, speed: f64| BanditSpec { kind: kind_of(i), pos, heading: heading_of(toward - pos), speed, skill: skill(i), patrol: None };
        bandits.push(match t.layout {
            Layout::HeadOn => {
                let p = start + fwd * (20000.0 + 6000.0 * pair + if wing { 2500.0 } else { 0.0 }) + right * (if wing { -1600.0 } else { 1400.0 } * (1.0 + 0.3 * pair));
                spec(at(p, 1300.0 + if wing { 300.0 } else { 0.0 }), p - fwd * 1000.0, 235.0)
            }
            Layout::Ambush => {
                let side = if (i / 2) % 2 == 0 { 1.0 } else { -1.0 } * if wing { -1.0 } else { 1.0 };
                let p = start + fwd * (8000.0 + 4000.0 * pair) + right * (side * 12000.0);
                spec(at(p, 1400.0 + 200.0 * pair), start + fwd * 12000.0, 240.0)
            }
            Layout::Furball => {
                let centre = start + fwd * 3000.0;
                let ang = 0.5 + i as f64 * TAU / n as f64;
                let p = centre + fwd * (6000.0 * ang.cos()) + right * (6000.0 * ang.sin());
                spec(at(p, 1200.0 + 150.0 * i as f64), centre, 230.0)
            }
            Layout::SamAlley => {
                const RADII: [f64; MAX_BANDITS as usize] = [14000.0, 9000.0, 11500.0, 7000.0, 16000.0, 5500.0];
                let mut b = spec(V3::ZERO, V3::ZERO, 235.0);
                b.patrol = Some((RADII[i], if i % 2 == 0 { 1 } else { -1 }, 1300.0 + 300.0 * i as f64, (i * 3) % 8));
                b
            }
        });
    }

    let mut sites = match kind {
        MissionKind::Strike | MissionKind::Free => targets::objectives(),
        _ => Vec::new(),
    };
    for (k, &(z, off)) in t.layout.sams().iter().take(t.sams.min(MAX_SAMS) as usize).enumerate() {
        // The strike's first battery is its SAM SITE steerpoint and must die.
        let strike = kind == MissionKind::Strike && k == 0;
        targets::battery(&mut sites, z, off, if strike { 1 } else { NO_GROUP }, strike);
    }
    for &(z, off) in t.layout.guns().iter().take(t.guns.min(MAX_GUNS) as usize) {
        targets::gun(&mut sites, z, off, NO_GROUP);
    }
    Deployment { bandits, sites }
}

/// A point on the river's centre line, for the map.
pub fn river(z: f64) -> V3 {
    v3(river_x(z), 0.0, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layout_fields_what_it_says_on_the_ground_and_in_the_air() {
        let m = Mission::new();
        for layout in Layout::ALL {
            for kind in [MissionKind::Dogfight, MissionKind::Strike, MissionKind::Free] {
                let t = Threats { layout, bandits: MAX_BANDITS, sams: MAX_SAMS, guns: MAX_GUNS, mixed: true };
                let d = deploy(kind, Type::Mig29, &t, &m);
                assert_eq!(d.bandits.len(), MAX_BANDITS as usize);
                assert!(d.bandits.iter().any(|b| b.kind == Type::F16) && d.bandits.iter().any(|b| b.kind == Type::Mig29));
                let count = |k: targets::Kind| d.sites.iter().filter(|s| s.kind == k).count();
                assert_eq!(count(targets::Kind::Launcher), MAX_SAMS as usize);
                assert_eq!(count(targets::Kind::Radar), MAX_SAMS as usize);
                assert_eq!(count(targets::Kind::Zsu), MAX_GUNS as usize);
                for b in d.bandits.iter().filter(|b| b.patrol.is_none()) {
                    assert!(b.pos.y >= terrain::floor(b.pos.x, b.pos.z) + 1000.0, "{layout:?} bandit under the ridges");
                }
            }
        }
        // The strike always keeps its bridge, and its SAM SITE while it has one.
        let none = Threats { layout: Layout::Ambush, bandits: 0, sams: 0, guns: 0, mixed: false };
        let d = deploy(MissionKind::Strike, Type::F16, &none, &m);
        assert!(d.bandits.is_empty() && d.sites.iter().any(|s| s.primary) && !d.sites.iter().any(|s| s.kind == targets::Kind::Zsu));
    }

    #[test]
    fn the_classic_strike_matches_the_standing_target_set() {
        let m = Mission::new();
        let d = deploy(MissionKind::Strike, Type::F16, &Threats::for_mission(MissionKind::Strike), &m);
        let mut a: Vec<(targets::Kind, i64, i64)> = d.sites.iter().map(|s| (s.kind, s.pos.x.round() as i64, s.pos.z.round() as i64)).collect();
        let mut b: Vec<(targets::Kind, i64, i64)> = targets::place().iter().map(|s| (s.kind, s.pos.x.round() as i64, s.pos.z.round() as i64)).collect();
        let key = |t: &(targets::Kind, i64, i64)| (t.1, t.2);
        a.sort_by_key(key);
        b.sort_by_key(key);
        assert_eq!(a, b);
    }
}
