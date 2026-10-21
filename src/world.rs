//! World renderer: sky, static terrain chunks with LOD, ocean, clouds, gates,
//! the exterior aircraft and the CRT post pass. All world geometry is drawn
//! relative to the eye so f32 precision holds anywhere on the map.
use crate::airframe::{self, Airframe};
use crate::fx::{Fx, Layer};
use crate::gfx::{self, Target};
use crate::math::*;
use crate::mesh::Mesh;
use crate::physio::Vision;
use crate::sim::{Aircraft, Mission, GATE_RADIUS};
use crate::terrain::{self, CHUNKS, CHUNK_M, X0, Z0};
use crate::texture;
use miniquad::*;

pub const SUN: V3 = v3(0.42, 0.62, -0.66);
/// Horizon haze: the far terrain, the sea and the bottom of the sky fade into it.
pub const FOG: [f32; 3] = [0.72, 0.80, 0.88];
const STRIDES: [usize; 4] = [1, 2, 4, 8];
/// Terrain material samplers, in the order texture::terrain() builds them.
const TERRAIN_TEX: [&str; 6] = ["t_grass", "t_scrub", "t_rock", "t_snow", "t_sand", "t_crops"];

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
"#;

/// 4x4 ordered-dither threshold of a pixel, in (0, 1).
const DITHER: &str = r#"
float bayer4(vec2 p) {
    int i = int(mod(p.x, 4.0)) + 4 * int(mod(p.y, 4.0));
    float b[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return (b[i] + 0.5) / 16.0;
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
uniform vec3 u_fog;
out vec4 frag;
void main() {
    vec2 ndc = v_uv * 2.0 - 1.0;
    vec3 dir = normalize(u_fwd + u_right * ndc.x * u_tan * u_aspect + u_up * ndc.y * u_tan);
    float y = dir.y;
    // Clear day: deep blue overhead, pale haze at the horizon, a hard sun disc.
    vec3 zenith = vec3(0.13, 0.33, 0.72);
    vec3 sky = mix(u_fog, zenith, pow(clamp(y, 0.0, 1.0), 0.42));
    float sun = max(dot(dir, u_sun), 0.0);
    sky += vec3(1.0, 0.94, 0.78) * pow(sun, 10.0) * 0.22;
    if (sun > 0.9994) sky = vec3(1.0, 0.98, 0.88);
    if (y < 0.0) sky = mix(u_fog, vec3(0.50, 0.58, 0.62), clamp(-y * 5.0, 0.0, 1.0));
    frag = vec4(sky, 1.0);
}"#;

const VS_TERRAIN: &str = r#"
in vec3 pos;
in vec3 normal;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform vec3 u_fog;
out vec3 v_world;
out vec3 v_normal;
out float v_light;
void main() {
    v_world = pos;
    v_normal = normal;
    // Gouraud: one lighting value per vertex, interpolated across the triangle.
    v_light = 0.30 + 0.20 * normal.y + 0.85 * max(dot(normal, u_sun), 0.0);
    gl_Position = u_vp * vec4(pos - u_eye, 1.0);
}"#;

const FS_TERRAIN: &str = r#"
in vec3 v_world;
in vec3 v_normal;
in float v_light;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform vec3 u_fog;
uniform sampler2D t_grass;
uniform sampler2D t_scrub;
uniform sampler2D t_rock;
uniform sampler2D t_snow;
uniform sampler2D t_sand;
uniform sampler2D t_crops;
out vec4 frag;
NOISE
DITHER
float river_x(float z) { return 1500.0 * sin(z / 5200.0) + 420.0 * sin(z / 1900.0 + 0.7); }
void main() {
    vec3 rel = v_world - u_eye;
    vec3 n = normalize(v_normal);
    float h = v_world.y;
    float slope = 1.0 - n.y;
    float th = bayer4(gl_FragCoord.xy);
    float macro = vnoise(v_world.xz * 0.0012) * 0.6 + vnoise(v_world.xz * 0.005) * 0.4 - 0.5;

    // Every material is sampled up front so the mip selection stays valid.
    vec2 uv = v_world.xz / 128.0;
    vec2 cliff = vec2((v_world.x + v_world.z) / 64.0, h / 64.0);
    vec2 fp = mat2(0.94, -0.34, 0.34, 0.94) * v_world.xz;
    vec2 field = fp / vec2(300.0, 210.0);
    float k = hash21(floor(field));
    vec3 grass = texture(t_grass, uv).rgb;
    vec3 scrub = texture(t_scrub, uv).rgb;
    vec3 rock = texture(t_rock, mix(uv, cliff, smoothstep(0.25, 0.5, slope))).rgb;
    vec3 snow = texture(t_snow, uv).rgb;
    vec3 sand = texture(t_sand, uv * 1.5).rgb;
    vec3 crops = texture(t_crops, (k > 0.5 ? fp.xy : fp.yx) / 64.0).rgb;

    // Hard, dithered borders between materials instead of soft blends.
    vec3 col = grass;
    if (smoothstep(620.0, 700.0, h + macro * 700.0) > th) col = scrub;
    if (smoothstep(0.30, 0.36, slope + macro * 0.16) + smoothstep(1650.0, 1750.0, h + macro * 500.0) > th) col = rock;
    if (smoothstep(2050.0, 2120.0, h + macro * 350.0) * (1.0 - smoothstep(0.45, 0.52, slope)) > th) col = snow;
    if (smoothstep(22.0, 12.0, h) > th) col = sand;

    // Patchwork farmland on the gentle valley floor, with hedgerows.
    float farm = (1.0 - smoothstep(0.03, 0.07, slope)) * (1.0 - smoothstep(560.0, 680.0, h)) * step(40.0, h);
    if (farm > th && macro > -0.25) {
        vec3 tint = k < 0.3 ? vec3(0.52, 0.66, 0.30) : k < 0.55 ? vec3(0.86, 0.76, 0.40) : k < 0.8 ? vec3(0.62, 0.48, 0.32) : vec3(0.40, 0.56, 0.26);
        col = crops * tint;
        vec2 e = min(fract(field), 1.0 - fract(field)) * vec2(300.0, 210.0);
        if (min(e.x, e.y) < 6.0) col = vec3(0.20, 0.30, 0.13);
    }

    // The Cold River down the gorge floor, with sand banks.
    float rd = abs(v_world.x - river_x(v_world.z) + (vnoise(v_world.xz * 0.012) - 0.5) * 24.0);
    float wet = 1.0 - smoothstep(0.05, 0.12, slope);
    float light = v_light;
    if (rd < 38.0 && wet > th) col = sand;
    if (rd < 22.0 && wet > th) {
        float ripple = step(0.8, vnoise(v_world.xz * vec2(0.08, 0.02)));
        col = mix(vec3(0.18, 0.40, 0.56), vec3(0.62, 0.78, 0.86), ripple * 0.6);
        light = 0.95;
    }

    vec3 lit = col * light;
    float dist = length(rel);
    float fog = clamp(1.0 - exp(-dist * 0.000075), 0.0, 1.0);
    fog = max(fog, smoothstep(20000.0, 36000.0, dist));
    frag = vec4(mix(lit, u_fog, fog), 1.0);
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
NOISE
void main() {
    vec3 rel = v_world - u_eye;
    vec3 view = normalize(rel);
    float fresnel = pow(1.0 - clamp(-view.y, 0.0, 1.0), 4.0);
    vec3 col = mix(vec3(0.06, 0.26, 0.44), vec3(0.32, 0.52, 0.70), fresnel);
    // Blocky wave crests drifting with time, and a sun glitter path.
    float crest = step(0.78, vnoise(v_world.xz * vec2(0.035, 0.09) + vec2(u_time * 0.15, 0.0)));
    col = mix(col, vec3(0.55, 0.72, 0.84), crest * 0.35);
    float sun = pow(max(dot(reflect(view, vec3(0.0, 1.0, 0.0)), u_sun), 0.0), 60.0);
    col += vec3(1.0, 0.95, 0.8) * sun * step(0.55, vnoise(v_world.xz * 0.2 + u_time)) * 0.9;
    float fog = clamp(1.0 - exp(-length(rel) * 0.00006), 0.0, 1.0);
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
DITHER
void main() {
    vec4 t = texture(u_tex, v_uv);
    // Screen-door transparency: whole pixels are either cloud or sky.
    if (smoothstep(0.30, 0.55, t.a) * v_alpha < bayer4(gl_FragCoord.xy)) discard;
    // Same haze as the terrain, so a distant cloud never looks crisper (and
    // so nearer) than the mountains in front of it.
    float dist = length(v_rel);
    float fog = max(1.0 - exp(-dist * 0.000075), smoothstep(20000.0, 36000.0, dist));
    frag = vec4(mix(t.rgb, u_fog, fog), 1.0);
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
    // Alternating painted segments, like the pylons of an air race.
    float band = 0.62 + 0.38 * step(0.5, fract(v_u * 8.0));
    float pulse = 0.85 + 0.15 * step(0.0, sin(u_time * 6.0 + v_u * 25.0));
    vec3 col = u_color * (0.62 + 0.55 * max(dot(n, normalize(vec3(0.4, 0.6, 0.3))), 0.0));
    col *= band * pulse;
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.00005), 0.0, 0.92);
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
uniform vec2 u_flex;
out vec3 v_rel;
out vec3 v_normal;
out vec3 v_color;
out vec3 v_local;
out vec2 v_uv;
out float v_light;
void main() {
    // Elastic wing: bent up under load, the tips swapping in the flutter
    // mode, the root and everything inboard or aft of the wing held still.
    vec3 p = pos;
    float span = clamp((abs(pos.x) - 1.2) / 3.6, 0.0, 1.0) * step(pos.z, 3.6);
    p.y += (u_flex.x - sign(pos.x) * u_flex.y) * span * span;
    vec4 rel = u_model * vec4(p, 1.0);
    v_rel = rel.xyz;
    v_local = pos;
    vec3 n = normalize(mat3(u_model) * normal);
    v_normal = n;
    v_color = color;
    v_uv = uv;
    // Gouraud: one light value per vertex, interpolated across the polygon.
    v_light = 0.36 + 0.16 * n.y + 0.80 * max(dot(n, normalize(u_sun)), 0.0);
    gl_Position = u_vp * rel;
}"#;

const FS_AIR: &str = r#"
in vec3 v_rel;
in vec3 v_normal;
in vec3 v_color;
in vec3 v_local;
in vec2 v_uv;
in float v_light;
uniform mat4 u_vp;
uniform mat4 u_model;
uniform vec3 u_sun;
uniform vec3 u_fog;
uniform float u_alpha;
uniform float u_glow;
uniform float u_cutout;
uniform sampler2D u_tex;
out vec4 frag;
void main() {
    // From the seat, the cockpit model replaces the skin inside the canopy.
    if (u_cutout > 0.5 && v_local.z > -4.70 && v_local.z < -1.35 && abs(v_local.x) < 0.50 && v_local.y > 0.30) discard;
    vec3 base = v_uv.x >= 0.0 ? texture(u_tex, v_uv).rgb * v_color : v_color;
    vec3 col = base * v_light;
    float a = u_alpha;
    if (u_alpha < 0.9 && u_glow < 0.5) {
        // Tinted canopy: Fresnel reflection of the sky and a sun glint.
        vec3 view_dir = normalize(-v_rel);
        vec3 n = normalize(v_normal);
        if (dot(n, view_dir) < 0.0) n = -n;
        vec3 sun = normalize(u_sun);
        float fres = pow(1.0 - max(dot(n, view_dir), 0.0), 4.0);
        float glint = step(0.985, max(dot(reflect(-sun, n), view_dir), 0.0));
        float sky_ref = 0.35 * max(n.y, 0.0);
        col = mix(v_color, vec3(0.62, 0.72, 0.82), fres * 0.7 + sky_ref) + glint * vec3(1.0, 0.95, 0.8);
        a = clamp(u_alpha + fres * 0.55 + sky_ref * 0.4 + glint, 0.0, 0.92);
    }
    if (u_glow > 0.5) col = v_color * 1.8;
    float fog = clamp(1.0 - exp(-length(v_rel) * 0.00005), 0.0, 0.9);
    col = mix(col, u_fog, fog);
    frag = vec4(col, a * (1.0 - fog * 0.8));
}"#;

const FS_POST: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
uniform vec2 u_res;
uniform vec2 u_src;
uniform float u_gloc;
uniform float u_seed;
out vec4 frag;
DITHER
void main() {
    // One sample per low-resolution pixel, shown as a crisp block.
    vec2 px = min(floor(v_uv * u_src), u_src - 1.0);
    vec3 col = texture(u_tex, (px + 0.5) / u_src).rgb;
    // A slightly punchier grade (cockpit::FACE_FS repeats it for the sharp
    // displays: keep the two in step), then 15-bit colour with ordered dithering
    // laid on the same low-resolution pixels.
    float l = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(l), col, 1.12);
    col = (col - 0.5) * 1.04 + 0.5;
    // Grey-out as the retina starves: colour and light drain from the view
    // (physio::Vision); the tunnel, blackout and redout go on in FS_VISION.
    l = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(col, vec3(l) * 0.72, u_gloc);
    col = floor(clamp(col, 0.0, 1.0) * 31.0 + bayer4(px)) / 31.0;
    frag = vec4(clamp(col, 0.0, 1.0), 1.0);
}"#;

/// The pilot's vision over everything in front of the eyes, HUD included:
/// light loss closing in from the edge, central blackout and the redout
/// veil, screen-doored on the scene's low-resolution pixels.
const FS_VISION: &str = r#"
in vec2 v_uv;
uniform vec2 u_res;
uniform vec2 u_src;
uniform vec4 u_vision;
out vec4 frag;
DITHER
void main() {
    vec2 px = floor(v_uv * u_src);
    // The eyes rest a little above the middle of the screen.
    float r = length((v_uv - vec2(0.5, 0.56)) * vec2(u_res.x / u_res.y, 1.0));
    float edge = mix(1.3, -0.1, u_vision.x);
    float dark = max(smoothstep(edge - 0.25, edge + 0.05, r) * step(0.002, u_vision.x), u_vision.z);
    float red = u_vision.w * (1.0 - dark);
    float a = max(dark, red * 0.7);
    a = clamp(floor(a * 8.0 + bayer4(px)) / 8.0, 0.0, 1.0);
    frag = vec4(vec3(0.40, 0.02, 0.02) * red / max(a, 0.001), a);
}"#;

fn src(s: &str) -> String {
    s.replace("NOISE", NOISE).replace("DITHER", DITHER)
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
    fog: [f32; 3],
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
    /// Static wingtip bend and the antisymmetric flutter deflection, m.
    pub flex: [f32; 2],
}
#[repr(C)]
struct PostU {
    res: [f32; 2],
    src: [f32; 2],
    gloc: f32,
    seed: f32,
}
#[repr(C)]
struct VisionU {
    res: [f32; 2],
    src: [f32; 2],
    vision: [f32; 4],
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
    terrain_tex: Vec<TextureId>,
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
    vision: Pipeline,
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

        let s = gfx::shader(ctx, VS_FULL, &src(FS_SKY), &[], &[("u_cam", Float3), ("u_right", Float3), ("u_up", Float3), ("u_fwd", Float3), ("u_sun", Float3), ("u_tan", Float1), ("u_aspect", Float1), ("u_time", Float1), ("u_fog", Float3)]);
        let sky = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, PipelineParams::default());

        // Terrain chunks: one vertex buffer per chunk and stride, shared index buffers.
        let terrain_u = [("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_fog", Float3)];
        let s = gfx::shader(ctx, VS_TERRAIN, &src(FS_TERRAIN), &TERRAIN_TEX, &terrain_u);
        let terrain_attrs = [VertexAttribute::new("pos", VertexFormat::Float3), VertexAttribute::new("normal", VertexFormat::Float3)];
        let terrain = ctx.new_pipeline(&[BufferLayout::default()], &terrain_attrs, s, depth);
        // Pixel tiles: nearest texels up close, nearest mip levels far away.
        let tile = texture::TILE as u32;
        let terrain_tex: Vec<TextureId> = texture::terrain()
            .iter()
            .map(|px| {
                let params = TextureParams { width: tile, height: tile, format: TextureFormat::RGBA8, wrap: TextureWrap::Repeat, min_filter: FilterMode::Nearest, mag_filter: FilterMode::Nearest, mipmap_filter: MipmapFilterMode::Nearest, allocate_mipmaps: true, ..Default::default() };
                let t = ctx.new_texture_from_data_and_format(px, params);
                ctx.texture_generate_mipmaps(t);
                t
            })
            .collect();
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
        let s = gfx::shader(ctx, VS_OCEAN, &src(FS_OCEAN), &[], &[("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_fog", Float3), ("u_time", Float1)]);
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

        // Clouds: camera-facing billboards with screen-door transparency.
        let s = gfx::shader(ctx, VS_CLOUD, &src(FS_CLOUD), &["u_tex"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_right", Float3), ("u_up", Float3), ("u_fog", Float3)]);
        let cloud = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("center", VertexFormat::Float3), VertexAttribute::new("corner", VertexFormat::Float2), VertexAttribute::new("size", VertexFormat::Float1), VertexAttribute::new("alpha", VertexFormat::Float1)],
            s,
            depth,
        );
        let mut rnd = Lcg(4317);
        let clouds: Vec<Cloud> = (0..58)
            .map(|_| {
                let x = (rnd.next() * 2.0 - 1.0) * 19000.0;
                let z = 13500.0 - rnd.next() * 29000.0;
                let ground = terrain::terrain_at(x, z);
                let y = (ground + 420.0).max(1250.0) + rnd.next() * 1600.0;
                let size = 1000.0 + rnd.next() * 2100.0;
                // The billboard turns to face the camera, so the puff (reaching about
                // 0.45 size below centre) must clear every peak within one size of
                // it, or a ridge would slice it along a line that moves as you turn.
                let mut peak = ground;
                for k in 0..32 {
                    let a = k as f64 * TAU / 32.0;
                    for r in [0.35, 0.7, 1.0] {
                        peak = peak.max(terrain::terrain_at(x + a.cos() * size * r, z + a.sin() * size * r));
                    }
                }
                let y = y.max(peak + 0.45 * size + 150.0);
                let alpha = 0.8 + rnd.next() * 0.2;
                Cloud { pos: v3(x, y, z), size, alpha }
            })
            .collect();
        let cloud_tex = ctx.new_texture_from_data_and_format(&texture::cloud(), TextureParams { width: 128, height: 64, format: TextureFormat::RGBA8, min_filter: FilterMode::Nearest, mag_filter: FilterMode::Nearest, ..Default::default() });
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
        let air_u = [("u_vp", Mat4), ("u_model", Mat4), ("u_sun", Float3), ("u_fog", Float3), ("u_alpha", Float1), ("u_glow", Float1), ("u_cutout", Float1), ("u_flex", Float2)];
        let s = gfx::shader(ctx, VS_AIR, FS_AIR, &["u_tex"], &air_u);
        let skin_params = TextureParams { width: airframe::SKIN_W as u32, height: airframe::SKIN_H as u32, format: TextureFormat::RGBA8, wrap: TextureWrap::Clamp, min_filter: FilterMode::Nearest, mag_filter: FilterMode::Nearest, mipmap_filter: MipmapFilterMode::Nearest, allocate_mipmaps: true, ..Default::default() };
        let skin = ctx.new_texture_from_data_and_format(&airframe.skin, skin_params);
        ctx.texture_generate_mipmaps(skin);
        let air = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, depth);
        let glass = |cull| PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::alpha_blend()), cull_face: cull, ..Default::default() };
        let glass_back = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, glass(CullFace::Back));
        let glass_front = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, glass(CullFace::Front));
        let flame_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::additive()), ..Default::default() });

        let s = gfx::shader(ctx, VS_FULL, &src(FS_POST), &["u_tex"], &[("u_res", Float2), ("u_src", Float2), ("u_gloc", Float1), ("u_seed", Float1)]);
        let post = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, PipelineParams::default());
        let s = gfx::shader(ctx, VS_FULL, &src(FS_VISION), &[], &[("u_res", Float2), ("u_src", Float2), ("u_vision", Float4)]);
        let vision = ctx.new_pipeline(&[BufferLayout::default()], &pos2, s, PipelineParams { color_blend: Some(gfx::alpha_blend()), ..Default::default() });

        let mut skinned = |m: &Mesh| {
            let mut p = part(ctx, m);
            p.bind.images = vec![skin];
            p
        };
        let (exterior, canopy, crew, flame) = (skinned(&airframe.exterior), skinned(&airframe.canopy), skinned(&airframe.crew), skinned(&airframe.flame));
        let stab = [skinned(&airframe.stab[0]), skinned(&airframe.stab[1])];
        let rudder = skinned(&airframe.rudder);
        let brakes = [skinned(&airframe.brakes[0]), skinned(&airframe.brakes[1]), skinned(&airframe.brakes[2]), skinned(&airframe.brakes[3])];
        World {
            post_bind: fullscreen.clone(),
            fullscreen,
            sky,
            terrain,
            terrain_tex,
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
            exterior,
            canopy,
            crew,
            flame,
            stab,
            rudder,
            brakes,
            post,
            vision,
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
            fog: FOG,
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
            ctx.apply_bindings(&Bindings { vertex_buffers: vec![vb], index_buffer: self.lod_ib[lod], images: self.terrain_tex.clone() });
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

        // Clouds are screen-door cutouts that write depth, so they go in with the
        // opaque geometry: plume, flame and particles drawn later without depth
        // writes would otherwise be painted over by a cloud behind them.
        self.update_clouds(ctx, cam);
        ctx.apply_pipeline(&self.cloud);
        ctx.apply_bindings(&self.cloud_bind);
        ctx.apply_uniforms(UniformsSource::table(&CloudU { vp, eye, right: cam.right.f32(), up: cam.up.f32(), fog: FOG }));
        ctx.draw(0, (self.clouds.len() * 6) as i32, 1);

        // Exterior aircraft, shared by both views; first person omits canopy and crew.
        self.fx.prepare(ctx, cam, time, aircraft);
        if let Some(a) = aircraft {
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let base = AirU { vp, model, sun, fog: FOG, alpha: 1.0, glow: 0.0, cutout: if cockpit { 1.0 } else { 0.0 }, flex: [a.structure.bend as f32, a.structure.flutter as f32] };
            ctx.apply_pipeline(&self.air);
            self.draw_part(ctx, &self.exterior, &base, &model);
            let pitch = clamp(a.surf_pitch, -1.0, 1.0);
            let roll = clamp(a.surf_roll, -1.0, 1.0);
            for (i, side) in [-1.0, 1.0].iter().enumerate() {
                // True stabilator deflection, plus the differential the FLCS mixes in for roll.
                let angle = clamp(-pitch * 25.0 * DEG + side * -roll * 5.0 * DEG, -25.0 * DEG, 25.0 * DEG);
                let pivot = v3(side * airframe::STAB_PIVOT.x, airframe::STAB_PIVOT.y, airframe::STAB_PIVOT.z);
                let local = rotate_about(pivot, Quat::axis_angle(airframe::stab_axis(*side), angle));
                self.draw_part(ctx, &self.stab[i], &base, &mat_mul(&model, &local));
            }
            let (hinge, axis) = airframe::rudder_axis();
            let local = rotate_about(hinge, Quat::axis_angle(axis, clamp(a.surf_yaw, -1.0, 1.0) * 30.0 * DEG));
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

        // Chase canopy: back faces first, then front faces, for correct layering.
        if let (Some(a), false) = (aircraft, cockpit) {
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let u = AirU { vp, model, sun, fog: FOG, alpha: 0.42, glow: 0.0, cutout: 0.0, flex: [0.0; 2] };
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
        ctx.apply_uniforms(UniformsSource::table(&PostU { res: [w, h], src: [scene.w as f32, scene.h as f32], gloc, seed }));
        ctx.draw(0, 3, 1);
    }

    /// The pilot's light loss and redout over the current pass.
    pub fn vision(&self, ctx: &mut dyn RenderingBackend, scene: &Target, w: f32, h: f32, v: &Vision) {
        if v.tunnel < 0.002 && v.black < 0.002 && v.red < 0.002 {
            return;
        }
        ctx.apply_pipeline(&self.vision);
        ctx.apply_bindings(&self.post_bind);
        let vision = [v.tunnel as f32, v.grey as f32, v.black as f32, v.red as f32];
        ctx.apply_uniforms(UniformsSource::table(&VisionU { res: [w, h], src: [scene.w as f32, scene.h as f32], vision }));
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
    let (major, tube) = (24usize, 6usize);
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

