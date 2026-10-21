//! World renderer: sky, static terrain chunks with LOD, ocean, clouds, gates,
//! the exterior aircraft and the CRT post pass. All world geometry is drawn
//! relative to the eye so f32 precision holds anywhere on the map.
use crate::airframe::{self, Airframe};
use crate::fx::{Fx, Layer};
use crate::gfx::{self, Target};
use crate::math::*;
use crate::mesh::Mesh;
use crate::sim::{Aircraft, Mission, GATE_RADIUS};
use crate::terrain::{self, CHUNKS, CHUNK_M, X0, Z0};
use miniquad::*;

pub const SUN: V3 = v3(0.52, 0.34, -0.78);
pub const FOG: [f32; 3] = [0.67, 0.64, 0.53];
const STRIDES: [usize; 4] = [1, 2, 4, 8];

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: V3,
    pub forward: V3,
    pub right: V3,
    pub up: V3,
    pub fov: f64,
}

impl Camera {
    pub fn view(&self) -> M4 {
        view_rot(self.right, self.up, self.forward)
    }
}

pub fn sun_dir() -> V3 {
    SUN.norm()
}

const NOISE: &str = r#"
float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}
float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    i = mod(i, 1024.0);
    float a = hash21(i);
    float b = hash21(mod(i + vec2(1.0, 0.0), 1024.0));
    float c = hash21(mod(i + vec2(0.0, 1.0), 1024.0));
    float d = hash21(mod(i + vec2(1.0, 1.0), 1024.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
float fbm5(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        v += a * vnoise(p);
        p = p * 2.03 + vec2(17.3, 9.2);
        a *= 0.5;
    }
    return v;
}
"#;

const VS_FULL: &str = r#"
in vec2 pos;
out vec2 v_uv;
void main() { v_uv = pos * 0.5 + 0.5; gl_Position = vec4(pos, 0.0, 1.0); }"#;

const FS_SKY: &str = r#"
in vec2 v_uv;
uniform vec3 u_cam;
uniform vec3 u_right;
uniform vec3 u_up;
uniform vec3 u_fwd;
uniform vec3 u_sun;
uniform float u_tan;
uniform float u_aspect;
uniform float u_time;
out vec4 frag;
NOISE
void main() {
    vec2 ndc = v_uv * 2.0 - 1.0;
    vec3 dir = normalize(u_fwd + u_right * ndc.x * u_tan * u_aspect + u_up * ndc.y * u_tan);
    float y = dir.y;
    float h = pow(clamp(y, 0.0, 1.0), 0.58);
    vec3 horizon = vec3(0.71, 0.65, 0.52);
    vec3 zenith = vec3(0.10, 0.26, 0.43);
    vec3 sky = mix(horizon, zenith, h);
    float sun = clamp(dot(dir, u_sun), 0.0, 1.0);
    sky += vec3(1.0, 0.53, 0.21) * pow(sun, 5.0) * 0.11;
    sky += vec3(1.0, 0.76, 0.43) * pow(sun, 44.0) * 0.35;
    sky += vec3(1.0, 0.96, 0.80) * smoothstep(0.9992, 0.9997, sun) * 5.0;
    if (y < 0.0) {
        vec3 under = mix(vec3(0.62, 0.59, 0.48), vec3(0.12, 0.18, 0.17), clamp(-y * 7.0, 0.0, 1.0));
        sky = mix(horizon, under, clamp(-y * 18.0, 0.15, 1.0));
    }
    if (y > 0.008) {
        float t = clamp((3600.0 - u_cam.y) / y, 0.0, 60000.0);
        vec2 cp = u_cam.xz + dir.xz * t;
        float n1 = fbm5(cp * 0.00038 + vec2(u_time * 0.0009, 0.0));
        float n2 = fbm5(cp * 0.00105 - vec2(u_time * 0.0014, u_time * 0.0004));
        float cover = smoothstep(0.54, 0.72, n1 + n2 * 0.2);
        float fade = smoothstep(0.0, 0.10, y) * smoothstep(60000.0, 9000.0, t);
        vec3 cloud = mix(vec3(0.67, 0.70, 0.66), vec3(1.0, 0.93, 0.77), pow(sun, 2.0));
        sky = mix(sky, cloud, cover * fade * 0.5);
    }
    float haze = 1.0 - smoothstep(0.0, 0.13, abs(y - 0.025));
    sky = mix(sky, vec3(0.70, 0.66, 0.53), haze * 0.15);
    frag = vec4(sky, 1.0);
}"#;

const VS_TERRAIN: &str = r#"
in vec3 pos;
uniform mat4 u_vp;
uniform vec3 u_eye;
out vec3 v_world;
void main() { v_world = pos; gl_Position = u_vp * vec4(pos - u_eye, 1.0); }"#;

const FS_TERRAIN: &str = r#"
in vec3 v_world;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform vec3 u_fog;
out vec4 frag;
NOISE
void main() {
    vec3 rel = v_world - u_eye;
    vec3 n = normalize(cross(dFdx(rel), dFdy(rel)));
    if (n.y < 0.0) n = -n;
    float h = v_world.y;
    float slope = 1.0 - n.y;
    float grain = vnoise(v_world.xz * 0.031) * 0.55 + vnoise(v_world.xz * 0.29) * 0.3;
    vec3 low = vec3(0.235, 0.275, 0.145);
    vec3 scrub = vec3(0.32, 0.32, 0.16);
    vec3 rock = vec3(0.33, 0.30, 0.25);
    vec3 high_rock = vec3(0.43, 0.41, 0.38);
    vec3 snow = vec3(0.79, 0.80, 0.76);
    vec3 sand = vec3(0.48, 0.40, 0.25);
    vec3 col = low;
    col = mix(col, sand, smoothstep(25.0, 8.0, h));
    col = mix(col, scrub, smoothstep(360.0, 760.0, h));
    col = mix(col, rock, smoothstep(800.0, 1450.0, h));
    col = mix(col, high_rock, smoothstep(1650.0, 2300.0, h));
    float snow_mask = smoothstep(2050.0, 2450.0, h) * (1.0 - smoothstep(0.42, 0.68, slope));
    col = mix(col, snow, snow_mask);
    col = mix(col, rock, smoothstep(0.23, 0.52, slope) * 0.72);
    col *= 0.82 + grain * 0.36;
    float sun_light = max(dot(n, u_sun), 0.0);
    float sky_light = 0.33 + 0.34 * (0.5 + 0.5 * n.y);
    vec3 lit = col * (sky_light + sun_light * 0.92);
    lit += vec3(0.10, 0.09, 0.06) * pow(1.0 - n.y, 2.0);
    lit = mix(lit, vec3(0.12, 0.16, 0.13), smoothstep(0.70, 1.0, slope) * 0.22);
    float dist = length(rel);
    float fog = 1.0 - exp(-dist * 0.000058);
    fog = clamp(fog + smoothstep(18000.0, 32000.0, dist) * 0.5, 0.0, 1.0);
    vec3 fog_col = mix(u_fog, vec3(0.58, 0.62, 0.57), smoothstep(3000.0, 0.0, v_world.y) * 0.25);
    frag = vec4(mix(lit, fog_col, fog), 1.0);
}"#;

const VS_OCEAN: &str = r#"
in vec2 pos;
uniform mat4 u_vp;
uniform vec3 u_eye;
out vec3 v_world;
void main() {
    vec2 p = u_eye.xz + pos;
    v_world = vec3(p.x, 0.0, p.y);
    gl_Position = u_vp * vec4(v_world - u_eye, 1.0);
}"#;

const FS_OCEAN: &str = r#"
in vec3 v_world;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform vec3 u_fog;
uniform float u_time;
out vec4 frag;
void main() {
    vec2 p = mod(v_world.xz, 8192.0);
    float t = u_time;
    float dx = cos(p.x * 0.075 + p.y * 0.043 + t * 1.15) * 0.055 + cos(p.x * 0.021 - p.y * 0.034 + t * 0.62) * 0.05;
    float dz = sin(p.y * 0.068 + p.x * 0.047 - t * 1.02) * 0.05 + sin(p.y * 0.019 + p.x * 0.028 + t * 0.71) * 0.05;
    vec3 n = normalize(vec3(-dx, 1.0, -dz));
    vec3 rel = v_world - u_eye;
    vec3 view = normalize(rel);
    float fresnel = pow(1.0 - clamp(dot(n, -view), 0.0, 1.0), 3.0);
    float sun = pow(max(dot(reflect(view, n), u_sun), 0.0), 110.0);
    vec3 col = mix(vec3(0.018, 0.095, 0.115), vec3(0.055, 0.24, 0.25), fresnel * 0.72 + 0.08);
    col += vec3(1.0, 0.84, 0.54) * sun * 1.55;
    col += vec3(0.13, 0.22, 0.19) * max(dot(n, u_sun), 0.0) * 0.18;
    float fog = clamp(1.0 - exp(-length(rel) * 0.000052), 0.0, 1.0);
    frag = vec4(mix(col, u_fog, fog), 1.0);
}"#;

const VS_CLOUD: &str = r#"
in vec3 center;
in vec2 corner;
in float size;
in float alpha;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_right;
uniform vec3 u_up;
uniform vec3 u_fog;
out vec2 v_uv;
out float v_alpha;
out vec3 v_rel;
void main() {
    vec3 rel = center - u_eye + u_right * corner.x * size + u_up * corner.y * size * 0.5;
    v_rel = rel;
    v_uv = vec2(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
    v_alpha = alpha;
    gl_Position = u_vp * vec4(rel, 1.0);
}"#;

const FS_CLOUD: &str = r#"
in vec2 v_uv;
in float v_alpha;
in vec3 v_rel;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_right;
uniform vec3 u_up;
uniform vec3 u_fog;
uniform sampler2D u_tex;
out vec4 frag;
void main() {
    float a = texture(u_tex, v_uv).a * v_alpha;
    if (a < 0.008) discard;
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.000048), 0.0, 0.95);
    vec3 col = mix(vec3(0.87, 0.86, 0.78), vec3(1.0, 0.92, 0.73), 0.35);
    col = mix(col, u_fog, fog * 0.9);
    frag = vec4(col, a * (1.0 - fog * 0.78));
}"#;

const VS_GATE: &str = r#"
in vec3 pos;
in vec3 normal;
in float u;
uniform mat4 u_vp;
uniform mat4 u_model;
uniform vec3 u_color;
uniform vec3 u_fog;
uniform float u_time;
uniform float u_glow;
uniform float u_inflate;
out vec3 v_rel;
out vec3 v_normal;
out float v_u;
void main() {
    vec4 rel = u_model * vec4(pos + normal * u_inflate, 1.0);
    v_rel = rel.xyz;
    v_normal = mat3(u_model) * normal;
    v_u = u;
    gl_Position = u_vp * rel;
}"#;

const FS_GATE: &str = r#"
in vec3 v_rel;
in vec3 v_normal;
in float v_u;
uniform mat4 u_vp;
uniform mat4 u_model;
uniform vec3 u_color;
uniform vec3 u_fog;
uniform float u_time;
uniform float u_glow;
uniform float u_inflate;
out vec4 frag;
void main() {
    vec3 n = normalize(v_normal);
    float band = 0.70 + 0.30 * step(0.48, fract(v_u * 12.0));
    float pulse = 0.80 + 0.20 * sin(u_time * 3.4 + v_u * 18.0);
    vec3 col = u_color * (0.70 + 0.6 * max(dot(n, normalize(vec3(0.4, 0.6, 0.3))), 0.0));
    col *= band * pulse;
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.000045), 0.0, 0.92);
    col = mix(col, u_fog, fog);
    if (u_glow > 0.5) frag = vec4(col * 1.4, 0.13);
    else frag = vec4(col, 1.0);
}"#;

const VS_AIR: &str = r#"
in vec3 pos;
in vec3 normal;
in vec3 color;
in vec2 uv;
uniform mat4 u_vp;
uniform mat4 u_model;
uniform vec3 u_sun;
uniform vec3 u_fog;
uniform float u_alpha;
uniform float u_glow;
uniform float u_cutout;
out vec3 v_rel;
out vec3 v_normal;
out vec3 v_color;
out vec3 v_local;
void main() {
    vec4 rel = u_model * vec4(pos, 1.0);
    v_rel = rel.xyz;
    v_local = pos;
    v_normal = mat3(u_model) * normal;
    v_color = color;
    gl_Position = u_vp * rel;
}"#;

const FS_AIR: &str = r#"
in vec3 v_rel;
in vec3 v_normal;
in vec3 v_color;
in vec3 v_local;
uniform mat4 u_vp;
uniform mat4 u_model;
uniform vec3 u_sun;
uniform vec3 u_fog;
uniform float u_alpha;
uniform float u_glow;
uniform float u_cutout;
out vec4 frag;
void main() {
    // From the seat, the cockpit model replaces the skin inside the canopy.
    if (u_cutout > 0.5 && v_local.z > -4.75 && v_local.z < -1.90 && abs(v_local.x) < 0.53 && v_local.y > 0.30) discard;
    vec3 view_dir = normalize(-v_rel);
    vec3 n = normalize(v_normal);
    if (dot(n, view_dir) < 0.0) n = -n;
    vec3 sun = normalize(u_sun);
    float lit = max(dot(n, sun), 0.0);
    // Sky/ground hemisphere plus a soft fill from the viewer, so neighbouring
    // flat facets always read as distinct shades, even against the sun.
    float head = max(dot(n, view_dir), 0.0);
    vec3 col = v_color * (0.30 + 0.14 * n.y + 0.22 * head + 0.62 * lit);
    float rim = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0);
    col += vec3(0.16, 0.2, 0.18) * rim * 0.30;
    float shine = pow(max(dot(reflect(-sun, n), view_dir), 0.0), 24.0);
    col += shine * 0.10;
    float a = u_alpha;
    if (u_alpha < 0.9 && u_glow < 0.5) {
        // Gold-tinted canopy: tint, Fresnel reflection of the sky and a sun glint.
        // Upward panes reflect more sky, so each flat pane reads separately.
        float fres = pow(1.0 - max(dot(n, view_dir), 0.0), 4.0);
        float glint = pow(max(dot(reflect(-sun, n), view_dir), 0.0), 60.0);
        float sky_ref = 0.35 * max(n.y, 0.0);
        col = mix(v_color, vec3(0.62, 0.68, 0.66), fres * 0.7 + sky_ref) + glint * vec3(1.0, 0.95, 0.8);
        a = clamp(u_alpha + fres * 0.55 + sky_ref * 0.4 + glint, 0.0, 0.92);
    }
    if (u_glow > 0.5) col = v_color * 1.8;
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.000045), 0.0, 0.9);
    col = mix(col, u_fog, fog);
    frag = vec4(col, a * (1.0 - fog * 0.8));
}"#;

const FS_POST: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
uniform vec2 u_res;
uniform float u_gloc;
uniform float u_seed;
out vec4 frag;
float h21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}
void main() {
    vec2 cc = v_uv - 0.5;
    float r2 = dot(cc, cc);
    vec2 uv = 0.5 + cc * (1.0 + r2 * 0.018);
    vec2 chroma = cc * 0.00045;
    vec3 col;
    col.r = texture(u_tex, uv + chroma).r;
    col.g = texture(u_tex, uv).g;
    col.b = texture(u_tex, uv - chroma).b;
    float scan = 0.955 + 0.045 * sin(gl_FragCoord.y * 2.0943951);
    col *= mix(1.0, scan, 0.55);
    float vign = smoothstep(0.98, 0.26, length(uv - 0.5));
    col *= mix(0.68, 1.0, vign);
    float grain = h21(mod(gl_FragCoord.xy + u_seed, 1024.0));
    col += (grain - 0.5) * 0.022;
    col *= vec3(1.025, 1.0, 0.955);
    // Grey-out under sustained G: a dark red ring that closes toward the centre.
    vec2 g = (v_uv - vec2(0.5, 0.58)) * vec2(u_res.x / u_res.y, 1.0);
    float ring = smoothstep(0.16, 0.75, length(g));
    col = mix(col, vec3(0.20, 0.0, 0.0), u_gloc * ring * 0.75);
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}"#;

fn src(s: &str) -> String {
    s.replace("NOISE", NOISE)
}

#[repr(C)]
struct SkyU {
    cam: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    fwd: [f32; 3],
    sun: [f32; 3],
    tan: f32,
    aspect: f32,
    time: f32,
}
#[repr(C)]
struct TerrainU {
    vp: M4,
    eye: [f32; 3],
    sun: [f32; 3],
    fog: [f32; 3],
}
#[repr(C)]
struct OceanU {
    vp: M4,
    eye: [f32; 3],
    sun: [f32; 3],
    fog: [f32; 3],
    time: f32,
}
#[repr(C)]
struct CloudU {
    vp: M4,
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    fog: [f32; 3],
}
#[repr(C)]
struct GateU {
    vp: M4,
    model: M4,
    color: [f32; 3],
    fog: [f32; 3],
    time: f32,
    glow: f32,
    inflate: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AirU {
    pub vp: M4,
    pub model: M4,
    pub sun: [f32; 3],
    pub fog: [f32; 3],
    pub alpha: f32,
    pub glow: f32,
    pub cutout: f32,
}
#[repr(C)]
struct PostU {
    res: [f32; 2],
    gloc: f32,
    seed: f32,
}

struct Chunk {
    vbs: [Option<BufferId>; 4],
    min_lod: usize,
    lo: f32,
    hi: f32,
    x0: f64,
    z0: f64,
}

pub struct Part {
    pub bind: Bindings,
    pub count: i32,
}

pub fn part(ctx: &mut dyn RenderingBackend, mesh: &Mesh) -> Part {
    let vb = gfx::immutable_vb(ctx, &mesh.v);
    let ib = gfx::sequential_ib(ctx, mesh.count().max(3));
    Part { bind: Bindings { vertex_buffers: vec![vb], index_buffer: ib, images: vec![] }, count: mesh.count() as i32 }
}

pub const MESH_ATTRS: [VertexAttribute; 4] = [
    VertexAttribute::new("pos", VertexFormat::Float3),
    VertexAttribute::new("normal", VertexFormat::Float3),
    VertexAttribute::new("color", VertexFormat::Float3),
    VertexAttribute::new("uv", VertexFormat::Float2),
];

struct Cloud {
    pos: V3,
    size: f64,
    alpha: f64,
}

pub struct World {
    fullscreen: Bindings,
    sky: Pipeline,
    terrain: Pipeline,
    chunks: Vec<Chunk>,
    lod_ib: [BufferId; 4],
    lod_count: [i32; 4],
    ocean: Pipeline,
    ocean_bind: Bindings,
    ocean_count: i32,
    cloud: Pipeline,
    cloud_bind: Bindings,
    clouds: Vec<Cloud>,
    cloud_data: Vec<f32>,
    gate: Pipeline,
    gate_glow: Pipeline,
    gate_bind: Bindings,
    gate_count: i32,
    air: Pipeline,
    glass_back: Pipeline,
    glass_front: Pipeline,
    flame_pip: Pipeline,
    exterior: Part,
    canopy: Part,
    crew: Part,
    flame: Part,
    stab: [Part; 2],
    rudder: Part,
    brakes: [Part; 4],
    post: Pipeline,
    post_bind: Bindings,
    /// Afterburner plume and crash effects.
    pub fx: Fx,
    pub drawn_chunks: usize,
}

impl World {
    pub fn new(ctx: &mut dyn RenderingBackend, airframe: &Airframe) -> World {
        use UniformType::*;
        let depth = PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() };
        let full_vb = gfx::immutable_vb(ctx, &[-1.0f32, -1.0, 3.0, -1.0, -1.0, 3.0]);
        let full_ib = gfx::immutable_ib(ctx, &[0u16, 1, 2]);
        let fullscreen = Bindings { vertex_buffers: vec![full_vb], index_buffer: full_ib, images: vec![] };
        let pos2 = [VertexAttribute::new("pos", VertexFormat::Float2)];

        let s = gfx::shader(ctx, VS_FULL, &src(FS_SKY), &[], &[("u_cam", Float3), ("u_right", Float3), ("u_up", Float3), ("u_fwd", Float3), ("u_sun", Float3), ("u_tan", Float1), ("u_aspect", Float1), ("u_time", Float1)]);
        let sky = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, PipelineParams::default());

        // Terrain chunks: one vertex buffer per chunk and stride, shared index buffers.
        let terrain_u = [("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_fog", Float3)];
        let s = gfx::shader(ctx, VS_TERRAIN, &src(FS_TERRAIN), &[], &terrain_u);
        let terrain = ctx.new_pipeline(&[BufferLayout::default()], &[VertexAttribute::new("pos", VertexFormat::Float3)], s, depth);
        let hm = terrain::init();
        let mut lod_count = [0; 4];
        let lod_ib: [BufferId; 4] = std::array::from_fn(|k| {
            let idx = terrain::chunk_indices(STRIDES[k]);
            lod_count[k] = idx.len() as i32;
            gfx::immutable_ib(ctx, &idx)
        });
        let mut chunks = Vec::new();
        for cz in 0..CHUNKS {
            for cx in 0..CHUNKS {
                let min_lod = if terrain::is_fine_chunk(cx, cz) { 0 } else { 1 };
                let mut vbs = [None; 4];
                for (k, vb) in vbs.iter_mut().enumerate().skip(min_lod) {
                    let v = hm.chunk_vertices(cx, cz, STRIDES[k]);
                    *vb = Some(gfx::immutable_vb(ctx, &v));
                }
                chunks.push(Chunk {
                    vbs,
                    min_lod,
                    lo: hm.chunk_min[cz * CHUNKS + cx],
                    hi: hm.chunk_max[cz * CHUNKS + cx],
                    x0: X0 + cx as f64 * CHUNK_M,
                    z0: Z0 + cz as f64 * CHUNK_M,
                });
            }
        }

        // Ocean: polar grid around the eye, so no triangle is enormous.
        let s = gfx::shader(ctx, VS_OCEAN, FS_OCEAN, &[], &[("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_fog", Float3), ("u_time", Float1)]);
        let ocean = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, depth);
        let (rings, segs) = (40usize, 96usize);
        let mut ov = vec![0f32, 0.0];
        for i in 1..=rings {
            let r = 20.0 * (65000.0f64 / 20.0).powf(i as f64 / rings as f64);
            for j in 0..segs {
                let a = j as f64 * TAU / segs as f64;
                ov.extend_from_slice(&[(a.cos() * r) as f32, (a.sin() * r) as f32]);
            }
        }
        let mut oi: Vec<u16> = Vec::new();
        for j in 0..segs {
            oi.extend_from_slice(&[0, 1 + j as u16, 1 + ((j + 1) % segs) as u16]);
        }
        for i in 0..rings - 1 {
            for j in 0..segs {
                let a = (1 + i * segs + j) as u16;
                let b = (1 + i * segs + (j + 1) % segs) as u16;
                let c = a + segs as u16;
                let d = b + segs as u16;
                oi.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        let ocean_bind = Bindings { vertex_buffers: vec![gfx::immutable_vb(ctx, &ov)], index_buffer: gfx::immutable_ib(ctx, &oi), images: vec![] };

        // Clouds: sorted camera-facing billboards with a procedural texture.
        let s = gfx::shader(ctx, VS_CLOUD, FS_CLOUD, &["u_tex"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_right", Float3), ("u_up", Float3), ("u_fog", Float3)]);
        let cloud = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("center", VertexFormat::Float3), VertexAttribute::new("corner", VertexFormat::Float2), VertexAttribute::new("size", VertexFormat::Float1), VertexAttribute::new("alpha", VertexFormat::Float1)],
            s,
            PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::alpha_blend()), ..Default::default() },
        );
        let mut rnd = Lcg(4317);
        let clouds: Vec<Cloud> = (0..58)
            .map(|_| {
                let x = (rnd.next() * 2.0 - 1.0) * 19000.0;
                let z = 13500.0 - rnd.next() * 29000.0;
                let ground = terrain::terrain_at(x, z);
                let y = (ground + 420.0).max(1250.0) + rnd.next() * 1600.0;
                let size = 1000.0 + rnd.next() * 2100.0;
                let alpha = 0.32 + rnd.next() * 0.34;
                Cloud { pos: v3(x, y, z), size, alpha }
            })
            .collect();
        let tex = cloud_texture();
        let cloud_tex = ctx.new_texture_from_data_and_format(&tex, TextureParams { width: 256, height: 128, format: TextureFormat::RGBA8, min_filter: FilterMode::Linear, mag_filter: FilterMode::Linear, ..Default::default() });
        let cloud_vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(clouds.len() * 6 * 7));
        let cloud_bind = Bindings { vertex_buffers: vec![cloud_vb], index_buffer: gfx::sequential_ib(ctx, clouds.len() * 6), images: vec![cloud_tex] };

        // Gates: banded torus with an additive glow shell.
        let gate_u = [("u_vp", Mat4), ("u_model", Mat4), ("u_color", Float3), ("u_fog", Float3), ("u_time", Float1), ("u_glow", Float1), ("u_inflate", Float1)];
        let s = gfx::shader(ctx, VS_GATE, FS_GATE, &[], &gate_u);
        let gate_attrs = [VertexAttribute::new("pos", VertexFormat::Float3), VertexAttribute::new("normal", VertexFormat::Float3), VertexAttribute::new("u", VertexFormat::Float1)];
        let gate = ctx.new_pipeline(&[BufferLayout::default()], &gate_attrs, s, depth);
        let gate_glow = ctx.new_pipeline(&[BufferLayout::default()], &gate_attrs, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::additive()), ..Default::default() });
        let (gv, gi) = torus();
        let gate_bind = Bindings { vertex_buffers: vec![gfx::immutable_vb(ctx, &gv)], index_buffer: gfx::immutable_ib(ctx, &gi), images: vec![] };

        // Exterior aircraft.
        let air_u = [("u_vp", Mat4), ("u_model", Mat4), ("u_sun", Float3), ("u_fog", Float3), ("u_alpha", Float1), ("u_glow", Float1), ("u_cutout", Float1)];
        let s = gfx::shader(ctx, VS_AIR, FS_AIR, &[], &air_u);
        let air = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, depth);
        let glass = |cull| PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::alpha_blend()), cull_face: cull, ..Default::default() };
        let glass_back = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, glass(CullFace::Back));
        let glass_front = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, glass(CullFace::Front));
        let flame_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::additive()), ..Default::default() });

        let s = gfx::shader(ctx, VS_FULL, FS_POST, &["u_tex"], &[("u_res", Float2), ("u_gloc", Float1), ("u_seed", Float1)]);
        let post = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, PipelineParams::default());

        World {
            post_bind: fullscreen.clone(),
            fullscreen,
            sky,
            terrain,
            chunks,
            lod_ib,
            lod_count,
            ocean,
            ocean_bind,
            ocean_count: oi.len() as i32,
            cloud,
            cloud_bind,
            cloud_data: Vec::with_capacity(clouds.len() * 42),
            clouds,
            gate,
            gate_glow,
            gate_bind,
            gate_count: gi.len() as i32,
            air,
            glass_back,
            glass_front,
            flame_pip,
            exterior: part(ctx, &airframe.exterior),
            canopy: part(ctx, &airframe.canopy),
            crew: part(ctx, &airframe.crew),
            flame: part(ctx, &airframe.flame),
            stab: [part(ctx, &airframe.stab[0]), part(ctx, &airframe.stab[1])],
            rudder: part(ctx, &airframe.rudder),
            brakes: [part(ctx, &airframe.brakes[0]), part(ctx, &airframe.brakes[1]), part(ctx, &airframe.brakes[2]), part(ctx, &airframe.brakes[3])],
            post,
            fx: Fx::new(ctx),
            drawn_chunks: 0,
        }
    }

    /// Renders sky, terrain, ocean, clouds, gates and the exterior into the current pass.
    pub fn draw(&mut self, ctx: &mut dyn RenderingBackend, cam: &Camera, aspect: f64, time: f64, mission: Option<&Mission>, aircraft: Option<&Aircraft>, cockpit: bool) {
        let near = if cockpit { 0.25 } else { 0.7 };
        let proj = perspective(cam.fov, aspect, near, 80000.0);
        let vp = mat_mul(&proj, &cam.view());
        let sun = sun_dir().f32();
        let eye = cam.eye.f32();

        ctx.apply_pipeline(&self.sky);
        ctx.apply_bindings(&self.fullscreen);
        ctx.apply_uniforms(UniformsSource::table(&SkyU {
            cam: eye,
            right: cam.right.f32(),
            up: cam.up.f32(),
            fwd: cam.forward.f32(),
            sun,
            tan: (cam.fov / 2.0).tan() as f32,
            aspect: aspect as f32,
            time: time as f32,
        }));
        ctx.draw(0, 3, 1);

        // Terrain: frustum-cull each chunk's box and pick a stride by distance.
        ctx.apply_pipeline(&self.terrain);
        ctx.apply_uniforms(UniformsSource::table(&TerrainU { vp, eye, sun, fog: FOG }));
        let tan_y = (cam.fov / 2.0).tan();
        let tan_x = tan_y * aspect;
        self.drawn_chunks = 0;
        for ch in &self.chunks {
            let (cx, cz) = (cam.eye.x.clamp(ch.x0, ch.x0 + CHUNK_M), cam.eye.z.clamp(ch.z0, ch.z0 + CHUNK_M));
            let d = ((cx - cam.eye.x).powi(2) + (cz - cam.eye.z).powi(2)).sqrt();
            if d > 42000.0 || !box_visible(cam, tan_x, tan_y, ch) {
                continue;
            }
            let lod = if d < 2500.0 {
                0
            } else if d < 6500.0 {
                1
            } else if d < 15000.0 {
                2
            } else {
                3
            };
            let lod = lod.max(ch.min_lod);
            let vb = ch.vbs[lod].unwrap();
            ctx.apply_bindings(&Bindings { vertex_buffers: vec![vb], index_buffer: self.lod_ib[lod], images: vec![] });
            ctx.draw(0, self.lod_count[lod], 1);
            self.drawn_chunks += 1;
        }

        ctx.apply_pipeline(&self.ocean);
        ctx.apply_bindings(&self.ocean_bind);
        ctx.apply_uniforms(UniformsSource::table(&OceanU { vp, eye, sun, fog: FOG, time: time as f32 }));
        ctx.draw(0, self.ocean_count, 1);

        // Gates.
        if let Some(m) = mission {
            ctx.apply_pipeline(&self.gate);
            ctx.apply_bindings(&self.gate_bind);
            for (i, g) in m.gates.iter().enumerate() {
                let rel = g.pos - cam.eye;
                if rel.len() > 24000.0 {
                    continue;
                }
                let ax = g.axis.norm();
                let right = ax.cross(V3::Y).norm();
                let up = right.cross(ax);
                let model = basis(right, up, ax, rel);
                let next = i == m.gate_index;
                let color = if g.passed {
                    [0.18, 0.5, 0.26]
                } else if next {
                    [1.0, 0.66, 0.22]
                } else {
                    [0.30, 0.79, 0.74]
                };
                ctx.apply_uniforms(UniformsSource::table(&GateU { vp, model, color, fog: FOG, time: time as f32, glow: 0.0, inflate: 0.0 }));
                ctx.draw(0, self.gate_count, 1);
            }
        }

        // Exterior aircraft, shared by both views; first person omits canopy and crew.
        self.fx.prepare(ctx, cam, time, aircraft);
        if let Some(a) = aircraft {
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let base = AirU { vp, model, sun, fog: FOG, alpha: 1.0, glow: 0.0, cutout: if cockpit { 1.0 } else { 0.0 } };
            ctx.apply_pipeline(&self.air);
            self.draw_part(ctx, &self.exterior, &base, &model);
            let pitch = clamp(a.surf_pitch, -1.0, 1.0);
            let roll = clamp(a.surf_roll, -1.0, 1.0);
            for (i, side) in [-1.0, 1.0].iter().enumerate() {
                let angle = clamp(-pitch * 14.0 * DEG + side * -roll * 5.0 * DEG, -21.0 * DEG, 21.0 * DEG);
                let pivot = v3(side * airframe::STAB_PIVOT.x, airframe::STAB_PIVOT.y, airframe::STAB_PIVOT.z);
                let local = rotate_about(pivot, Quat::axis_angle(airframe::stab_axis(*side), angle));
                self.draw_part(ctx, &self.stab[i], &base, &mat_mul(&model, &local));
            }
            let (hinge, axis) = airframe::rudder_axis();
            let local = rotate_about(hinge, Quat::axis_angle(axis, clamp(a.surf_yaw, -1.0, 1.0) * 22.0 * DEG));
            self.draw_part(ctx, &self.rudder, &base, &mat_mul(&model, &local));
            for (i, b) in self.brakes.iter().enumerate() {
                let up = i % 2 == 0;
                let hinge = v3(0.0, 0.06, airframe::BRAKE_HINGE_Z);
                let angle = a.brake * 58.0 * DEG * if up { -1.0 } else { 1.0 };
                let local = rotate_about(hinge, Quat::axis_angle(V3::X, angle));
                self.draw_part(ctx, b, &base, &mat_mul(&model, &local));
            }
            if !cockpit {
                self.draw_part(ctx, &self.crew, &base, &model);
            }
            // The flame sits between the plume's shock diamonds and its sheath.
            self.fx.draw(ctx, vp, Layer::PlumeCore);
            if !cockpit && a.afterburner_visual > 0.03 {
                ctx.apply_pipeline(&self.flame_pip);
                let u = AirU { alpha: (0.18 + 0.30 * a.afterburner_visual) as f32, glow: 1.0, ..base };
                gfx::depth_writes(false);
                self.draw_part(ctx, &self.flame, &u, &model);
                gfx::depth_writes(true);
            }
            self.fx.draw(ctx, vp, Layer::PlumeSheath);
        }

        if let Some(m) = mission {
            ctx.apply_pipeline(&self.gate_glow);
            ctx.apply_bindings(&self.gate_bind);
            for (i, g) in m.gates.iter().enumerate() {
                let rel = g.pos - cam.eye;
                if rel.len() > 24000.0 || g.passed {
                    continue;
                }
                let ax = g.axis.norm();
                let right = ax.cross(V3::Y).norm();
                let up = right.cross(ax);
                let model = basis(right, up, ax, rel);
                let color = if i == m.gate_index { [1.0, 0.66, 0.22] } else { [0.30, 0.79, 0.74] };
                ctx.apply_uniforms(UniformsSource::table(&GateU { vp, model, color, fog: FOG, time: time as f32, glow: 1.0, inflate: 4.0 }));
                ctx.draw(0, self.gate_count, 1);
            }
        }

        self.fx.draw(ctx, vp, Layer::Particles);

        // Clouds after opaque geometry, sorted back to front.
        self.update_clouds(ctx, cam);
        ctx.apply_pipeline(&self.cloud);
        ctx.apply_bindings(&self.cloud_bind);
        ctx.apply_uniforms(UniformsSource::table(&CloudU { vp, eye, right: cam.right.f32(), up: cam.up.f32(), fog: FOG }));
        ctx.draw(0, (self.clouds.len() * 6) as i32, 1);

        // Chase canopy: back faces first, then front faces, for correct layering.
        if let (Some(a), false) = (aircraft, cockpit) {
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let u = AirU { vp, model, sun, fog: FOG, alpha: 0.42, glow: 0.0, cutout: 0.0 };
            ctx.apply_pipeline(&self.glass_front);
            self.draw_part(ctx, &self.canopy, &u, &model);
            ctx.apply_pipeline(&self.glass_back);
            self.draw_part(ctx, &self.canopy, &u, &model);
        }
    }

    fn draw_part(&self, ctx: &mut dyn RenderingBackend, p: &Part, u: &AirU, model: &M4) {
        ctx.apply_bindings(&p.bind);
        ctx.apply_uniforms(UniformsSource::table(&AirU { model: *model, ..*u }));
        ctx.draw(0, p.count, 1);
    }

    fn update_clouds(&mut self, ctx: &mut dyn RenderingBackend, cam: &Camera) {
        let mut order: Vec<(f64, usize)> = self.clouds.iter().enumerate().map(|(i, c)| ((c.pos - cam.eye).dot(c.pos - cam.eye), i)).collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.cloud_data.clear();
        for &(_, i) in &order {
            let c = &self.clouds[i];
            for (cx, cy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                self.cloud_data.extend_from_slice(&[c.pos.x as f32, c.pos.y as f32, c.pos.z as f32, cx, cy, c.size as f32, c.alpha as f32]);
            }
        }
        ctx.buffer_update(self.cloud_bind.vertex_buffers[0], BufferSource::slice(&self.cloud_data));
    }

    /// CRT composite of the low-resolution scene into the current pass.
    pub fn post(&self, ctx: &mut dyn RenderingBackend, scene: &Target, w: f32, h: f32, gloc: f32, seed: f32) {
        ctx.apply_pipeline(&self.post);
        ctx.apply_bindings(&Bindings { images: vec![scene.color], ..self.post_bind.clone() });
        ctx.apply_uniforms(UniformsSource::table(&PostU { res: [w, h], gloc, seed }));
        ctx.draw(0, 3, 1);
    }
}

/// Local transform rotating about a pivot point.
pub fn rotate_about(pivot: V3, q: Quat) -> M4 {
    let (r, u, b) = (q.rotate(V3::X), q.rotate(V3::Y), q.rotate(V3::Z));
    let t = pivot - q.rotate(pivot);
    basis(r, u, b, t)
}

fn box_visible(cam: &Camera, tan_x: f64, tan_y: f64, ch: &Chunk) -> bool {
    // Test the eight corners against the four side planes of the frustum.
    let corners = [
        v3(ch.x0, ch.lo as f64, ch.z0), v3(ch.x0 + CHUNK_M, ch.lo as f64, ch.z0), v3(ch.x0, ch.lo as f64, ch.z0 + CHUNK_M), v3(ch.x0 + CHUNK_M, ch.lo as f64, ch.z0 + CHUNK_M),
        v3(ch.x0, ch.hi as f64, ch.z0), v3(ch.x0 + CHUNK_M, ch.hi as f64, ch.z0), v3(ch.x0, ch.hi as f64, ch.z0 + CHUNK_M), v3(ch.x0 + CHUNK_M, ch.hi as f64, ch.z0 + CHUNK_M),
    ];
    let planes = [(1.0, tan_x, true), (-1.0, tan_x, true), (1.0, tan_y, false), (-1.0, tan_y, false)];
    for (sign, tan, horizontal) in planes {
        let axis = if horizontal { cam.right } else { cam.up };
        let mut inside = false;
        for c in &corners {
            let d = *c - cam.eye;
            let z = d.dot(cam.forward);
            let s = d.dot(axis) * sign;
            if s <= z * tan * 1.05 + 60.0 {
                inside = true;
                break;
            }
        }
        if !inside {
            return false;
        }
    }
    true
}

fn torus() -> (Vec<f32>, Vec<u16>) {
    let (major, tube) = (48usize, 10usize);
    let (big_r, r) = (GATE_RADIUS, 3.2);
    let mut v = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=major {
        let u = i as f64 / major as f64 * TAU;
        for j in 0..=tube {
            let w = j as f64 / tube as f64 * TAU;
            let rr = big_r + r * w.cos();
            v.extend_from_slice(&[(u.cos() * rr) as f32, (u.sin() * rr) as f32, (r * w.sin()) as f32, (u.cos() * w.cos()) as f32, (u.sin() * w.cos()) as f32, w.sin() as f32, (i as f64 / major as f64) as f32]);
        }
    }
    for i in 0..major {
        for j in 0..tube {
            let a = (i * (tube + 1) + j) as u16;
            let b = a + tube as u16 + 1;
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (v, idx)
}

/// Soft cloud sprite: overlapping radial blobs, blurred.
fn cloud_texture() -> Vec<u8> {
    let (w, h) = (256usize, 128usize);
    let mut a = vec![0f32; w * h];
    let mut rnd = Lcg(9127);
    for _ in 0..34 {
        let x = 35.0 + rnd.next() * 186.0;
        let spread = 1.0 - (x - 128.0).abs() / 150.0;
        let y = 68.0 + (rnd.next() - 0.5) * 38.0 * spread;
        let r = 18.0 + rnd.next() * 32.0 * (0.6 + spread);
        let core = 0.24 + rnd.next() * 0.22;
        for py in 0..h {
            for px in 0..w {
                let d = ((px as f64 - x).powi(2) + (py as f64 - y).powi(2)).sqrt() / r;
                if d >= 1.0 {
                    continue;
                }
                let g = if d < 0.55 { mix(core, 0.10, d / 0.55) } else { mix(0.10, 0.0, (d - 0.55) / 0.45) } as f32;
                let o = &mut a[py * w + px];
                *o = *o + g * (1.0 - *o);
            }
        }
    }
    for _ in 0..2 {
        let src = a.clone();
        for y in 0..h {
            for x in 0..w {
                let mut s = 0.0;
                let mut n = 0.0;
                for k in -3i32..=3 {
                    let xx = x as i32 + k;
                    if xx >= 0 && (xx as usize) < w {
                        s += src[y * w + xx as usize];
                        n += 1.0;
                    }
                }
                a[y * w + x] = s / n;
            }
        }
        let src = a.clone();
        for y in 0..h {
            for x in 0..w {
                let mut s = 0.0;
                let mut n = 0.0;
                for k in -3i32..=3 {
                    let yy = y as i32 + k;
                    if yy >= 0 && (yy as usize) < h {
                        s += src[yy as usize * w + x];
                        n += 1.0;
                    }
                }
                a[y * w + x] = s / n;
            }
        }
    }
    a.iter().flat_map(|&v| [255u8, 255, 255, (v.clamp(0.0, 1.0) * 255.0) as u8]).collect()
}
