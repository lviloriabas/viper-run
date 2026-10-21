//! Low-poly effects drawn in the world pass: the afterburner plume with its
//! shock diamonds, and the crash fireball, debris, sparks, smoke and spray.
//! Every puff is a handful of flat-shaded facets, like the rest of the art.
use crate::gfx;
use crate::math::*;
use crate::sim::Aircraft;
use crate::terrain;
use crate::world::{sun_dir, Camera, FOG};
use miniquad::*;

const VS_FX: &str = r#"
in vec3 pos;
in vec4 color;
uniform mat4 u_vp;
uniform vec3 u_fog;
out vec4 v_color;
out vec3 v_rel;
void main() {
    v_color = color;
    v_rel = pos;
    gl_Position = u_vp * vec4(pos, 1.0);
}"#;

const FS_FX: &str = r#"
in vec4 v_color;
in vec3 v_rel;
uniform mat4 u_vp;
uniform vec3 u_fog;
out vec4 frag;
void main() {
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.000045), 0.0, 0.9);
    frag = vec4(mix(v_color.rgb, u_fog, fog), v_color.a * (1.0 - fog * 0.8));
}"#;

const MAX_PARTICLES: usize = 720;
const MAX_VERTS: usize = 36000;
const FLOATS: usize = 7;
/// Nozzle exit in body axes (x right, y up, z aft).
const NOZZLE: V3 = v3(0.0, 0.02, 7.30);
/// Drift of smoke columns.
const WIND: V3 = v3(3.2, 0.0, -1.6);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Self-lit, drawn additively.
    Fire,
    Spark,
    /// Sun-lit, alpha-blended.
    Smoke,
    Spray,
    Debris,
}

#[derive(Clone, Copy)]
struct Particle {
    kind: Kind,
    pos: V3,
    vel: V3,
    axis: V3,
    angle: f64,
    spin: f64,
    /// Negative while the particle waits to appear.
    age: f64,
    life: f64,
    size: (f64, f64),
    color: ([f64; 3], [f64; 3]),
    alpha: f64,
    drag: f64,
    /// Downward acceleration; negative rises like hot gas.
    gravity: f64,
    resting: bool,
    /// Seconds between smoke puffs left behind a burning fragment; zero for none.
    trail: f64,
    trail_clock: f64,
}

const BASE: Particle = Particle {
    kind: Kind::Smoke,
    pos: V3::ZERO,
    vel: V3::ZERO,
    axis: V3::Y,
    angle: 0.0,
    spin: 0.0,
    age: 0.0,
    life: 1.0,
    size: (1.0, 1.0),
    color: ([1.0; 3], [1.0; 3]),
    alpha: 1.0,
    drag: 0.0,
    gravity: 0.0,
    resting: false,
    trail: 0.0,
    trail_clock: 0.0,
};

/// Keeps a wreck burning and smoking, or a splash steaming, after the impact.
struct Emitter {
    pos: V3,
    age: f64,
    life: f64,
    water: bool,
    clock: f64,
}

#[repr(C)]
struct FxU {
    vp: M4,
    fog: [f32; 3],
}

pub struct Fx {
    parts: Vec<Particle>,
    emitters: Vec<Emitter>,
    rnd: Lcg,
    blend: Pipeline,
    add: Pipeline,
    bind: Bindings,
    data: Vec<f32>,
    order: Vec<(f64, usize)>,
    /// Vertex ranges of the blended particles, plume core, plume sheath and glowing particles.
    ranges: [(usize, usize); 4],
}

impl Fx {
    pub fn new(ctx: &mut dyn RenderingBackend) -> Fx {
        let s = gfx::shader(ctx, VS_FX, FS_FX, &[], &[("u_vp", UniformType::Mat4), ("u_fog", UniformType::Float3)]);
        let attrs = [VertexAttribute::new("pos", VertexFormat::Float3), VertexAttribute::new("color", VertexFormat::Float4)];
        // Depth writes are on only so miniquad keeps the depth test; draw()
        // masks them off. Particles are sorted back to front instead.
        let params = |blend| PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(blend), ..Default::default() };
        let blend = ctx.new_pipeline(&[BufferLayout::default()], &attrs, s, params(gfx::alpha_blend()));
        let add = ctx.new_pipeline(&[BufferLayout::default()], &attrs, s, params(gfx::additive()));
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(MAX_VERTS * FLOATS));
        let bind = Bindings { vertex_buffers: vec![vb], index_buffer: gfx::sequential_ib(ctx, MAX_VERTS), images: vec![] };
        Fx { parts: Vec::new(), emitters: Vec::new(), rnd: Lcg(7919), blend, add, bind, data: Vec::with_capacity(MAX_VERTS * FLOATS), order: Vec::new(), ranges: [(0, 0); 4] }
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.emitters.clear();
    }

    fn r(&mut self, a: f64, b: f64) -> f64 {
        mix(a, b, self.rnd.next())
    }

    fn dir(&mut self) -> V3 {
        let y = self.r(-1.0, 1.0);
        let t = self.r(0.0, TAU);
        let s = (1.0 - y * y).sqrt();
        v3(s * t.cos(), y, s * t.sin())
    }

    fn push(&mut self, mut p: Particle) {
        if self.parts.len() < MAX_PARTICLES {
            p.axis = self.dir();
            p.angle = self.r(0.0, TAU);
            self.parts.push(p);
        }
    }

    fn fire(&mut self, at: V3, carry: V3, scale: f64, delay: f64) {
        let d = self.dir();
        let p = Particle {
            kind: Kind::Fire,
            pos: at + d * (self.r(0.0, 3.5) * scale),
            vel: carry + d * (self.r(6.0, 20.0) * scale) + V3::Y * self.r(3.0, 11.0),
            age: -delay,
            life: self.r(0.8, 1.7),
            size: (self.r(2.0, 3.5) * scale, self.r(7.0, 13.0) * scale),
            color: ([1.0, 0.95, 0.70], [0.60, 0.12, 0.03]),
            drag: 2.2,
            gravity: -4.0,
            spin: self.r(-1.5, 1.5),
            ..BASE
        };
        self.push(p);
    }

    fn smoke(&mut self, at: V3, vel: V3, size: (f64, f64), life: f64, shade: (f64, f64), alpha: f64, delay: f64) {
        let p = Particle {
            pos: at,
            vel,
            age: -delay,
            life,
            size,
            color: ([shade.0; 3], [shade.1, shade.1 * 0.97, shade.1 * 0.92]),
            alpha,
            drag: 0.7,
            gravity: -1.2,
            spin: self.r(-0.6, 0.6),
            ..BASE
        };
        self.push(p);
    }

    /// The impact: fireball, fragments, sparks and a dust ring on land; a spray
    /// column and steam on water. Its size follows the impact speed.
    pub fn explode(&mut self, a: &Aircraft) {
        let pos = a.pos;
        let along = if a.vel.len() > 1e-6 { a.vel.norm() } else { a.q.axes().forward };
        // What is left of the momentum skids along the ground and bounces up.
        let carry = v3(along.x, along.y.abs() * 0.35, along.z) * (a.impact_speed * 0.22);
        let energy = smoothstep(20.0, 260.0, a.impact_speed);
        let scale = 0.65 + 0.55 * energy;
        if a.impact_water {
            for _ in 0..46 {
                let d = self.dir();
                let up = self.r(22.0, 58.0) * scale;
                let p = Particle {
                    kind: Kind::Spray,
                    pos: pos + v3(d.x, 0.0, d.z) * self.r(0.0, 4.0),
                    vel: carry * 0.6 + v3(d.x, 0.0, d.z) * self.r(4.0, 14.0) + V3::Y * up,
                    life: self.r(1.6, 2.8),
                    size: (self.r(1.2, 2.2), self.r(4.0, 7.5) * scale),
                    color: ([0.94, 0.97, 1.0], [0.80, 0.86, 0.88]),
                    alpha: 0.85,
                    drag: 0.5,
                    gravity: 9.8,
                    ..BASE
                };
                self.push(p);
            }
            for k in 0..20 {
                let t = k as f64 / 20.0 * TAU;
                let out = v3(t.cos(), 0.0, t.sin());
                let p = Particle {
                    kind: Kind::Spray,
                    pos: pos + out * 3.0,
                    vel: out * self.r(16.0, 28.0) * scale + V3::Y * self.r(5.0, 10.0),
                    life: self.r(1.0, 1.6),
                    size: (2.0, self.r(6.0, 9.0) * scale),
                    color: ([0.96, 0.98, 1.0], [0.84, 0.88, 0.9]),
                    alpha: 0.8,
                    drag: 1.4,
                    gravity: 9.8,
                    ..BASE
                };
                self.push(p);
            }
            self.debris(pos, carry, 10, scale, false);
            self.emitters.push(Emitter { pos, age: 0.0, life: 4.0, water: true, clock: 0.0 });
            return;
        }
        for _ in 0..16 {
            self.fire(pos, carry, scale, 0.0);
        }
        // Secondary blasts as fuel and fragments go up along the skid.
        for k in 0..3 {
            let at = pos + v3(along.x, 0.0, along.z) * (self.r(8.0, 30.0) * scale * (k + 1) as f64) + self.dir() * 4.0;
            let delay = self.r(0.25, 0.5) + k as f64 * self.r(0.25, 0.45);
            for _ in 0..5 {
                self.fire(at, carry * 0.3, scale * 0.7, delay);
            }
        }
        self.debris(pos, carry, 30, scale, true);
        for _ in 0..42 {
            let d = self.dir();
            let p = Particle {
                kind: Kind::Spark,
                pos,
                vel: carry * 1.5 + d * self.r(28.0, 85.0) + V3::Y * self.r(8.0, 30.0),
                life: self.r(0.35, 0.95),
                size: (0.22, 0.12),
                color: ([1.0, 0.92, 0.55], [1.0, 0.38, 0.08]),
                drag: 0.4,
                gravity: 9.8,
                ..BASE
            };
            self.push(p);
        }
        // Dust thrown out low along the ground.
        for k in 0..14 {
            let t = k as f64 / 14.0 * TAU + self.r(-0.2, 0.2);
            let out = v3(t.cos(), 0.0, t.sin());
            let vel = out * self.r(14.0, 26.0) * scale + V3::Y * self.r(1.0, 4.0);
            let life = self.r(2.2, 3.4);
            let grown = self.r(5.0, 8.0) * scale;
            self.smoke(pos + out * 2.0, vel, (2.0, grown), life, (0.52, 0.47), 0.55, 0.0);
        }
        self.emitters.push(Emitter { pos, age: 0.0, life: 16.0, water: false, clock: 0.0 });
    }

    fn debris(&mut self, pos: V3, carry: V3, count: usize, scale: f64, burning: bool) {
        for k in 0..count {
            let d = self.dir();
            let grey = self.r(0.26, 0.5);
            let p = Particle {
                kind: Kind::Debris,
                pos: pos + d * self.r(0.0, 3.0),
                vel: carry * self.r(0.6, 1.4) + d * (self.r(12.0, 42.0) * scale) + V3::Y * self.r(10.0, 32.0) * scale,
                life: self.r(5.0, 8.0),
                size: (self.r(0.6, 2.2), 0.0),
                color: ([grey, grey * 1.04, grey * 1.08], [0.08, 0.075, 0.07]),
                drag: 0.25,
                gravity: 9.8,
                spin: self.r(4.0, 13.0),
                trail: if burning && k % 5 == 0 { 0.07 } else { 0.0 },
                ..BASE
            };
            self.push(p);
        }
    }

    /// Torching puff at the nozzle as the afterburner lights.
    pub fn ab_light(&mut self, a: &Aircraft) {
        let ax = a.q.axes();
        let nozzle = a.pos + ax.right * NOZZLE.x + ax.up * NOZZLE.y + ax.back * NOZZLE.z;
        for _ in 0..8 {
            let d = self.dir();
            let p = Particle {
                kind: Kind::Fire,
                pos: nozzle + ax.back * self.r(0.5, 2.5),
                vel: a.vel + ax.back * self.r(8.0, 26.0) + d * 3.0,
                life: self.r(0.22, 0.42),
                size: (0.5, self.r(1.6, 2.4)),
                color: ([1.0, 0.78, 0.45], [1.0, 0.38, 0.1]),
                spin: self.r(-3.0, 3.0),
                ..BASE
            };
            self.push(p);
        }
    }

    pub fn update(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let mut puffs: Vec<V3> = Vec::new();
        for p in self.parts.iter_mut() {
            p.age += dt;
            if p.age < 0.0 || p.resting {
                continue;
            }
            p.vel += V3::Y * (-p.gravity * dt);
            p.vel *= (-p.drag * dt).exp();
            if matches!(p.kind, Kind::Smoke | Kind::Fire) {
                p.vel += WIND * (p.drag * dt);
            }
            p.pos += p.vel * dt;
            p.angle += p.spin * dt;
            if p.trail > 0.0 {
                p.trail_clock += dt;
                if p.trail_clock >= p.trail && p.age < p.life * 0.6 {
                    p.trail_clock = 0.0;
                    puffs.push(p.pos);
                }
            }
            let floor = terrain::floor(p.pos.x, p.pos.z);
            if p.pos.y < floor + 0.05 {
                p.pos.y = floor + 0.05;
                match p.kind {
                    Kind::Debris => {
                        p.resting = true;
                        p.vel = V3::ZERO;
                        p.trail = 0.0;
                    }
                    Kind::Spark if p.vel.y < -4.0 => p.vel = v3(p.vel.x * 0.5, -p.vel.y * 0.3, p.vel.z * 0.5),
                    Kind::Spray => p.age = p.life,
                    _ => p.vel.y = p.vel.y.max(0.0),
                }
            }
        }
        self.parts.retain(|p| p.age < p.life);
        for at in puffs {
            let vel = V3::Y * self.r(1.0, 3.0);
            let life = self.r(1.6, 2.6);
            let grown = self.r(4.0, 6.5);
            self.smoke(at, vel, (0.8, grown), life, (0.16, 0.36), 0.6, 0.0);
        }

        // The wreck burns down: thick black smoke and flames at first, then a thinning column.
        let mut spawn: Vec<(V3, bool, f64)> = Vec::new();
        for e in self.emitters.iter_mut() {
            e.age += dt;
            e.clock += dt;
            let fade = e.age / e.life;
            let rate = if e.water { mix(10.0, 2.0, fade) } else { mix(16.0, 3.0, fade) };
            while e.clock >= 1.0 / rate {
                e.clock -= 1.0 / rate;
                spawn.push((e.pos, e.water, fade));
            }
        }
        self.emitters.retain(|e| e.age < e.life);
        for (at, water, fade) in spawn {
            let j = self.dir();
            let base = at + v3(j.x, 0.0, j.z) * self.r(0.0, 4.0);
            if water {
                let vel = V3::Y * self.r(3.0, 6.0) + j * 1.5;
                let life = self.r(2.0, 3.2);
                let grown = self.r(6.0, 9.0);
                self.smoke(base, vel, (2.0, grown), life, (0.9, 0.84), 0.5 * (1.0 - fade), 0.0);
                continue;
            }
            let vel = V3::Y * self.r(7.0, 12.0) + j * 2.0;
            let life = self.r(5.0, 8.0);
            let size = (self.r(2.5, 4.0), self.r(9.0, 15.0));
            self.smoke(base, vel, size, life, (0.09, 0.32), 0.78 * (1.0 - 0.6 * fade), 0.0);
            if fade < 0.45 && self.rnd.next() < 0.8 - fade {
                let p = Particle {
                    kind: Kind::Fire,
                    pos: base + V3::Y * self.r(0.0, 2.0),
                    vel: V3::Y * self.r(4.0, 9.0) + j * 1.5,
                    life: self.r(0.4, 0.8),
                    size: (self.r(1.5, 3.0), self.r(4.0, 7.0)),
                    color: ([1.0, 0.82, 0.42], [0.7, 0.16, 0.04]),
                    drag: 1.0,
                    spin: self.r(-2.0, 2.0),
                    ..BASE
                };
                self.push(p);
            }
        }
    }

    /// Writes and uploads this frame's geometry: blended smoke, spray and
    /// fragments back to front, the plume core and sheath, then fire and sparks.
    pub fn prepare(&mut self, ctx: &mut dyn RenderingBackend, cam: &Camera, time: f64, aircraft: Option<&Aircraft>) {
        let eye = cam.eye;
        let sun = sun_dir();
        self.order.clear();
        for (i, p) in self.parts.iter().enumerate() {
            if p.age >= 0.0 {
                self.order.push(((p.pos - eye).len(), i));
            }
        }
        self.order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.data.clear();
        for &(_, i) in &self.order {
            let p = &self.parts[i];
            if matches!(p.kind, Kind::Smoke | Kind::Spray | Kind::Debris) {
                emit(&mut self.data, p, eye, sun);
            }
        }
        let mut ends = [self.data.len() / FLOATS; 4];
        if let Some(a) = aircraft {
            plume_core(&mut self.data, a, eye, time);
            ends[1] = self.data.len() / FLOATS;
            plume_sheath(&mut self.data, a, eye, time);
        }
        ends[2] = self.data.len() / FLOATS;
        for &(_, i) in &self.order {
            let p = &self.parts[i];
            if matches!(p.kind, Kind::Fire | Kind::Spark) {
                emit(&mut self.data, p, eye, sun);
            }
        }
        self.data.truncate(MAX_VERTS * FLOATS);
        ends[3] = self.data.len() / FLOATS;
        for k in 0..4 {
            ends[k] = ends[k].min(ends[3]);
        }
        self.ranges = [(0, ends[0]), (ends[0], ends[1]), (ends[1], ends[2]), (ends[2], ends[3])];
        if ends[3] > 0 {
            ctx.buffer_update(self.bind.vertex_buffers[0], BufferSource::slice(&self.data));
        }
    }

    /// Draws one layer of what `prepare` uploaded, depth-tested against the
    /// world but without writing depth, so overlapping glows and puffs blend
    /// instead of cutting into each other.
    pub fn draw(&self, ctx: &mut dyn RenderingBackend, vp: M4, layer: Layer) {
        let u = FxU { vp, fog: FOG };
        let [blended, core, sheath, glow] = self.ranges;
        let draws: &[(&Pipeline, (usize, usize))] = match layer {
            Layer::PlumeCore => &[(&self.add, core)],
            Layer::PlumeSheath => &[(&self.add, sheath)],
            Layer::Particles => &[(&self.blend, blended), (&self.add, glow)],
        };
        for &(pip, (start, end)) in draws {
            if end <= start {
                continue;
            }
            ctx.apply_pipeline(pip);
            ctx.apply_bindings(&self.bind);
            ctx.apply_uniforms(UniformsSource::table(&u));
            gfx::depth_writes(false);
            ctx.draw(start as i32, (end - start) as i32, 1);
            gfx::depth_writes(true);
        }
    }
}

#[derive(Clone, Copy)]
pub enum Layer {
    PlumeCore,
    PlumeSheath,
    Particles,
}

fn vert(data: &mut Vec<f32>, p: V3, c: [f64; 3], a: f64) {
    data.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32, c[0] as f32, c[1] as f32, c[2] as f32, a.clamp(0.0, 1.0) as f32]);
}

fn tri(data: &mut Vec<f32>, p: [V3; 3], c: [f64; 3], a: f64) {
    for q in p {
        vert(data, q, c, a);
    }
}

fn scale3(c: [f64; 3], k: f64) -> [f64; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn mix3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}

/// One particle as eye-relative facets: a six-sided bipyramid for gas, a
/// tumbling triangle for a fragment, a streak for a spark.
fn emit(data: &mut Vec<f32>, p: &Particle, eye: V3, sun: V3) {
    let t = clamp(p.age / p.life, 0.0, 1.0);
    let c = p.pos - eye;
    let rot = Quat::axis_angle(p.axis, p.angle);
    match p.kind {
        Kind::Debris => {
            let s = p.size.0;
            let pts = [v3(s, 0.0, 0.0), v3(-0.5 * s, 0.0, 0.8 * s), v3(-0.3 * s, 0.1 * s, -0.9 * s)].map(|q| c + rot.rotate(q));
            let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]).norm();
            let lit = 0.45 + 0.55 * n.dot(sun).abs();
            let col = mix3(p.color.0, p.color.1, smoothstep(0.0, 0.5, t));
            tri(data, pts, scale3(col, lit), 1.0 - smoothstep(0.85, 1.0, t));
        }
        Kind::Spark => {
            let tail = c - p.vel * 0.045;
            let side = (p.vel.cross(c)).norm() * p.size.0;
            let col = mix3(p.color.0, p.color.1, t);
            let a = 1.0 - t * t;
            tri(data, [c + side, c - side, tail], col, a);
        }
        _ => {
            let grow = 1.0 - (1.0 - t) * (1.0 - t);
            let r = mix(p.size.0, p.size.1, grow);
            let (col, alpha) = match p.kind {
                Kind::Fire => (mix3(p.color.0, p.color.1, t.powf(0.6)), (1.0 - t).powf(1.4)),
                _ => (mix3(p.color.0, p.color.1, t), p.alpha * smoothstep(0.0, 0.06, t) * (1.0 - t).powf(1.3)),
            };
            let view = (-c).norm();
            let top = c + rot.rotate(v3(0.0, r * 1.05, 0.0));
            let bottom = c + rot.rotate(v3(0.0, -r * 1.05, 0.0));
            let ring: [V3; 6] = std::array::from_fn(|k| {
                let a = k as f64 * TAU / 6.0;
                c + rot.rotate(v3(a.cos() * r, 0.0, a.sin() * r))
            });
            for k in 0..6 {
                let (u, w) = (ring[k], ring[(k + 1) % 6]);
                for (apex, pts) in [(top, [top, u, w]), (bottom, [bottom, w, u])] {
                    let n = (pts[1] - pts[0]).cross(pts[2] - pts[0]).norm();
                    let n = if n.dot(apex - c) < 0.0 { -n } else { n };
                    let shade = if p.kind == Kind::Fire {
                        // Facets facing the viewer glow hottest.
                        0.6 + 0.4 * n.dot(view).max(0.0)
                    } else {
                        0.5 + 0.38 * n.dot(sun).max(0.0) + 0.12 * n.y
                    };
                    tri(data, pts, scale3(col, shade), alpha);
                }
            }
        }
    }
}

/// Shape of the afterburner plume this frame. Thin air at altitude lets it
/// bloom and stretch; the length flickers with the burning.
struct PlumeShape {
    ab: f64,
    bloom: f64,
    length: f64,
}

fn plume_shape(a: &Aircraft, time: f64) -> Option<PlumeShape> {
    let ab = a.afterburner_visual;
    if ab < 0.03 {
        return None;
    }
    let bloom = 1.0 + 0.45 * smoothstep(0.0, 12000.0, a.pos.y);
    let flicker = 1.0 + 0.07 * (time * 47.0).sin() + 0.05 * (time * 83.0 + 1.7).sin() + 0.03 * (time * 131.0).sin();
    Some(PlumeShape { ab, bloom, length: (2.6 + 4.4 * ab) * bloom * flicker })
}

/// Faceted cone through rings of (distance aft of the nozzle, radius, colour, alpha).
fn cone(data: &mut Vec<f32>, a: &Aircraft, eye: V3, rings: &[(f64, f64, [f64; 3], f64)], sides: usize, twist: f64) {
    let ax = a.q.axes();
    let p = |z: f64, r: f64, t: f64| a.pos - eye + ax.right * (t.cos() * r) + ax.up * (NOZZLE.y + t.sin() * r) + ax.back * (NOZZLE.z + z);
    for w in rings.windows(2) {
        let (za, ra, ca, alpha_a) = w[0];
        let (zb, rb, cb, alpha_b) = w[1];
        for k in 0..sides {
            let t0 = k as f64 * TAU / sides as f64 + twist;
            let t1 = (k + 1) as f64 * TAU / sides as f64 + twist;
            // Alternate facets a little darker so the cone reads as flat panels.
            let shade = if k % 2 == 0 { 1.0 } else { 0.78 };
            let (c0, c1) = (scale3(ca, shade), scale3(cb, shade));
            let (a0, a1, b0, b1) = (p(za, ra, t0), p(za, ra, t1), p(zb, rb, t0), p(zb, rb, t1));
            vert(data, a0, c0, alpha_a);
            vert(data, a1, c0, alpha_a);
            vert(data, b1, c1, alpha_b);
            vert(data, a0, c0, alpha_a);
            vert(data, b1, c1, alpha_b);
            vert(data, b0, c1, alpha_b);
        }
    }
}

/// Train of shock diamonds inside the airframe's flame, the first one the
/// bright Mach disk.
fn plume_core(data: &mut Vec<f32>, a: &Aircraft, eye: V3, time: f64) {
    let Some(s) = plume_shape(a, time) else { return };
    let spacing = 0.95 * s.bloom * (1.0 + 0.25 * s.ab);
    for i in 0..5 {
        let zc = 0.75 + i as f64 * spacing;
        if zc > s.length * 0.85 {
            break;
        }
        let r = (0.30 - 0.03 * i as f64) * s.bloom;
        let half = 0.3 * spacing;
        let col = if i == 0 { [0.80, 0.86, 1.0] } else { [1.0, 0.80, 0.45] };
        let alpha = 0.8 * s.ab * (1.0 - i as f64 / 5.5);
        cone(data, a, eye, &[(zc - half, 0.0, col, alpha), (zc, r, col, alpha), (zc + half, 0.0, col, alpha)], 6, 0.3 * i as f64);
    }
}

/// Translucent outer sheath, turning slowly so its panels shimmer.
fn plume_sheath(data: &mut Vec<f32>, a: &Aircraft, eye: V3, time: f64) {
    let Some(s) = plume_shape(a, time) else { return };
    let rings = [
        (0.0, 0.40, [1.0, 0.55, 0.18], 0.24 * s.ab),
        (s.length * 0.3, 0.46 * s.bloom, [1.0, 0.45, 0.12], 0.26 * s.ab),
        (s.length * 0.75, 0.30 * s.bloom, [0.85, 0.24, 0.06], 0.12 * s.ab),
        (s.length, 0.04, [0.6, 0.15, 0.04], 0.0),
    ];
    cone(data, a, eye, &rings, 10, time * 9.0);
}
