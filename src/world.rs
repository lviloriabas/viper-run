//! World renderer: sky, static terrain chunks with LOD, ocean, clouds, gates,
//! the exterior aircraft and the CRT post pass. All world geometry is drawn
//! relative to the eye so f32 precision holds anywhere on the map.
use crate::airframe::{self, Airframe};
use crate::fx::{Fx, Layer};
use crate::gfx::{self, Target};
use crate::math::*;
use crate::mesh::Mesh;
use crate::physio::Vision;
use crate::sim::{Aircraft, Mission, Piece, Type, GATE_RADIUS};
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

/// An F-16 flaperon about its hinge: drooped with the flaps, 20 degrees
/// full down, and moving against its twin for roll, 21.5 degrees either
/// way, within its 23 up and 20 down; bent further down when the air load
/// has damaged it.
fn flaperon_local(a: &Aircraft, side: f64) -> M4 {
    let (hz, hy, _, _) = airframe::FLAPERON_HINGE;
    let roll = clamp(a.surf_roll, -1.0, 1.0);
    let angle = clamp(a.flaps * 20.0 * DEG - side * roll * 21.5 * DEG, -23.0 * DEG, 20.0 * DEG) + a.flap_damage * 8.0 * DEG;
    rotate_about(v3(0.0, hy, hz), Quat::axis_angle(V3::X, angle))
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

    // The airbase (terrain::RUNWAY_*): mown grass round a runway with its
    // markings, the parallel taxiway with its links and the apron.
    vec2 rw = v_world.xz - vec2(1045.0, 9800.0);
    float ru = dot(rw, vec2(-0.095983, -0.995383));
    float rv = dot(rw, vec2(0.995383, -0.095983));
    float au = abs(ru);
    if (au < 1600.0 && rv > -360.0 && rv < 110.0) {
        col = grass * vec3(0.92, 1.0, 0.85);
        float grit = vnoise(v_world.xz * 0.35) * 0.05;
        vec3 asphalt = vec3(0.21, 0.215, 0.22) + grit;
        vec3 paint = vec3(0.86, 0.86, 0.82);
        // Apron, and the taxiway with links at both ends and the middle.
        if (rv > -340.0 && rv < -185.0 && abs(ru) < 520.0) col = vec3(0.50, 0.50, 0.47) + grit;
        bool link = (abs(au - 1360.0) < 12.0 || au < 12.0) && rv > -170.0 && rv < 0.0;
        if ((abs(rv + 165.0) < 11.0 && au < 1372.0) || link) {
            col = asphalt * 1.15;
            if (abs(rv + 165.0) < 0.5 || (link && abs(au - (au < 12.0 ? 0.0 : 1360.0)) < 0.5)) col = vec3(0.85, 0.72, 0.18);
        }
        if (au < 1400.0 && abs(rv) < 22.5) {
            col = asphalt;
            float edge = abs(rv) > 21.4 ? 1.0 : 0.0;
            float centre = (abs(rv) < 0.6 && fract(ru / 60.0) < 0.5 && au < 1300.0) ? 1.0 : 0.0;
            float keys = (au > 1355.0 && au < 1385.0 && abs(rv) < 19.0 && fract((rv + 19.0) / 3.8) < 0.55) ? 1.0 : 0.0;
            float aim = (au > 1020.0 && au < 1065.0 && abs(rv) > 6.0 && abs(rv) < 11.0) ? 1.0 : 0.0;
            float zone = (au > 1150.0 && au < 1300.0 && abs(rv) > 4.0 && abs(rv) < 7.0 && fract(au / 50.0) < 0.45) ? 1.0 : 0.0;
            if (edge + centre + keys + aim + zone > 0.0) col = paint;
            // Tyre marks in the touchdown zones.
            if (au > 900.0 && au < 1330.0 && abs(abs(rv) - 3.0) < 1.5 && vnoise(v_world.xz * vec2(0.05, 0.5)) > 0.55) col *= 0.55;
        } else if (au < 1460.0 && abs(rv) < 22.5) {
            col = asphalt * 0.85;
            if (abs(rv) < 18.0 && abs(fract(au / 14.0 + abs(rv) / 28.0) - 0.5) < 0.18) col = vec3(0.85, 0.72, 0.18);
        }
    }

    vec3 lit = col * light;
    float dist = length(rel);
    float fog = clamp(1.0 - exp(-dist * 0.000075), 0.0, 1.0);
    fog = max(fog, smoothstep(20000.0, 36000.0, dist));
    frag = vec4(mix(lit, u_fog, fog), 1.0);
}"#;

const VS_OCEAN
: &str = r#"
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
    // Turned about the vertical only, so a puff stays upright when the view
    // rolls or pitches instead of swinging round with the camera.
    vec3 to = center - u_eye;
    vec3 side = vec3(-to.z, 0.0, to.x);
    side = dot(side, side) > 1.0 ? normalize(side) : u_right;
    vec3 rel = to + side * corner.x * size + vec3(0.0, 1.0, 0.0) * corner.y * size * 0.5;
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
uniform vec4 u_flex;
uniform float u_flex_aft;
uniform vec4 u_paint;
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
    float span = clamp((abs(pos.x) - u_flex.z) / (u_flex.w - u_flex.z), 0.0, 1.0) * step(pos.z, u_flex_aft);
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
uniform vec4 u_paint;
uniform vec4 u_cut;
uniform float u_flex_aft;
uniform sampler2D u_tex;
out vec4 frag;
void main() {
    // A wing broken off: outboard of the break, ahead of the tail and low
    // enough to miss the fins. The rest of the jet leaves it out; the
    // falling piece is drawn with only it.
    float wing = step(u_cut.w, abs(v_local.x)) * step(v_local.z, u_flex_aft) * step(v_local.y, 1.0);
    float gone = wing * (v_local.x < 0.0 ? u_cut.x : u_cut.y);
    if (u_cut.z > 0.5 ? gone < 0.5 : gone > 0.5) discard;
    // From the seat, the cockpit model replaces the skin inside the canopy.
    // The MiG-29 box is mig29::CUTOUT.
    if (u_cutout > 3.5) {
        if (v_local.z > -7.15 && v_local.z < -4.55 && abs(v_local.x) < 0.48 && v_local.y > 0.45) discard;
    } else if (u_cutout > 2.5) {
        if (v_local.z > -5.15 && v_local.z < -1.95 && abs(v_local.x) < 0.43 && v_local.y > 0.40) discard;
    } else if (u_cutout > 1.5) {
        if (abs(v_local.x) < 0.44 && v_local.y > 0.55 && v_local.z > -4.78 && v_local.z < -2.0) discard;
    } else if (u_cutout > 0.5 && v_local.z > -4.70 && v_local.z < -1.35 && abs(v_local.x) < 0.50 && v_local.y > 0.30) discard;
    vec3 base = v_uv.x >= 0.0 ? texture(u_tex, v_uv).rgb * v_color : v_color;
    if (v_uv.x < -2.5) {
        // Flanker blue-grey scheme (su27::flag_camo): larger, softer cells
        // and a tighter palette than the MiG's high-contrast digital paint.
        vec3 cell = floor(v_local * 3.1);
        vec3 c = (cell + 0.5) / 3.1;
        float r1 = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.5453) - 0.5;
        float r2 = fract(sin(dot(cell, vec3(39.346, 11.135, 83.155))) * 24634.6345) - 0.5;
        float dark = sin(c.z * 0.72 + 1.15 * sin(c.x * 0.62)) + sin(c.x * 1.22 - c.z * 0.48 + c.y * 1.55) + 0.8 * r1;
        float mid = sin(c.z * 1.42 - 0.55 * c.x + 1.8) + sin(c.x * 1.85 + c.z * 0.92 + c.y) + 0.8 * r2;
        vec3 paint = dark > 0.72 ? vec3(0.38, 0.44, 0.50) : (mid > 0.12 ? vec3(0.50, 0.57, 0.62) : vec3(0.61, 0.67, 0.70));
        base = paint * (v_color.g / 0.72);
    } else if (v_uv.x < -1.5) {
        // Digital camouflage (mig29::flag_camo): 22 cm square cells in body
        // space, each taking one of three grey-blues from two blob fields
        // sampled at the cell centre, so the blobs keep stepped pixel edges,
        // with a scatter of stray cells along them. The vertex colour's
        // brightness (darker on the moving surfaces) carries over.
        vec3 cell = floor(v_local * 4.5);
        vec3 c = (cell + 0.5) / 4.5;
        float r1 = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.5453) - 0.5;
        float r2 = fract(sin(dot(cell, vec3(39.346, 11.135, 83.155))) * 24634.6345) - 0.5;
        float dark = sin(c.z * 0.85 + 1.3 * sin(c.x * 0.7)) + sin(c.x * 1.35 - c.z * 0.55 + c.y * 1.7) + 0.9 * r1;
        float mid = sin(c.z * 1.6 - 0.6 * c.x + 2.0) + sin(c.x * 2.1 + c.z * 1.05 + c.y) + 0.9 * r2;
        vec3 paint = dark > 0.75 ? vec3(0.34, 0.40, 0.47) : (mid > 0.15 ? vec3(0.52, 0.60, 0.67) : vec3(0.72, 0.77, 0.80));
        base = paint * (v_color.g / 0.72);
    }
    // Repaint 2 tints everything (burnt wrecks); aggressors wear their own
    // skin texture instead (World::draw_jet).
    if (u_paint.w > 1.5) base *= u_paint.rgb;
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
/// A rigid part: no bend, and a span that keeps the shader's division finite.
const NO_FLEX: [f32; 4] = [0.0, 0.0, 1.0, 2.0];
const NO_CUT: [f32; 4] = [0.0; 4];

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
    /// Wingtip bend and the antisymmetric flutter deflection (m), and the
    /// span stations of the root and the tip; nothing aft of `flex_aft` bends.
    pub flex: [f32; 4],
    pub flex_aft: f32,
    /// Repaint colour and mode (0 none, 1 aggressor camouflage, 2 tint).
    pub paint: [f32; 4],
    /// Wings broken off: left and right gone (1), whether to draw only what
    /// is gone (the falling piece) instead of the rest, and the span station
    /// of the break.
    pub cut: [f32; 4],
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

/// How far a cloud wanders from its spot on the breeze, metres.
const DRIFT: f64 = 450.0;

struct Cloud {
    pos: V3,
    size: f64,
    alpha: f64,
}

struct MovingPart {part:Part,hinge:crate::articulation::Hinge}
struct TwinParts {
    body:Part, canopy:Part, stabs:[Part;2], flaps:[Part;2],
    rudders:[Part;2],brake:Part,gear:[Part;3],controls:Vec<MovingPart>,
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
    flaperons: [Part; 2],
    f16_controls:Vec<MovingPart>,
    mig_controls:Vec<MovingPart>,
    rudder: Part,
    brakes: [Part; 4],
    gear: [Part; 3],
    gear_doors: [Part; 1],
    /// MiG-29: airframe, the right and left stabilators and its own gear legs.
    mig: Part,
    mig_stab: [Part; 2],
    mig_gear: [Part; 3],
    mig_canopy: Part,
    mig_brakes: [Part; 2],
    twins: [TwinParts;2],
    /// Skins of the aggressor F-16s, one per airframe::SCHEMES entry.
    aggressor: Vec<TextureId>,
    /// Buildings, trees and pylons (scenery.rs), each at its cluster origin.
    scenery: Vec<(V3, Part)>,
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
                // The billboard turns to face the camera and drifts up to DRIFT
                // metres from its spot, so the puff (reaching about 0.45 size below
                // centre) must clear every peak within that reach, or a ridge would
                // slice it along a line that moves as you turn.
                let mut peak = ground;
                for k in 0..32 {
                    let a = k as f64 * TAU / 32.0;
                    for r in [0.35, 0.7, 1.0] {
                        peak = peak.max(terrain::terrain_at(x + a.cos() * (size + DRIFT) * r, z + a.sin() * (size + DRIFT) * r));
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
        let air_u = [("u_vp", Mat4), ("u_model", Mat4), ("u_sun", Float3), ("u_fog", Float3), ("u_alpha", Float1), ("u_glow", Float1), ("u_cutout", Float1), ("u_flex", Float4), ("u_flex_aft", Float1), ("u_paint", Float4), ("u_cut", Float4)];
        let s = gfx::shader(ctx, VS_AIR, FS_AIR, &["u_tex"], &air_u);
        let skin_params = TextureParams { width: airframe::SKIN_W as u32, height: airframe::SKIN_H as u32, format: TextureFormat::RGBA8, wrap: TextureWrap::Clamp, min_filter: FilterMode::Nearest, mag_filter: FilterMode::Nearest, mipmap_filter: MipmapFilterMode::Nearest, allocate_mipmaps: true, ..Default::default() };
        let skin = ctx.new_texture_from_data_and_format(&airframe.skin, skin_params);
        ctx.texture_generate_mipmaps(skin);
        let aggressor = airframe::SCHEMES
            .iter()
            .map(|k| {
                let t = ctx.new_texture_from_data_and_format(&airframe::aggressor_skin(k), skin_params);
                ctx.texture_generate_mipmaps(t);
                t
            })
            .collect();
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
        let flaperons = [skinned(&airframe.flaperons[0]), skinned(&airframe.flaperons[1])];
        let f16_controls=airframe.slats.iter().map(|c|MovingPart{part:skinned(&c.mesh),hinge:c.hinge}).collect();
        let brakes = [skinned(&airframe.brakes[0]), skinned(&airframe.brakes[1]), skinned(&airframe.brakes[2]), skinned(&airframe.brakes[3])];
        let gear = [skinned(&airframe.gear[0]), skinned(&airframe.gear[1]), skinned(&airframe.gear[2])];
        let gear_doors = [skinned(&airframe.gear_doors[0])];
        let fulcrum = crate::mig29::build();
        let (mig, mig_stab) = (skinned(&fulcrum.body), [skinned(&fulcrum.stabs[0]), skinned(&fulcrum.stabs[1])]);
        let mig_controls=fulcrum.controls.iter().map(|c|MovingPart{part:skinned(&c.mesh),hinge:c.hinge}).collect();
        let mig_gear = [skinned(&fulcrum.gear[0]), skinned(&fulcrum.gear[1]), skinned(&fulcrum.gear[2])];
        let mig_canopy = skinned(&fulcrum.canopy);
        let mig_brakes = [skinned(&fulcrum.brakes[0]), skinned(&fulcrum.brakes[1])];
        let scenery = crate::scenery::build().into_iter().map(|c| (c.origin, skinned(&c.mesh))).collect();
        let twins=[crate::sim::Type::F18,crate::sim::Type::Su27].map(|kind|{
            let m=if kind==Type::F18{crate::hornet::build()}else{crate::su27::build()};
            let bytes=if kind==Type::F18{include_bytes!("../assets/models/f18-skin.qoi").as_slice()}else{include_bytes!("../assets/models/su27-skin.qoi").as_slice()};
            let(w,h,px)=crate::flightgear::image(bytes);
            let tex=ctx.new_texture_from_data_and_format(&px,TextureParams{width:w,height:h,format:TextureFormat::RGBA8,min_filter:FilterMode::Nearest,mag_filter:FilterMode::Nearest,..Default::default()});
            let mut native=|mesh:&Mesh|{let mut p=part(ctx,mesh);p.bind.images=vec![tex];p};
            TwinParts {body:native(&m.body),canopy:native(&m.canopy),
                stabs:m.stabs.map(|m|native(&m)),flaps:m.flaps.map(|m|native(&m)),
                rudders:m.rudders.map(|m|native(&m)),brake:native(&m.brake),gear:m.gear.map(|m|native(&m)),controls:m.controls.into_iter().map(|c|MovingPart{part:native(&c.mesh),hinge:c.hinge}).collect()}
        });
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
            flaperons,
            f16_controls,
            mig_controls,
            rudder,
            brakes,
            gear,
            gear_doors,
            mig,
            mig_stab,
            mig_gear,
            mig_canopy,
            mig_brakes,
            twins,
            aggressor,
            scenery,
            post,
            vision,
            fx: Fx::new(ctx),
            drawn_chunks: 0,
        }
    }

    /// Renders sky, terrain, ocean, clouds, gates, the combat props and the
    /// exterior into the current pass.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, ctx: &mut dyn RenderingBackend, cam: &Camera, aspect: f64, time: f64, mission: Option<&Mission>, aircraft: Option<&Aircraft>, cockpit: bool, props: Option<&dyn Props>) {
        let near = if props.is_some_and(|p| p.first_person()) { 0.03 } else if cockpit { 0.25 } else { 0.7 };
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
        self.update_clouds(ctx, cam, time);
        ctx.apply_pipeline(&self.cloud);
        ctx.apply_bindings(&self.cloud_bind);
        ctx.apply_uniforms(UniformsSource::table(&CloudU { vp, eye, right: cam.right.f32(), up: cam.up.f32(), fog: FOG }));
        ctx.draw(0, (self.clouds.len() * 6) as i32, 1);

        // Scenery within 14 km.
        for (o, part) in &self.scenery {
            let rel = *o - cam.eye;
            if rel.len() < 14000.0 && rel.dot(cam.forward) > -600.0 {
                self.draw_prop(ctx, vp, part, basis(V3::X, V3::Y, V3::Z, rel), [0.0; 4]);
            }
        }
        // Bandits, stores, missiles, bombs and ground targets: opaque, with the jet.
        if let Some(p) = props {
            p.draw(self, ctx, vp, cam);
        }

        // Exterior aircraft, shared by both views; first person omits canopy and crew.
        self.fx.prepare(ctx, cam, time, aircraft);
        if let Some(a) = aircraft {
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let base = AirU { vp, model, sun, fog: FOG, alpha: 1.0, glow: 0.0, cutout: if !cockpit { 0.0 } else { (a.kind.index()+1) as f32 }, flex: a.flex().0, flex_aft: a.flex().1, paint: [0.0; 4], cut: cut_of(a) };
            ctx.apply_pipeline(&self.air);
            if matches!(a.kind,Type::F18|Type::Su27) {
                self.draw_twin(ctx,a,&base,&model,false);
            } else if a.kind == Type::Mig29 {
                // The player's canopy is glass from outside (below) and gone from the seat.
                self.draw_mig(ctx, a, &base, &model, false);
            } else {
            self.draw_part(ctx, &self.exterior, &base, &model);
                self.draw_controls(ctx,a,&base,&model,&self.f16_controls);
            let pitch = clamp(a.surf_pitch, -1.0, 1.0);
            let roll = clamp(a.surf_roll, -1.0, 1.0);
            for (i, side) in [-1.0, 1.0].iter().enumerate() {
                // True stabilator deflection, plus the differential the FLCS mixes in for roll.
                let angle = clamp(-pitch * 25.0 * DEG + side * -roll * 5.0 * DEG, -25.0 * DEG, 25.0 * DEG);
                let pivot = v3(side * airframe::STAB_PIVOT.x, airframe::STAB_PIVOT.y, airframe::STAB_PIVOT.z);
                let local = rotate_about(pivot, Quat::axis_angle(airframe::stab_axis(*side), angle));
                if !a.lost.tail {
                    self.draw_part(ctx, &self.stab[i], &base, &mat_mul(&model, &local));
                }
                if !a.lost.wing[i] {
                    self.draw_part(ctx, &self.flaperons[i], &base, &mat_mul(&model, &flaperon_local(a, *side)));
                }
            }
            let (hinge, axis) = airframe::rudder_axis();
            let local = rotate_about(hinge, Quat::axis_angle(axis, clamp(a.surf_yaw, -1.0, 1.0) * 30.0 * DEG));
            self.draw_part(ctx, &self.rudder, &base, &mat_mul(&model, &local));
            for (i, b) in self.brakes.iter().enumerate() {
                let up = i % 2 == 0;
                // Each petal hinges on its own leading edge, flush with the boom.
                let hinge = v3(0.0, airframe::BOOM_Y + if up { 0.034 } else { -0.034 }, airframe::BRAKE_HINGE_Z);
                let angle = a.brake * 58.0 * DEG * if up { -1.0 } else { 1.0 };
                let local = rotate_about(hinge, Quat::axis_angle(V3::X, angle));
                self.draw_part(ctx, b, &base, &mat_mul(&model, &local));
            }
            self.draw_gear(ctx, a, &base, &model);
            }
            if !cockpit && !a.ejected {
                // Crew placement follows the aircraft's actual seat and design eye.
                let seat = basis(V3::X,V3::Y,V3::Z,crate::cockpit::design_eye(a)-airframe::EYE);
                self.draw_part(ctx, &self.crew, &base, &mat_mul(&model, &seat));
            }
            // The flame sits between the plume's shock diamonds and its sheath.
            self.fx.draw(ctx, vp, Layer::PlumeCore);
            if !cockpit && a.afterburner_visual > 0.03 {
                ctx.apply_pipeline(&self.flame_pip);
                let u = AirU { alpha: (0.18 + 0.30 * a.afterburner_visual) as f32, glow: 1.0, ..base };
                gfx::depth_writes(false);
                for &n in a.kind.nozzles() {
                    let shift=n-airframe::NOZZLE_EXIT;
                    self.draw_part(ctx,&self.flame,&u,&mat_mul(&model,&basis(V3::X,V3::Y,V3::Z,shift)));
                }
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
        if let Some(a) = aircraft.filter(|a| !cockpit && !a.ejected) {
            let canopy = self.twin(a.kind).map(|p|&p.canopy).unwrap_or(if a.kind==Type::Mig29 {&self.mig_canopy}else{&self.canopy});
            let ax = a.q.axes();
            let model = basis(ax.right, ax.up, ax.back, a.pos - cam.eye);
            let u = AirU { vp, model, sun, fog: FOG, alpha: 0.42, glow: 0.0, cutout: 0.0, flex: NO_FLEX, flex_aft: 0.0, paint: [0.0; 4], cut: NO_CUT };
            ctx.apply_pipeline(&self.glass_front);
            self.draw_part(ctx, canopy, &u, &model);
            ctx.apply_pipeline(&self.glass_back);
            self.draw_part(ctx, canopy, &u, &model);
        }
    }

    fn draw_part(&self, ctx: &mut dyn RenderingBackend, p: &Part, u: &AirU, model: &M4) {
        ctx.apply_bindings(&p.bind);
        ctx.apply_uniforms(UniformsSource::table(&AirU { model: *model, ..*u }));
        ctx.draw(0, p.count, 1);
    }

    /// A skinned part wearing another skin texture, if one is given.
    fn draw_part_in(&self, ctx: &mut dyn RenderingBackend, p: &Part, skin: Option<TextureId>, u: &AirU, model: &M4) {
        match skin {
            Some(t) => {
                let bind = Bindings { images: vec![t], ..p.bind.clone() };
                self.draw_part(ctx, &Part { bind, count: p.count }, u, model);
            }
            None => self.draw_part(ctx, p, u, model),
        }
    }

    /// Another F-16 in the scene (a bandit or a tumbling wreck), repainted.
    /// Paint mode 1 puts an F-16 in the aggressor scheme numbered by the
    /// paint's first value.
    /// Moving surfaces and the canopy are only drawn within 2.5 km; beyond
    /// that the jet is a few pixels and the skin alone will do.
    pub fn draw_jet(&self, ctx: &mut dyn RenderingBackend, vp: M4, cam: &Camera, a: &Aircraft, paint: [f32; 4]) {
        let ax = a.q.axes();
        let rel = a.pos - cam.eye;
        let model = basis(ax.right, ax.up, ax.back, rel);
        let base = AirU { vp, model, sun: sun_dir().f32(), fog: FOG, alpha: 1.0, glow: 0.0, cutout: 0.0, flex: a.flex().0, flex_aft: a.flex().1, paint, cut: cut_of(a) };
        ctx.apply_pipeline(&self.air);
        if matches!(a.kind,Type::F18|Type::Su27) {
            self.draw_twin(ctx,a,&base,&model,true);return;
        }
        if a.kind == Type::Mig29 {
            self.draw_mig(ctx, a, &base, &model, true);
            return;
        }
        let camo = (paint[3] == 1.0).then(|| self.aggressor[paint[0] as usize % self.aggressor.len()]);
        let base = AirU { paint: if camo.is_some() { [0.0; 4] } else { paint }, ..base };
        self.draw_part_in(ctx, &self.exterior, camo, &base, &model);
        self.draw_controls(ctx,a,&base,&model,&self.f16_controls);
        if rel.len() > 2500.0 {
            return;
        }
        for (i, side) in [-1.0, 1.0].iter().enumerate() {
            let angle = clamp(-a.surf_pitch * 25.0 * DEG + side * -a.surf_roll * 5.0 * DEG, -25.0 * DEG, 25.0 * DEG);
            let pivot = v3(side * airframe::STAB_PIVOT.x, airframe::STAB_PIVOT.y, airframe::STAB_PIVOT.z);
            let local = rotate_about(pivot, Quat::axis_angle(airframe::stab_axis(*side), angle));
            if !a.lost.tail {
                self.draw_part_in(ctx, &self.stab[i], camo, &base, &mat_mul(&model, &local));
            }
            if !a.lost.wing[i] {
                self.draw_part_in(ctx, &self.flaperons[i], camo, &base, &mat_mul(&model, &flaperon_local(a, *side)));
            }
        }
        let (hinge, axis) = airframe::rudder_axis();
        let local = rotate_about(hinge, Quat::axis_angle(axis, clamp(a.surf_yaw, -1.0, 1.0) * 30.0 * DEG));
        self.draw_part_in(ctx, &self.rudder, camo, &base, &mat_mul(&model, &local));
        self.draw_gear(ctx, a, &base, &model);
        // Opaque canopy: the gold tint reads at a distance without the glass passes.
        self.draw_part(ctx, &self.canopy, &base, &model);
    }

    fn twin(&self,kind:Type)->Option<&TwinParts> {
        match kind {Type::F18|Type::F18E|Type::EA18G=>Some(&self.twins[0]),Type::Su27=>Some(&self.twins[1]),_=>None}
    }
    fn draw_twin(&self,ctx:&mut dyn RenderingBackend,a:&Aircraft,base:&AirU,model:&M4,canopy:bool) {
        let Some(p)=self.twin(a.kind)else{return};
        self.draw_part(ctx,&p.body,base,model);
        self.draw_controls(ctx,a,base,model,&p.controls);
        if canopy{self.draw_part(ctx,&p.canopy,base,model);}
        let flanker=a.kind==Type::Su27;
        let pivot=if flanker{crate::su27::STAB_PIVOT}else{crate::hornet::STAB_PIVOT};
        for (i,side) in [-1.0,1.0].into_iter().enumerate(){
            let angle=(-a.surf_pitch*25.0-side*a.surf_roll*5.0).clamp(-25.0,25.0)*DEG;
            if !a.lost.tail{
                let local=rotate_about(v3(side*pivot.x,pivot.y,pivot.z),Quat::axis_angle(V3::X,angle));
                self.draw_part(ctx,&p.stabs[i],base,&mat_mul(model,&local));
            }
            if !a.lost.wing[i]{
                let (fp,axis)=crate::hornet::flap_hinge(flanker,side);
                let angle=(a.flaps*a.kind.flap_angle()-if flanker{side*a.surf_roll*16.0}else{0.0}+a.flap_damage*8.0)*DEG;
                self.draw_part(ctx,&p.flaps[i],base,&mat_mul(model,&rotate_about(fp,Quat::axis_angle(axis,angle))));
            }
            let (pivot,axis)=crate::hornet::rudder_hinge(flanker,side);
            let local=rotate_about(pivot,Quat::axis_angle(axis,a.surf_yaw*25.0*DEG));
            self.draw_part(ctx,&p.rudders[i],base,&mat_mul(model,&local));
        }
        let hinge=crate::hornet::brake_hinge(flanker);
        self.draw_part(ctx,&p.brake,base,&mat_mul(model,&rotate_about(hinge,Quat::axis_angle(V3::X,-a.brake*55.0*DEG))));
        self.draw_legs(ctx,a,base,model,&p.gear);
    }

    /// The MiG-29 with its stabilators at their deflection and its gear.
    /// `canopy` draws the canopy opaque, as on the bandits.
    fn draw_mig(&self, ctx: &mut dyn RenderingBackend, a: &Aircraft, base: &AirU, model: &M4, canopy: bool) {
        self.draw_part(ctx, &self.mig, base, model);
        self.draw_controls(ctx,a,base,model,&self.mig_controls);
        if canopy {
            self.draw_part(ctx, &self.mig_canopy, base, model);
        }
        let pitch = clamp(a.surf_pitch, -1.0, 1.0);
        let roll = clamp(a.surf_roll, -1.0, 1.0);
        for (i, side) in [1.0, -1.0].iter().enumerate() {
            let angle = clamp(-pitch * 25.0 * DEG + side * -roll * 5.0 * DEG, -25.0 * DEG, 25.0 * DEG);
            let p = crate::mig29::STAB_PIVOT;
            let local = rotate_about(v3(side * p.x, p.y, p.z), Quat::axis_angle(V3::X, angle));
            if !a.lost.tail {
                self.draw_part(ctx, &self.mig_stab[i], base, &mat_mul(model, &local));
            }
        }
        // The petals swing apart about their leading edges, up and down.
        for (i, upper) in [true, false].into_iter().enumerate() {
            let angle = crate::mig29::brake_angle(upper, a.brake);
            let local = rotate_about(crate::mig29::brake_hinge(upper), Quat::axis_angle(V3::X, angle));
            self.draw_part(ctx, &self.mig_brakes[i], base, &mat_mul(model, &local));
        }
        self.draw_legs(ctx, a, base, model, &self.mig_gear);
    }

    fn draw_controls(&self,ctx:&mut dyn RenderingBackend,a:&Aircraft,base:&AirU,model:&M4,parts:&[MovingPart]) {
        for p in parts {
            let wing=if p.hinge.side<0.0{0}else{1};
            if !matches!(p.hinge.drive,crate::articulation::Drive::Rudder)&&a.lost.wing[wing]{continue;}
            self.draw_part(ctx,&p.part,base,&mat_mul(model,&p.hinge.pose(a)));
        }
    }

    fn draw_gear(&self, ctx: &mut dyn RenderingBackend, a: &Aircraft, base: &AirU, model: &M4) {
        self.draw_legs(ctx, a, base, model, &self.gear);
        self.draw_doors(ctx, a, base, model, &self.gear_doors, &airframe::GEAR_DOORS);
    }

    /// Landing gear legs, swung up toward their wells as the gear retracts,
    /// and gone once it is up and locked. The doors open first, then the
    /// legs travel with an eased start and stop, the nose leg leading.
    fn draw_legs(&self, ctx: &mut dyn RenderingBackend, a: &Aircraft, base: &AirU, model: &M4, legs: &[Part; 3]) {
        if a.gear < 0.01 {
            return;
        }
        for (i, part) in legs.iter().enumerate() {
            if a.gear_broken[i] { continue; }
            let (pivot, rotation) = a.gear_pose(i);
            let local = rotate_about(pivot, rotation);
            self.draw_part(ctx, part, base, &mat_mul(model, &local));
        }
    }

    /// Gear well doors hinged on the airframe: shut and flush when the gear
    /// is up, swung open over the first part of the cycle.
    fn draw_doors(&self, ctx: &mut dyn RenderingBackend, a: &Aircraft, base: &AirU, model: &M4, doors: &[Part], hinges: &[(V3, V3, f64)]) {
        if a.gear < 0.01 {
            return;
        }
        // Doors torn off by the air load are gone.
        if a.gear_damage > 0.3 {
            return;
        }
        for (part, &(hinge, axis, open)) in doors.iter().zip(hinges) {
            let local = rotate_about(hinge, Quat::axis_angle(axis, smoothstep(0.0, 0.2, a.gear) * open));
            self.draw_part(ctx, part, base, &mat_mul(model, &local));
        }
    }

    /// A piece broken off a jet, its centre at `pos` (relative to nothing:
    /// world space) turned by `q`: the wing cut from the whole airframe, or
    /// the stabilators at rest.
    pub fn draw_debris(&self, ctx: &mut dyn RenderingBackend, vp: M4, cam: &Camera, kind: Type, piece: Piece, pos: V3, q: Quat, paint: [f32; 4]) {
        let ax = q.axes();
        let c = piece.centre(kind);
        let model = mat_mul(&basis(ax.right, ax.up, ax.back, pos - cam.eye), &basis(V3::X, V3::Y, V3::Z, -c));
        let (root, tip, aft) = kind.flex_span();
        let mut base = AirU { vp, model, sun: sun_dir().f32(), fog: FOG, alpha: 1.0, glow: 0.0, cutout: 0.0, flex: [0.0, 0.0, root as f32, tip as f32], flex_aft: aft as f32, paint, cut: NO_CUT };
        ctx.apply_pipeline(&self.air);
        match piece {
            Piece::Wing(i) => {
                base.cut = [(i == 0) as i32 as f32, (i == 1) as i32 as f32, 1.0, kind.break_station() as f32];
                self.draw_part(ctx,self.twin(kind).map(|p|&p.body).unwrap_or(if kind==Type::Mig29{&self.mig}else{&self.exterior}),&base,&model);
            }
            Piece::Gear(i) => {
                let legs = self.twin(kind).map(|p| &p.gear).unwrap_or(if kind == Type::Mig29 { &self.mig_gear } else { &self.gear });
                self.draw_part(ctx, &legs[i], &base, &model);
            }
            Piece::Tail => {
                for part in self.twin(kind).map(|p|&p.stabs).unwrap_or(if kind==Type::Mig29{&self.mig_stab}else{&self.stab}) {
                    self.draw_part(ctx, part, &base, &model);
                }
            }
        }
    }

    /// A prop part (store, missile, bomb or ground target) at a model
    /// transform relative to the eye.
    pub fn draw_prop(&self, ctx: &mut dyn RenderingBackend, vp: M4, part: &Part, model: M4, paint: [f32; 4]) {
        let base = AirU { vp, model, sun: sun_dir().f32(), fog: FOG, alpha: 1.0, glow: 0.0, cutout: 0.0, flex: NO_FLEX, flex_aft: 0.0, paint, cut: NO_CUT };
        ctx.apply_pipeline(&self.air);
        self.draw_part(ctx, part, &base, &model);
    }

    /// The painted skin texture, for props that share the aircraft shader.
    pub fn skin(&self) -> TextureId {
        self.exterior.bind.images[0]
    }

    fn update_clouds(&mut self, ctx: &mut dyn RenderingBackend, cam: &Camera, time: f64) {
        // Each cloud wanders a slow loop of its own round its spot, about ten
        // minutes a lap, out of step with its neighbours.
        let at = |i: usize, c: &Cloud| {
            let (phase, w) = (i as f64 * 2.39, TAU / (480.0 + (i % 7) as f64 * 40.0));
            c.pos + v3((time * w + phase).sin(), 0.0, (time * w * 0.73 + phase * 1.7).cos()) * DRIFT
        };
        let mut order: Vec<(f64, usize)> = self.clouds.iter().enumerate().map(|(i, c)| ((at(i, c) - cam.eye).dot(at(i, c) - cam.eye), i)).collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.cloud_data.clear();
        for &(_, i) in &order {
            let c = &self.clouds[i];
            let p = at(i, c);
            for (cx, cy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                self.cloud_data.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32, cx, cy, c.size as f32, c.alpha as f32]);
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

/// Extra opaque geometry drawn in the world pass with the aircraft shader:
/// bandits, stores, missiles, bombs and ground targets.
pub trait Props {
    fn first_person(&self) -> bool { false }
    fn draw(&self, world: &World, ctx: &mut dyn RenderingBackend, vp: M4, cam: &Camera);
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


/// The shader's wing cut for a jet: the wings it has lost and where they
/// broke off.
fn cut_of(a: &Aircraft) -> [f32; 4] {
    [a.lost.wing[0] as i32 as f32, a.lost.wing[1] as i32 as f32, 0.0, a.kind.break_station() as f32]
}
