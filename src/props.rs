//! What the fight looks and sounds like: the stores on the player's jet, the
//! bandits in aggressor paint and their falling wrecks, missiles and bombs in
//! flight and the ground targets, all drawn with the aircraft shader; smoke
//! trails and tracers handed to the effects as extra geometry; and the
//! combat events turned into explosions, sounds and messages.
use crate::airframe::{self, Airframe};
use crate::audio::{Audio, Wave};
use crate::combat::{Combat, Event, Smoke, Store};
use crate::fx::{Fx, Impact};
use crate::math::*;
use crate::mesh::{Mesh, Rgb};
use crate::sim::{Aircraft, Type};
use crate::targets::{self, Kind, KINDS};
use crate::weapons::{Spec, Who, AIM120C, AIM9M, MK82};
use crate::world::{self, Camera, Part, Props, World};
use miniquad::*;

/// Burnt and blackened.
const BURNT: [f32; 4] = [0.20, 0.18, 0.16, 2.0];
const NONE: [f32; 4] = [0.0; 4];

pub struct Models {
    tips: [Part; 2],
    aim9: Part,
    aim120: Part,
    mk82: Part,
    sites: Vec<Part>,
    ruin: Part,
    /// Ejection, per jet (F-16, MiG-29): the seat (the K-36DM's with its
    /// booms out), the pilot seated and hanging, the main canopy and the
    /// cockpit canopy blown off.
    seat: [Part; 4],
    pilot: [[Part; 2]; 4],
    seated: [[[Part; 2]; crate::eject::HANDLE_FRAMES]; 4],
    handle: [[Part; crate::eject::HANDLE_FRAMES]; 4],
    chute: [Part; 4],
    canopy: [Part; 4],
}

/// AIM-120 AMRAAM, 3.66 m and 178 mm, nose at the origin pointing forward:
/// radome, mid-body wings and tail fins.
fn aim120() -> Mesh {
    let mut m = Mesh::new();
    let white = Rgb(0.84, 0.84, 0.80);
    m.lathe(V3::ZERO, V3::Z, &[(0.0, 0.0), (0.12, 0.05), (0.42, 0.089), (3.62, 0.089), (3.66, 0.0)], 6, white);
    m.lathe(V3::ZERO, V3::Z, &[(-0.001, 0.0), (0.10, 0.045), (0.11, 0.0)], 6, Rgb(0.55, 0.53, 0.48));
    for f in 0..4 {
        let a = PI / 4.0 + f as f64 * PI / 2.0;
        let d = v3(a.cos(), a.sin(), 0.0);
        let c = d * 0.089;
        m.quad(c + V3::Z * 1.30, c + V3::Z * 1.75, c + d * 0.20 + V3::Z * 1.75, c + d * 0.20 + V3::Z * 1.62, white.scale(0.9));
        m.quad(c + V3::Z * 3.20, c + V3::Z * 3.64, c + d * 0.22 + V3::Z * 3.64, c + d * 0.22 + V3::Z * 3.50, white.scale(0.9));
    }
    m
}

/// Mk-82 low-drag bomb, 2.21 m and 273 mm, centred on the origin, nose
/// forward: olive body with a yellow nose band and conical tail fins.
fn mk82() -> Mesh {
    let mut m = Mesh::new();
    let olive = Rgb(0.30, 0.34, 0.22);
    m.lathe(v3(0.0, 0.0, -1.10), V3::Z, &[(0.0, 0.0), (0.08, 0.05), (0.45, 0.125), (0.75, 0.137), (1.45, 0.137), (1.80, 0.10), (1.90, 0.0)], 8, olive);
    m.lathe(v3(0.0, 0.0, -1.10), V3::Z, &[(0.30, 0.118), (0.38, 0.126)], 8, Rgb(0.85, 0.72, 0.15));
    for f in 0..4 {
        let a = PI / 4.0 + f as f64 * PI / 2.0;
        let d = v3(a.cos(), a.sin(), 0.0);
        m.quad(d * 0.09 + V3::Z * 0.62, d * 0.09 + V3::Z * 1.11, d * 0.19 + V3::Z * 1.11, d * 0.17 + V3::Z * 0.70, olive.scale(0.85));
    }
    m
}

impl Models {
    pub fn new(ctx: &mut dyn RenderingBackend, frame: &Airframe, world: &World) -> Models {
        let skin = world.skin();
        let mut part = |m: &Mesh| {
            let mut p = world::part(ctx, m);
            p.bind.images = vec![skin];
            p
        };
        let mut a9 = Mesh::new();
        airframe::aim9(&mut a9, V3::ZERO);
        let sites = KINDS.iter().map(|&k| part(&targets::mesh(k))).collect();
        use crate::eject;
        let mig = crate::mig29::build();
        let seat=Type::ALL.map(|kind|part(&eject::seat_mesh(kind,kind.russian())));
        let pilot = std::array::from_fn(|k| std::array::from_fn(|fp|
            part(&eject::pilot_mesh(Type::ALL[k], true, fp != 0, 1.0))));
        let seated = std::array::from_fn(|k| std::array::from_fn(|frame| std::array::from_fn(|fp|
            part(&eject::pilot_mesh(Type::ALL[k], false, fp != 0,
                frame as f64 / (eject::HANDLE_FRAMES - 1) as f64)))));
        let handle = std::array::from_fn(|k| std::array::from_fn(|frame| part(&eject::handle_mesh(
            Type::ALL[k], frame as f64 / (eject::HANDLE_FRAMES - 1) as f64))));
        let chute=Type::ALL.map(|kind|part(&eject::chute_mesh(eject::spec(kind))));
        let canopy=[part(&frame.canopy),part(&mig.canopy),part(&crate::hornet::build().canopy),part(&crate::su27::build().canopy)];
        Models { tips: [part(&frame.tips[0]), part(&frame.tips[1])], aim9: part(&a9), aim120: part(&aim120()), mk82: part(&mk82()), sites, ruin: part(&targets::bridge_ruin()), seat, pilot, seated, handle, chute, canopy }
    }
}

/// The combat scene for one frame, drawn inside the world pass.
pub struct Scene<'a> {
    pub models: &'a Models,
    pub combat: &'a Combat,
    /// The player's jet, while it is drawn.
    pub player: Option<&'a Aircraft>,
    /// The player's ejection, once the handle is pulled.
    pub ejection: Option<&'a crate::eject::Ejection>,
    pub first_person: bool,
}

/// Model transform of a body moving along `dir` at `pos`, relative to the eye.
fn along(dir: V3, pos: V3, eye: V3) -> M4 {
    let ax = Quat::from_forward_up(dir, V3::Y).axes();
    basis(ax.right, ax.up, ax.back, pos - eye)
}

/// Stretches a store's model across (its right and up columns) and along
/// (its back column): the R-73, R-27R and FAB bombs drawn as the Sidewinder,
/// AMRAAM and Mk-82 shapes at their own diameters and lengths.
fn stretch(mut model: M4, across: f32, along: f32) -> M4 {
    for (i, v) in model.iter_mut().take(11).enumerate() {
        *v *= if i < 8 { across } else { along };
    }
    model
}

/// The shape a missile is drawn with and how it is stretched onto it: the
/// Sidewinder's for the slim dogfight missiles, the AMRAAM's for the long
/// medium-range ones, scaled to the real missile's size.
fn missile_shape<'m>(m: &'m Models, spec: &Spec) -> (&'m Part, f32, f32) {
    let base = if spec.length > 3.3 { &AIM120C } else { &AIM9M };
    let part = if spec.length > 3.3 { &m.aim120 } else { &m.aim9 };
    (part, (spec.diameter / base.diameter) as f32, (spec.length / base.length) as f32)
}

/// The shape a store is drawn with and how it is stretched onto it.
fn store_shape<'m>(m: &'m Models, store: Store) -> (&'m Part, f32, f32) {
    match (store.missile(), store.bomb()) {
        (Some(spec), _) => missile_shape(m, spec),
        (None, Some(b)) => (&m.mk82, (b.diameter / MK82.diameter) as f32, (b.length / MK82.length) as f32),
        _ => (&m.mk82, 1.0, 1.0),
    }
}

fn jet_model(a: &Aircraft, at: V3, eye: V3) -> M4 {
    let ax = a.q.axes();
    basis(ax.right, ax.up, ax.back, a.pos + a.q.rotate(at) - eye)
}

impl Props for Scene<'_> {
    fn first_person(&self) -> bool { self.first_person }

    fn draw(&self, w: &World, ctx: &mut dyn RenderingBackend, vp: M4, cam: &Camera) {
        let (m, c, eye) = (self.models, self.combat, cam.eye);
        // The stations sit where the F-16 hangs them; a MiG carries them on
        // its own pylons. Either way they ride the wing as it bends.
        let shift = |a: &Aircraft, at: V3| {
            let d = a.kind.store_shift(at);
            d + V3::Y * a.flex_lift(at + d)
        };
        // What hung under a wing that broke off went with it.
        let kept = |a: &Aircraft, at: V3| !a.lost.wing[(at.x > 0.0) as usize] || at.x.abs() < a.kind.break_station();
        if let Some(a) = self.player {
            for (i, st) in c.loadout.stations.iter().enumerate().filter(|(_, s)| s.loaded) {
                if !kept(a, st.at) {
                    continue;
                }
                if st.tip && a.kind==Type::F16 {
                    w.draw_prop(ctx, vp, &m.tips[i.min(1)], jet_model(a, shift(a, st.at), eye), NONE);
                    continue;
                }
                let (part, across, along) = store_shape(m, st.store);
                w.draw_prop(ctx, vp, part, stretch(jet_model(a, st.at + shift(a, st.at), eye), across, along), NONE);
            }
        }
        for (i, b) in c.bandits.iter().enumerate().filter(|(_, b)| b.alive) {
            if (b.jet.pos - eye).len() > 30000.0 {
                continue;
            }
            // Aggressor paint: the F-16s take the 64th's schemes in turn.
            w.draw_jet(ctx, vp, cam, &b.jet, [i as f32, 0.0, 0.0, 1.0]);
            for st in b.stores.stations.iter().filter(|s| s.loaded && kept(&b.jet, s.at)) {
                let (part, across, along) = store_shape(m, st.store);
                w.draw_prop(ctx, vp, part, stretch(jet_model(&b.jet, st.at + shift(&b.jet, st.at), eye), across, along), NONE);
            }
        }
        for wr in &c.wrecks {
            w.draw_jet(ctx, vp, cam, &wr.jet, BURNT);
        }
        if let Some(e) = self.ejection {
            use crate::eject::Phase;
            let k=e.kind.index();
            let at = |b: &crate::eject::Body| {
                let ax = b.q.axes();
                basis(ax.right, ax.up, ax.back, b.pos - eye)
            };
            w.draw_prop(ctx, vp, &m.canopy[k], at(&e.canopy), NONE);
            let hanging = matches!(e.phase, Phase::Chute | Phase::Landed);
            let frame = (e.handle_pull() * (crate::eject::HANDLE_FRAMES - 1) as f64).round() as usize;
            let pilot = if hanging { &m.pilot[k][self.first_person as usize] } else { &m.seated[k][frame][self.first_person as usize] };
            if e.phase != Phase::Canopy || !self.first_person {
                w.draw_prop(ctx, vp, pilot, at(&e.pilot), NONE);
            }
            match &e.seat {
                Some(seat) => {
                    w.draw_prop(ctx, vp, &m.seat[k], at(seat), NONE);
                    w.draw_prop(ctx, vp, &m.handle[k][crate::eject::HANDLE_FRAMES - 1], at(seat), NONE);
                }
                None if e.phase != Phase::Canopy || !self.first_person => {
                    w.draw_prop(ctx, vp, &m.seat[k], at(&e.pilot), NONE);
                    w.draw_prop(ctx, vp, &m.handle[k][frame], at(&e.pilot), NONE);
                }
                None => {}
            }
            if e.chute > 0.01 {
                // Streaming out of its pack and filling: long and thin first,
                // then the full spread.
                let (right, up, back, origin) = e.chute_frame();
                let model = basis(right, up, back, origin - eye);
                w.draw_prop(ctx, vp, &m.chute[k], model, NONE);
            }
        }
        for d in &c.debris {
            w.draw_debris(ctx, vp, cam, d.kind, d.piece, d.pos, d.q, if d.burnt { BURNT } else { NONE });
        }
        for ms in &c.missiles {
            if (ms.pos - eye).len() < 9000.0 && ms.vel.len() > 1.0 {
                // Each missile in its own size; the SA-6 as the AMRAAM shape
                // scaled up evenly.
                let model = along(ms.vel, ms.pos, eye);
                let (part, across, length) = missile_shape(m, ms.spec);
                let model = if ms.spec.name == "3M9" { stretch(model, length, length) } else { stretch(model, across, length) };
                w.draw_prop(ctx, vp, part, model, NONE);
            }
        }
        for b in &c.bombs {
            if (b.pos - eye).len() < 8000.0 {
                let s = b.spec;
                w.draw_prop(ctx, vp, &m.mk82, stretch(along(b.vel, b.pos, eye), (s.diameter / MK82.diameter) as f32, (s.length / MK82.length) as f32), NONE);
            }
        }
        for s in &c.sites {
            if (s.pos - eye).len() > 16000.0 {
                continue;
            }
            let rot = s.rot();
            let (r, u, b) = (rot.rotate(V3::X), rot.rotate(V3::Y), rot.rotate(V3::Z));
            let k = KINDS.iter().position(|&k| k == s.kind).unwrap_or(0);
            match (s.dead, s.kind) {
                (false, _) => w.draw_prop(ctx, vp, &m.sites[k], basis(r, u, b, s.pos - eye), NONE),
                (true, Kind::Bridge) => w.draw_prop(ctx, vp, &m.ruin, basis(r, u, b, s.pos - eye), NONE),
                // A burnt-out hulk, slumped on its wheels or tracks.
                (true, _) => w.draw_prop(ctx, vp, &m.sites[k], basis(r, u * 0.55, b, s.pos - eye), BURNT),
            }
        }
    }
}

fn vert(data: &mut Vec<f32>, p: V3, c: [f64; 3], a: f64) {
    data.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32, c[0] as f32, c[1] as f32, c[2] as f32, a.clamp(0.0, 1.0) as f32]);
}

/// Camera-facing strip from `p0` to `p1`, `w0` and `w1` wide.
#[allow(clippy::too_many_arguments)]
fn strip(data: &mut Vec<f32>, eye: V3, p0: V3, p1: V3, w0: f64, w1: f64, c: [f64; 3], a0: f64, a1: f64) {
    let (q0, q1) = (p0 - eye, p1 - eye);
    let side = (q1 - q0).cross(q0).norm();
    if side.len() < 0.5 {
        return;
    }
    let (s0, s1) = (side * (w0 * 0.5), side * (w1 * 0.5));
    for (p, a) in [(q0 - s0, a0), (q0 + s0, a0), (q1 + s1, a1), (q0 - s0, a0), (q1 + s1, a1), (q1 - s1, a1)] {
        vert(data, p, c, a);
    }
}

/// Hands this frame's smoke trails, tracers, motor flames and the flames
/// of burning jets to the effects.
pub fn fill_fx(c: &Combat, fx: &mut Fx, cam: &Camera, player: Option<&Aircraft>) {
    let eye = cam.eye;
    fx.extra_blend.clear();
    fx.extra_glow.clear();
    let now = c.clock;
    let sun = crate::world::sun_dir();
    for t in &c.trails {
        let (col, alpha, w0, w1) = match t.kind {
            Smoke::Motor => ([0.88, 0.88, 0.86], 0.55, 0.5, 5.0),
            Smoke::Damage => ([0.40, 0.40, 0.41], 0.6, 1.2, 7.0),
            Smoke::Fire => ([0.10, 0.09, 0.08], 0.75, 2.0, 10.0),
            Smoke::Fuel => ([0.92, 0.93, 0.95], 0.35, 0.4, 3.0),
        };
        let life = t.kind.life();
        let pts: Vec<(V3, f64)> = t.points.iter().copied().collect();
        for s in pts.windows(2) {
            let ((p0, t0), (p1, t1)) = (s[0], s[1]);
            if (p0 - eye).len() > 14000.0 || (p1 - p0).len() > 80.0 {
                continue;
            }
            let (g0, g1) = (((now - t0) / life).clamp(0.0, 1.0), ((now - t1) / life).clamp(0.0, 1.0));
            // Lit a little on the sunny side, like the particle smoke.
            let lit = 0.85 + 0.15 * (p1 - p0).norm().cross(V3::Y).dot(sun).abs();
            let shade = [col[0] * lit, col[1] * lit, col[2] * lit];
            strip(&mut fx.extra_blend, eye, p0, p1, mix(w0, w1, g0.sqrt()), mix(w0, w1, g1.sqrt()), shade, alpha * (1.0 - g0).powf(1.3), alpha * (1.0 - g1).powf(1.3));
        }
    }
    for r in c.rounds.iter().filter(|r| r.tracer) {
        let d = (r.pos - eye).len();
        if d > 6000.0 || r.age < 0.02 {
            continue;
        }
        // A saturated core that reads against the bright sky, and a glow over it.
        let (core, glow) = if matches!(r.from, Who::Ground(_)) { ([1.0, 0.30, 0.12], [1.0, 0.5, 0.3]) } else { ([1.0, 0.62, 0.15], [1.0, 0.9, 0.55]) };
        // A short streak that never reaches back past the muzzle: a fresh
        // round has only flown its age, less what the jet flew beside it.
        let tail = r.pos - r.vel * (0.6 * r.age).min(0.012);
        let w = (d * 0.0015).max(0.12);
        strip(&mut fx.extra_blend, eye, tail, r.pos, w * 0.5, w, core, 0.5, 1.0);
        strip(&mut fx.extra_glow, eye, tail, r.pos, w * 0.3, w * 0.6, glow, 0.2, 0.9);
    }
    // A fire licks back from the burning system into the slipstream,
    // flickering and growing as it burns.
    let burning = player.filter(|_| !c.player_dead).map(|a| (a, &c.hull)).into_iter().chain(c.bandits.iter().filter(|b| b.alive).map(|b| (&b.jet, &b.hull)));
    for (jet, hull) in burning {
        let Some((i, t)) = hull.fire else { continue };
        let m = &crate::damage::layout(hull.kind)[i];
        let at = jet.pos + jet.q.rotate(m.at + V3::Z * m.half.z);
        if (at - eye).len() > 6000.0 {
            continue;
        }
        let flicker = 0.75 + 0.25 * (now * 23.0 + i as f64).sin() * (now * 7.3).cos();
        let back = if jet.vel.len() > 1.0 { -jet.vel.norm() } else { V3::Y };
        let len = (2.5 + t.min(20.0) * 0.25) * flicker;
        strip(&mut fx.extra_glow, eye, at, at + back * len, 1.1, 0.3, [1.0, 0.55, 0.15], 1.0, 0.2);
        strip(&mut fx.extra_glow, eye, at, at + back * (len * 0.5), 0.5, 0.15, [1.0, 0.9, 0.5], 1.0, 0.4);
    }
    for m in c.missiles.iter().filter(|m| m.burning()) {
        let back = m.vel.norm() * -1.0;
        let tail = m.pos + back * m.spec.length;
        let d = (tail - eye).len();
        let w = (d * 0.002).max(0.5) * if m.spec.length > 4.0 { 2.0 } else { 1.0 };
        strip(&mut fx.extra_glow, eye, tail, tail + back * (w * 3.0), w, w * 0.3, [1.0, 0.75, 0.40], 1.0, 0.1);
    }
}

/// What the game loop should show after the fight's events.
#[derive(Default)]
pub struct Feedback {
    pub toast: Option<String>,
    pub flash: Option<[u8; 3]>,
    pub kick: f64,
}

/// Turns the combat events into explosions, sounds and messages. `ear` is
/// where the listener is; distant blasts are quieter and arrive later.
pub fn present(c: &mut Combat, fx: &mut Fx, audio: &Audio, ear: V3, muted: bool) -> Feedback {
    let mut out = Feedback::default();
    let volume = |p: V3| clamp(1.0 - (p - ear).len() / 7000.0, 0.0, 1.0) as f32;
    let delay = |p: V3| ((p - ear).len() / 343.0).min(4.0) as f32;
    for e in c.events.drain(..) {
        match e {
            Event::Launch { pos, from, name } => {
                if !muted {
                    audio.burst(1.4, 0.9 * volume(pos).max(0.1), 3200.0, 400.0, 0.0);
                }
                if from == Who::Player {
                    out.toast = Some(name.to_string());
                } else {
                    out.toast = Some(name.to_string());
                    if !muted {
                        for k in 0..4 {
                            audio.tone(1400.0, 0.08, 0.04, Wave::Square, k as f32 * 0.16);
                        }
                    }
                }
            }
            Event::Release { pos } => {
                if !muted {
                    audio.burst(0.18, volume(pos), 600.0, 120.0, 0.0);
                }
            }
            Event::Hit { pos } => fx.impact(pos, Impact::Metal),
            Event::Dirt { pos } => fx.impact(pos, Impact::Dirt),
            Event::Splash { pos } => fx.impact(pos, Impact::Water),
            Event::Airburst { pos, vel, scale } => {
                fx.airburst(pos, vel, scale);
                if !muted {
                    audio.burst(1.2, 3.0 * volume(pos) * scale as f32 + 0.05, 900.0, 60.0, delay(pos));
                }
            }
            Event::Blast { pos, scale, burn } => {
                fx.blast(pos, scale, burn);
                if !muted {
                    audio.burst(1.6, 4.0 * volume(pos) * scale as f32 + 0.05, 500.0, 35.0, delay(pos));
                }
            }
            Event::Dud { pos } => {
                fx.impact(pos, Impact::Dirt);
                out.toast = Some("DUD // FUZE NOT ARMED".to_string());
            }
            Event::Kill { text } => out.toast = Some(text),
            Event::Lock { text } | Event::Toast(text) => out.toast = Some(text),
            Event::Decoy { pos, vel } => {
                fx.decoy(pos, vel);
                if !muted && (pos - ear).len() < 100.0 {
                    audio.burst(0.12, 0.8, 2400.0, 600.0, 0.0);
                }
            }
            Event::PlayerHit { damage } => {
                out.kick = out.kick.max(clamp(damage / 40.0, 0.15, 1.0));
                out.flash = Some([255, 90, 50]);
                if !muted {
                    audio.burst(0.3, 2.5, 1800.0, 200.0, 0.0);
                }
            }
        }
    }
    out
}

/// What the RWR last told the pilot, so it only speaks up on a change.
#[derive(Default)]
pub struct RwrVoice {
    seen: usize,
    hot: usize,
    clock: f64,
}

/// RWR tones: a ping for a new emitter, a quick two-tone when one locks on,
/// and a fast warble for as long as a missile guides on the jet.
pub fn rwr_tones(c: &Combat, a: &Aircraft, audio: &Audio, v: &mut RwrVoice, dt: f64) {
    let e = c.emitters(a);
    let hot = e.iter().filter(|t| t.2).count();
    if e.len() > v.seen {
        audio.tone(820.0, 0.07, 0.03, Wave::Square, 0.0);
    }
    if hot > v.hot {
        audio.tone(1250.0, 0.06, 0.03, Wave::Square, 0.0);
        audio.tone(950.0, 0.06, 0.03, Wave::Square, 0.08);
        audio.tone(1250.0, 0.06, 0.03, Wave::Square, 0.16);
    }
    v.seen = e.len();
    v.hot = hot;
    v.clock -= dt;
    if c.inbound().next().is_some() && v.clock <= 0.0 {
        v.clock = 0.45;
        for k in 0..3 {
            audio.tone(if k % 2 == 0 { 1500.0 } else { 1100.0 }, 0.07, 0.035, Wave::Square, k as f32 * 0.08);
        }
    }
}
