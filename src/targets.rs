//! Ground targets for the STRIKE mission, strung down the Cold River gorge:
//! a truck convoy with a ZSU-23-4 escort, an SA-6 battery (2P25 launcher and
//! 1S91 radar) with its own guns, the road bridge over the river and a
//! supply depot. Each has hit points, an armour factor against 20 mm rounds
//! and a hit box; the models are a few dozen flat-shaded polygons in plain
//! paint colours, in the same low-poly style as the jet.
use crate::math::*;
use crate::mesh::{Frame, Mesh, Rgb};
use crate::terrain::{self, river_x};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Truck,
    /// ZSU-23-4 Shilka: four 23 mm guns under a tracking radar.
    Zsu,
    /// 2P25 launcher of the SA-6 (2K12 Kub), three 3M9 missiles.
    Launcher,
    /// 1S91 "Straight Flush" fire-control radar.
    Radar,
    Bridge,
    Warehouse,
    FuelTank,
}

pub const KINDS: [Kind; 7] = [Kind::Truck, Kind::Zsu, Kind::Launcher, Kind::Radar, Kind::Bridge, Kind::Warehouse, Kind::FuelTank];

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Truck => "TRUCK",
            Kind::Zsu => "ZSU-23-4",
            Kind::Launcher => "SA-6 TEL",
            Kind::Radar => "SA-6 RADAR",
            Kind::Bridge => "BRIDGE",
            Kind::Warehouse => "WAREHOUSE",
            Kind::FuelTank => "FUEL TANK",
        }
    }

    pub fn hp(self) -> f64 {
        match self {
            Kind::Truck => 40.0,
            Kind::Zsu => 60.0,
            Kind::Launcher => 70.0,
            Kind::Radar => 60.0,
            Kind::Bridge => 350.0,
            Kind::Warehouse => 220.0,
            Kind::FuelTank => 50.0,
        }
    }

    /// Damage of one 20 mm high-explosive round: soft-skinned trucks and
    /// tanks suffer, light armour less, concrete and steel spans not at all.
    pub fn round_damage(self) -> f64 {
        match self {
            Kind::Truck => 6.0,
            Kind::Zsu | Kind::Launcher => 3.0,
            Kind::Radar | Kind::FuelTank => 5.0,
            Kind::Bridge => 0.0,
            Kind::Warehouse => 1.0,
        }
    }

    pub fn points(self) -> f64 {
        match self {
            Kind::Truck => 200.0,
            Kind::Zsu => 400.0,
            Kind::Launcher | Kind::Radar => 800.0,
            Kind::Bridge => 1500.0,
            Kind::Warehouse => 500.0,
            Kind::FuelTank => 300.0,
        }
    }

    /// Hit-box half extents in the target's axes (x right, y up, z aft), m.
    pub fn half(self) -> V3 {
        match self {
            Kind::Truck => v3(1.25, 1.35, 3.7),
            Kind::Zsu => v3(1.55, 1.3, 3.25),
            Kind::Launcher => v3(1.6, 1.7, 3.7),
            Kind::Radar => v3(1.6, 2.2, 3.5),
            Kind::Bridge => v3(4.0, 1.0, 55.0),
            Kind::Warehouse => v3(15.0, 4.5, 8.0),
            Kind::FuelTank => v3(6.0, 4.0, 6.0),
        }
    }

    /// Height of the hit box's centre above the ground: the bridge deck
    /// stands on its piers.
    pub fn centre_height(self) -> f64 {
        if self == Kind::Bridge {
            7.5
        } else {
            self.half().y
        }
    }

    /// Size of the explosion and seconds the wreck burns.
    pub fn blast(self) -> (f64, f64) {
        match self {
            Kind::Truck => (0.8, 25.0),
            Kind::Zsu => (0.9, 30.0),
            Kind::Launcher => (1.2, 35.0),
            Kind::Radar => (0.9, 25.0),
            Kind::Bridge => (1.4, 12.0),
            Kind::Warehouse => (1.3, 45.0),
            Kind::FuelTank => (1.9, 60.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Site {
    pub kind: Kind,
    /// Ground point under the target's centre.
    pub pos: V3,
    /// Heading of the target's nose, radians clockwise from north (-z).
    pub heading: f64,
    pub hp: f64,
    pub dead: bool,
    /// Must die for the mission to succeed.
    pub primary: bool,
    /// Steerpoint group.
    pub group: usize,
    /// Gun or launcher state: burst timer, smoothed aim, missiles left.
    pub clock: f64,
    pub aim: V3,
    pub ammo: u32,
}

impl Site {
    fn new(kind: Kind, pos: V3, heading: f64, group: usize, primary: bool) -> Site {
        Site { kind, pos, heading, hp: kind.hp(), dead: false, primary, group, clock: 0.0, aim: V3::Y, ammo: if kind == Kind::Launcher { 3 } else { 0 } }
    }

    /// Rotation from the target's axes to the world.
    pub fn rot(&self) -> Quat {
        Quat::axis_angle(V3::Y, -self.heading)
    }

    pub fn centre(&self) -> V3 {
        self.pos + V3::Y * self.kind.centre_height()
    }

    /// A world point in the hit box's centred axes.
    pub fn local(&self, p: V3) -> V3 {
        self.rot().conj().rotate(p - self.centre())
    }

    /// Distance from a point to the hit box, 0 inside it.
    pub fn distance(&self, p: V3) -> f64 {
        let l = self.local(p);
        let h = self.kind.half();
        v3((l.x.abs() - h.x).max(0.0), (l.y.abs() - h.y).max(0.0), (l.z.abs() - h.z).max(0.0)).len()
    }

    /// Where the guns or the radar look out from.
    pub fn eye(&self) -> V3 {
        self.pos + V3::Y * (self.kind.half().y * 2.0 + 0.5)
    }
}

/// River-flow heading at a station (the route runs south, toward -z).
fn along(z: f64) -> f64 {
    let dx = river_x(z - 1.0) - river_x(z + 1.0);
    dx.atan2(2.0)
}

/// Ground point beside the river: `off` metres east of its centre line.
fn bank(z: f64, off: f64) -> V3 {
    let x = river_x(z) + off;
    v3(x, terrain::floor(x, z), z)
}

/// Steerpoint names, in the order the route meets them.
pub const GROUPS: [&str; 4] = ["CONVOY", "SAM SITE", "BRIDGE", "DEPOT"];

/// The STRIKE target set along the gorge, between gates 3 and 9 of the run.
pub fn place() -> Vec<Site> {
    let mut s = Vec::new();
    // Convoy on the east bank heading south, with its gun escort behind.
    for k in 0..4 {
        let z = 3380.0 - k as f64 * 42.0;
        s.push(Site::new(Kind::Truck, bank(z, 70.0), along(z), 0, false));
    }
    s.push(Site::new(Kind::Zsu, bank(3440.0, 75.0), along(3440.0), 0, false));
    // SA-6 battery on the west bank: launcher, radar, a resupply truck, guns.
    s.push(Site::new(Kind::Launcher, bank(1250.0, -165.0), along(1250.0) + 2.6, 1, true));
    s.push(Site::new(Kind::Radar, bank(1300.0, -205.0), along(1300.0) + 2.2, 1, true));
    s.push(Site::new(Kind::Truck, bank(1205.0, -180.0), along(1205.0) + 1.4, 1, false));
    s.push(Site::new(Kind::Zsu, bank(1170.0, -120.0), along(1170.0), 1, false));
    // The road bridge across the river, its deck square to the stream.
    let z = -700.0;
    let mut bridge = Site::new(Kind::Bridge, bank(z, 0.0), along(z) + PI / 2.0, 2, true);
    bridge.pos.y = terrain::floor(river_x(z), z);
    s.push(bridge);
    s.push(Site::new(Kind::Zsu, bank(-770.0, 110.0), along(-770.0), 2, false));
    // Supply depot on the east bank: two warehouses and two fuel tanks.
    s.push(Site::new(Kind::Warehouse, bank(-2650.0, 215.0), along(-2650.0), 3, false));
    s.push(Site::new(Kind::Warehouse, bank(-2715.0, 255.0), along(-2715.0), 3, false));
    s.push(Site::new(Kind::FuelTank, bank(-2770.0, 175.0), 0.0, 3, false));
    s.push(Site::new(Kind::FuelTank, bank(-2800.0, 200.0), 0.0, 3, false));
    s
}

/// Centre of a steerpoint group, a little above the ground.
pub fn group_point(sites: &[Site], group: usize) -> Option<V3> {
    let members: Vec<V3> = sites.iter().filter(|s| s.group == group).map(|s| s.pos).collect();
    if members.is_empty() {
        return None;
    }
    Some(members.iter().fold(V3::ZERO, |a, &p| a + p) / members.len() as f64 + V3::Y * 60.0)
}

// ================================================================ models
const OLIVE: Rgb = Rgb(0.33, 0.37, 0.22);
const OLIVE_DARK: Rgb = Rgb(0.22, 0.25, 0.15);
const CANVAS: Rgb = Rgb(0.52, 0.49, 0.34);
const TYRE: Rgb = Rgb(0.08, 0.08, 0.08);
const GLASS: Rgb = Rgb(0.10, 0.14, 0.16);
const CONCRETE: Rgb = Rgb(0.62, 0.61, 0.57);
const STEEL: Rgb = Rgb(0.36, 0.38, 0.40);
const MISSILE: Rgb = Rgb(0.82, 0.82, 0.78);

fn frame(o: V3) -> Frame {
    Frame { o, x: V3::X, y: V3::Y, z: V3::Z }
}

fn tilted(o: V3, pitch: f64) -> Frame {
    let q = Quat::axis_angle(V3::X, pitch);
    Frame { o, x: V3::X, y: q.rotate(V3::Y), z: q.rotate(V3::Z) }
}

fn wheel(m: &mut Mesh, x: f64, z: f64, r: f64) {
    let side = x.signum();
    m.lathe(v3(x - side * 0.15, r, z), V3::X * side, &[(0.0, r), (0.3, r), (0.3, 0.0)], 8, TYRE);
}

fn tracks(m: &mut Mesh, half_w: f64, half_l: f64) {
    for side in [-1.0, 1.0] {
        m.block(&frame(v3(side * (half_w - 0.25), 0.45, 0.0)), v3(0.25, 0.45, half_l), TYRE);
        for k in 0..6 {
            let z = -half_l + 0.6 + k as f64 * (2.0 * half_l - 1.2) / 5.0;
            m.lathe(v3(side * (half_w + 0.01), 0.42, z), V3::X * side, &[(0.0, 0.36), (0.04, 0.36), (0.04, 0.0)], 6, OLIVE_DARK);
        }
    }
}

/// Model of a target in its own axes, standing on y = 0, nose toward -z.
pub fn mesh(kind: Kind) -> Mesh {
    let mut m = Mesh::new();
    match kind {
        Kind::Truck => {
            m.block(&frame(v3(0.0, 1.55, -2.75)), v3(1.15, 0.8, 0.85), OLIVE);
            m.block(&frame(v3(0.0, 2.05, -3.62)), v3(1.0, 0.3, 0.02), GLASS);
            m.block(&frame(v3(0.0, 1.05, 0.8)), v3(1.2, 0.15, 2.8), OLIVE_DARK);
            m.block(&frame(v3(0.0, 1.85, 0.8)), v3(1.22, 0.65, 2.75), CANVAS);
            m.block(&frame(v3(0.0, 2.55, 0.8)), v3(0.9, 0.12, 2.7), CANVAS.scale(0.92));
            for (x, z) in [(-1.0, -2.7), (1.0, -2.7), (-1.0, 0.6), (1.0, 0.6), (-1.0, 2.2), (1.0, 2.2)] {
                wheel(&mut m, x * 1.1, z, 0.55);
            }
        }
        Kind::Zsu => {
            tracks(&mut m, 1.55, 3.1);
            m.block(&frame(v3(0.0, 1.15, 0.0)), v3(1.4, 0.45, 3.2), OLIVE);
            m.block(&frame(v3(0.0, 1.95, 0.4)), v3(1.25, 0.4, 1.4), OLIVE.scale(1.08));
            for x in [-0.55, -0.25, 0.25, 0.55] {
                m.lathe(v3(x, 2.05, -0.9), -V3::Z, &[(0.0, 0.06), (2.4, 0.045), (2.4, 0.0)], 4, STEEL);
            }
            m.lathe(v3(0.0, 2.35, 1.2), V3::Y, &[(0.0, 0.1), (0.5, 0.1)], 4, STEEL);
            m.block(&tilted(v3(0.0, 3.0, 1.2), -0.3), v3(0.7, 0.35, 0.06), OLIVE_DARK);
        }
        Kind::Launcher => {
            tracks(&mut m, 1.6, 3.6);
            m.block(&frame(v3(0.0, 1.2, 0.0)), v3(1.45, 0.4, 3.6), OLIVE);
            m.lathe(v3(0.0, 1.6, 0.6), V3::Y, &[(0.0, 1.0), (0.25, 1.0), (0.25, 0.0)], 8, OLIVE_DARK);
            for x in [-0.75, 0.0, 0.75] {
                let base = v3(x, 2.25, 2.0);
                let dir = v3(0.0, 0.42, -1.0).norm();
                m.lathe(base, dir, &[(0.0, 0.0), (0.02, 0.17), (5.0, 0.17), (5.6, 0.0)], 6, MISSILE);
                m.block(&tilted(base + dir * 2.6 + V3::Y * -0.25, -0.4), v3(0.12, 0.1, 2.6), STEEL);
            }
        }
        Kind::Radar => {
            tracks(&mut m, 1.6, 3.4);
            m.block(&frame(v3(0.0, 1.2, 0.0)), v3(1.45, 0.4, 3.4), OLIVE);
            m.block(&frame(v3(0.0, 2.0, -1.6)), v3(1.2, 0.4, 0.9), OLIVE.scale(1.08));
            m.lathe(v3(0.0, 1.6, 1.0), V3::Y, &[(0.0, 0.18), (2.2, 0.14)], 6, STEEL);
            m.block(&tilted(v3(0.0, 3.9, 1.0), -0.25), v3(1.5, 0.9, 0.1), OLIVE_DARK);
            m.block(&tilted(v3(0.0, 3.0, 1.6), -0.2), v3(0.9, 0.4, 0.1), OLIVE_DARK.scale(1.2));
        }
        Kind::Bridge => bridge(&mut m, true),
        Kind::Warehouse => {
            let (w, l, h) = (15.0, 8.0, 6.0);
            m.block(&frame(v3(0.0, h * 0.5, 0.0)), v3(w, h * 0.5, l), Rgb(0.62, 0.58, 0.48));
            let roof = Rgb(0.55, 0.26, 0.17);
            let ridge = h + 3.0;
            for s in [-1.0, 1.0] {
                m.quad(v3(-w - 0.4, h - 0.2, s * (l + 0.5)), v3(w + 0.4, h - 0.2, s * (l + 0.5)), v3(w + 0.4, ridge, 0.0), v3(-w - 0.4, ridge, 0.0), roof);
                m.tri(v3(s * w, h, -l), v3(s * w, ridge, 0.0), v3(s * w, h, l), Rgb(0.58, 0.54, 0.45));
            }
            m.block(&frame(v3(0.0, 2.2, -l - 0.05)), v3(4.0, 2.2, 0.06), Rgb(0.30, 0.30, 0.28));
        }
        Kind::FuelTank => {
            m.lathe(v3(0.0, 0.0, 0.0), V3::Y, &[(0.0, 0.0), (0.0, 6.0), (7.0, 6.0), (8.0, 4.0), (8.2, 0.0)], 10, Rgb(0.80, 0.80, 0.74));
            m.lathe(v3(0.0, 0.0, 0.0), V3::Y, &[(0.0, 6.6), (0.8, 6.6), (0.8, 6.2)], 10, CONCRETE.scale(0.8));
        }
    }
    m.facet();
    m
}

/// The bridge, or what is left of it: piers and abutments, with the deck
/// and its girders only while it stands. The span runs along the z axis.
fn bridge(m: &mut Mesh, whole: bool) {
    let (half, deck) = (55.0, 7.5);
    for z in [-half, half] {
        m.block(&frame(v3(0.0, deck * 0.5, z)), v3(4.6, deck * 0.5 + 0.8, 3.0), CONCRETE.scale(0.9));
    }
    for z in [-18.0, 18.0] {
        m.block(&frame(v3(0.0, deck * 0.5 - 1.0, z)), v3(3.2, deck * 0.5, 1.2), CONCRETE.scale(0.85));
    }
    if whole {
        m.block(&frame(v3(0.0, deck, 0.0)), v3(4.0, 0.5, half), CONCRETE);
        m.block(&frame(v3(0.0, deck - 1.0, 0.0)), v3(3.0, 0.5, half), STEEL);
        for x in [-3.9, 3.9] {
            m.block(&frame(v3(x, deck + 1.0, 0.0)), v3(0.1, 0.5, half), STEEL.scale(1.2));
        }
    } else {
        // Stubs of the fallen spans hang from the abutments.
        for s in [-1.0, 1.0] {
            m.block(&tilted(v3(0.0, deck - 2.5, s * (half - 9.0)), s * 0.35), v3(4.0, 0.5, 7.0), CONCRETE.scale(0.7));
        }
    }
}

/// The bridge after a hit brought the span down.
pub fn bridge_ruin() -> Mesh {
    let mut m = Mesh::new();
    bridge(&mut m, false);
    m.facet();
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapons::{blast_damage, hits_box, BOMB_DAMAGE, BOMB_LETHAL, BOMB_OUTER};

    #[test]
    fn targets_sit_on_the_valley_floor_by_the_river() {
        let sites = place();
        assert!(sites.iter().any(|s| s.primary));
        for s in &sites {
            assert!((s.pos.y - terrain::floor(s.pos.x, s.pos.z)).abs() < 1.0, "{} floats", s.kind.name());
            assert!((s.pos.x - river_x(s.pos.z)).abs() < 400.0, "{} off the valley floor", s.kind.name());
        }
        for g in 0..GROUPS.len() {
            assert!(group_point(&sites, g).is_some());
        }
        for k in KINDS {
            let m = mesh(k);
            assert!(m.count() > 0 && m.v.iter().all(|x| x.is_finite()));
        }
    }

    /// Rounds kill trucks and do nothing to the bridge; a Mk-82 near the
    /// deck drops it, and its blast reaches through the hit box.
    #[test]
    fn damage_follows_armour_and_blast() {
        let sites = place();
        let truck = sites.iter().find(|s| s.kind == Kind::Truck).unwrap();
        let rounds = (truck.kind.hp() / truck.kind.round_damage()).ceil();
        assert!(rounds <= 8.0, "{rounds} rounds for a truck");
        assert_eq!(Kind::Bridge.round_damage(), 0.0);
        // A round fired down onto the truck's roof hits its box.
        let top = truck.centre() + V3::Y * 20.0;
        assert!(hits_box(truck.local(top), truck.local(truck.centre() - V3::Y * 0.5), truck.kind.half()).is_some());
        let bridge = sites.iter().find(|s| s.kind == Kind::Bridge).unwrap();
        // A bomb on the river bed under the deck, at a quarter span.
        let along_span = bridge.rot().rotate(V3::Z) * 25.0;
        let impact = bridge.pos + along_span;
        let d = bridge.distance(impact);
        assert!(d < BOMB_LETHAL, "{d:.1} m from the deck");
        assert!(blast_damage(BOMB_DAMAGE, BOMB_LETHAL, BOMB_OUTER, d) >= bridge.kind.hp());
        // Thirty metres off, a warehouse survives one bomb.
        assert!(blast_damage(BOMB_DAMAGE, BOMB_LETHAL, BOMB_OUTER, 30.0) < Kind::Warehouse.hp());
    }
}
