//! F-16C cockpit: flat-shaded, low-polygon 3D geometry in body metres,
//! live instrument pages rendered into a texture atlas, and the collimated
//! NAV HUD clipped to the physical combiner glass.
use crate::air::{self, AirData};
use crate::airframe::{self, EYE};
use crate::canvas::{rgba, Align, Canvas, Col};
use crate::gfx;
use crate::math::*;
use crate::mesh::{rounded_rect, Frame, Mesh, Rgb};
use crate::combat::{Combat, Mode};
use crate::sim::{Aircraft, Mission};
use crate::symbology;
use crate::world::{self, Camera, Part, MESH_ATTRS};
use miniquad::*;

mod consoles;
mod hud;
mod mig;
mod pilot;

/// Atlas height; the instrument pages fill the left 1024 square and the
/// console lightplates the right half.
pub const ATLAS: u32 = 2048;
pub const ATLAS_W: u32 = 2048;
const A: f64 = ATLAS as f64;
const AW: f64 = ATLAS_W as f64;

#[derive(Clone, Copy)]
struct R {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
const fn r(x: f32, y: f32, w: f32, h: f32) -> R {
    R { x, y, w, h }
}
fn uv(r: R) -> (f64, f64, f64, f64) {
    (r.x as f64 / AW, 1.0 - r.y as f64 / A, (r.x + r.w) as f64 / AW, 1.0 - (r.y + r.h) as f64 / A)
}

const R_MFD_L: R = r(0.0, 0.0, 256.0, 256.0);
const R_MFD_R: R = r(256.0, 0.0, 256.0, 256.0);
const R_HSI: R = r(512.0, 0.0, 240.0, 240.0);
const R_DED: R = r(752.0, 0.0, 272.0, 64.0);
const R_ICP: R = r(752.0, 64.0, 272.0, 176.0);
const R_RWR: R = r(752.0, 448.0, 128.0, 128.0);
const R_MC: R = r(880.0, 240.0, 64.0, 64.0);
const R_DIALS: [R; 7] = [
    r(0.0, 320.0, 128.0, 128.0),
    r(128.0, 320.0, 128.0, 128.0),
    r(256.0, 320.0, 128.0, 128.0),
    r(384.0, 320.0, 128.0, 128.0),
    r(512.0, 320.0, 128.0, 128.0),
    r(640.0, 320.0, 128.0, 128.0),
    r(768.0, 320.0, 128.0, 128.0),
];
/// Angle-of-attack indicator, the tape left of the ADI.
const R_AOAI: R = r(896.0, 320.0, 128.0, 128.0);
/// The rear-view picture (see `mirror_camera`), for the cockpits with mirrors
/// (the MiG-29; the F-16 has none).
const R_MIRROR: R = r(1024.0, 1280.0, 512.0, 160.0);
/// Vertical field of view of the rear-view camera; its picture is 512 by 160.
pub const MIRROR_SIZE: (u32, u32) = (512, 160);
const MIRROR_FOV: f64 = 40.0 * DEG;
/// Warning lights on the right eyebrow.
const R_CAUTION: R = r(0.0, 448.0, 256.0, 128.0);
/// Eyebrow panels' distance out from the centreline, m.
const EYEBROW: f64 = 0.30;
const R_ARM: R = r(256.0, 448.0, 96.0, 256.0);
const R_AOA: R = r(512.0, 448.0, 48.0, 128.0);
const R_KNEE: R = r(560.0, 448.0, 192.0, 240.0);
/// Dials on the right auxiliary console: hydraulic pressure A and B, EPU
/// fuel, fuel quantity, cabin altitude and the clock.
const R_AUX_DIALS: [R; 6] = [
    r(0.0, 1024.0, 128.0, 128.0),
    r(128.0, 1024.0, 128.0, 128.0),
    r(256.0, 1024.0, 128.0, 128.0),
    r(384.0, 1024.0, 128.0, 128.0),
    r(512.0, 1024.0, 128.0, 128.0),
    r(640.0, 1024.0, 128.0, 128.0),
];
/// Accelerometer, low on the left wing of the panel.
const R_ACCEL: R = r(768.0, 1024.0, 128.0, 128.0);
const R_VVI: R = r(896.0, 1024.0, 128.0, 128.0);
const R_TWP: R = r(0.0, 1152.0, 256.0, 96.0);
const R_FFLOW: R = r(256.0, 1152.0, 192.0, 64.0);

const GREEN: Col = rgba(0x98, 0xff, 0xa3, 1.0);
const WHITE: Col = rgba(0xdd, 0xdc, 0xcf, 1.0);
const DIM: Col = rgba(0xa2, 0xb8, 0xb6, 1.0);
const AMBER: Col = rgba(0xe6, 0xbc, 0x7a, 1.0);
const RED: Col = rgba(0xff, 0x66, 0x54, 1.0);
pub const HUD: Col = rgba(0xa9, 0xff, 0xb2, 1.0);
/// The ILS-31's single colour, a yellower green than the F-16's.
const ILS_GREEN: Col = rgba(0xb4, 0xff, 0x6a, 1.0);

// Materials.
const PANEL: Rgb = Rgb(0.165, 0.185, 0.195);
const BEZEL: Rgb = Rgb(0.215, 0.235, 0.245);
const GLARE: Rgb = Rgb(0.075, 0.085, 0.095);
const WALL: Rgb = Rgb(0.19, 0.21, 0.215);
const SEAT: Rgb = Rgb(0.17, 0.20, 0.17);
const METALC: Rgb = Rgb(0.55, 0.57, 0.56);

/// Round gauges are 16-sided, chunky like the rest of the low-poly art.
const GAUGE_SIDES: usize = 16;

// Main instrument panel: tilted 8.5 degrees, its base 0.4 m above the datum.
const TILT: f64 = 8.5 * DEG;
fn panel() -> Frame {
    Frame { o: v3(0.0, 0.40, -4.12), x: V3::X, y: v3(0.0, TILT.cos(), -TILT.sin()), z: v3(0.0, TILT.sin(), TILT.cos()) }
}
fn pv(y: f64) -> f64 {
    (y - 0.40) / TILT.cos()
}
/// The outboard parts of the panel fold toward the pilot about vertical
/// hinges just outside the MFDs, so the gauge columns on them face the seat.
const HINGE: f64 = 0.322;
const CANT: f64 = 30.0 * DEG;

/// Axes of the folded wing on side `s`: across it (to the right) and its normal.
fn wing_axes(s: f64) -> (V3, V3) {
    let p = panel();
    let x = p.x * CANT.cos() + p.z * (s * CANT.sin());
    (x, x.cross(p.y))
}

/// Point on the panel at face coordinates (`u` across, `v` up the tilted
/// face, `w` out of it), folding with the wings.
fn pp(u: f64, v: f64, w: f64) -> V3 {
    let p = panel();
    if u.abs() <= HINGE {
        return p.p(u, v, w);
    }
    let s = u.signum();
    let (x, n) = wing_axes(s);
    p.p(s * HINGE, v, 0.0) + x * (u - s * HINGE) + n * w
}

fn panel_at(u: f64, y: f64, w: f64) -> Frame {
    if u.abs() <= HINGE {
        return panel().at(u, pv(y), w);
    }
    let (x, n) = wing_axes(u.signum());
    Frame { o: pp(u, pv(y), w), x, y: panel().y, z: n }
}

/// Auxiliary consoles: under the panel wings, ramps that carry each side
/// console on forward and up at 35 degrees, keeping its 10 degree lean and
/// toed 7.3 degrees toward the pilot (the FlightGear cockpit's toe), from
/// just under the console's front edge, outboard of the knees, to the wall.
/// Laid back that far its top edge nearly meets the panel, as on the real
/// jet; a short shelf closes what is left. Returns the
/// face's bottom-left corner, its axes across (to the right), up and out,
/// and its width and height in metres.
fn aux_face(s: f64) -> (V3, V3, V3, V3, f64, f64) {
    const PITCH: f64 = 35.0 * DEG;
    const TOE: f64 = 7.3 * DEG;
    // The inboard top edge stays under the sight line to the MFD bottoms.
    const TOP: f64 = 0.68;
    let t = -s * TOE;
    let yaw = |p: V3| v3(p.x * t.cos() + p.z * t.sin(), p.y, -p.x * t.sin() + p.z * t.cos());
    let out = yaw(v3(s * consoles::ROLL.cos(), consoles::ROLL.sin(), 0.0));
    let up = yaw(v3(0.0, PITCH.sin(), -PITCH.cos()));
    let up = (up - out * out.dot(up)).norm();
    // Inboard bottom corner set forward so the outboard one meets the console's front edge.
    let w = (consoles::WALL_X - HINGE) / out.x.abs();
    let inboard = v3(s * HINGE, consoles::top_y(s * HINGE, consoles::FRONT_Z) - 0.03, consoles::FRONT_Z - out.z * w);
    let h = (TOP - inboard.y) / up.y;
    let across = out * s;
    let corner = if s > 0.0 { inboard } else { inboard + out * w };
    (corner, across, up, across.cross(up), w, h)
}

/// Point on the panel face (folding with the wings) at body `x` and height `y`.
fn panel_point(x: f64, y: f64) -> V3 {
    let ax = x.abs();
    let u = if ax <= HINGE { ax } else { HINGE + (ax - HINGE) / CANT.cos() };
    let p = pp(x.signum() * u, pv(y), 0.0);
    p + panel().y * ((y - p.y) / TILT.cos())
}
/// Interior half-width of the sloping tub wall at a body-space point.
fn tub_half_width(p: V3) -> f64 {
    let sill = airframe::canopy_profile(p.z).1 - 0.03;
    mix(consoles::WALL_X, 0.47, clamp((p.y - consoles::WALL_Y) / (sill - consoles::WALL_Y), 0.0, 1.0))
}

/// A panel-plane point just inside the side wall, found against the wall
/// at this point's own z instead of a width from an unrelated height.
fn panel_wall_point(side: f64, y: f64) -> V3 {
    let (mut lo, mut hi) = (0.44, consoles::WALL_X);
    for _ in 0..32 {
        let x = (lo + hi) * 0.5;
        let p = panel_point(side * x, y);
        if x <= tub_half_width(p) - 0.003 { lo = x; } else { hi = x; }
    }
    panel_point(side * lo, y)
}

fn panel_shoulder(side: f64, bottom: f64) -> [V3; 4] {
    // OUTLINE's outer upper corner is y = 0.76, not canopy-rail height.
    let top = 0.76;
    [
        panel_point(side * 0.45, bottom),
        panel_wall_point(side, bottom),
        panel_wall_point(side, top),
        panel_point(side * 0.45, top),
    ]
}

/// Panel outline in face coordinates, with points at the wing hinges.
const OUTLINE: [(f64, f64); 16] = [
    (-0.47, 0.0), (-HINGE, 0.0), (HINGE, 0.0), (0.47, 0.0), (0.47, 0.36), (0.45, 0.44), (0.40, 0.52), (0.34, 0.57),
    (HINGE, 0.579), (0.28, 0.60), (-0.28, 0.60), (-HINGE, 0.579), (-0.34, 0.57), (-0.40, 0.52), (-0.45, 0.44), (-0.47, 0.36),
];

/// Leg well through the centre panel: the half-widths (m) of the HSI, of
/// the ADI with the small gauges beside it, and of the well, then the
/// heights up the panel face of the HSI's foot (open right across below),
/// the gauges' foot and the MFDs' foot, where the well stops.
const WELL_X: [f64; 3] = [0.066, 0.135, 0.305];
const WELL_V: [f64; 3] = [0.054, 0.200, 0.290];
/// How far the leg well reaches forward of the panel face, m.
const WELL_DEPTH: f64 = 0.32;

/// Combiner glass corners in body coordinates: bottom-left, bottom-right, top-right, top-left.
pub const HUD_GLASS: [V3; 4] = [v3(-0.10, 1.078, -4.012), v3(0.10, 1.078, -4.012), v3(0.10, 1.35, -3.958), v3(-0.10, 1.35, -3.958)];

#[derive(Clone, Copy, Default)]
pub struct Hands {
    /// Hands reach for the seat handle, then lift it, 0 to 1.
    pub eject: f64,
    pub roll: f64,
    pub pitch: f64,
    pub throttle: f64,
    pub rudder: f64,
    /// Landing gear handle, 0 up to 1 down, and its warning light.
    pub gear: f64,
    pub gear_warn: bool,
}

#[repr(C)]
struct CockpitU {
    vp: M4,
    eye: [f32; 3],
    sun: [f32; 3],
    sun_vis: f32,
    alpha: f32,
    tint: [f32; 3],
}

const VS: &str = r#"
in vec3 pos;
in vec3 normal;
in vec3 color;
in vec2 uv;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform float u_sun_vis;
uniform float u_alpha;
uniform vec3 u_tint;
out vec3 v_n;
out vec3 v_c;
out vec2 v_uv;
out vec3 v_rel;
void main() {
    v_rel = pos - u_eye;
    v_n = normal;
    v_c = color;
    v_uv = uv;
    gl_Position = u_vp * vec4(v_rel, 1.0);
}"#;

const FS: &str = r#"
in vec3 v_n;
in vec3 v_c;
in vec2 v_uv;
in vec3 v_rel;
uniform sampler2D u_atlas;
uniform vec3 u_eye;
uniform vec3 u_sun;
uniform float u_sun_vis;
uniform float u_alpha;
uniform vec3 u_tint;
out vec4 frag;
void main() {
    vec3 n = normalize(v_n);
    vec3 view = normalize(-v_rel);
    if (dot(n, view) < 0.0) n = -n;
    float diffuse = max(dot(n, u_sun), 0.0) * u_sun_vis;
    // Fixed key light from above the left shoulder: flat facets keep distinct
    // shades even when the sun is behind the jet.
    float key = max(dot(n, vec3(-0.30, 0.84, 0.45)), 0.0);
    float sky = 0.22 + 0.34 * key + 0.06 * n.y;
    if (u_alpha < 0.99) {
        // Glass: faint tint, Fresnel sheen and a sun glint, never a solid wash.
        float fres = pow(1.0 - max(dot(n, view), 0.0), 5.0);
        float glint = pow(max(dot(reflect(-u_sun, n), view), 0.0), 80.0) * u_sun_vis;
        vec3 col = u_tint + vec3(0.55, 0.62, 0.60) * fres + glint;
        frag = vec4(col, clamp(u_alpha + fres * 0.22 + glint * 0.5, 0.0, 0.6));
        return;
    }
    if (v_uv.x >= 0.0) {
        vec3 tex = texture(u_atlas, v_uv).rgb;
        // Displays are self-lit; strong sun washes them out a little.
        frag = vec4(tex * 1.08 + v_c * 0.0 + vec3(0.06, 0.07, 0.07) * diffuse * 0.35, 1.0);
        return;
    }
    vec3 col = v_c * (sky + diffuse * 0.78);
    float spec = pow(max(dot(reflect(-u_sun, n), view), 0.0), 18.0) * u_sun_vis * 0.06;
    frag = vec4(col + spec, 1.0);
}"#;

#[repr(C)]
struct FaceU {
    vp: M4,
    eye: [f32; 3],
    res: [f32; 2],
    gloc: f32,
}

/// Instrument faces redrawn at window resolution over the pixelated scene.
const FACE_VS: &str = r#"
in vec3 pos;
in vec3 normal;
in vec3 color;
in vec2 uv;
uniform mat4 u_vp;
uniform vec3 u_eye;
out vec2 v_uv;
void main() {
    v_uv = uv;
    // A hair toward the eye, so each face wins the depth test against its own
    // triangle laid down by the depth-only pass.
    gl_Position = u_vp * vec4((pos - u_eye) * 0.998, 1.0);
}"#;

const FACE_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_atlas;
uniform vec2 u_res;
uniform float u_gloc;
out vec4 frag;
void main() {
    vec3 col = texture(u_atlas, v_uv).rgb * 1.08;
    // The post pass grade, without its dither, so colours match the panel.
    col = mix(vec3(dot(col, vec3(0.299, 0.587, 0.114))), col, 1.12);
    col = clamp((col - 0.5) * 1.04 + 0.5, 0.0, 1.0);
    // The same grey-out the post pass lays over the rest of the view.
    float l = dot(col, vec3(0.299, 0.587, 0.114));
    frag = vec4(mix(col, vec3(l) * 0.72, u_gloc), 1.0);
}"#;

/// Room for the textured faces of the moving geometry, in vertices.
const DYN_FACE_CAP: usize = 600;

/// The textured triangles of `m`: displays, gauge dials, placards.
fn faces(m: &Mesh) -> Mesh {
    let mut out = Mesh::new();
    for tri in m.v.chunks(3 * crate::mesh::STRIDE) {
        if tri[9] >= 0.0 {
            out.v.extend_from_slice(tri);
        }
    }
    out
}

pub struct Cockpit {
    pip: Pipeline,
    glass_pip: Pipeline,
    depth_pip: Pipeline,
    face_pip: Pipeline,
    statics: Part,
    faces: Part,
    dynamic: Mesh,
    dyn_bind: Bindings,
    dyn_cap: usize,
    dyn_count: usize,
    /// Textured faces of the moving geometry (the kneeboard), for draw_faces.
    dyn_face_bind: Bindings,
    dyn_face_count: usize,
    canopy: Part,
    /// The MiG-29's cockpit: its own tub, panel and canopy, in the same space.
    mig: [Part; 3],
    hud_glass: Part,
    pub atlas: gfx::Target,
    atlas_pass: RenderPass,
    pub atlas_clock: f64,
}

impl Cockpit {
    pub fn new(ctx: &mut dyn RenderingBackend, canopy: &Mesh, mig_canopy: &Mesh) -> Cockpit {
        use UniformType::*;
        let s = gfx::shader(ctx, VS, FS, &["u_atlas"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_sun_vis", Float1), ("u_alpha", Float1), ("u_tint", Float3)]);
        let pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() });
        let glass_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::alpha_blend()), ..Default::default() });
        let depth_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_write: (false, false, false, false), ..Default::default() });
        let fs = gfx::shader(ctx, FACE_VS, FACE_FS, &["u_atlas"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_res", Float2), ("u_gloc", Float1)]);
        let face_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, fs, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() });
        let atlas_tex = ctx.new_render_texture(TextureParams {
            width: ATLAS_W,
            height: ATLAS,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            allocate_mipmaps: true,
            ..Default::default()
        });
        let atlas_pass = ctx.new_render_pass(atlas_tex, None);
        let atlas = gfx::Target { pass: atlas_pass, color: atlas_tex, w: ATLAS_W, h: ATLAS };

        let mut m = Mesh::new();
        build_static(&mut m);
        let mut statics = world::part(ctx, &m);
        statics.bind.images = vec![atlas_tex];
        let mut faces = world::part(ctx, &faces(&m));
        faces.bind.images = vec![atlas_tex];
        let dyn_cap = 60000;
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(dyn_cap * crate::mesh::STRIDE));
        let dyn_bind = Bindings { vertex_buffers: vec![vb], index_buffer: gfx::sequential_ib(ctx, dyn_cap), images: vec![atlas_tex] };
        let fvb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(DYN_FACE_CAP * crate::mesh::STRIDE));
        let dyn_face_bind = Bindings { vertex_buffers: vec![fvb], index_buffer: gfx::sequential_ib(ctx, DYN_FACE_CAP), images: vec![atlas_tex] };
        let mut canopy = world::part(ctx, canopy);
        canopy.bind.images = vec![atlas_tex];
        let mut mm = Mesh::new();
        mig::build_static(&mut mm);
        let mut shifted = Mesh { v: mig_canopy.v.clone() };
        for v in shifted.v.chunks_mut(crate::mesh::STRIDE) {
            v[0] += mig::OFFSET.x as f32;
            v[1] += mig::OFFSET.y as f32;
            v[2] += mig::OFFSET.z as f32;
        }
        let mig = [world::part(ctx, &mm), world::part(ctx, &self::faces(&mm)), world::part(ctx, &shifted)].map(|mut p| {
            p.bind.images = vec![atlas_tex];
            p
        });
        let mut glass = Mesh::new();
        glass.quad(HUD_GLASS[0], HUD_GLASS[1], HUD_GLASS[2], HUD_GLASS[3], Rgb(0.4, 0.6, 0.5));
        let mut hud_glass = world::part(ctx, &glass);
        hud_glass.bind.images = vec![atlas_tex];
        Cockpit { pip, glass_pip, depth_pip, face_pip, statics, faces, dynamic: Mesh::new(), dyn_bind, dyn_cap, dyn_count: 0, dyn_face_bind, dyn_face_count: 0, canopy, mig, hud_glass, atlas, atlas_pass, atlas_clock: -1.0 }
    }

    /// Redraws every instrument page into the atlas (call outside any pass).
    pub fn update_atlas(&mut self, ctx: &mut dyn RenderingBackend, cv: &mut Canvas, a: &Aircraft, m: &Mission, pages: [Page; 2], time: f64, c: &Combat) {
        ctx.begin_pass(Some(self.atlas_pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        draw_instruments(cv, a, m, pages, time, c);
        cv.flush(ctx, AW as f32, A as f32);
        ctx.end_render_pass();
        ctx.texture_generate_mipmaps(self.atlas.color);
    }

    /// Copies the rear-view picture into the atlas, where the mirror glass
    /// shows it, after `update_atlas` has redrawn the instruments.
    pub fn blit_mirror(&self, ctx: &mut dyn RenderingBackend, world: &world::World, mirror: &gfx::Target) {
        ctx.begin_pass(Some(self.atlas_pass), PassAction::Nothing);
        let r = R_MIRROR;
        ctx.apply_viewport(r.x as i32, (A - (r.y + r.h) as f64) as i32, r.w as i32, r.h as i32);
        world.post(ctx, mirror, r.w, r.h, 0.0, 0.0);
        ctx.end_render_pass();
        ctx.texture_generate_mipmaps(self.atlas.color);
    }

    /// Draws the cockpit into the current pass, the MiG's when `mig`.
    /// `cam_b` is the camera in cockpit space (see body_camera).
    pub fn draw(&mut self, ctx: &mut dyn RenderingBackend, cam_b: &Camera, aspect: f64, sun_b: V3, hands: &Hands, mig: bool, canopy_on: bool) {
        let (statics, canopy) = if mig { (&self.mig[0], &self.mig[2]) } else { (&self.statics, &self.canopy) };
        let proj = perspective(cam_b.fov, aspect, 0.02, 40.0);
        let vp = mat_mul(&proj, &cam_b.view());
        // The fuselage shades the tub when the sun is below the canopy rails.
        let sun_vis = smoothstep(-0.22, 0.08, sun_b.y) as f32;
        let u = CockpitU { vp, eye: cam_b.eye.f32(), sun: sun_b.norm().f32(), sun_vis, alpha: 1.0, tint: [0.0; 3] };
        ctx.apply_pipeline(&self.pip);
        ctx.apply_bindings(&statics.bind);
        ctx.apply_uniforms(UniformsSource::table(&u));
        ctx.draw(0, statics.count, 1);

        self.dynamic.clear();
        if mig {
            pilot::build_dynamic_mig(&mut self.dynamic, hands);
        } else {
            pilot::build_dynamic(&mut self.dynamic, hands);
        }
        let n = self.dynamic.count().min(self.dyn_cap);
        ctx.buffer_update(self.dyn_bind.vertex_buffers[0], BufferSource::slice(&self.dynamic.v[..n * crate::mesh::STRIDE]));
        ctx.apply_bindings(&self.dyn_bind);
        ctx.draw(0, n as i32, 1);
        self.dyn_count = n;
        let f = faces(&self.dynamic);
        self.dyn_face_count = f.count().min(DYN_FACE_CAP);
        if self.dyn_face_count > 0 {
            ctx.buffer_update(self.dyn_face_bind.vertex_buffers[0], BufferSource::slice(&f.v[..self.dyn_face_count * crate::mesh::STRIDE]));
        }

        ctx.apply_pipeline(&self.glass_pip);
        ctx.apply_bindings(&self.hud_glass.bind);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { alpha: 0.05, tint: [0.10, 0.16, 0.13], ..u }));
        ctx.draw(0, self.hud_glass.count, 1);
        if !canopy_on { return; }
        ctx.apply_bindings(&canopy.bind);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { alpha: 0.035, tint: if mig { [0.08, 0.12, 0.12] } else { [0.16, 0.13, 0.05] }, ..u }));
        ctx.draw(0, canopy.count, 1);
    }

    /// Redraws the displays, dials and placards at full resolution into the
    /// current pass, after the low-resolution scene has been composited, so
    /// their text stays sharp. The cockpit from the last `draw` first lays
    /// down depth only, so hands, arms and knees still cover them.
    /// `res` is the pass size in pixels; `gloc` matches the post pass.
    pub fn draw_faces(&self, ctx: &mut dyn RenderingBackend, cam_b: &Camera, aspect: f64, res: [f32; 2], gloc: f32, mig: bool) {
        let (statics, faces) = if mig { (&self.mig[0], &self.mig[1]) } else { (&self.statics, &self.faces) };
        let proj = perspective(cam_b.fov, aspect, 0.02, 40.0);
        let vp = mat_mul(&proj, &cam_b.view());
        let eye = cam_b.eye.f32();
        ctx.apply_pipeline(&self.depth_pip);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { vp, eye, sun: [0.0, 1.0, 0.0], sun_vis: 0.0, alpha: 1.0, tint: [0.0; 3] }));
        ctx.apply_bindings(&statics.bind);
        ctx.draw(0, statics.count, 1);
        ctx.apply_bindings(&self.dyn_bind);
        ctx.draw(0, self.dyn_count as i32, 1);
        ctx.apply_pipeline(&self.face_pip);
        ctx.apply_bindings(&faces.bind);
        ctx.apply_uniforms(UniformsSource::table(&FaceU { vp, eye, res, gloc }));
        ctx.draw(0, faces.count, 1);
        ctx.apply_bindings(&self.dyn_face_bind);
        ctx.draw(0, self.dyn_face_count as i32, 1);
    }
}

// ================================================================ geometry
fn screen(m: &mut Mesh, f: &Frame, w: f64, h: f64, rect: R) {
    let (u0, v0, u1, v1) = uv(rect);
    m.quad_uv(f.p(-w / 2.0, h / 2.0, 0.0), f.p(w / 2.0, h / 2.0, 0.0), f.p(w / 2.0, -h / 2.0, 0.0), f.p(-w / 2.0, -h / 2.0, 0.0), [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]);
}

/// Rounded bezel around a recessed screen, with optional option-select buttons.
fn display(m: &mut Mesh, f: &Frame, bw: f64, bh: f64, sw: f64, sh: f64, rect: R, osb: bool) {
    let front = f.at(0.0, 0.0, 0.016);
    m.bezel(&front, &rounded_rect(bw, bh, 0.016, 1), &rounded_rect(sw + 0.006, sh + 0.006, 0.005, 1), 0.016, 0.010, BEZEL);
    screen(m, &f.at(0.0, 0.0, 0.006), sw, sh, rect);
    if osb {
        for i in 0..5 {
            let t = (i as f64 - 2.0) / 4.0;
            for &s in &[-1.0, 1.0] {
                let b = front.at(t * sw * 0.82, s * (sh / 2.0 + (bh - sh) * 0.25), 0.003);
                m.block(&b, v3(0.0085, 0.0055, 0.004), Rgb(0.30, 0.32, 0.32));
                let b = front.at(s * (sw / 2.0 + (bw - sw) * 0.25), t * sh * 0.74, 0.003);
                m.block(&b, v3(0.0055, 0.0085, 0.004), Rgb(0.30, 0.32, 0.32));
            }
        }
    }
}

fn gauge(m: &mut Mesh, f: &Frame, radius: f64, rect: R) {
    m.ring(&f.at(0.0, 0.0, 0.014), radius, radius + 0.008, 0.014, BEZEL.scale(1.1), GAUGE_SIDES);
    m.disc_uv(&f.at(0.0, 0.0, 0.004), radius, uv(rect), GAUGE_SIDES);
    // Square mounting flange with four screws, as on the real gauges.
    let flange = f.at(0.0, 0.0, 0.002);
    m.bezel(&flange, &rounded_rect(radius * 2.0 + 0.026, radius * 2.0 + 0.026, 0.006, 1), &rounded_rect(radius * 2.0 + 0.012, radius * 2.0 + 0.012, 0.004, 1), 0.002, 0.0, BEZEL.scale(0.8));
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let c = radius + 0.007;
        m.block(&f.at(sx * c, sy * c, 0.004), v3(0.0025, 0.0025, 0.002), METALC.scale(0.6));
    }
}

fn plate(m: &mut Mesh, f: &Frame, w: f64, h: f64, rect: R) {
    m.block(&f.at(0.0, 0.0, 0.004), v3(w / 2.0 + 0.006, h / 2.0 + 0.006, 0.004), BEZEL.scale(0.85));
    screen(m, &f.at(0.0, 0.0, 0.0085), w, h, rect);
}

fn build_static(m: &mut Mesh) {
    // Panel face in three parts, the centre and the two folded wings, and
    // its shoulder rim.
    let face = |pts: &[(f64, f64)]| -> Vec<V3> { pts.iter().map(|&(u, v)| pp(u, v, 0.0)).collect() };
    // As in the real cockpit the centre part is cut away under the MFDs for
    // the knees: the leg well runs from the floor up to the MFDs' foot either
    // side of the ADI and HSI stack, and right across under the HSI, boxed in
    // behind, so the shins pass through the panel to the pedals.
    let xs = [-HINGE, -WELL_X[2], -WELL_X[1], -WELL_X[0], WELL_X[0], WELL_X[1], WELL_X[2], HINGE];
    let vs = [0.0, WELL_V[0], WELL_V[1], WELL_V[2]];
    let open = |i: usize, j: usize| -> bool {
        let (c, row) = ((xs[i] + xs[i + 1]) * 0.5, j);
        let ac = c.abs();
        ac < WELL_X[2] && (row == 0 || ac > WELL_X[0] && (row == 1 || ac > WELL_X[1]))
    };
    let d = -WELL_DEPTH;
    for i in 0..xs.len() - 1 {
        for j in 0..vs.len() - 1 {
            let (u0, u1, v0, v1) = (xs[i], xs[i + 1], vs[j], vs[j + 1]);
            if !open(i, j) {
                m.polygon(&face(&[(u0, v0), (u1, v0), (u1, v1), (u0, v1)]), PANEL);
                if j == 0 {
                    m.quad(pp(u0, 0.0, 0.0), pp(u1, 0.0, 0.0), pp(u1, 0.0, -0.20), pp(u0, 0.0, -0.20), PANEL.scale(0.7));
                }
                continue;
            }
            // Back wall, then a side or roof against each closed neighbour.
            m.polygon(&[pp(u0, v0, d), pp(u1, v0, d), pp(u1, v1, d), pp(u0, v1, d)], PANEL.scale(0.3));
            let closed = |di: i32, dj: i32| {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                nj >= vs.len() as i32 - 1 || ni < 0 || ni >= xs.len() as i32 - 1 || nj >= 0 && !open(ni as usize, nj as usize)
            };
            if closed(-1, 0) {
                m.quad(pp(u0, v1, 0.0), pp(u0, v0, 0.0), pp(u0, v0, d), pp(u0, v1, d), PANEL.scale(0.4));
            }
            if closed(1, 0) {
                m.quad(pp(u1, v0, 0.0), pp(u1, v1, 0.0), pp(u1, v1, d), pp(u1, v0, d), PANEL.scale(0.4));
            }
            if closed(0, 1) {
                m.quad(pp(u0, v1, 0.0), pp(u1, v1, 0.0), pp(u1, v1, d), pp(u0, v1, d), PANEL.scale(0.35));
            }
        }
    }
    m.polygon(&face(&[(-HINGE, vs[3]), (HINGE, vs[3]), OUTLINE[8], OUTLINE[9], OUTLINE[10], OUTLINE[11]]), PANEL);
    m.polygon(&face(&OUTLINE[2..9]), PANEL.scale(0.96));
    m.polygon(&face(&[OUTLINE[0], OUTLINE[1], OUTLINE[11], OUTLINE[12], OUTLINE[13], OUTLINE[14], OUTLINE[15]]), PANEL.scale(0.96));
    for k in (0..OUTLINE.len()).filter(|&k| k != 1) {
        let (a, b) = (OUTLINE[k], OUTLINE[(k + 1) % OUTLINE.len()]);
        m.quad(pp(a.0, a.1, 0.0), pp(b.0, b.1, 0.0), pp(b.0, b.1, -0.20), pp(a.0, a.1, -0.20), PANEL.scale(0.7));
    }
    // Glare shield: a hood from the panel top edge up toward the windscreen.
    let top = &OUTLINE[4..];
    let shield = |u: f64, v: f64, t: f64| pp(u * (1.0 - 0.06 * t), v + 0.012 + 0.05 * t, 0.045 - 0.42 * t);
    for k in 0..top.len() - 1 {
        let (a, b) = (top[k], top[k + 1]);
        for s in 0..4 {
            let (t0, t1) = (s as f64 / 4.0, (s + 1) as f64 / 4.0);
            m.quad(shield(a.0, a.1, t0), shield(b.0, b.1, t0), shield(b.0, b.1, t1), shield(a.0, a.1, t1), GLARE);
        }
        m.capsule(shield(a.0, a.1, 0.0), shield(b.0, b.1, 0.0), 0.011, 0.011, GLARE.scale(1.6), 6);
        m.quad(pp(a.0, a.1, 0.0), pp(b.0, b.1, 0.0), shield(b.0, b.1, 0.0), shield(a.0, a.1, 0.0), GLARE.scale(1.2));
    }

    // First visual pass from the user's F-16C/D reference: square MFDs
    // flank an analogue centre stack; the ICP is directly below the HUD.
    display(m, &panel_at(-0.232, 0.795, 0.0), 0.190, 0.190, 0.137, 0.137, R_MFD_L, true);
    display(m, &panel_at(0.232, 0.795, 0.0), 0.190, 0.190, 0.137, 0.137, R_MFD_R, true);
    let ufc = panel_at(0.0, 0.945, 0.0);
    m.block(&ufc.at(0.0, 0.0, 0.060), v3(0.108, 0.083, 0.060), BEZEL.scale(0.95));
    screen(m, &ufc.at(0.0, 0.0, 0.1205), 0.194, 0.146, R_ICP);
    // The DED is beside the ICP, not stacked above it.
    plate(m, &panel_at(0.24, 0.954, 0.0), 0.18, 0.054, R_DED);

    // HUD: body on the glare shield, side posts and the tall combiner frame.
    // The body sits forward, inside the glare shield; only the glass rises above it.
    let hud_body = Frame { o: v3(0.0, 1.054, -4.15), x: V3::X, y: V3::Y, z: V3::Z };
    m.block(&hud_body, v3(0.10, 0.018, 0.12), GLARE.scale(1.5));
    m.block(&Frame { o: v3(0.0, 1.074, -4.04), ..hud_body }, v3(0.095, 0.004, 0.035), GLARE.scale(2.2));
    for &s in &[-1.0, 1.0] {
        let (b, t) = (v3(s * 0.104, 1.07, -4.02), v3(s * 0.104, 1.352, -3.958));
        m.capsule(b, t, 0.0045, 0.004, GLARE.scale(2.0), 6);
    }
    m.capsule(v3(-0.104, 1.352, -3.958), v3(0.104, 1.352, -3.958), 0.0025, 0.0025, GLARE.scale(2.0), 6);

    // Glare-shield lamps: the AOA indexer left of the HUD; on the eyebrows
    // out over the panel's top corners, the master caution on the left and
    // the warning lights on the right.
    let lamp = |u: f64| Frame { o: v3(u, 1.045, -4.035), x: V3::X, y: v3(0.0, 0.94, -0.34), z: v3(0.0, 0.34, 0.94) };
    m.block(&lamp(-0.14).at(0.0, 0.0, -0.005), v3(0.017, 0.045, 0.01), GLARE.scale(1.4));
    screen(m, &lamp(-0.14).at(0.0, 0.0, 0.006), 0.02, 0.07, R_AOA);
    m.block(&lamp(-EYEBROW).at(0.0, -0.01, -0.005), v3(0.026, 0.034, 0.01), GLARE.scale(1.4));
    screen(m, &lamp(-EYEBROW).at(0.0, 0.0, 0.006), 0.036, 0.036, R_MC);
    m.block(&lamp(EYEBROW).at(0.0, -0.01, -0.005), v3(0.064, 0.034, 0.01), GLARE.scale(1.4));
    screen(m, &lamp(EYEBROW).at(0.0, 0.0, 0.006), 0.118, 0.052, R_CAUTION);

    // Airspeed and altimeter above the larger attitude/heading instruments.
    gauge(m, &panel_at(-0.061, 0.785, 0.0), 0.043, R_DIALS[0]);
    gauge(m, &panel_at(0.061, 0.785, 0.0), 0.043, R_DIALS[1]);
    gauge(m, &panel_at(0.0, 0.655, 0.0), 0.048, R_DIALS[2]);
    gauge(m, &panel_at(0.0, 0.515, 0.0), 0.048, R_HSI);
    // The AOA tape left of the ADI and the VVI right of it, as in the jet.
    gauge(m, &panel_at(-0.092, 0.635, 0.0), 0.022, R_AOAI);
    gauge(m, &panel_at(0.091, 0.635, 0.0), 0.022, R_VVI);
    // RWR above the left MFD; engine gauges follow the right shoulder.
    display(m, &panel_at(-0.245, 0.951, 0.0), 0.085, 0.080, 0.063, 0.061, R_RWR, false);
    for (i, y) in [0.84, 0.77, 0.70, 0.63].iter().enumerate() {
        gauge(m, &panel_at(0.392, *y, 0.0), 0.026, R_DIALS[3 + i]);
    }
    plate(m, &panel_at(-0.376, 0.834, 0.0), 0.046, 0.14, R_ARM);
    // Accelerometer low on the left wing, clear of the leg well; the fuel
    // quantity is on the right auxiliary console.
    gauge(m, &panel_at(-0.385, 0.714, 0.0), 0.032, R_ACCEL);
    plate(m, &panel_at(-0.39, 0.91, 0.0), 0.062, 0.030, R_TWP);
    plate(m, &panel_at(0.356, 0.90, 0.0), 0.065, 0.024, R_FFLOW);
    // Small switch rows low on the panel, mostly behind the knees.
    for &s in &[-1.0, 1.0] {
        for i in 0..3 {
            let f = panel_at(s * (0.13 + i as f64 * 0.045), 0.47, 0.0);
            m.block(&f.at(0.0, 0.0, 0.006), v3(0.016, 0.022, 0.006), BEZEL);
            m.capsule(f.p(0.0, 0.0, 0.01), f.p(0.0, 0.012, 0.03), 0.003, 0.0025, METALC, 6);
        }
    }

    // Tub: floor, footwell, side walls, sills and the canopy rails. The tub is
    // wider than the canopy below the sills, where the side consoles run out
    // to the walls; above the consoles the walls lean in to the sills.
    let (wx, wy) = (consoles::WALL_X, consoles::WALL_Y);
    m.quad(v3(-wx, 0.22, -4.40), v3(wx, 0.22, -4.40), v3(wx, 0.22, -2.40), v3(-wx, 0.22, -2.40), WALL.scale(0.55));
    m.quad(v3(-wx, 0.22, -4.40), v3(wx, 0.22, -4.40), v3(wx, 0.42, -4.12), v3(-wx, 0.42, -4.12), WALL.scale(0.45));
    for &s in &[-1.0, 1.0] {
        let n = 12;
        for k in 0..n {
            let (z0, z1) = (mix(-4.30, -2.30, k as f64 / n as f64), mix(-4.30, -2.30, (k + 1) as f64 / n as f64));
            let (b0, b1) = (airframe::canopy_profile(z0).1, airframe::canopy_profile(z1).1);
            let (w0, w1) = (airframe::canopy_profile(z0).0.max(0.47), airframe::canopy_profile(z1).0.max(0.47));
            m.quad(v3(s * wx, 0.22, z0), v3(s * wx, 0.22, z1), v3(s * wx, wy, z1), v3(s * wx, wy, z0), WALL);
            m.quad(v3(s * wx, wy, z0), v3(s * wx, wy, z1), v3(s * 0.47, b1 - 0.03, z1), v3(s * 0.47, b0 - 0.03, z0), WALL.scale(1.08));
            m.quad(v3(s * 0.47, b0 - 0.03, z0), v3(s * 0.47, b1 - 0.03, z1), v3(s * w1, b1, z1), v3(s * w0, b0, z0), WALL.scale(1.15));
            let d = (v3(s * w1, b1, z1) - v3(s * w0, b0, z0)).norm() * 0.012;
            pilot::bar(m, v3(s * w0, b0, z0) - d, v3(s * w1, b1, z1) + d, 0.024, 0.018, V3::Y, Rgb(0.12, 0.13, 0.14));
        }
        // Auxiliary console: its face and rim, the shelf from its top edge
        // to the panel, the inboard side down to the floor, and the panel
        // carried out from the wing edge to the wall above the shelf. The
        // leaning wall hides what reaches past it.
        let (o, x, y, n, w, h) = aux_face(s);
        let c = |a: f64, b: f64| o + x * a + y * b;
        m.quad(c(0.0, 0.0), c(w, 0.0), c(w, h), c(0.0, h), PANEL.scale(0.92));
        for (a, b) in [((0.0, 0.0), (w, 0.0)), ((w, 0.0), (w, h)), ((w, h), (0.0, h)), ((0.0, h), (0.0, 0.0))] {
            m.quad(c(a.0, a.1), c(b.0, b.1), c(b.0, b.1) - n * 0.02, c(a.0, a.1) - n * 0.02, PANEL.scale(0.7));
        }
        let (bi, ti, to) = if s > 0.0 { (c(0.0, 0.0), c(0.0, h), c(w, h)) } else { (c(w, 0.0), c(w, h), c(0.0, h)) };
        m.quad(ti, to, panel_point(to.x, to.y), panel_point(ti.x, ti.y), PANEL.scale(0.75));
        let floor = |p: V3| v3(p.x, 0.22, p.z);
        let low = panel_point(ti.x, 0.40);
        m.polygon(&[bi, ti, panel_point(ti.x, ti.y), low, floor(low), floor(bi)], WALL.scale(0.6));
        // Close only the shoulder below the panel's actual outer corner.
        // Extending this rectangle up to canopy height formed the two
        // pointed ears outside the sills.
        let shoulder = panel_shoulder(s, to.y);
        m.quad(shoulder[0], shoulder[1], shoulder[2], shoulder[3], PANEL.scale(0.8));
    }
    // On the walls leaning in above the consoles: the canopy jettison
    // T-handle and the map light on the left, the canopy switch box and the
    // map case on the right.
    let wall = |s: f64, t: f64, z: f64| -> Frame {
        let b = airframe::canopy_profile(z).1 - 0.03;
        let (lo, hi) = (v3(s * wx, wy, z), v3(s * 0.47, b, z));
        let up = (hi - lo).norm();
        let n = (V3::Z.cross(up) * s).norm();
        Frame { o: lo.lerp(hi, t) + n * 0.004, x: V3::Z * -s, y: up, z: n }
    };
    let f = wall(-1.0, 0.35, -3.0);
    m.block(&f.at(0.0, 0.0, 0.004), v3(0.07, 0.045, 0.004), BEZEL.scale(0.9));
    for k in 0..6 {
        let x0 = -0.045 + k as f64 * 0.015;
        let col = if k % 2 == 0 { Rgb(0.85, 0.68, 0.16) } else { Rgb(0.08, 0.08, 0.06) };
        m.block(&f.at(x0 + 0.0075, 0.0, 0.03), v3(0.0075, 0.012, 0.01), col);
    }
    m.capsule(f.p(0.0, 0.0, 0.006), f.p(0.0, 0.0, 0.022), 0.006, 0.006, METALC, 6);
    let f = wall(-1.0, 0.4, -2.6);
    m.block(&f.at(0.0, 0.0, 0.01), v3(0.03, 0.02, 0.01), BEZEL);
    m.capsule(f.p(0.0, 0.0, 0.02), f.p(0.0, 0.03, 0.05), 0.006, 0.01, Rgb(0.10, 0.10, 0.10), 6);
    let f = wall(1.0, 0.35, -3.45);
    m.block(&f.at(0.0, 0.0, 0.008), v3(0.045, 0.03, 0.008), BEZEL);
    m.capsule(f.p(0.0, 0.0, 0.016), f.p(0.0, 0.02, 0.036), 0.003, 0.0025, METALC, 6);
    let f = wall(1.0, 0.25, -2.85);
    m.block(&f.at(0.0, 0.0, 0.025), v3(0.15, 0.08, 0.025), Rgb(0.13, 0.15, 0.13));
    m.block(&f.at(0.0, 0.05, 0.052), v3(0.15, 0.035, 0.004), Rgb(0.17, 0.19, 0.16));
    consoles::build(m);
    pilot::build_static(m);

    // Canopy aft bow and bulkhead behind the seat, on the canopy's facet lines.
    let nv = airframe::CANOPY_NV;
    let bow: Vec<V3> = (0..=nv).map(|k| airframe::canopy_point(0.985, k as f64 / nv as f64)).collect();
    pilot::band(m, &bow, 0.03, 0.026, V3::Z, Rgb(0.13, 0.15, 0.16));
    let bulk: Vec<V3> = (0..=nv).map(|k| airframe::canopy_point(0.995, k as f64 / nv as f64)).collect();
    m.polygon(&bulk, Rgb(0.12, 0.13, 0.13));    m.facet();
}

#[cfg(test)]
mod geometry_tests {
    use super::*;

    #[test]
    fn panel_shoulders_stay_below_the_outline_and_inside_the_tub() {
        for side in [-1.0, 1.0] {
            let (o, x, y, _, w, h) = aux_face(side);
            let bottom = (o + x * (if side > 0.0 { w } else { 0.0 }) + y * h).y;
            let shoulder = panel_shoulder(side, bottom);
            for (i, p) in shoulder.iter().enumerate() {
                assert!(p.is_finite() && p.y <= 0.760001, "shoulder corner {i}: {p:?}");
                assert!(p.x.abs() <= tub_half_width(*p) + 1e-8, "shoulder outside wall: {p:?}");
            }
            for tri in [[0, 1, 2], [0, 2, 3]] {
                let [a, b, c] = tri.map(|i| shoulder[i]);
                assert!((b - a).cross(c - a).len() > 1e-7);
                for i in 0..=8 {
                    for j in 0..=8-i {
                        let p = a + (b - a) * (i as f64 / 8.0) + (c - a) * (j as f64 / 8.0);
                        assert!(p.x.abs() <= tub_half_width(p) + 1e-6, "shoulder surface outside wall: {p:?}");
                    }
                }
            }
        }
        for (u, v) in OUTLINE {
            let p = pp(u, v, 0.0);
            assert!(p.x.abs() <= tub_half_width(p));
        }
    }
}

// ================================================================ pages
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Fcr,
    Hsd,
    Eng,
    Air,
}

impl Page {
    pub fn name(self) -> &'static str {
        match self {
            Page::Fcr => "FCR",
            Page::Hsd => "HSD",
            Page::Eng => "ENG",
            Page::Air => "AIR",
        }
    }
    pub fn next(self) -> Page {
        match self {
            Page::Fcr => Page::Hsd,
            Page::Hsd => Page::Eng,
            Page::Eng => Page::Air,
            Page::Air => Page::Fcr,
        }
    }
}

pub fn pad(n: f64, l: usize) -> String {
    format!("{:0>l$}", n.max(0.0).round() as i64, l = l)
}

/// mm:ss, or dashes when there is no closure.
fn clock(sec: Option<f64>) -> String {
    match sec {
        Some(s) if s < 6000.0 => format!("{:02}:{:02}", (s / 60.0).floor() as i64, (s % 60.0).floor() as i64),
        _ => "--:--".to_string(),
    }
}

/// Distance (m) and time to go (s) to the next steerpoint at the current
/// ground-speed closure.
fn steering(a: &Aircraft, m: &Mission) -> (f64, Option<f64>) {
    let rel = m.current_gate().pos - a.pos;
    let flat = v3(rel.x, 0.0, rel.z);
    let closure = v3(a.vel.x, 0.0, a.vel.z).dot(flat.norm());
    (rel.len(), if closure > 1.0 { Some(flat.len() / closure) } else { None })
}

fn hardware(cv: &mut Canvas, r: R, label: &str) {
    cv.rect_grad(r.x, r.y, r.w, r.h, rgba(0x33, 0x39, 0x3d, 1.0), rgba(0x22, 0x28, 0x2c, 1.0));
    cv.stroke_rect(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 1.0, rgba(0x6a, 0x72, 0x76, 1.0));
    cv.stroke_rect(r.x + 5.0, r.y + 5.0, r.w - 10.0, r.h - 10.0, 1.0, rgba(0x07, 0x0c, 0x10, 1.0));
    for (x, y) in [(9.0, 9.0), (r.w - 9.0, 9.0), (9.0, r.h - 9.0), (r.w - 9.0, r.h - 9.0)] {
        cv.disc(r.x + x, r.y + y, 3.0, rgba(0x10, 0x15, 0x17, 1.0));
        cv.line(r.x + x - 2.0, r.y + y, r.x + x + 2.0, r.y + y, 1.0, rgba(0x77, 0x81, 0x7b, 1.0));
    }
    if !label.is_empty() {
        cv.text(label, r.x + r.w / 2.0, r.y + r.h - 12.0, 1.0, rgba(0xb7, 0xc0, 0xba, 1.0), Align::Center);
    }
}

/// Round instrument face; returns its centre and radius.
fn gauge_face(cv: &mut Canvas, r: R) -> (f32, f32, f32) {
    let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let rad = r.w * 0.5 - 2.0;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x10, 0x16, 0x19, 1.0));
    cv.disc(x, y, rad, rgba(0x03, 0x06, 0x06, 1.0));
    cv.circle(x, y, rad - 1.0, 2.0, rgba(0x2c, 0x33, 0x32, 1.0));
    (x, y, rad)
}

/// Radial stroke at canvas angle `a` (radians, clockwise from three o'clock),
/// between radii `r0` and `r1`.
#[allow(clippy::too_many_arguments)]
fn spoke(cv: &mut Canvas, x: f32, y: f32, a: f64, r0: f32, r1: f32, w: f32, col: Col) {
    let (c, s) = ((a as f32).cos(), (a as f32).sin());
    cv.line(x + c * r0, y + s * r0, x + c * r1, y + s * r1, w, col);
}

fn spoke_text(cv: &mut Canvas, x: f32, y: f32, a: f64, r: f32, s: &str, col: Col) {
    cv.text(s, x + (a as f32).cos() * r, y + (a as f32).sin() * r, 1.0, col, Align::Center);
}

const POINTER: Col = rgba(0xf8, 0xf3, 0xd5, 1.0);
const MACH_RING: Col = rgba(0x8c, 0xd0, 0xe8, 1.0);

/// Airspeed/Mach indicator. One pointer reads KCAS on the fixed outer scale
/// and Mach on the inner ring. The ring turns with static pressure, as on the
/// real instrument, because a given Mach number is fewer calibrated knots the
/// higher the jet flies. The red pointer is the limit speed for this level:
/// 800 KCAS low down, Mach 2.05 upstairs.
fn asi_mach(cv: &mut Canvas, r: R, ad: &AirData) {
    let (x, y, rad) = gauge_face(cv, r);
    let ang = |kt: f64| (-90.0 + 315.0 * clamp(kt / 850.0, 0.0, 1.0)) * DEG;
    for k in 0..=17 {
        let major = k % 2 == 0;
        spoke(cv, x, y, ang(k as f64 * 50.0), rad * if major { 0.74 } else { 0.81 }, rad * 0.90, 2.0, WHITE);
        if major && k > 0 {
            spoke_text(cv, x, y, ang(k as f64 * 50.0), rad * 0.62, &format!("{}", k / 2), WHITE);
        }
    }
    for k in 5..=20 {
        let kt = air::cas_for_mach(k as f64 / 10.0, ad.atm.pressure) * air::KT;
        if kt > 850.0 {
            break;
        }
        let major = k % 5 == 0;
        spoke(cv, x, y, ang(kt), rad * 0.38, rad * if major { 0.52 } else { 0.45 }, 2.0, if k == 10 { AMBER } else { MACH_RING });
    }
    spoke(cv, x, y, ang(ad.vmax_kcas), rad * 0.56, rad * 0.92, 3.0, RED);
    cv.text(&pad(ad.kcas, 3), x, y - rad * 0.08, 2.0, GREEN, Align::Center);
    cv.text(&format!("M{:.2}", ad.mach), x, y + rad * 0.20, 1.0, AMBER, Align::Center);
    spoke(cv, x, y, ang(ad.kcas), -rad * 0.12, rad * 0.86, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));
}

/// Counter-pointer altimeter: the pointer turns once per 1,000 ft, the counter
/// shows the barometric altitude, the Kollsman window the setting (QNH, in Hg).
fn altimeter(cv: &mut Canvas, r: R, ad: &AirData) {
    let (x, y, rad) = gauge_face(cv, r);
    let ang = |t: f64| (-90.0 + 360.0 * t) * DEG;
    for k in 0..50 {
        let major = k % 5 == 0;
        spoke(cv, x, y, ang(k as f64 / 50.0), rad * if major { 0.74 } else { 0.83 }, rad * 0.90, if major { 2.0 } else { 1.0 }, WHITE);
    }
    for k in 0..10 {
        spoke_text(cv, x, y, ang(k as f64 / 10.0), rad * 0.62, &format!("{k}"), WHITE);
    }
    let alt = ad.baro_ft.max(0.0);
    cv.rect(x - 20.0, y - rad * 0.22 - 8.0, 40.0, 16.0, rgba(0x00, 0x00, 0x00, 1.0));
    cv.text(&pad(alt, 5), x, y - rad * 0.22, 1.0, WHITE, Align::Center);
    cv.rect(x - 16.0, y + rad * 0.26 - 7.0, 32.0, 14.0, rgba(0x00, 0x00, 0x00, 1.0));
    cv.text(&format!("{:.2}", ad.qnh_inhg), x, y + rad * 0.26, 1.0, AMBER, Align::Center);
    spoke(cv, x, y, ang(alt % 1000.0 / 1000.0), -rad * 0.12, rad * 0.86, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));
}

/// Accelerometer: normal load factor from -5 to +10 G, +1 at nine o'clock,
/// red beyond the +9 / -3 airframe limits, with amber memory pointers that
/// hold the peak positive and negative G of the sortie.
/// AOA indicator: a tape of degrees scrolling past a fixed index, the
/// approach band (11 to 15) green, the high-AoA end red.
fn aoa_tape(cv: &mut Canvas, r: R, a: &Aircraft) {
    let (x, y, rad) = gauge_face(cv, r);
    let (l, w) = (x - rad * 0.32, rad * 0.64);
    let (top, bottom) = (y - rad * 0.78, y + rad * 0.78);
    cv.rect(l, top, w, bottom - top, rgba(0x1a, 0x1e, 0x1c, 1.0));
    let per = rad * 0.07;
    let at = |deg: f64| y - ((deg - a.alpha_deg) as f32) * per;
    cv.set_clip(Some(vec![[l, top], [l + w, top], [l + w, bottom], [l, bottom]]));
    cv.rect(l, at(15.0), w * 0.25, at(11.0) - at(15.0), GREEN);
    cv.rect(l, at(32.0), w * 0.25, at(25.0) - at(32.0), RED);
    for deg in (-5..=32).step_by(5) {
        let yy = at(deg as f64);
        cv.line(l + w * 0.28, yy, l + w * 0.5, yy, 2.0, WHITE);
        cv.text(&format!("{deg}"), l + w * 0.95, yy, 1.0, WHITE, Align::Right);
    }
    cv.set_clip(None);
    cv.line(l - 4.0, y, l + w + 4.0, y, 3.0, POINTER);
    cv.text("AOA", x, top - 6.0, 1.0, WHITE, Align::Center);
}

fn g_meter(cv: &mut Canvas, r: R, ad: &AirData) {
    let (x, y, rad) = gauge_face(cv, r);
    let ang = |g: f64| (180.0 + (clamp(g, -5.0, 10.0) - 1.0) * 22.0) * DEG;
    cv.arc(x, y, rad * 0.87, ang(9.0) as f32, ang(10.0) as f32, 4.0, RED);
    cv.arc(x, y, rad * 0.87, ang(-5.0) as f32, ang(-3.0) as f32, 4.0, RED);
    for g in -5..=10 {
        let major = g % 2 == 0;
        spoke(cv, x, y, ang(g as f64), rad * if major { 0.74 } else { 0.81 }, rad * 0.90, 2.0, WHITE);
        if major {
            spoke_text(cv, x, y, ang(g as f64), rad * 0.62, &format!("{g}"), WHITE);
        }
    }
    cv.text("G", x, y - rad * 0.24, 1.0, WHITE, Align::Center);
    cv.text(&format!("{:+.1}", ad.g), x, y + rad * 0.30, 1.0, GREEN, Align::Center);
    spoke(cv, x, y, ang(ad.g_max), rad * 0.10, rad * 0.80, 2.0, AMBER);
    spoke(cv, x, y, ang(ad.g_min), rad * 0.10, rad * 0.80, 2.0, AMBER);
    spoke(cv, x, y, ang(ad.g), -rad * 0.12, rad * 0.86, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));
}

#[allow(clippy::too_many_arguments)]
fn dial(cv: &mut Canvas, r: R, value: f64, min: f64, max: f64, label: &str, readout: &str, red_from: usize) {
    let (x, y, rad) = gauge_face(cv, r);
    for i in 0..=10 {
        let a = ((-220.0 + i as f64 * 26.0) * DEG) as f32;
        let (c, s) = (a.cos(), a.sin());
        let col = if i >= red_from { RED } else { WHITE };
        let len = if i % 2 == 0 { 0.70 } else { 0.78 };
        cv.line(x + c * rad * len, y + s * rad * len, x + c * rad * 0.90, y + s * rad * 0.90, 2.0, col);
        if i % 2 == 0 {
            let n = min + (max - min) * i as f64 / 10.0;
            let label = if max >= 1000.0 { format!("{}", (n / 100.0).round() as i64) } else { format!("{}", n.round() as i64) };
            cv.text(&label, x + c * rad * 0.52, y + s * rad * 0.52, 1.0, WHITE, Align::Center);
        }
    }
    let t = clamp((value - min) / (max - min), 0.0, 1.0);
    let a = ((-220.0 + t * 260.0) * DEG) as f32;
    cv.line(x - a.cos() * rad * 0.15, y - a.sin() * rad * 0.15, x + a.cos() * rad * 0.80, y + a.sin() * rad * 0.80, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));
    cv.text(label, x, y - rad * 0.24, 1.0, WHITE, Align::Center);
    cv.text(readout, x, y + rad * 0.40, 2.0, GREEN, Align::Center);
}

fn osb_labels(cv: &mut Canvas, r: R, top: [&str; 5], bottom: [&str; 5]) {
    for i in 0..5 {
        let x = r.x + 26.0 + i as f32 * (r.w - 52.0) / 4.0;
        cv.text(top[i], x, r.y + 12.0, 1.0, GREEN, Align::Center);
        cv.text(bottom[i], x, r.y + r.h - 12.0, 1.0, GREEN, Align::Center);
    }
}

#[allow(clippy::too_many_arguments)]
fn mfd(cv: &mut Canvas, r: R, page: Page, a: &Aircraft, ad: &AirData, m: &Mission, time: f64, c: &Combat) {
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x01, 0x07, 0x0a, 1.0));
    let (x0, y0, w, h) = (r.x, r.y, r.w, r.h);
    osb_labels(cv, r, ["OVRD", "", page.name(), "", "CNTL"], ["SWAP", "FCR", "HSD", "ENG", "AIR"]);
    match page {
        Page::Hsd => {
            let (cx, cy) = (x0 + w / 2.0, y0 + h * 0.58);
            let range = 9260.0;
            let scale = (w * 0.40) as f64 / range;
            for k in 1..=3 {
                cv.circle(cx, cy, w * 0.40 * k as f32 / 3.0, 1.0, rgba(0x18, 0x3d, 0x38, 1.0));
            }
            cv.line(cx, y0 + 30.0, cx, y0 + h - 30.0, 1.0, rgba(0x16, 0x36, 0x31, 1.0));
            cv.line(x0 + 8.0, cy, x0 + w - 8.0, cy, 1.0, rgba(0x16, 0x36, 0x31, 1.0));
            let hd = a.heading * DEG;
            let nav = |p: V3| {
                let d = p - a.pos;
                let x = d.x * hd.cos() + d.z * hd.sin();
                let y = d.x * hd.sin() - d.z * hd.cos();
                [cx + (x * scale) as f32, cy - (y * scale) as f32]
            };
            cv.set_clip(Some(vec![[x0 + 4.0, y0 + 24.0], [x0 + w - 4.0, y0 + 24.0], [x0 + w - 4.0, y0 + h - 24.0], [x0 + 4.0, y0 + h - 24.0]]));
            for (i, g) in m.gates.iter().enumerate() {
                let p = nav(g.pos);
                if let Some(n) = m.gates.get(i + 1) {
                    let q = nav(n.pos);
                    cv.line(p[0], p[1], q[0], q[1], 2.0, rgba(0x49, 0x8c, 0xa9, 1.0));
                }
            }
            for (i, g) in m.gates.iter().enumerate() {
                let p = nav(g.pos);
                let next = i == m.gate_index;
                cv.circle(p[0], p[1], if next { 6.0 } else { 3.5 }, 2.0, if next { rgba(0xf0, 0x7a, 0xc9, 1.0) } else { rgba(0x7a, 0xc5, 0xb0, 1.0) });
                cv.text(&format!("{}", i + 1), p[0] + 9.0, p[1], 1.0, rgba(0xb6, 0xc8, 0xdf, 1.0), Align::Left);
            }
            cv.set_clip(None);
            cv.polyline(&[[cx - 8.0, cy + 7.0], [cx, cy - 8.0], [cx + 8.0, cy + 7.0]], 2.0, true, GREEN);
            cv.text("5 NM", x0 + w - 10.0, y0 + 34.0, 1.0, GREEN, Align::Right);
            cv.text("NAV  GPS  NORM", cx, y0 + 46.0, 1.0, rgba(0x7c, 0xbc, 0xc2, 1.0), Align::Center);
            cv.text(&format!("{} HDG", pad(a.heading, 3)), x0 + 10.0, y0 + h - 44.0, 1.0, GREEN, Align::Left);
            cv.text(&format!("STPT {}", pad((m.gate_index + 1).min(m.gates.len()) as f64, 2)), x0 + w - 10.0, y0 + h - 44.0, 1.0, GREEN, Align::Right);
            cv.text(&format!("VS {:+}", (ad.vs_fpm / 10.0).round() as i64 * 10), x0 + 10.0, y0 + h - 32.0, 1.0, DIM, Align::Left);
            cv.text(&format!("AOA {:.1}", a.alpha_deg), x0 + w - 10.0, y0 + h - 32.0, 1.0, DIM, Align::Right);
        }
        Page::Fcr => {
            cv.text(if c.mode.air_to_ground() { "GM  MAP" } else if c.radar_lock().is_some() { "CRM  STT  NORM" } else if c.acquiring.is_some() { "CRM  ACQ  NORM" } else { "CRM  RWS  NORM" }, x0 + w / 2.0, y0 + 34.0, 1.0, GREEN, Align::Center);
            let nm = symbology::fcr_scale(a, c);
            cv.text(&format!("{nm}"), x0 + 10.0, y0 + 60.0, 1.0, GREEN, Align::Left);
            let (l, rr, t, b) = (x0 + 28.0, x0 + w - 28.0, y0 + 50.0, y0 + h - 50.0);
            for i in 1..4 {
                let yy = t + (b - t) * i as f32 / 4.0;
                cv.line(l, yy, rr, yy, 1.0, rgba(0x15, 0x37, 0x31, 1.0));
            }
            cv.line(l, t, l, b, 1.0, GREEN);
            cv.line(rr, t, rr, b, 1.0, GREEN);
            if c.radar_lock().is_none() {
                let sweep = 1.0 - ((time * 0.48).fract() as f32 * 2.0 - 1.0).abs();
                let sx = l + 4.0 + sweep * (rr - l - 8.0);
                cv.line(sx, t + 2.0, sx, b - 2.0, 2.0, rgba(0x39, 0x79, 0x57, 1.0));
            }
            if !symbology::fcr(cv, [l, t, rr, b], a, c, nm) {
                cv.text("NO TRACK", x0 + w / 2.0, (t + b) / 2.0, 2.0, rgba(0x80, 0xb7, 0x91, 1.0), Align::Center);
            }
            cv.text(&format!("{} C", pad(ad.kcas, 3)), l + 4.0, b + 14.0, 1.0, GREEN, Align::Left);
            cv.text(&format!("{:.0} ALT", ad.baro_ft / 100.0), rr - 4.0, b + 14.0, 1.0, GREEN, Align::Right);
        }
        Page::Air => {
            // Air-data computer: static air at the jet and what it does to the airspeeds.
            let rows = [
                ("OAT / TAT", format!("{:+.0} / {:+.0} C", ad.oat_c, ad.tat_c)),
                ("STATIC PRESS", format!("{:.1} HPA", ad.atm.pressure / 100.0)),
                ("PRESS ALT", format!("{} FT", ad.pressure_alt_ft.round() as i64)),
                ("DENS ALT", format!("{} FT", ad.density_alt_ft.round() as i64)),
                ("DENSITY  SIGMA", format!("{:.3} {:.2}", ad.atm.rho, ad.sigma)),
                ("SOUND / Q", format!("{} KT {:.0}KPA", pad(ad.atm.sound * air::KT, 3), ad.qbar / 1000.0)),
                ("TAS / EAS", format!("{} / {}", pad(ad.ktas, 3), pad(ad.keas, 3))),
                ("GS / VS", format!("{} {:+}", pad(ad.gs_kt, 3), (ad.vs_fpm / 10.0).round() as i64 * 10)),
            ];
            for (i, (k, v)) in rows.iter().enumerate() {
                let yy = y0 + 40.0 + i as f32 * 31.0;
                cv.text(k, x0 + 14.0, yy, 1.0, DIM, Align::Left);
                cv.text(v, x0 + w - 14.0, yy + 12.0, 2.0, GREEN, Align::Right);
                cv.line(x0 + 12.0, yy + 22.0, x0 + w - 12.0, yy + 22.0, 1.0, rgba(0x1c, 0x34, 0x31, 1.0));
            }
        }
        Page::Eng => {
            let rows = [
                ("RPM", format!("{:.1} %", a.rpm)),
                ("FTIT", format!("{} C", pad(a.ftit, 3))),
                ("NOZZLE", format!("{} %", pad(a.nozzle, 2))),
                ("FUEL", format!("{} LB", pad(a.fuel * 2.20462, 4))),
                ("FLOW", format!("{} PPH", pad(a.fuel_flow * 7936.64, 5))),
                ("G / AOA", format!("{:.1} / {:.1}", a.g_smooth, a.alpha_deg)),
            ];
            for (i, (k, v)) in rows.iter().enumerate() {
                let yy = y0 + 44.0 + i as f32 * 30.0;
                cv.text(k, x0 + 14.0, yy, 1.0, DIM, Align::Left);
                cv.text(v, x0 + w - 14.0, yy + 12.0, 2.0, GREEN, Align::Right);
                cv.line(x0 + 12.0, yy + 24.0, x0 + w - 12.0, yy + 24.0, 1.0, rgba(0x1c, 0x34, 0x31, 1.0));
            }
            let (s, c) = if a.ab { ("AFTERBURNER", AMBER) } else { ("FLCS NORM", GREEN) };
            // Status line in its own row under the table, clear of the readouts.
            cv.text(s, x0 + w / 2.0, y0 + h - 30.0, 1.0, c, Align::Center);
        }
    }
}

fn draw_instruments(cv: &mut Canvas, a: &Aircraft, m: &Mission, pages: [Page; 2], time: f64, c: &Combat) {
    let ad = AirData::of(a);
    mfd(cv, R_MFD_L, pages[0], a, &ad, m, time, c);
    mfd(cv, R_MFD_R, pages[1], a, &ad, m, time, c);
    hsi(cv, R_HSI, a, m);

    // DED.
    let r = R_DED;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x02, 0x08, 0x04, 1.0));
    let (dist, ttg) = steering(a, m);
    let lines = [
        format!("UHF  305.00   STPT {}", pad((m.gate_index + 1).min(m.gates.len()) as f64, 2)),
        format!("ALT {:.2}IN  OAT {:+.0}C", ad.qnh_inhg, ad.oat_c),
        format!("M {:.2}       {} DEG", ad.mach, pad(a.heading, 3)),
        format!("DIST {:.1} NM   TTG {}", dist / 1852.0, clock(ttg)),
        format!("TIME {}:{}  FUEL {}", pad((m.elapsed / 60.0).floor(), 2), pad((m.elapsed % 60.0).floor(), 2), pad(a.fuel * 2.20462, 4)),
    ];
    for (i, s) in lines.iter().enumerate() {
        cv.text(s, r.x + 8.0, r.y + 7.0 + i as f32 * 12.2, 1.0, GREEN, Align::Left);
    }

    // ICP keypad.
    let r = R_ICP;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x30, 0x38, 0x3b, 1.0));
    let labels = [["COM1", "COM2", "IFF", "LIST"], ["1 ILS", "2 ALW", "3", "RCL"], ["4 STP", "5 CRS", "6 TIM", "ENTR"], ["7 MRK", "8 FIX", "9 CAL", "DCS"], ["A-A", "0 SEL", "A-G", "DRFT"]];
    for (yy, row) in labels.iter().enumerate() {
        for (xx, s) in row.iter().enumerate() {
            let (kx, ky) = (r.x + 6.0 + xx as f32 * 66.0, r.y + 5.0 + yy as f32 * 34.0);
            cv.rect(kx, ky, 60.0, 29.0, rgba(0x12, 0x19, 0x1b, 1.0));
            cv.stroke_rect(kx, ky, 60.0, 29.0, 1.0, rgba(0x70, 0x7b, 0x74, 1.0));
            cv.text(s, kx + 30.0, ky + 15.0, 1.0, WHITE, Align::Center);
        }
    }

    // RWR: the threats of the fight, if any.
    let r = R_RWR;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x02, 0x08, 0x05, 1.0));
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    for k in [18.0, 36.0, 54.0] {
        cv.circle(cx, cy, k, 1.0, rgba(0x31, 0x53, 0x40, 1.0));
    }
    cv.line(cx - 56.0, cy, cx + 56.0, cy, 1.0, rgba(0x46, 0x6d, 0x50, 1.0));
    cv.line(cx, cy - 56.0, cx, cy + 56.0, 1.0, rgba(0x46, 0x6d, 0x50, 1.0));
    cv.text("N", cx, r.y + 9.0, 1.0, GREEN, Align::Center);
    if !symbology::rwr(cv, cx, cy, a, c, time) {
        cv.text("NO EMITTER", cx, cy + 22.0, 1.0, GREEN, Align::Center);
    }
    cv.text("RWR", cx, r.y + r.h - 9.0, 1.0, WHITE, Align::Center);

    // Master caution lamp.
    let r = R_MC;
    let caution = a.fuel < 450.0 || a.fuel <= 0.0 || a.alpha_deg > 25.0;
    let on = caution && (time * 2.0).fract() < 0.6;
    cv.rect(r.x, r.y, r.w, r.h, if on { rgba(0xd4, 0x96, 0x38, 1.0) } else { rgba(0x2a, 0x1e, 0x0c, 1.0) });
    cv.text("MASTER", r.x + r.w / 2.0, r.y + 24.0, 1.0, if on { rgba(0x1c, 0x14, 0x05, 1.0) } else { rgba(0x6b, 0x55, 0x30, 1.0) }, Align::Center);
    cv.text("CAUTION", r.x + r.w / 2.0, r.y + 38.0, 1.0, if on { rgba(0x1c, 0x14, 0x05, 1.0) } else { rgba(0x6b, 0x55, 0x30, 1.0) }, Align::Center);

    // Dials: airspeed/Mach, altimeter, accelerometer, standby ADI, RPM, FTIT, nozzle, oil.
    asi_mach(cv, R_DIALS[0], &ad);
    altimeter(cv, R_DIALS[1], &ad);
    g_meter(cv, R_ACCEL, &ad);
    aoa_tape(cv, R_AOAI, a);
    adi(cv, R_DIALS[2], a);
    dial(cv, R_DIALS[3], a.rpm, 0.0, 110.0, "RPM", &pad(a.rpm, 2), 10);
    dial(cv, R_DIALS[4], a.ftit, 200.0, 1200.0, "FTIT", &pad(a.ftit, 3), 9);
    dial(cv, R_DIALS[5], a.nozzle, 0.0, 100.0, "NOZ", &pad(a.nozzle, 2), 11);
    dial(cv, R_DIALS[6], a.oil_psi, 0.0, 100.0, "OIL", &pad(a.oil_psi, 2), 11);

    // Warning lights (the right eyebrow), lit red.
    let r = R_CAUTION;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x20, 0x27, 0x29, 1.0));
    let low_slow = a.agl < 3000.0 && a.airspeed_kt < 190.0 && !a.on_ground;
    let labels = [("ENG FIRE", a.battle.fire), ("ENGINE", a.fuel <= 0.0 || a.battle.thrust() == 0.0), ("HYD/OIL", a.oil_psi < 20.0), ("FLCS", a.alpha_deg > 25.0 || a.battle.controls > 0.5), ("TO/LDG", low_slow && a.gear < 0.999), ("CANOPY", a.ejected)];
    for (i, (s, lit)) in labels.iter().enumerate() {
        let (x, y) = (r.x + (i % 2) as f32 * 128.0, r.y + (i / 2) as f32 * 42.6);
        cv.rect(x + 3.0, y + 3.0, 122.0, 36.0, if *lit { rgba(0xe0, 0x40, 0x30, 1.0) } else { rgba(0x0c, 0x14, 0x14, 1.0) });
        cv.text(s, x + 64.0, y + 21.0, 2.0, if *lit { rgba(0x1c, 0x14, 0x05, 1.0) } else { rgba(0x6b, 0x75, 0x69, 1.0) }, Align::Center);
    }

    // Narrow master-arm strip, with an atlas aspect matching the physical
    // lightplate rather than stretching the former wide placard vertically.
    let r = R_ARM;
    hardware(cv, r, "");
    cv.text("MASTER", r.x + 48.0, r.y + 22.0, 1.0, WHITE, Align::Center);
    cv.text("ARM", r.x + 48.0, r.y + 36.0, 1.0, WHITE, Align::Center);
    let switch = if c.mode == Mode::Nav { 2 } else { 0 };
    for (i, s) in ["ARM", "SIM", "SAFE"].iter().enumerate() {
        let y = r.y + 62.0 + i as f32 * 22.0;
        cv.text(s, r.x + 44.0, y, 1.0, DIM, Align::Left);
        if i == switch { cv.line(r.x + 18.0, y, r.x + 35.0, y, 4.0, WHITE); }
    }
    cv.line(r.x + 10.0, r.y + 128.0, r.x + 86.0, r.y + 128.0, 1.0, DIM);
    cv.text("LASER", r.x + 48.0, r.y + 147.0, 1.0, WHITE, Align::Center);
    cv.text("ARM", r.x + 48.0, r.y + 161.0, 1.0, WHITE, Align::Center);
    cv.circle(r.x + 48.0, r.y + 192.0, 9.0, 2.0, DIM);
    cv.line(r.x + 48.0, r.y + 192.0, r.x + 48.0, r.y + 209.0, 4.0, WHITE);
    cv.text("OFF", r.x + 48.0, r.y + 229.0, 1.0, DIM, Align::Center);

    // AOA indexer: high, on-speed, low chevrons.
    let r = R_AOA;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x12, 0x1b, 0x1c, 1.0));
    let (hi, on_speed, lo) = (a.alpha_deg > 14.0, a.alpha_deg >= 11.0 && a.alpha_deg <= 14.0, a.alpha_deg < 11.0);
    let cx = r.x + r.w / 2.0;
    cv.polyline(&[[cx - 14.0, r.y + 14.0], [cx, r.y + 34.0], [cx + 14.0, r.y + 14.0]], 5.0, false, if hi { rgba(0xff, 0x5f, 0x46, 1.0) } else { rgba(0x60, 0x38, 0x2c, 1.0) });
    cv.circle(cx, r.y + 64.0, 11.0, 5.0, if on_speed { GREEN } else { rgba(0x27, 0x45, 0x34, 1.0) });
    cv.polyline(&[[cx - 14.0, r.y + 114.0], [cx, r.y + 94.0], [cx + 14.0, r.y + 114.0]], 5.0, false, if lo { rgba(0xe9, 0xba, 0x53, 1.0) } else { rgba(0x5c, 0x4e, 0x2e, 1.0) });

    // Kneeboard.
    let r = R_KNEE;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0xb7, 0xbc, 0xa0, 1.0));
    let ink = rgba(0x28, 0x3c, 0x30, 1.0);
    // Landing card: the limits and speeds the model flies to, in knots on
    // the F-16 and km/h on the MiG-29.
    use crate::sim::{spec, Type};
    let mig = a.kind == Type::Mig29;
    let speed = |kt: f64| if mig { format!("{:.0} KM/H", (kt * 1.852 / 10.0).round() * 10.0) } else { format!("{kt:.0} KT") };
    let (rotate, approach, aoa, g) = if mig { (130.0, 140.0, "11 DEG", "+8.0 / -3.0") } else { (150.0, 150.0, "13 DEG", "+9.0 / -3.0") };
    cv.text("LANDING CARD", r.x + r.w / 2.0, r.y + 18.0, 2.0, ink, Align::Center);
    let rows = [
        ("ROTATE", speed(rotate)),
        ("GEAR LIMIT", speed(spec::GEAR_LIMIT_KT)),
        ("FLAP BLOWUP", speed(spec::FLAP_BLOWBACK_KT)),
        ("FINAL", speed(approach)),
        ("FINAL AOA", aoa.to_string()),
        ("FLAPS", format!("{:.0} DEG DOWN", spec::FLAP_MAX)),
        ("G LIMIT", g.to_string()),
    ];
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = r.y + 46.0 + i as f32 * 24.0;
        cv.text(&format!("{label}  {value}"), r.x + 10.0, y, 1.0, ink, Align::Left);
    }

    // Vertical velocity: thousands of feet a minute, 0 at nine o'clock.
    let (x, y, rad) = gauge_face(cv, R_VVI);
    let ang = |fpm: f64| (180.0 + clamp(fpm / 1000.0, -6.0, 6.0) * 26.0) * DEG;
    for k in -6i32..=6 {
        spoke(cv, x, y, ang(k as f64 * 1000.0), rad * if k % 2 == 0 { 0.72 } else { 0.80 }, rad * 0.90, 2.0, WHITE);
        if k % 2 == 0 {
            spoke_text(cv, x, y, ang(k as f64 * 1000.0), rad * 0.58, &format!("{}", k.abs()), WHITE);
        }
    }
    cv.text("VVI", x + rad * 0.25, y - rad * 0.3, 1.0, WHITE, Align::Center);
    spoke(cv, x, y, ang(ad.vs_fpm), -rad * 0.12, rad * 0.86, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));

    // Fuel quantity: aft-left and forward-right pointers, totalizer below.
    let (x, y, rad) = gauge_face(cv, R_AUX_DIALS[3]);
    let lb = a.fuel * 2.20462;
    let ang = |t: f64| (-225.0 + 270.0 * clamp(t, 0.0, 1.0)) * DEG;
    for k in 0..=8 {
        spoke(cv, x, y, ang(k as f64 / 8.0), rad * 0.74, rad * 0.90, 2.0, WHITE);
        if k % 2 == 0 {
            spoke_text(cv, x, y, ang(k as f64 / 8.0), rad * 0.60, &format!("{}", k / 2), WHITE);
        }
    }
    cv.rect(x - 26.0, y + rad * 0.30 - 8.0, 52.0, 16.0, rgba(0, 0, 0, 1.0));
    cv.text(&pad(lb, 4), x, y + rad * 0.30, 1.0, WHITE, Align::Center);
    cv.text("FUEL", x, y - rad * 0.28, 1.0, WHITE, Align::Center);
    spoke(cv, x, y, ang(lb * 0.45 / 4000.0), -rad * 0.1, rad * 0.84, 3.0, rgba(0xf0, 0x60, 0x50, 1.0));
    spoke(cv, x, y, ang(lb * 0.55 / 4000.0), -rad * 0.1, rad * 0.78, 3.0, POINTER);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));

    // Threat warning prime: LAUNCH lights with a missile in the air.
    let r = R_TWP;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x20, 0x27, 0x29, 1.0));
    let launch = c.warn > 0.0 || c.inbound().next().is_some();
    for (i, (s, lit)) in [("HANDOFF", false), ("LAUNCH", launch), ("PRI MODE", true), ("UNKNOWN", false)].iter().enumerate() {
        let (x, y) = (r.x + (i % 2) as f32 * 128.0, r.y + (i / 2) as f32 * 48.0);
        let on = if *lit { if i == 1 { rgba(0xff, 0x55, 0x40, 1.0) } else { GREEN } } else { rgba(0x0c, 0x14, 0x14, 1.0) };
        cv.rect(x + 3.0, y + 3.0, 122.0, 42.0, on);
        cv.text(s, x + 64.0, y + 24.0, 1.0, if *lit { rgba(0x10, 0x10, 0x08, 1.0) } else { rgba(0x6b, 0x75, 0x69, 1.0) }, Align::Center);
    }

    // Fuel flow counter, pounds an hour.
    let r = R_FFLOW;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x08, 0x0a, 0x0a, 1.0));
    cv.text("FUEL FLOW", r.x + r.w / 2.0, r.y + 12.0, 1.0, WHITE, Align::Center);
    cv.text(&format!("{} PPH", pad(a.fuel_flow * 7936.6, 5)), r.x + r.w / 2.0, r.y + 40.0, 2.0, WHITE, Align::Center);

    // Right auxiliary console dials.
    let hyd = if a.rpm > 55.0 { 3000.0 } else { a.rpm * 50.0 };
    dial(cv, R_AUX_DIALS[0], hyd, 0.0, 4000.0, "HYD A", &pad(hyd, 4), 9);
    dial(cv, R_AUX_DIALS[1], hyd * 0.99, 0.0, 4000.0, "HYD B", &pad(hyd * 0.99, 4), 9);
    dial(cv, R_AUX_DIALS[2], 100.0, 0.0, 100.0, "EPU %", "100", 11);
    // Cabin altitude: 8,000 ft until the 5 psi schedule takes over near 23,000.
    let cabin = if ad.pressure_alt_ft < 23000.0 { ad.pressure_alt_ft.min(8000.0) } else { 8000.0 + (ad.pressure_alt_ft - 23000.0) * 0.45 };
    dial(cv, R_AUX_DIALS[4], cabin, 0.0, 50000.0, "CABIN", &pad(cabin / 1000.0, 2), 9);
    let (x, y, rad) = gauge_face(cv, R_AUX_DIALS[5]);
    for k in 0..12 {
        spoke(cv, x, y, (k as f64 * 30.0 - 90.0) * DEG, rad * 0.74, rad * 0.90, 2.0, WHITE);
    }
    let t = 7.0 * 3600.0 + 15.0 * 60.0 + m.elapsed;
    spoke(cv, x, y, ((t / 43200.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.5, 3.0, POINTER);
    spoke(cv, x, y, ((t / 3600.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.8, 2.0, POINTER);
    spoke(cv, x, y, ((t / 60.0).fract() * 360.0 - 90.0) * DEG, 0.0, rad * 0.85, 1.0, RED);
    cv.disc(x, y, 4.0, rgba(0x9d, 0xa7, 0x9b, 1.0));

    // Side and auxiliary console lightplates, with their lamps.
    let gear_down = a.gear >= 0.999;
    let lamps = consoles::Lamps {
        // Then the caution panel: FLCS FAULT, ENGINE FAULT, FWD and AFT FUEL
        // LOW, ELEC SYS, ANTI SKID, CABIN PRESS, SEAT NOT ARMED.
        on: [gear_down, gear_down, gear_down, a.flaps > 0.5, a.wheel_brake > 0.1, a.battle.controls > 0.2, a.battle.engine > 0.3, a.fuel < 450.0, a.fuel < 450.0 || a.battle.leak > 0.0, false, a.gear_broken.iter().any(|&b| b), false, false],
    };
    consoles::draw(cv, &lamps);
    if a.kind == crate::sim::Type::Mig29 {
        mig::draw(cv, a, &ad, m, c);
    }
}

/// Analogue heading indicator in the centre stack of the reference cockpit.
fn hsi(cv: &mut Canvas, r: R, a: &Aircraft, m: &Mission) {
    let (x, y, rad) = gauge_face(cv, r);
    for heading in (0..360).step_by(5) {
        let angle = (heading as f64 - a.heading - 90.0) * DEG;
        let major = heading % 30 == 0;
        spoke(cv, x, y, angle, rad * if major { 0.77 } else { 0.85 }, rad * 0.94, if major { 3.0 } else { 1.5 }, WHITE);
        if major {
            let label = match heading { 0 => "N".to_string(), 90 => "E".to_string(), 180 => "S".to_string(), 270 => "W".to_string(), _ => (heading / 10).to_string() };
            spoke_text(cv, x, y, angle, rad * 0.64, &label, WHITE);
        }
    }
    let gate = m.current_gate();
    let bearing = (gate.pos.x - a.pos.x).atan2(-(gate.pos.z - a.pos.z)) * RAD;
    let course = (bearing - a.heading - 90.0) * DEG;
    spoke(cv, x, y, course, -rad * 0.54, rad * 0.53, 3.0, WHITE);
    let (dx, dy) = (course.cos() as f32, course.sin() as f32);
    let (tx, ty) = (x + dx * rad * 0.54, y + dy * rad * 0.54);
    cv.line(tx, ty, tx - dx * 12.0 - dy * 6.0, ty - dy * 12.0 + dx * 6.0, 3.0, WHITE);
    cv.line(tx, ty, tx - dx * 12.0 + dy * 6.0, ty - dy * 12.0 - dx * 6.0, 3.0, WHITE);
    cv.line(x - 22.0, y, x + 22.0, y, 4.0, AMBER);
    cv.line(x, y - 13.0, x, y + 23.0, 4.0, AMBER);
    cv.line(x - 10.0, y + 18.0, x + 10.0, y + 18.0, 3.0, AMBER);
    cv.polyline(&[[x - 7.0, y - rad * 0.98], [x, y - rad * 0.87], [x + 7.0, y - rad * 0.98]], 3.0, false, WHITE);
}

/// Standby attitude indicator: rotating sky/ground ball with a fixed aircraft symbol.
fn adi(cv: &mut Canvas, r: R, a: &Aircraft) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let rad = r.w * 0.5 - 2.0;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x10, 0x16, 0x19, 1.0));
    let circle: Vec<[f32; 2]> = (0..40).map(|k| {
        let t = k as f32 / 40.0 * std::f32::consts::TAU;
        [cx + t.cos() * (rad - 3.0), cy + t.sin() * (rad - 3.0)]
    }).collect();
    cv.set_clip(Some(circle));
    let k = rad / 45.0;
    cv.set_transform(cx, cy, (-a.bank * DEG) as f32);
    let off = (a.pitch as f32) * k;
    cv.rect(-200.0, -400.0 + off, 400.0, 400.0, rgba(0x46, 0x6d, 0x83, 1.0));
    cv.rect(-200.0, off, 400.0, 400.0, rgba(0x86, 0x60, 0x44, 1.0));
    cv.line(-200.0, off, 200.0, off, 2.0, WHITE);
    for n in (-40..=40).step_by(10) {
        if n != 0 {
            let yy = off - n as f32 * k;
            let w = if n % 20 == 0 { 16.0 } else { 9.0 };
            cv.line(-w, yy, w, yy, 1.0, WHITE);
        }
    }
    cv.reset_transform();
    cv.set_clip(None);
    cv.circle(cx, cy, rad - 2.0, 3.0, rgba(0x2c, 0x33, 0x32, 1.0));
    let y = cy;
    cv.line(cx - 34.0, y, cx - 12.0, y, 3.0, rgba(0xe8, 0xb9, 0x5c, 1.0));
    cv.line(cx + 12.0, y, cx + 34.0, y, 3.0, rgba(0xe8, 0xb9, 0x5c, 1.0));
    cv.disc(cx, y, 3.0, rgba(0xe8, 0xb9, 0x5c, 1.0));
    // Bank scale ticks.
    for b in [-60.0f32, -30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0, 60.0] {
        let t = (b - 90.0).to_radians();
        cv.line(cx + t.cos() * (rad - 4.0), cy + t.sin() * (rad - 4.0), cx + t.cos() * (rad - 11.0), cy + t.sin() * (rad - 11.0), 2.0, WHITE);
    }
}

// ================================================================ HUD
/// Projection of body-space points for the current cockpit camera, in window pixels.
pub struct BodyProjector {
    pub eye: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    pub focal: f64,
    pub w: f64,
    pub h: f64,
}

impl BodyProjector {
    pub fn project(&self, p: V3) -> Option<[f32; 2]> {
        let d = p - self.eye;
        let z = d.dot(self.fwd);
        if z < 0.03 {
            return None;
        }
        Some([(self.w * 0.5 + d.dot(self.right) * self.focal / z) as f32, (self.h * 0.5 - d.dot(self.up) * self.focal / z) as f32])
    }
}

/// NAV HUD symbology. Every symbol is a world direction projected with the
/// same camera as the scene, so the ladder and flight-path marker stay on the
/// real horizon, and the whole display is clipped to the combiner glass.
pub fn draw_hud(cv: &mut Canvas, a: &Aircraft, m: &Mission, cam: &Camera, body: &BodyProjector, c: &Combat) {
    let glass: Vec<[f32; 2]> = HUD_GLASS.iter().filter_map(|&p| body.project(p)).collect();
    if glass.len() < 4 {
        return;
    }
    cv.set_clip(Some(glass.clone()));
    let (w, h) = (body.w, body.h);
    let focal = h * 0.5 / (cam.fov / 2.0).tan();
    let world_dir = |d: V3| -> Option<[f32; 2]> {
        let z = d.dot(cam.forward);
        if z <= 0.01 {
            return None;
        }
        Some([(w * 0.5 + d.dot(cam.right) * focal / z) as f32, (h * 0.5 - d.dot(cam.up) * focal / z) as f32])
    };
    let ax = a.q.axes();
    // HUD layout units are 1/650 rad, measured from the aircraft boresight.
    let unit = 1.0 / 650.0;
    let at = |x: f64, y: f64| world_dir((ax.forward + ax.right * (x * unit) - ax.up * (y * unit)).norm());
    let big = ((h / 540.0).round() as f32).max(1.0);
    let small = (big - 1.0).max(1.0);
    let col = HUD;
    let ht = |cv: &mut Canvas, s: &str, x: f64, y: f64, scale: f32, align: Align| {
        if let Some(p) = at(x, y) {
            cv.text(s, p[0], p[1], scale, col, align);
        }
    };
    let lw = (big * 0.6).max(1.0);
    let hl = |cv: &mut Canvas, x: f64, y: f64, x2: f64, y2: f64| {
        if let (Some(p), Some(q)) = (at(x, y), at(x2, y2)) {
            cv.line(p[0], p[1], q[0], q[1], lw, col);
        }
    };
    // Speed and altitude figures, a step smaller than the big lettering.
    let mid = (big * 0.75).max(1.0);
    // Boxed readout: text anchored at a HUD point, frame drawn in pixels around it.
    let boxed = |cv: &mut Canvas, s: &str, x: f64, y: f64, align: Align| {
        if let Some(p) = at(x, y) {
            let tw = Canvas::text_width(s, mid);
            let x0 = match align {
                Align::Left => p[0],
                Align::Center => p[0] - tw * 0.5,
                Align::Right => p[0] - tw,
            };
            cv.text(s, p[0], p[1], mid, col, align);
            let pad = 2.0 * mid;
            cv.stroke_rect(x0 - pad - 1.0, p[1] - 3.5 * mid - pad, tw + 2.0 * pad + 2.0, 7.0 * mid + 2.0 * pad, lw, col);
        }
    };
    if a.kind == crate::sim::Type::Mig29 {
        ils31(cv, a, c, cam, &world_dir, &at, big, small, lw);
        cv.set_clip(None);
        return;
    }
    let half = 70.0;
    let ad = AirData::of(a);
    let alt_ft = ad.baro_ft;
    // Left: load factor, the KCAS box, then Mach, peak G and the master
    // mode; AOA has no readout, only the bracket with the gear down. Right: the barometric altitude box (altimeter on QNH),
    // radar altitude, vertical velocity and steering to the next gate.
    ht(cv, &format!("G {:.1}", ad.g), -half, -62.0, small, Align::Left);
    boxed(cv, &pad(ad.kcas, 3), -half, 0.0, Align::Left);
    ht(cv, "C", -half, 13.0, small, Align::Left);
    boxed(cv, &hud::altitude(alt_ft), half, 0.0, Align::Right);
    ht(cv, "B", half, 13.0, small, Align::Right);
    ht(cv, &format!("M {:.2}", ad.mach), -half, 34.0, small, Align::Left);
    ht(cv, &format!("GMAX {:.1}", ad.g_max), -half, 46.0, small, Align::Left);
    ht(cv, &if c.mode == crate::combat::Mode::Nav { "NAV".to_string() } else { format!("{} ARM", c.mode.label()) }, -half, 58.0, small, Align::Left);
    // Radar altitude is a separately boxed repeater, resolved to ten feet.
    if ad.radar_ft <= 5000.0 {
        if let Some(p) = at(half, 34.0) {
            let text = format!("R {}", hud::altitude(ad.radar_ft.max(0.0)));
            let tw = Canvas::text_width(&text, small);
            cv.stroke_rect(p[0] - tw - 2.0 * small, p[1] - 5.0 * small, tw + 4.0 * small, 10.0 * small, lw, col);
            cv.text(&text, p[0], p[1], small, col, Align::Right);
        }
    }
    ht(cv, &format!("VV {:+}", (ad.vs_fpm / 10.0).round() as i64 * 10), half, 46.0, small, Align::Right);
    let gate = m.current_gate();
    let (dist, ttg) = steering(a, m);
    ht(cv, &format!("{:.1}>{:02}", dist / 1852.0, (m.gate_index + 1).min(m.gates.len())), half, 58.0, small, Align::Right);
    ht(cv, &clock(ttg), half, 70.0, small, Align::Right);
    // Heading scale and steerpoint bearing caret, clamped at the edge.
    let landing = a.gear > 0.5;
    let tape_y = if landing { -37.0 } else { 100.0 };
    let base = (a.heading / 5.0).round() * 5.0;
    for k in -5..=5 {
        let n = wrap(base + k as f64 * 5.0, 360.0);
        let diff = wrap180(n - a.heading);
        if diff.abs() > 13.0 {
            continue;
        }
        let major = (n.round() as i64) % 10 == 0;
        hl(cv, diff * 3.0, tape_y, diff * 3.0, tape_y + if major { 6.0 } else { 3.0 });
        if major {
            ht(cv, &pad((n / 10.0).round() % 36.0, 2), diff * 3.0, tape_y - 7.0, small, Align::Center);
        }
    }
    hl(cv, 0.0, tape_y + 8.0, -2.5, tape_y + 12.0);
    hl(cv, 0.0, tape_y + 8.0, 2.5, tape_y + 12.0);
    let bearing = (gate.pos.x - a.pos.x).atan2(-(gate.pos.z - a.pos.z)) * RAD;
    let steer_x = clamp(wrap180(bearing - a.heading) * 3.0, -39.0, 39.0);
    hl(cv, steer_x - 3.0, tape_y - 15.0, steer_x, tape_y - 11.0);
    hl(cv, steer_x, tape_y - 11.0, steer_x + 3.0, tape_y - 15.0);
    ht(cv, &pad(a.heading, 3), 0.0, tape_y + 19.0, small, Align::Center);
    // Speed and altitude scales flank the flight path.
    for i in -6..=6 {
        let speed = (ad.kcas / 10.0).round() * 10.0 + i as f64 * 10.0;
        let dy = (ad.kcas - speed) * 0.9;
        let major = speed.round() as i64 % 50 == 0;
        if (-51.0..28.0).contains(&dy) && dy.abs() > 9.0 && speed >= 0.0 {
            hl(cv, -half - 6.0, dy, -half - if major { 0.0 } else { 3.0 }, dy);
            if major && (dy - 13.0).abs() > 7.0 {
                ht(cv, &format!("{}", speed.round() as i64), -half + 2.0, dy, small, Align::Left);
            }
        }
        let alt = (alt_ft / 100.0).round() * 100.0 + i as f64 * 100.0;
        let ay = (alt_ft - alt) * 0.09;
        let major = alt.round() as i64 % 500 == 0;
        if (-51.0..28.0).contains(&ay) && ay.abs() > 9.0 {
            hl(cv, half + 6.0, ay, half + if major { 0.0 } else { 3.0 }, ay);
            if major && (ay - 13.0).abs() > 7.0 {
                ht(cv, &format!("{}", (alt / 100.0).round() as i64), half - 2.0, ay, small, Align::Right);
            }
        }
    }
    // Current-value index notches alongside the digital readouts.
    for side in [-1.0, 1.0] {
        hl(cv, side * 76.0, -3.0, side * 79.0, 0.0);
        hl(cv, side * 79.0, 0.0, side * 76.0, 3.0);
    }
    // Gun cross at the top of the field.
    hl(cv, -8.0, -55.0, -3.0, -55.0);
    hl(cv, 3.0, -55.0, 8.0, -55.0);
    hl(cv, 0.0, -60.0, 0.0, -50.0);

    // Keep moving attitude bars out of the fixed data columns.
    let ladder_clip: Vec<_> = [(-49.0, -79.0), (49.0, -79.0), (49.0, 134.0), (-49.0, 134.0)]
        .into_iter().filter_map(|(x, y)| at(x, y)).collect();
    if ladder_clip.len() == 4 {
        cv.set_clip(Some(ladder_clip));
    }
    // Inertial ladder follows lateral drift and declutters above 60 degrees.
    let hd = hud::ladder_heading(a.vel, a.heading);
    for pitch in (-85i32..=85).step_by(5) {
        if !hud::ladder_pitch(pitch) {
            continue;
        }
        let e = pitch as f64 * DEG;
        let dir = |az: f64| {
            let hh = hd + az * DEG;
            v3(hh.sin() * e.cos(), e.sin(), -hh.cos() * e.cos())
        };
        if world_dir(dir(0.0)).is_none() {
            continue;
        }
        for &s in &[-1.0, 1.0] {
            let (i0, i1) = if pitch == 0 { (1.5, 4.0) } else { (1.0, 3.0) };
            if let (Some(p), Some(q)) = (world_dir(dir(s * i0)), world_dir(dir(s * i1))) {
                if pitch < 0 {
                    cv.dashed(p[0], p[1], q[0], q[1], lw, 4.0 * big, 3.0 * big, col);
                } else {
                    cv.line(p[0], p[1], q[0], q[1], lw, col);
                }
                // End ticks point toward the horizon.
                if pitch != 0 {
                    let up_dir = |az: f64| {
                        let hh = hd + az * DEG;
                        let e2 = e - (pitch.signum() as f64) * 0.8 * DEG;
                        v3(hh.sin() * e2.cos(), e2.sin(), -hh.cos() * e2.cos())
                    };
                    if let Some(t) = world_dir(up_dir(s * i1)) {
                        cv.line(q[0], q[1], t[0], t[1], lw, col);
                    }
                }
            }
        }
        if pitch != 0 && (pitch as f64 - a.pitch).abs() < 18.0 {
            for side in [-1.0, 1.0] {
                if let Some(q) = world_dir(dir(side * 3.5)) {
                    cv.text(&format!("{}", pitch.abs()), q[0], q[1], small, col, if side < 0.0 { Align::Right } else { Align::Left });
                }
            }
        }
    }
    if landing {
        // Extra -2.5 degree descent reference, inertial like the ladder.
        let e = -2.5 * DEG;
        for side in [-1.0, 1.0] {
            let dir = |az: f64| v3((hd + az * DEG).sin() * e.cos(), e.sin(), -(hd + az * DEG).cos() * e.cos());
            if let (Some(p), Some(q)) = (world_dir(dir(side * 1.0)), world_dir(dir(side * 3.0))) {
                cv.dashed(p[0], p[1], q[0], q[1], lw, 4.0 * big, 3.0 * big, col);
            }
        }
    }
    cv.set_clip(Some(glass));
    // An X identifies a flight-path marker limited by the combiner field.
    let velocity_body = v3(a.vel.dot(ax.right), a.vel.dot(ax.up), -a.vel.dot(ax.forward));
    if let Some((fx, fy, limited)) = hud::flight_path(velocity_body) {
        if let Some(f) = at(fx, fy) {
            let s = big * 2.6;
            cv.circle(f[0], f[1], s, lw, col);
            cv.line(f[0] - s * 2.4, f[1], f[0] - s, f[1], lw, col);
            cv.line(f[0] + s, f[1], f[0] + s * 2.4, f[1], lw, col);
            cv.line(f[0], f[1] - s, f[0], f[1] - s * 2.0, lw, col);
            if limited {
                cv.line(f[0] - s, f[1] - s, f[0] + s, f[1] + s, lw, col);
                cv.line(f[0] - s, f[1] + s, f[0] + s, f[1] - s, lw, col);
            }
            if landing {
                let (top, mid, bottom) = hud::aoa_bracket(a.alpha_deg, fy);
                let x = fx - 15.0;
                hl(cv, x, top, x, bottom);
                hl(cv, x, top, x + 5.0, top);
                hl(cv, x, mid, x + 3.0, mid);
                hl(cv, x, bottom, x + 5.0, bottom);
            }
        }
    }
    // Steerpoint diamond on the next gate.
    if let Some(t) = world_dir((gate.pos - cam.eye).norm()) {
        let s = big * 3.2;
        cv.polyline(&[[t[0], t[1] - s], [t[0] + s, t[1]], [t[0], t[1] + s], [t[0] - s, t[1]]], lw, true, col);
    }
    let warning = if a.fuel <= 0.0 {
        Some("ENGINE")
    } else if a.agl < 150.0 && a.vs < -10.0 {
        Some("PULL UP")
    } else if a.alpha_deg > 25.0 {
        Some("AOA")
    } else if a.g_smooth > 9.15 {
        Some("G LIMIT")
    } else if ad.overspeed() {
        Some("OVERSPEED")
    } else if a.fuel < 450.0 {
        Some("BINGO")
    } else {
        None
    };
    if let Some(wn) = warning {
        ht(cv, wn, 0.0, 22.0, big, Align::Center);
    }
    let pen = symbology::Pen { dir: &world_dir, at: &at, eye: cam.eye, big, small, lw, col, russian: false };
    symbology::hud(cv, &pen, a, c, c.clock);
    cv.set_clip(None);
}


/// The MiG-29's ILS-31 head-up display: one yellow-green colour, Russian
/// lettering and metric units. Speed in km/h top left with the Mach number
/// under it, barometric altitude in metres top right with the radio
/// altitude (Р) under 1,500 m and the climb rate in m/s; heading in tens
/// of degrees along the top and the bank scale along the bottom; a fixed
/// aircraft symbol on the boresight against a pitch ladder every 10
/// degrees, the velocity vector only for landing; AoA (α, a scale for
/// landing) and load factor (n) lower left; the mode bottom left (МАРШ route, ПОС landing, БВБ close and ДВБ
/// long-range air combat, ЗЕМЛЯ ground attack) and the sensor bottom right
/// (РЛ radar, ТП infrared); warnings in the middle, as the voice warner
/// calls them.
#[allow(clippy::too_many_arguments)]
fn ils31(cv: &mut Canvas, a: &Aircraft, c: &Combat, cam: &Camera, world_dir: &dyn Fn(V3) -> Option<[f32; 2]>, at: &dyn Fn(f64, f64) -> Option<[f32; 2]>, big: f32, small: f32, lw: f32) {
    use crate::combat::Mode;
    let col = ILS_GREEN;
    let ht = |cv: &mut Canvas, s: &str, x: f64, y: f64, scale: f32, align: Align| {
        if let Some(p) = at(x, y) {
            cv.text(s, p[0], p[1], scale, col, align);
        }
    };
    let hl = |cv: &mut Canvas, x: f64, y: f64, x2: f64, y2: f64| {
        if let (Some(p), Some(q)) = (at(x, y), at(x2, y2)) {
            cv.line(p[0], p[1], q[0], q[1], lw, col);
        }
    };
    let half = 70.0;
    let ad = AirData::of(a);
    let radio_m = ad.radar_ft * 0.3048;
    ht(cv, &format!("{:.0}", (ad.kcas * 1.852 / 5.0).round() * 5.0), -half, -40.0, (big * 0.75).max(1.0), Align::Left);
    ht(cv, &format!("М{:.2}", ad.mach), -half, -28.0, small, Align::Left);
    ht(cv, &format!("{:.0}", (ad.baro_ft * 0.3048 / 10.0).round() * 10.0), half, -40.0, (big * 0.75).max(1.0), Align::Right);
    if radio_m < 1500.0 {
        ht(cv, &format!("{:.0}Р", radio_m), half, -28.0, small, Align::Right);
    }
    ht(cv, &format!("{:+.0}", ad.vs_fpm * 0.00508), half, -16.0, small, Align::Right);
    let landing = a.gear > 0.5;
    if landing {
        // Landing: a vertical α scale, 0 to 25 degrees with a tick every 5,
        // and a pointer at the present angle.
        let (sx, y0) = (-62.0, 30.0);
        let y = |deg: f64| y0 - deg * 1.6;
        hl(cv, sx, y(0.0), sx, y(25.0));
        for d in (0..=25).step_by(5) {
            hl(cv, sx - if d % 10 == 0 { 4.0 } else { 2.5 }, y(d as f64), sx, y(d as f64));
        }
        let p = y(clamp(a.alpha_deg, 0.0, 25.0));
        hl(cv, sx + 1.0, p, sx + 4.0, p - 2.0);
        hl(cv, sx + 1.0, p, sx + 4.0, p + 2.0);
        hl(cv, sx + 4.0, p - 2.0, sx + 4.0, p + 2.0);
        ht(cv, "α", sx, y(25.0) - 6.0, small, Align::Center);
    } else {
        ht(cv, &format!("α{:.0}", a.alpha_deg), -half, 46.0, small, Align::Left);
    }
    ht(cv, &format!("ⁿ{:.1}", ad.g), -half, 58.0, small, Align::Left);
    let mode = match c.mode {
        Mode::Nav if landing => "ПОС",
        Mode::Nav => "МАРШ",
        Mode::Gun | Mode::Srm => "БВБ",
        Mode::Mrm => "ДВБ",
        Mode::Ccip | Mode::Strf => "ЗЕМЛЯ",
    };
    ht(cv, mode, -half, 70.0, small, Align::Left);
    if c.mode != Mode::Nav {
        ht(cv, if c.irst { "ТП" } else { "РЛ" }, half, 70.0, small, Align::Right);
    }

    // Heading scale: ticks every 5 degrees, tens of degrees over them.
    let base = (a.heading / 5.0).round() * 5.0;
    for k in -4..=4 {
        let n = wrap(base + k as f64 * 5.0, 360.0);
        let diff = wrap180(n - a.heading);
        if diff.abs() > 11.0 {
            continue;
        }
        let x = diff * 4.0;
        let major = (n.round() as i64) % 10 == 0;
        hl(cv, x, -74.0, x, if major { -78.0 } else { -76.0 });
        if major {
            ht(cv, &pad((n / 10.0).round() % 36.0, 2), x, -84.0, small, Align::Center);
        }
    }
    hl(cv, 0.0, -72.0, -2.0, -69.0);
    hl(cv, 0.0, -72.0, 2.0, -69.0);

    // Bank scale: an arc of ticks under the centre, the pointer at the bank.
    let (bx, by, br) = (0.0, 52.0, 26.0);
    for b in [-60.0f64, -45.0, -30.0, -15.0, 0.0, 15.0, 30.0, 45.0, 60.0] {
        let t = b * DEG;
        let r2 = if b == 0.0 { br + 5.0 } else { br + 3.0 };
        hl(cv, bx + t.sin() * br, by + t.cos() * br, bx + t.sin() * r2, by + t.cos() * r2);
    }
    let t = clamp(-a.bank, -65.0, 65.0) * DEG;
    let tip = (bx + t.sin() * (br - 1.0), by + t.cos() * (br - 1.0));
    let side = (t.cos() * 2.0, -t.sin() * 2.0);
    let foot = (bx + t.sin() * (br - 5.0), by + t.cos() * (br - 5.0));
    hl(cv, tip.0, tip.1, foot.0 + side.0, foot.1 + side.1);
    hl(cv, tip.0, tip.1, foot.0 - side.0, foot.1 - side.1);

    // Pitch ladder every 10 degrees: the horizon long and broken in the
    // middle, the climbing bars solid and the diving ones dashed, numbered.
    let hd = a.heading * DEG;
    for pitch in (-80i32..=80).step_by(10) {
        if (pitch as f64 - a.pitch).abs() > 25.0 {
            continue;
        }
        let e = pitch as f64 * DEG;
        let dir = |az: f64| {
            let hh = hd + az * DEG;
            v3(hh.sin() * e.cos(), e.sin(), -hh.cos() * e.cos())
        };
        let (i0, i1) = if pitch == 0 { (3.0, 14.0) } else { (3.0, 8.0) };
        for s in [-1.0, 1.0] {
            if let (Some(p), Some(q)) = (world_dir(dir(s * i0)), world_dir(dir(s * i1))) {
                if pitch < 0 {
                    cv.dashed(p[0], p[1], q[0], q[1], lw, 4.0 * big, 3.0 * big, col);
                } else {
                    cv.line(p[0], p[1], q[0], q[1], lw, col);
                }
                if pitch != 0 {
                    cv.text(&format!("{}", pitch.abs()), q[0] + s as f32 * 3.0 * big, q[1], small, col, if s > 0.0 { Align::Left } else { Align::Right });
                }
            }
        }
    }

    // Fixed aircraft symbol on the boresight: wings, a fin and a centre ring.
    if let Some(p) = at(0.0, 0.0) {
        let s = big * 2.2;
        cv.circle(p[0], p[1], s, lw, col);
        cv.line(p[0] - s * 4.0, p[1], p[0] - s, p[1], lw, col);
        cv.line(p[0] + s, p[1], p[0] + s * 4.0, p[1], lw, col);
        cv.line(p[0], p[1] - s, p[0], p[1] - s * 2.2, lw, col);
    }
    // Landing: the velocity vector, a ring with stubs.
    if landing {
        if let Some(f) = world_dir(a.vel.norm()) {
            let s = big * 1.6;
            cv.circle(f[0], f[1], s, lw, col);
            cv.line(f[0] - s * 2.2, f[1], f[0] - s, f[1], lw, col);
            cv.line(f[0] + s, f[1], f[0] + s * 2.2, f[1], lw, col);
        }
    }

    let warning = if a.fuel <= 0.0 {
        Some("ДВИГАТЕЛЬ")
    } else if a.agl < 150.0 && a.vs < -10.0 {
        Some("ОПАСНАЯ ВЫСОТА")
    } else if a.alpha_deg > 25.0 {
        Some("КРИТИЧЕСКИЙ УГОЛ")
    } else if a.g_smooth > a.kind.g_limit(a.mass, a.mach) + 0.15 {
        Some("ПЕРЕГРУЗКА")
    } else if ad.overspeed() {
        Some("СКОРОСТЬ")
    } else if a.fuel < 450.0 {
        Some("ОСТАТОК ТОПЛИВА")
    } else {
        None
    };
    if let Some(wn) = warning {
        ht(cv, wn, 0.0, 22.0, small, Align::Center);
    }
    let pen = symbology::Pen { dir: world_dir, at, eye: cam.eye, big, small, lw, col, russian: true };
    symbology::hud(cv, &pen, a, c, c.clock);
}

/// Camera in body axes, for the cockpit pass and the combiner projection.
/// The pilot's design eye in the jet's body axes.
pub fn design_eye(a: &Aircraft) -> V3 {
    match a.kind {
        crate::sim::Type::Mig29 => crate::mig29::EYE,
        crate::sim::Type::F16 => EYE,
    }
}

/// The camera in cockpit space: body axes, shifted so the design eye sits
/// at airframe::EYE, where both cockpits are built.
pub fn body_camera(cam: &Camera, a: &Aircraft) -> Camera {
    let inv = a.q.conj();
    Camera { eye: inv.rotate(cam.eye - a.pos) - design_eye(a) + EYE, forward: inv.rotate(cam.forward), right: inv.rotate(cam.right), up: inv.rotate(cam.up), fov: cam.fov }
}

pub fn eye_world(a: &Aircraft) -> V3 {
    a.pos + a.q.rotate(design_eye(a))
}

/// The rear-view mirrors' camera: from just over the design eye, back along
/// the body and a little up, over the spine and the tail. Its right is the
/// body's left; the mirror glass flips the picture back.
pub fn mirror_camera(a: &Aircraft) -> Camera {
    let ax = a.q.axes();
    let f = (ax.back * (8.0 * DEG).cos() + ax.up * (8.0 * DEG).sin()).norm();
    let r = f.cross(ax.up).norm();
    Camera { eye: eye_world(a) + ax.up * 0.12, forward: f, right: r, up: r.cross(f).norm(), fov: MIRROR_FOV }
}

/// A rear-view mirror `w` by `h` laid out by `place(x, y, depth)`: x along
/// its width to the pilot's right, y up its face, depth toward the pilot.
/// A rounded outline, the glass bulging forward like a car's wide-angle
/// mirror and showing the whole rear picture left for right as a mirror
/// does, in a shallow housing whose back closes in to a dome.
fn curved_mirror(m: &mut Mesh, w: f64, h: f64, place: &dyn Fn(f64, f64, f64) -> V3, housing: Rgb, rear_offset: f64) {
    let corners = rounded_rect(w, h, h * 0.25, 4);
    // Subdivide the long edges as well as the rounded corners: the panes
    // follow the canopy arch, so a single chord would flatten their rims.
    let mut outline = Vec::new();
    for k in 0..corners.len() {
        let (a, b) = (corners[k], corners[(k + 1) % corners.len()]);
        let steps = (((b.0 - a.0).hypot(b.1 - a.1) / 0.015).ceil() as usize).max(1);
        for j in 0..steps {
            let t = j as f64 / steps as f64;
            outline.push((mix(a.0, b.0, t), mix(a.1, b.1, t)));
        }
    }
    let n = outline.len();
    // Rim: a lip round the glass, then the back closing in to a dome.
    let rim = 0.007;
    let ring = |s: f64, grow: f64, depth: f64| -> Vec<V3> { outline.iter().map(|&(x, y)| place(x * s + x.signum() * grow, y * s + y.signum() * grow, depth)).collect() };
    let rings = [ring(1.0, 0.0, 0.004), ring(1.0, rim, 0.004), ring(1.0, rim, -0.008), ring(0.94, rim, -0.018), ring(0.7, 0.0, -0.024), ring(0.4, 0.0, -0.026)];
    for (j, pair) in rings.windows(2).enumerate() {
        let col = if j == 0 { housing.scale(1.15) } else { housing };
        for k in 0..n {
            let k2 = (k + 1) % n;
            m.quad(pair[0][k2], pair[0][k], pair[1][k], pair[1][k2], col);
        }
    }
    let back: Vec<V3> = rings[5].iter().rev().copied().collect();
    m.polygon(&back, housing);
    // Map the reflection in the pilot's view so the shoulder mirrors do
    // not rotate the reflected horizon with their diagonal housings.
    let project = |p: V3| {
        let p = p - EYE;
        (p.x / -p.z, p.y / -p.z)
    };
    let mut bounds = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in &outline {
        let (u, v) = project(place(x, y, 0.004));
        bounds = (bounds.0.min(u), bounds.1.min(v), bounds.2.max(u), bounds.3.max(v));
    }
    let rm = R_MIRROR;
    let scale = (rm.w as f64 / (bounds.2 - bounds.0)).min(rm.h as f64 / (bounds.3 - bounds.1));
    let centre = ((bounds.0 + bounds.2) * 0.5, (bounds.1 + bounds.3) * 0.5);
    // The shoulder panes sample opposite sides of the rear view; the
    // crown keeps the central panorama. All three preserve its aspect.
    let sample_half_width = (bounds.2 - bounds.0) * scale * 0.5;
    let tex_centre_x = rm.x as f64 + rm.w as f64 * 0.5 + rear_offset * (rm.w as f64 * 0.5 - sample_half_width);
    // A slight convex face in a shallow dark housing.
    let bulge = h.min(w) * 0.02;
    let e = 0.002;
    let at = |k: usize, t: f64| {
        let (x, y) = (outline[k % n].0 * t, outline[k % n].1 * t);
        let z = |x: f64, y: f64| 0.004 + bulge * (1.0 - (2.0 * x / w).powi(2).max((2.0 * y / h).powi(2)));
        let p = place(x, y, z(x, y));
        let normal = (place(x + e, y, z(x + e, y)) - place(x - e, y, z(x - e, y))).cross(place(x, y + e, z(x, y + e)) - place(x, y - e, z(x, y - e))).norm();
        let (u, v) = project(p);
        let tex_x = tex_centre_x - (u - centre.0) * scale;
        let tex_y = rm.y as f64 + rm.h as f64 * 0.5 - (v - centre.1) * scale;
        (p, normal, (clamp(tex_x, rm.x as f64 + 0.5, (rm.x + rm.w) as f64 - 0.5) / AW, 1.0 - clamp(tex_y, rm.y as f64 + 0.5, (rm.y + rm.h) as f64 - 0.5) / A))
    };
    let white = Rgb(1.0, 1.0, 1.0);
    let steps = [0.0, 0.45, 0.8, 1.0];
    for s in steps.windows(2) {
        for k in 0..n {
            let quad = [at(k, s[0]), at(k + 1, s[0]), at(k + 1, s[1]), at(k, s[1])];
            for i in [0, 2, 1, 0, 3, 2] {
                m.vert(quad[i].0, quad[i].1, white, quad[i].2);
            }
        }
    }
}
