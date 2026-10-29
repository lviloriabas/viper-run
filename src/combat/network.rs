//! Multiplayer adaptation of the existing combat model. The host steps each
//! aircraft once and reuses the same weapon/sensor rules for remote pilots.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Control {
    pub drop_lock: u32,
    pub axes: [f32; 3],
    pub throttle: i8,
    pub flags: u16,
    pub actions: [u32; 5],
    pub look: [f32; 3],
}
impl Control {
    pub fn from_input(
        i: Input,
        trigger: bool,
        cm: bool,
        eject: bool,
        actions: [u32; 5],
        look: Option<V3>,
    ) -> Self {
        let flags = (i.afterburner as u16)
            | ((i.airbrake as u16) << 1)
            | ((i.gear_down as u16) << 2)
            | ((i.flaps as u16) << 3)
            | ((i.wheel_brake as u16) << 4)
            | ((i.limiters_off as u16) << 5)
            | ((trigger as u16) << 6)
            | ((cm as u16) << 7)
            | ((eject as u16) << 8);
        Self {
            drop_lock: 0,
            axes: [i.pitch() as f32, i.roll() as f32, i.yaw() as f32],
            throttle: i.throttle_up as i8 - i.throttle_down as i8,
            flags,
            actions,
            look: look.map(array).unwrap_or([0.0; 3]),
        }
    }
    pub fn valid(&self) -> bool {
        self.axes
            .iter()
            .chain(self.look.iter())
            .all(|v| v.is_finite() && v.abs() <= 1.01)
            && (-1..=1).contains(&self.throttle)
            && self.flags < 512
    }
    pub fn input(&self) -> Input {
        Input {
            pitch_axis: self.axes[0] as f64,
            roll_axis: self.axes[1] as f64,
            yaw_axis: self.axes[2] as f64,
            throttle_up: self.throttle > 0,
            throttle_down: self.throttle < 0,
            afterburner: self.flags & 1 != 0,
            airbrake: self.flags & 2 != 0,
            gear_down: self.flags & 4 != 0,
            flaps: self.flags & 8 != 0,
            wheel_brake: self.flags & 16 != 0,
            limiters_off: self.flags & 32 != 0,
            ..Default::default()
        }
    }
    pub fn trigger(&self) -> bool {
        self.flags & 64 != 0
    }
    pub fn idle(&self) -> Self {
        Self {
            drop_lock: self.drop_lock,
            flags: self.flags & 14,
            actions: self.actions,
            ..Default::default()
        }
    }
}
pub struct RemotePilot {
    mode: Mode,
    pick: Option<Store>,
    lock: Option<Who>,
    acquiring: Option<Who>,
    acquire_clock: f64,
    irst: bool,
    seeker: Option<Who>,
    ccip: Option<(V3, f64)>,
    shoot: bool,
    helmet: Option<V3>,
    zone_clock: f64,
    warn: f64,
    firing: bool,
    trigger_was: bool,
    gun_acc: f64,
    fired: u32,
    pub actions: [u32; 5],
    last_drop_lock: u32,
    cm_clock: f64,
    eject_clock: f64,
    pub pilot: crate::physio::Pilot,
}
impl Default for RemotePilot {
    fn default() -> Self {
        Self {
            mode: Mode::Nav,
            pick: None,
            lock: None,
            acquiring: None,
            acquire_clock: 0.0,
            irst: false,
            seeker: None,
            ccip: None,
            shoot: false,
            helmet: None,
            zone_clock: 0.0,
            warn: 0.0,
            firing: false,
            trigger_was: false,
            gun_acc: 0.0,
            fired: 0,
            actions: [0; 5],
            last_drop_lock: 0,
            cm_clock: 0.0,
            eject_clock: 0.0,
            pilot: crate::physio::Pilot::default(),
        }
    }
}
impl RemotePilot {
    pub fn ejected(&self) -> bool {
        self.eject_clock >= 0.5
    }
    fn exchange(&mut self, c: &mut Combat) {
        macro_rules! swap{($($field:ident),*)=>{$(std::mem::swap(&mut self.$field,&mut c.$field);)*}}
        swap!(
            mode,
            pick,
            lock,
            acquiring,
            acquire_clock,
            irst,
            seeker,
            ccip,
            shoot,
            helmet,
            zone_clock,
            warn,
            firing,
            trigger_was,
            gun_acc,
            fired
        );
    }
}
pub fn array(v: V3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}
pub fn vector(v: [f32; 3]) -> V3 {
    v3(v[0] as f64, v[1] as f64, v[2] as f64)
}
fn remap(w: Who, i: usize) -> Who {
    match w {
        Who::Player => Who::Bandit(i),
        Who::Bandit(j) if j == i => Who::Player,
        _ => w,
    }
}
impl Combat {
    pub fn enemy(&self, a: Who, b: Who) -> bool {
        let team = |w| match w {
            Who::Player => self.player_team,
            Who::Bandit(i) => self.bandits.get(i).map_or(255, |b| b.team),
            Who::Ground(_) => 1,
        };
        a != b && team(a) != team(b)
    }
    pub fn network_spawn(
        &mut self,
        jet: Aircraft,
        team: u8,
        remote: bool,
        kind: MissionKind,
    ) -> usize {
        let mut b = bandit(jet, 0.6, CALLSIGNS[self.bandits.len() % CALLSIGNS.len()]);
        b.team = team;
        b.remote = remote;
        if remote {
            b.stores = Loadout::configure(&Config::for_jet(kind, b.jet.kind));
            load(&mut b.jet, &b.stores);
            b.rounds = b.stores.rounds;
        }
        self.bandits.push(b);
        self.bandits.len() - 1
    }
    pub fn network_lose_player(&mut self, a: &Aircraft, cause: &'static str) {
        if !self.player_dead {
            self.lose_player(cause, a);
        }
    }
    pub fn network_route(&mut self, i: usize, goal: V3, dt: f64) {
        let Some(b) = self.bandits.get_mut(i).filter(|b| b.alive) else {
            return;
        };
        let mut mark = b.jet.clone();
        mark.pos = goal;
        mark.vel = V3::ZERO;
        let picture = ai::Picture {
            foe: &mark,
            missile: None,
            missiles: 0,
            missile_spec: &AIM9M,
            rounds: 0,
            tactical: ai::Tactical {
                combat: false,
                ..Default::default()
            },
        };
        let control = b.pilot.fly(&b.jet, &picture, dt);
        b.jet.update(dt, &control.input);
        if b.jet.crashed || b.jet.lost.any() {
            self.kill_bandit(i);
        }
    }
    pub fn network_visual_step(&mut self, dt: f64) {
        self.clock += dt;
        for i in 0..self.bandits.len() { self.shed_bandit_gear(i); }
        self.step_wrecks(dt);
        for b in &mut self.bandits {
            if b.alive {
                b.jet.last_pos = b.jet.pos;
                b.jet.pos += b.jet.vel * dt;
            }
        }
        for r in &mut self.rounds {
            r.step(dt);
        }
        let mut smoke = Vec::new();
        for (i, m) in self.missiles.iter_mut().enumerate() {
            m.pos += m.vel * dt;
            m.age += dt;
            if m.burning() {
                smoke.push((self.missile_trails[i], m.pos));
            }
        }
        for (t, p) in smoke {
            self.lay(t, p);
        }
        for b in &mut self.bombs {
            b.pos += b.vel * dt;
            b.age += dt;
        }
        self.age_trails();
    }
    pub fn training_target(&mut self, a: &Aircraft) {
        if let Some(b) = self.bandits.first_mut() {
            b.jet = spawn_type(
                b.jet.kind,
                a.pos + a.q.axes().forward * 4500.0 + V3::Y * 150.0,
                a.heading * DEG,
                170.0,
            );
            load(&mut b.jet, &b.stores);
        }
    }
    pub fn network_clear_ai(&mut self) {
        for i in 0..self.bandits.len() { self.forget_target(Who::Bandit(i)); }
        self.bandits.clear();
        self.reserve.clear();
        self.flight_track = None;
        self.flight_target = Who::Player;
        self.track_age = 0.0;
        self.hostile = false;
        self.done_at = None;
    }
    pub fn network_hostile(&mut self, on: bool) {
        self.hostile = on;
    }
    pub fn network_reset_player(&mut self, a: &mut Aircraft, jet: Aircraft, kind: MissionKind) {
        self.forget_target(Who::Player);
        *a = jet;
        self.equip(&Config::for_jet(kind, a.kind));
        load(a, &self.loadout);
        self.hull = Hull::new(a.kind);
        self.hp = 100.0;
        self.player_dead = false;
        self.player_lost = Breakup::default();
        self.player_hitter = None;
        self.drop_lock();
    }
    pub fn network_reset_remote(&mut self, i: usize, jet: Aircraft, kind: MissionKind) {
        self.forget_target(Who::Bandit(i));
        let team = self.bandits[i].team;
        let mut b = bandit(jet, 0.6, CALLSIGNS[i % CALLSIGNS.len()]);
        b.team = team;
        b.remote = true;
        b.stores = Loadout::configure(&Config::for_jet(kind, b.jet.kind));
        load(&mut b.jet, &b.stores);
        self.bandits[i] = b;
    }
    pub fn network_controls(
        &mut self,
        a: &Aircraft,
        control: &Control,
        pilot: &mut RemotePilot,
        dt: f64,
    ) {
        if pilot.last_drop_lock != control.drop_lock {
            pilot.last_drop_lock = control.drop_lock;
            self.drop_lock();
        }
        for k in 0..5 {
            if pilot.actions[k] != control.actions[k] {
                pilot.actions[k] = control.actions[k];
                match k {
                    0 => self.cycle_weapon(),
                    1 => self.designate(a),
                    2 => self.countermeasures(a),
                    3 => self.jettison(a),
                    4 => self.toggle_irst(a),
                    _ => {}
                }
            }
        }
        pilot.cm_clock = if control.flags & 128 != 0 {
            pilot.cm_clock + dt
        } else {
            0.0
        };
        if pilot.cm_clock >= 1.0 {
            self.countermeasures(a);
            pilot.cm_clock = 0.0;
        }
        pilot.eject_clock = if control.flags & 256 != 0 {
            pilot.eject_clock + dt
        } else {
            0.0
        };
        self.helmet = (vector(control.look).len() > 0.5).then(|| vector(control.look).norm());
    }
    fn network_remap(&mut self, i: usize) {
        for r in &mut self.rounds {
            r.from = remap(r.from, i);
        }
        for b in &mut self.bombs {
            b.from = remap(b.from, i);
        }
        for m in &mut self.missiles {
            m.from = remap(m.from, i);
            m.target = m.target.map(|w| remap(w, i));
        }
    }
    pub fn network_remote_step(
        &mut self,
        i: usize,
        host: &mut Aircraft,
        control: &Control,
        p: &mut RemotePilot,
        dt: f64,
    ) {
        if !self.bandits.get(i).is_some_and(|b| b.alive && b.remote) {
            return;
        }
        let host_dead = self.player_dead;
        // A temporary change of viewpoint keeps one implementation of launch,
        // acquisition, IRST, jettison and stores mass for every human pilot.
        let event_start = self.events.len();
        self.network_remap(i);
        p.exchange(self);
        {
            let b = &mut self.bandits[i];
            std::mem::swap(host, &mut b.jet);
            std::mem::swap(&mut self.loadout, &mut b.stores);
            std::mem::swap(&mut self.hull, &mut b.hull);
            std::mem::swap(&mut self.hp, &mut b.hp);
            std::mem::swap(&mut self.player_team, &mut b.team);
            b.alive = !host_dead;
            self.player_dead = false;
        }
        self.lock = self.lock.map(|w| remap(w, i));
        self.acquiring = self.acquiring.map(|w| remap(w, i));
        self.seeker = self.seeker.map(|w| remap(w, i));
        self.network_controls(host, control, p, dt);
        let input = p.pilot.controls(host, control.input());
        host.update(dt, &input);
        self.shed_gear(host);
        p.pilot.update(dt, host);
        self.sensors(dt, host);
        let trigger = control.trigger() && p.pilot.authority() > 0.5 && p.eject_clock == 0.0;
        self.player_weapons(dt, host, trigger, trigger && !self.trigger_was);
        self.trigger_was = trigger;
        let supported = self.radar_lock().filter(|w| self.track(*w, host).is_some_and(|(pos, _)| self.radar_sees(host, pos, true)));
        for m in &mut self.missiles {
            if m.from == Who::Player {
                m.supported = m.target.is_some() && supported == m.target;
            }
        }
        host.stores_mass = self.loadout.mass();
        host.stores_drag = self.loadout.drag();
        host.wing_stores = self.loadout.wing_mass(host.kind);
        let lost = host.crashed || host.lost.any() || p.eject_clock >= 0.5;
        if p.eject_clock >= 0.5 {
            host.ejected = true;
        }
        self.lock = self.lock.map(|w| remap(w, i));
        self.acquiring = self.acquiring.map(|w| remap(w, i));
        self.seeker = self.seeker.map(|w| remap(w, i));
        {
            let b = &mut self.bandits[i];
            std::mem::swap(host, &mut b.jet);
            std::mem::swap(&mut self.loadout, &mut b.stores);
            std::mem::swap(&mut self.hull, &mut b.hull);
            std::mem::swap(&mut self.hp, &mut b.hp);
            std::mem::swap(&mut self.player_team, &mut b.team);
            b.alive = true;
            self.player_dead = host_dead;
        }
        p.exchange(self);
        self.network_remap(i);
        // Launch event identities also belong to the original world.
        for event in &mut self.events[event_start..] {
            if let Event::Launch { from, .. } = event {
                if *from == Who::Player {
                    *from = Who::Bandit(i);
                }
            }
        }
        if lost {
            self.kill_bandit(i);
        }
    }
    pub fn network_collisions(&mut self, host: &Aircraft) {
        let mut pairs = Vec::new();
        if !self.player_dead && !host.crashed {
            for (i, b) in self.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
                if jets_collide(host, &b.jet) {
                    pairs.push((Who::Player, Who::Bandit(i)));
                }
            }
        }
        for i in 0..self.bandits.len() {
            if !self.bandits[i].alive {
                continue;
            }
            for j in i + 1..self.bandits.len() {
                if self.bandits[j].alive && jets_collide(&self.bandits[i].jet, &self.bandits[j].jet)
                {
                    pairs.push((Who::Bandit(i), Who::Bandit(j)));
                }
            }
        }
        for (a, b) in pairs {
            for who in [a, b] {
                match who {
                    Who::Player => {
                        if !self.player_dead {
                            self.lose_player("MID-AIR COLLISION", host);
                        }
                    }
                    Who::Bandit(i) => self.kill_bandit(i),
                    _ => {}
                }
            }
        }
    }
}
fn point_segment(p: V3, a: V3, b: V3) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.dot(d).max(1e-9)).clamp(0.0, 1.0);
    (p - a - d * t).len()
}
fn segment_distance(a: V3, b: V3, c: V3, d: V3) -> f64 {
    let u = b - a;
    let v = d - c;
    let w = a - c;
    let aa = u.dot(u);
    let bb = u.dot(v);
    let cc = v.dot(v);
    let dd = u.dot(w);
    let ee = v.dot(w);
    let det = aa * cc - bb * bb;
    let s = if det > 1e-9 {
        ((bb * ee - cc * dd) / det).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let t = ((bb * s + ee) / cc.max(1e-9)).clamp(0.0, 1.0);
    let s = ((bb * t - dd) / aa.max(1e-9)).clamp(0.0, 1.0);
    (a + u * s - c - v * t)
        .len()
        .min(point_segment(a, c, d))
        .min(point_segment(b, c, d))
}
pub fn jets_collide(a: &Aircraft, b: &Aircraft) -> bool {
    let (r0, r1) = (a.last_pos - b.last_pos, a.pos - b.pos);
    if closest(r0, r1).1 > a.kind.span() + b.kind.span() + 15.0 {
        return false;
    }
    let check = |a: &Aircraft, b: &Aircraft| {
        let inv = b.q.conj();
        let contacts = b.kind.contacts();
        let nose = contacts
            .iter()
            .min_by(|a, b| a.local.z.total_cmp(&b.local.z))
            .unwrap()
            .local;
        let tail = contacts
            .iter()
            .max_by(|a, b| a.local.z.total_cmp(&b.local.z))
            .unwrap()
            .local;
        let left = contacts
            .iter()
            .min_by(|a, b| a.local.x.total_cmp(&b.local.x))
            .unwrap()
            .local;
        let right = contacts
            .iter()
            .max_by(|a, b| a.local.x.total_cmp(&b.local.x))
            .unwrap()
            .local;
        a.kind.contacts().iter().any(|p| {
            let at = a.q.rotate(p.local);
            let p0 = inv.rotate(a.last_pos + at - b.last_pos);
            let p1 = inv.rotate(a.pos + at - b.pos);
            segment_distance(p0, p1, nose, tail) < 1.0
                || segment_distance(p0, p1, left, right) < 0.55
        })
    };
    check(a, b) || check(b, a)
}
const STORES: [Store; 15] = [
    Store::Aim9,
    Store::Aim9x,
    Store::Aim7,
    Store::Aim120,
    Store::Aim120d,
    Store::Mk82,
    Store::R60m,
    Store::R73,
    Store::R27r,
    Store::R27t,
    Store::R77,
    Store::Fab250,
    Store::Fab500,
    Store::Mk82Ter,
    Store::Fab250Twin,
];
fn store_id(s: Store) -> u8 {
    STORES.iter().position(|&t| s == t).unwrap() as u8
}
fn actor(w: Who) -> u16 {
    match w {
        Who::Player => 0,
        Who::Bandit(i) => i as u16 + 1,
        Who::Ground(i) => i as u16 + 1000,
    }
}
fn who(id: u16, own: u16, host_slot: usize) -> Who {
    if id == own {
        Who::Player
    } else if id == 0 {
        Who::Bandit(host_slot)
    } else if id >= 1000 {
        Who::Ground((id - 1000) as usize)
    } else {
        Who::Bandit(id as usize - 1)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ActorState {
    pub id: u16,
    pub peer: Option<u64>,
    pub team: u8,
    pub kind: u8,
    pub alive: bool,
    pub hp: f32,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub q: [f32; 4],
    pub omega: [f32; 3],
    pub systems: [f32; 15],
    pub flags: u16,
    pub rounds: u32,
    pub chaff: u32,
    pub flares: u32,
    pub stations: Vec<(u8, [f32; 3], bool, bool)>,
    pub mode: u8,
    pub lock: Option<u16>,
    pub seeker: Option<u16>,
}
impl ActorState {
    fn new(
        id: u16,
        peer: Option<u64>,
        team: u8,
        a: &Aircraft,
        l: &Loadout,
        hp: f64,
        alive: bool,
        mode: Mode,
        lock: Option<Who>,
        seeker: Option<Who>,
    ) -> Self {
        Self {
            id,
            peer,
            team,
            kind: a.kind.index() as u8,
            alive,
            hp: hp as f32,
            pos: array(a.pos),
            vel: array(a.vel),
            q: [a.q.x as f32, a.q.y as f32, a.q.z as f32, a.q.w as f32],
            omega: array(a.omega),
            systems: [
                a.throttle as f32,
                a.power as f32,
                a.fuel as f32,
                a.gear as f32,
                a.flaps as f32,
                a.brake as f32,
                a.g_load as f32,
                a.alpha_deg as f32,
                a.gear_damage as f32,
                a.flap_damage as f32,
                a.surf_pitch as f32,
                a.surf_roll as f32,
                a.surf_yaw as f32,
                (l.initial_rounds + 1) as f32,
                0.0,
            ],
            flags: (a.on_ground as u16)
                | ((a.ejected as u16) << 1)
                | ((a.lost.wing[0] as u16) << 2)
                | ((a.lost.wing[1] as u16) << 3)
                | ((a.lost.tail as u16) << 4)
                | ((a.crashed as u16) << 5)
                | ((a.gear_broken[0] as u16) << 6)
                | ((a.gear_broken[1] as u16) << 7)
                | ((a.gear_broken[2] as u16) << 8),
            rounds: l.rounds,
            chaff: l.chaff,
            flares: l.flares,
            stations: l
                .stations
                .iter()
                .map(|s| (store_id(s.store), array(s.at), s.loaded, s.tip))
                .collect(),
            mode: mode as u8,
            lock: lock.map(actor),
            seeker: seeker.map(actor),
        }
    }
    fn apply(&self, a: &mut Aircraft, l: &mut Loadout) {
        let kind = Type::ALL[self.kind as usize];
        if a.kind != kind {
            *a = spawn_type(
                kind,
                vector(self.pos),
                0.0,
                vector(self.vel).len().max(50.0),
            );
        }
        a.last_pos = a.pos;
        a.pos = vector(self.pos);
        a.vel = vector(self.vel);
        a.q = Quat {
            x: self.q[0] as f64,
            y: self.q[1] as f64,
            z: self.q[2] as f64,
            w: self.q[3] as f64,
        }
        .norm();
        a.omega = vector(self.omega);
        a.throttle = self.systems[0] as f64;
        a.power = self.systems[1] as f64;
        a.fuel = self.systems[2] as f64;
        a.gear = self.systems[3] as f64;
        a.flaps = self.systems[4] as f64;
        a.brake = self.systems[5] as f64;
        a.g_load = self.systems[6] as f64;
        a.alpha_deg = self.systems[7] as f64;
        a.gear_damage = self.systems[8] as f64;
        a.flap_damage = self.systems[9] as f64;
        a.surf_pitch = self.systems[10] as f64;
        a.surf_roll = self.systems[11] as f64;
        a.surf_yaw = self.systems[12] as f64;
        a.on_ground = self.flags & 1 != 0;
        a.ejected = self.flags & 2 != 0;
        a.lost = Breakup {
            wing: [self.flags & 4 != 0, self.flags & 8 != 0],
            tail: self.flags & 16 != 0,
        };
        a.crashed = self.flags & 32 != 0;
        a.gear_broken = [self.flags & 64 != 0, self.flags & 128 != 0, self.flags & 256 != 0];
        for i in 0..3 { if !a.gear_broken[i] { a.gear_shed[i] = false; } }
        a.speed = a.vel.len();
        let atm = crate::air::atmosphere_at(a.pos.y);
        a.mach = a.speed / atm.sound;
        a.airspeed_kt = crate::air::calibrated_airspeed(a.speed, &atm) * crate::air::KT;
        a.agl = a.pos.y - terrain::floor(a.pos.x, a.pos.z);
        a.vs = a.vel.y;
        let axes = a.q.axes();
        a.heading = wrap(axes.forward.x.atan2(-axes.forward.z) * RAD, 360.0);
        a.pitch = axes.forward.y.clamp(-1.0, 1.0).asin() * RAD;
        a.bank = (-axes.right.y).atan2(axes.up.y) * RAD;
        l.gun = gun_of(kind);
        l.initial_rounds = if self.systems[13] >= 1.0 { (self.systems[13] as u32 - 1).min(l.gun.rounds) } else { l.gun.rounds };
        l.rounds = self.rounds.min(l.initial_rounds);
        l.chaff = self.chaff;
        l.flares = self.flares;
        l.stations = self
            .stations
            .iter()
            .map(|&(store, at, loaded, tip)| Station {
                store: STORES[store as usize],
                at: vector(at),
                loaded,
                tip,
            })
            .collect();
        a.stores_mass = l.mass();
        a.wing_stores = l.wing_mass(a.kind);
        a.stores_drag = l.drag();
        a.mass = kind.empty_mass() + a.fuel + a.stores_mass;
    }
    fn valid(&self) -> bool {
        self.kind < Type::ALL.len() as u8
            && self.hp.is_finite()
            && (0.0..=100.0).contains(&self.hp)
            && self.rounds <= gun_of(Type::ALL[self.kind as usize]).rounds
            && self.chaff <= 1000
            && self.flares <= 1000
            && self.flags < 512
            && self.stations.len() <= 24
            && self
                .stations
                .iter()
                .all(|s| s.0 < 15 && s.1.iter().all(|n| n.is_finite() && n.abs() < 30.0))
            && self.pos.iter().all(|n| n.is_finite() && n.abs() < 1e7)
            && self.vel.iter().all(|n| n.is_finite() && n.abs() < 4000.0)
            && self
                .q
                .iter()
                .chain(self.omega.iter())
                .chain(self.systems.iter())
                .all(|n| n.is_finite())
            && self.q.iter().map(|q| q * q).sum::<f32>() > 0.5
            && self.q.iter().map(|q| q * q).sum::<f32>() < 1.5
    }
}

#[cfg(test)]
mod ammo_snapshot_tests {
    use super::*;
    #[test]
    fn partial_and_empty_drums_keep_their_mass_across_snapshot_application() {
        for kind in Type::ALL {for count in [0,50] {
            let mut cfg=Config::for_jet(MissionKind::Free,kind);cfg.gun_rounds=Some(count);
            let a=spawn_type(kind,v3(0.0,4000.0,0.0),0.0,150.0);
            let l=Loadout::configure(&cfg);
            let wire=ActorState::new(0,None,0,&a,&l,100.0,true,Mode::Gun,None,None);
            let mut target=a.clone();let mut stores=Loadout::for_mission(MissionKind::Free);
            wire.apply(&mut target,&mut stores);
            assert_eq!((stores.initial_rounds,stores.rounds),(count,count));
            assert!((stores.mass()-l.mass()).abs()<1e-8);
        }}
    }
}
#[derive(Serialize, Deserialize)]
pub struct Projectile {
    kind: u8,
    spec: u8,
    from: u16,
    target: Option<u16>,
    pos: [f32; 3],
    vel: [f32; 3],
    age: f32,
    tracking: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Frame {
    pub tick: u32,
    pub elapsed: f64,
    pub left: f64,
    pub actors: Vec<ActorState>,
    pub sites: Vec<(f32, bool)>,
    projectiles: Vec<Projectile>,
    pub objective: String,
    pub finished: bool,
    pub scores: Vec<(u64, u8, u32, u32)>,
}
impl Combat {
    pub fn network_frame(
        &self,
        a: &Aircraft,
        m: &Mission,
        tick: u32,
        peers: &[(u64, usize, &RemotePilot)],
        objective: String,
        finished: bool,
    ) -> Frame {
        let mut actors = vec![ActorState::new(
            0,
            Some(0),
            self.player_team,
            a,
            &self.loadout,
            self.hp,
            !self.player_dead,
            self.mode,
            self.lock,
            self.seeker,
        )];
        for (i, b) in self.bandits.iter().enumerate() {
            let remote = peers.iter().find(|(_, j, _)| *j == i);
            let (mode, lock, seeker) = remote
                .map(|(_, _, p)| (p.mode, p.lock, p.seeker))
                .unwrap_or((Mode::Nav, None, None));
            actors.push(ActorState::new(
                i as u16 + 1,
                remote.map(|(id, _, _)| *id),
                b.team,
                &b.jet,
                &b.stores,
                b.hp,
                b.alive,
                mode,
                lock,
                seeker,
            ));
        }
        let mut projectiles = Vec::new();
        for r in self.rounds.iter().filter(|r| r.tracer).take(150) {
            projectiles.push(Projectile {
                kind: 0,
                spec: 0,
                from: actor(r.from),
                target: None,
                pos: array(r.pos),
                vel: array(r.vel),
                age: r.age as f32,
                tracking: false,
            });
        }
        for r in self.missiles.iter().take(64) {
            let spec = STORES
                .iter()
                .position(|s| s.missile().is_some_and(|m| std::ptr::eq(m, r.spec)))
                .unwrap_or(0) as u8;
            projectiles.push(Projectile {
                kind: 1,
                spec,
                from: actor(r.from),
                target: r.target.map(actor),
                pos: array(r.pos),
                vel: array(r.vel),
                age: r.age as f32,
                tracking: r.tracking,
            });
        }
        for r in self.bombs.iter().take(64) {
            let spec = STORES
                .iter()
                .position(|s| s.bomb().is_some_and(|m| std::ptr::eq(m, r.spec)))
                .unwrap_or(5) as u8;
            projectiles.push(Projectile {
                kind: 2,
                spec,
                from: actor(r.from),
                target: None,
                pos: array(r.pos),
                vel: array(r.vel),
                age: r.age as f32,
                tracking: false,
            });
        }
        Frame {
            tick,
            elapsed: m.elapsed,
            left: m.time_left,
            actors,
            sites: self.sites.iter().map(|s| (s.hp as f32, s.dead)).collect(),
            projectiles,
            objective,
            finished,
            scores: Vec::new(),
        }
    }
    pub fn network_apply_frame(
        &mut self,
        a: &mut Aircraft,
        m: &mut Mission,
        frame: &Frame,
        peer: u64,
    ) -> bool {
        if frame.scores.len() > 64
            || frame.sites.iter().any(|(hp, _)| !hp.is_finite())
            || frame.actors.len() > 32
            || frame.sites.len() != self.sites.len()
            || frame.projectiles.len() > 278
            || !frame.elapsed.is_finite()
            || !frame.left.is_finite()
            || frame.objective.len() > 160
            || frame.actors.iter().any(|a| !a.valid())
            || frame
                .actors
                .iter()
                .enumerate()
                .any(|(i, a)| a.id != i as u16)
            || frame.projectiles.iter().any(|p| {
                p.kind > 2
                    || p.spec >= 15
                    || !p.age.is_finite()
                    || p.pos.iter().chain(p.vel.iter()).any(|v| !v.is_finite())
            })
        {
            return false;
        }
        let Some(own) = frame.actors.iter().find(|a| a.peer == Some(peer)) else {
            return false;
        };
        let host_slot = frame.actors.len() - 1;
        while self.bandits.len() < frame.actors.len() {
            let b = bandit(
                spawn_type(Type::F16, v3(0.0, 4000.0, 0.0), 0.0, 200.0),
                0.5,
                CALLSIGNS[0],
            );
            self.bandits.push(b);
        }
        for state in &frame.actors {
            if state.id == own.id {
                if state.hp < self.hp as f32 {
                    self.events.push(Event::PlayerHit {
                        damage: (self.hp - state.hp as f64).max(2.0),
                    });
                }
                self.firing = state.rounds < self.loadout.rounds;
                state.apply(a, &mut self.loadout);
                self.shed_gear(a);
                self.hp = state.hp as f64;
                self.player_dead = !state.alive;
                self.player_team = state.team;
                self.mode = match state.mode {
                    1 => Mode::Gun,
                    2 => Mode::Strf,
                    3 => Mode::Srm,
                    4 => Mode::Mrm,
                    5 => Mode::Ccip,
                    _ => Mode::Nav,
                };
                self.lock = state.lock.map(|w| who(w, own.id, host_slot));
                self.seeker = state.seeker.map(|w| who(w, own.id, host_slot));
                continue;
            }
            let idx = if state.id == 0 {
                host_slot
            } else {
                state.id as usize - 1
            };
            let b = &mut self.bandits[idx];
            if b.alive && !state.alive {
                self.events.push(Event::Airburst {
                    pos: vector(state.pos),
                    vel: vector(state.vel),
                    scale: 1.0,
                });
            }
            state.apply(&mut b.jet, &mut b.stores);
            b.hp = state.hp as f64;
            b.alive = state.alive;
            b.remote = state.peer.is_some();
            b.team = state.team;
        }
        if own.id > 0 {
            self.bandits[own.id as usize - 1].alive = false;
        }
        for (s, (hp, dead)) in self.sites.iter_mut().zip(&frame.sites) {
            if *dead && !s.dead {
                self.events.push(Event::Blast {
                    pos: s.pos,
                    scale: 1.25,
                    burn: 8.0,
                });
            }
            s.hp = *hp as f64;
            s.dead = *dead;
        }
        for t in &mut self.trails {
            t.open = false;
        }
        self.age_trails();
        self.rounds.clear();
        self.missiles.clear();
        self.missile_trails.clear();
        self.bombs.clear();
        for p in &frame.projectiles {
            let from = who(p.from, own.id, host_slot);
            let target = p.target.map(|w| who(w, own.id, host_slot));
            match p.kind {
                0 => {
                    let mut r = Round::new(vector(p.pos), vector(p.vel), true, from, 0.0);
                    r.age = p.age as f64;
                    self.rounds.push(r);
                }
                1 => {
                    if let Some(spec) = STORES[p.spec as usize].missile() {
                        let mut r =
                            Missile::launch(spec, vector(p.pos), vector(p.vel), from, target);
                        r.age = p.age as f64;
                        r.tracking = p.tracking;
                        self.fire_missile(r);
                    }
                }
                2 => {
                    if let Some(spec) = STORES[p.spec as usize].bomb() {
                        self.bombs.push(Bomb {
                            from,
                            spec,
                            pos: vector(p.pos),
                            vel: vector(p.vel),
                            age: p.age as f64,
                        });
                    }
                }
                _ => {}
            }
        }
        m.elapsed = frame.elapsed;
        m.time_left = frame.left;
        self.clock = frame.elapsed;
        self.sight.step(crate::sim::FIXED_DT, a);
        self.ccip = self.station().and_then(|i| {
            self.loadout.stations[i]
                .store
                .bomb()
                .and_then(|s| ccip(s, a, self.loadout.stations[i].at, &terrain::floor))
        });
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_aircraft_collide_but_near_misses_do_not() {
        let mut a = spawn_type(Type::F16, v3(0.0, 4000.0, -20.0), 0.0, 250.0);
        let mut b = spawn_type(Type::Mig29, v3(0.0, 4000.0, 20.0), PI, 250.0);
        a.last_pos = v3(0.0, 4000.0, 20.0);
        b.last_pos = v3(0.0, 4000.0, -20.0);
        assert!(jets_collide(&a, &b));
        b.pos.y += 20.0;
        b.last_pos.y += 20.0;
        assert!(!jets_collide(&a, &b));
    }
    #[test]
    fn remote_weapon_release_restores_host_inventory_and_actor_ownership() {
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Free, &mut m);
        c.network_clear_ai();
        c.networked = true;
        let mut host = spawn_type(Type::F16, v3(0.0, 4000.0, 0.0), 0.0, 200.0);
        let i = c.network_spawn(
            spawn_type(Type::Mig29, v3(500.0, 4000.0, 0.0), 0.0, 200.0),
            1,
            true,
            MissionKind::Dogfight,
        );
        let before = c.loadout.rounds;
        let mut p = RemotePilot {
            mode: Mode::Gun,
            ..Default::default()
        };
        c.network_remote_step(
            i,
            &mut host,
            &Control {
                flags: 64,
                ..Default::default()
            },
            &mut p,
            1.0 / 120.0,
        );
        assert_eq!(c.loadout.rounds, before);
        assert!(c.bandits[i].stores.rounds < 150);
        assert!(c.rounds.iter().all(|r| r.from == Who::Bandit(i)));
        assert_eq!(host.kind, Type::F16);
        assert_eq!(c.bandits[i].team, 1);
    }
}

#[cfg(test)]
mod missile_support_tests {
    use super::*;

    #[test]
    fn remote_launchers_supply_their_own_lock_and_terminal_range_uses_the_actual_target() {
        terrain::init();
        for kind in Type::ALL {
            let specs = if kind.russian() { [&R27R, &R77] } else { [&AIM7M, &AIM120C] };
            for spec in specs {
                let mut c = Combat::new(MissionKind::Dogfight, &mut Mission::new());
                c.network_clear_ai();
                c.networked = true;
                let mut host = spawn_type(Type::F16, v3(70000.0, 6000.0, 0.0), 0.0, 230.0);
                let source = c.network_spawn(spawn_type(kind, v3(0.0, 6000.0, 0.0), 0.0, 230.0),
                    1, true, MissionKind::Dogfight);
                let target = c.network_spawn(spawn_type(Type::F16, v3(0.0, 6000.0, -40000.0), 0.0, 200.0),
                    2, true, MissionKind::Dogfight);
                let mut missile = Missile::launch(spec, c.bandits[source].jet.pos, -V3::Z * 500.0,
                    Who::Bandit(source), Some(Who::Bandit(target)));
                missile.cue(Some((c.bandits[target].jet.pos, c.bandits[target].jet.vel)), false);
                c.fire_missile(missile);
                let mut pilot = RemotePilot { lock: Some(Who::Bandit(target)), ..Default::default() };
                c.network_remote_step(source, &mut host, &Control::default(), &mut pilot, crate::sim::FIXED_DT);
                c.step_missiles(crate::sim::FIXED_DT, &host);
                assert!(c.missiles[0].supported && c.missiles[0].tracking, "{kind:?} {}", spec.name);
                assert!(c.lock.is_none(), "host must not supply the remote player's lock");
                pilot.lock = None;
                c.network_remote_step(source, &mut host, &Control::default(), &mut pilot, crate::sim::FIXED_DT);
                c.step_missiles(crate::sim::FIXED_DT, &host);
                assert!(!c.missiles[0].supported);
                assert_eq!(c.missiles[0].tracking, spec.active());
                c.bandits[target].jet.pos = c.missiles[0].pos - V3::Z * 5000.0;
                c.kill_bandit(source);
                c.step_missiles(crate::sim::FIXED_DT, &host);
                assert_eq!(c.missiles[0].tracking, spec.active(), "terminal acquisition must ignore the distant host");
                c.kill_bandit(target);
                assert!(c.missiles[0].target.is_none() && !c.missiles[0].tracking);
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn snapshot_restores_client_airframe_units_inventory_and_ownership() {
        let mut m = Mission::new();
        let mut host = Combat::new(MissionKind::Dogfight, &mut m);
        host.network_clear_ai();
        let a = spawn_type(Type::F18, v3(1000.0, 4500.0, -2000.0), 1.2, 210.0);
        let remote = host.network_spawn(
            spawn_type(Type::Su27, v3(3000.0, 4800.0, -3000.0), -0.7, 200.0),
            1,
            true,
            MissionKind::Dogfight,
        );
        let pilot = RemotePilot::default();
        let mut frame = host.network_frame(
            &a,
            &m,
            12,
            &[(9, remote, &pilot)],
            "PVP DOGFIGHT".into(),
            false,
        );
        let data = postcard::to_stdvec(&frame).unwrap();
        frame = postcard::from_bytes(&data).unwrap();
        let mut cm = Mission::new();
        let mut client = Combat::new(MissionKind::Dogfight, &mut cm);
        client.network_clear_ai();
        let mut own = spawn_type(Type::F16, v3(0.0, 4500.0, 0.0), 0.0, 200.0);
        assert!(client.network_apply_frame(&mut own, &mut cm, &frame, 9));
        assert_eq!(own.kind, Type::Su27);
        assert!((own.pos - host.bandits[remote].jet.pos).len() < 0.01);
        assert!((own.heading - host.bandits[remote].jet.heading).abs() < 0.01);
        assert_eq!(client.loadout.rounds, host.bandits[remote].stores.rounds);
        assert!(!client.bandits[remote].alive);
        assert_eq!(client.bandits[frame.actors.len() - 1].jet.kind, Type::F18);
        frame.actors[1].q[0] = f32::NAN;
        assert!(!client.network_apply_frame(&mut own, &mut cm, &frame, 9));
    }
    #[test]
    fn remote_rounds_hit_other_remote_players_and_keep_their_source() {
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Dogfight, &mut m);
        c.network_clear_ai();
        c.networked = true;
        let a = spawn_type(Type::F16, v3(20000.0, 4500.0, 0.0), 0.0, 200.0);
        let shooter = c.network_spawn(
            spawn_type(Type::F16, v3(0.0, 4500.0, 0.0), 0.0, 200.0),
            1,
            true,
            MissionKind::Dogfight,
        );
        let target = c.network_spawn(
            spawn_type(Type::Mig29, v3(0.0, 4500.0, -500.0), 0.0, 200.0),
            2,
            true,
            MissionKind::Dogfight,
        );
        let p = c.bandits[target].jet.pos;
        c.rounds.push(Round::of(
            &M61A1,
            p + V3::Z * 20.0,
            -V3::Z * 1000.0,
            true,
            Who::Bandit(shooter),
        ));
        c.step_rounds(0.03, &a);
        assert!(c.bandits[target].hp < 100.0);
        assert_eq!(c.bandits[target].last_hitter, Some(Who::Bandit(shooter)));
        assert_eq!(c.bandits[shooter].hp, 100.0);
    }
    #[test]
    fn untrusted_controls_reject_nonfinite_and_out_of_range_values_without_panics() {
        assert!(Control::default().valid());
        assert!(!Control {
            throttle: i8::MIN,
            ..Default::default()
        }
        .valid());
        assert!(!Control {
            axes: [f32::NAN, 0.0, 0.0],
            ..Default::default()
        }
        .valid());
        assert!(!Control {
            flags: 65535,
            ..Default::default()
        }
        .valid());
        let c = Control {
            axes: [1.0, -1.0, 0.5],
            flags: 1 | 64 | 4,
            actions: [3; 5],
            ..Default::default()
        }
        .idle();
        assert!(!c.trigger());
        assert!(!c.input().afterburner);
        assert!(c.input().gear_down);
        assert_eq!(c.axes, [0.0; 3]);
        assert_eq!(c.actions, [3; 5]);
    }
}

#[cfg(test)]
mod bomb_network_tests {
    use super::*;
    #[test]
    fn remote_bombs_keep_their_shooter_without_spending_host_stores() {
        let mut m = Mission::new();
        let mut c = Combat::new(MissionKind::Free, &mut m);
        c.network_clear_ai();
        let mut host = spawn_type(Type::F16, v3(0.0, 4500.0, 0.0), 0.0, 200.0);
        let i = c.network_spawn(
            spawn_type(Type::F18, v3(500.0, 4500.0, 0.0), 0.0, 200.0),
            1,
            true,
            MissionKind::Strike,
        );
        let before = c.loadout.stations.iter().filter(|s| s.loaded).count();
        let mut pilot = RemotePilot {
            mode: Mode::Ccip,
            ..Default::default()
        };
        c.network_remote_step(
            i,
            &mut host,
            &Control {
                flags: 64,
                ..Default::default()
            },
            &mut pilot,
            1.0 / 120.0,
        );
        assert_eq!(c.bombs.len(), 1);
        assert_eq!(c.bombs[0].from, Who::Bandit(i));
        assert_eq!(
            c.loadout.stations.iter().filter(|s| s.loaded).count(),
            before
        );
    }
}


#[cfg(test)]
mod gear_snapshot_tests {
    use super::*;
    #[test]
    fn individual_missing_legs_survive_actor_roundtrip() {
        for kind in Type::ALL {
            let mut a=spawn_type(kind,v3(0.0,4500.0,0.0),0.0,200.0);
            let mut l=Loadout::configure(&Config::for_jet(MissionKind::Free,kind));
            a.gear_broken=[true,false,true];
            let state=ActorState::new(0,None,0,&a,&l,100.0,true,Mode::Nav,None,None);
            assert!(state.valid());
            let mut b=a.clone();
            b.gear_broken=[false;3];
            b.gear_shed=[true,false,false];
            state.apply(&mut b,&mut l);
            assert_eq!(b.gear_broken,[true,false,true]);
            assert_eq!(b.gear_shed,[true,false,false]);
            let mut malformed=state.clone();
            malformed.rounds=l.gun.rounds+1;
            assert!(!malformed.valid(),"reject ammunition beyond the selected gun capacity");
        }
    }
}
