//! Modernised F-16 Viper cockpit: flat-shaded, low-polygon 3D geometry in body metres,
//! live instrument pages rendered into a texture atlas, and the collimated
//! NAV HUD clipped to the physical combiner glass.
use crate::air::{self, AirData};
use crate::airframe::{self, EYE};
use crate::canvas::{rgba, Align, Canvas, Col};
use crate::gfx;
use crate::math::*;
use crate::mesh::{rounded_rect, Frame, Mesh, Rgb};
use crate::sim::{Aircraft, Mission};
use crate::world::{self, Camera, Part, MESH_ATTRS};
use miniquad::*;

mod pilot;

pub const ATLAS: u32 = 1024;
const A: f64 = ATLAS as f64;

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
    (r.x as f64 / A, 1.0 - r.y as f64 / A, (r.x + r.w) as f64 / A, 1.0 - (r.y + r.h) as f64 / A)
}

const R_MFD_L: R = r(0.0, 0.0, 256.0, 320.0);
const R_MFD_R: R = r(256.0, 0.0, 256.0, 320.0);
const R_CPD: R = r(512.0, 0.0, 240.0, 320.0);
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
const R_GMETER: R = r(896.0, 320.0, 128.0, 128.0);
const R_CAUTION: R = r(0.0, 448.0, 256.0, 128.0);
const R_ARM: R = r(256.0, 448.0, 256.0, 128.0);
const R_AOA: R = r(512.0, 448.0, 48.0, 128.0);
const R_KNEE: R = r(560.0, 448.0, 192.0, 240.0);
fn r_console(i: usize) -> R {
    r((i % 8) as f32 * 128.0, 704.0 + (i / 8) as f32 * 160.0, 128.0, 160.0)
}

const GREEN: Col = rgba(0x98, 0xff, 0xa3, 1.0);
const WHITE: Col = rgba(0xdd, 0xdc, 0xcf, 1.0);
const DIM: Col = rgba(0xa2, 0xb8, 0xb6, 1.0);
const AMBER: Col = rgba(0xe6, 0xbc, 0x7a, 1.0);
const RED: Col = rgba(0xff, 0x66, 0x54, 1.0);
pub const HUD: Col = rgba(0xa9, 0xff, 0xb2, 1.0);

// Materials.
const PANEL: Rgb = Rgb(0.165, 0.185, 0.195);
const BEZEL: Rgb = Rgb(0.215, 0.235, 0.245);
const GLARE: Rgb = Rgb(0.075, 0.085, 0.095);
const WALL: Rgb = Rgb(0.19, 0.21, 0.215);
const CONSOLE: Rgb = Rgb(0.15, 0.17, 0.18);
const SEAT: Rgb = Rgb(0.17, 0.20, 0.17);
const METALC: Rgb = Rgb(0.55, 0.57, 0.56);

/// Round gauges are 16-sided, like the polygon cockpits of early-90s sims.
const GAUGE_SIDES: usize = 16;

// Main instrument panel: tilted 8.5 degrees, its base 0.4 m above the datum.
const TILT: f64 = 8.5 * DEG;
fn panel() -> Frame {
    Frame { o: v3(0.0, 0.40, -4.12), x: V3::X, y: v3(0.0, TILT.cos(), -TILT.sin()), z: v3(0.0, TILT.sin(), TILT.cos()) }
}
fn pv(y: f64) -> f64 {
    (y - 0.40) / TILT.cos()
}
fn panel_at(u: f64, y: f64, w: f64) -> Frame {
    panel().at(u, pv(y), w)
}
const OUTLINE: [(f64, f64); 12] = [
    (-0.47, 0.0), (0.47, 0.0), (0.47, 0.36), (0.45, 0.44), (0.40, 0.52), (0.34, 0.57),
    (0.28, 0.60), (-0.28, 0.60), (-0.34, 0.57), (-0.40, 0.52), (-0.45, 0.44), (-0.47, 0.36),
];

/// Combiner glass corners in body coordinates: bottom-left, bottom-right, top-right, top-left.
pub const HUD_GLASS: [V3; 4] = [v3(-0.10, 1.078, -4.012), v3(0.10, 1.078, -4.012), v3(0.10, 1.35, -3.958), v3(-0.10, 1.35, -3.958)];

#[derive(Clone, Copy, Default)]
pub struct Hands {
    pub roll: f64,
    pub pitch: f64,
    pub throttle: f64,
    pub rudder: f64,
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
    // The same grey-out ring the post pass lays over the rest of the view.
    vec2 g = (gl_FragCoord.xy / u_res - vec2(0.5, 0.58)) * vec2(u_res.x / u_res.y, 1.0);
    float ring = smoothstep(0.16, 0.75, length(g));
    frag = vec4(mix(col, vec3(0.20, 0.0, 0.0), u_gloc * ring * 0.75), 1.0);
}"#;

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
    canopy: Part,
    hud_glass: Part,
    pub atlas: gfx::Target,
    atlas_pass: RenderPass,
    pub atlas_clock: f64,
}

impl Cockpit {
    pub fn new(ctx: &mut dyn RenderingBackend, canopy: &Mesh) -> Cockpit {
        use UniformType::*;
        let s = gfx::shader(ctx, VS, FS, &["u_atlas"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_sun", Float3), ("u_sun_vis", Float1), ("u_alpha", Float1), ("u_tint", Float3)]);
        let pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() });
        let glass_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_blend: Some(gfx::alpha_blend()), ..Default::default() });
        let depth_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, s, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, color_write: (false, false, false, false), ..Default::default() });
        let fs = gfx::shader(ctx, FACE_VS, FACE_FS, &["u_atlas"], &[("u_vp", Mat4), ("u_eye", Float3), ("u_res", Float2), ("u_gloc", Float1)]);
        let face_pip = ctx.new_pipeline(&[BufferLayout::default()], &MESH_ATTRS, fs, PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() });
        let atlas_tex = ctx.new_render_texture(TextureParams {
            width: ATLAS,
            height: ATLAS,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            allocate_mipmaps: true,
            ..Default::default()
        });
        let atlas_pass = ctx.new_render_pass(atlas_tex, None);
        let atlas = gfx::Target { pass: atlas_pass, color: atlas_tex, w: ATLAS, h: ATLAS };

        let mut m = Mesh::new();
        build_static(&mut m);
        let mut statics = world::part(ctx, &m);
        statics.bind.images = vec![atlas_tex];
        let mut faces = world::part(ctx, &faces(&m));
        faces.bind.images = vec![atlas_tex];
        let dyn_cap = 60000;
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(dyn_cap * crate::mesh::STRIDE));
        let dyn_bind = Bindings { vertex_buffers: vec![vb], index_buffer: gfx::sequential_ib(ctx, dyn_cap), images: vec![atlas_tex] };
        let mut canopy = world::part(ctx, canopy);
        canopy.bind.images = vec![atlas_tex];
        let mut glass = Mesh::new();
        glass.quad(HUD_GLASS[0], HUD_GLASS[1], HUD_GLASS[2], HUD_GLASS[3], Rgb(0.4, 0.6, 0.5));
        let mut hud_glass = world::part(ctx, &glass);
        hud_glass.bind.images = vec![atlas_tex];
        Cockpit { pip, glass_pip, depth_pip, face_pip, statics, faces, dynamic: Mesh::new(), dyn_bind, dyn_cap, dyn_count: 0, canopy, hud_glass, atlas, atlas_pass, atlas_clock: -1.0 }
    }

    /// Redraws every instrument page into the atlas (call outside any pass).
    pub fn update_atlas(&mut self, ctx: &mut dyn RenderingBackend, cv: &mut Canvas, a: &Aircraft, m: &Mission, pages: [Page; 2], time: f64) {
        ctx.begin_pass(Some(self.atlas_pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        draw_instruments(cv, a, m, pages, time);
        cv.flush(ctx, A as f32, A as f32);
        ctx.end_render_pass();
        ctx.texture_generate_mipmaps(self.atlas.color);
    }

    /// Draws the cockpit into the current pass. `cam_b` is the camera in body axes.
    pub fn draw(&mut self, ctx: &mut dyn RenderingBackend, cam_b: &Camera, aspect: f64, sun_b: V3, hands: &Hands) {
        let proj = perspective(cam_b.fov, aspect, 0.02, 40.0);
        let vp = mat_mul(&proj, &cam_b.view());
        // The fuselage shades the tub when the sun is below the canopy rails.
        let sun_vis = smoothstep(-0.22, 0.08, sun_b.y) as f32;
        let u = CockpitU { vp, eye: cam_b.eye.f32(), sun: sun_b.norm().f32(), sun_vis, alpha: 1.0, tint: [0.0; 3] };
        ctx.apply_pipeline(&self.pip);
        ctx.apply_bindings(&self.statics.bind);
        ctx.apply_uniforms(UniformsSource::table(&u));
        ctx.draw(0, self.statics.count, 1);

        self.dynamic.clear();
        pilot::build_dynamic(&mut self.dynamic, hands);
        let n = self.dynamic.count().min(self.dyn_cap);
        ctx.buffer_update(self.dyn_bind.vertex_buffers[0], BufferSource::slice(&self.dynamic.v[..n * crate::mesh::STRIDE]));
        ctx.apply_bindings(&self.dyn_bind);
        ctx.draw(0, n as i32, 1);
        self.dyn_count = n;

        ctx.apply_pipeline(&self.glass_pip);
        ctx.apply_bindings(&self.hud_glass.bind);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { alpha: 0.05, tint: [0.10, 0.16, 0.13], ..u }));
        ctx.draw(0, self.hud_glass.count, 1);
        ctx.apply_bindings(&self.canopy.bind);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { alpha: 0.035, tint: [0.16, 0.13, 0.05], ..u }));
        ctx.draw(0, self.canopy.count, 1);
    }

    /// Redraws the displays, dials and placards at full resolution into the
    /// current pass, after the low-resolution scene has been composited, so
    /// their text stays sharp. The cockpit from the last `draw` first lays
    /// down depth only, so hands, arms and the kneeboard still cover them.
    /// `res` is the pass size in pixels; `gloc` matches the post pass.
    pub fn draw_faces(&self, ctx: &mut dyn RenderingBackend, cam_b: &Camera, aspect: f64, res: [f32; 2], gloc: f32) {
        let proj = perspective(cam_b.fov, aspect, 0.02, 40.0);
        let vp = mat_mul(&proj, &cam_b.view());
        let eye = cam_b.eye.f32();
        ctx.apply_pipeline(&self.depth_pip);
        ctx.apply_uniforms(UniformsSource::table(&CockpitU { vp, eye, sun: [0.0, 1.0, 0.0], sun_vis: 0.0, alpha: 1.0, tint: [0.0; 3] }));
        ctx.apply_bindings(&self.statics.bind);
        ctx.draw(0, self.statics.count, 1);
        ctx.apply_bindings(&self.dyn_bind);
        ctx.draw(0, self.dyn_count as i32, 1);
        ctx.apply_pipeline(&self.face_pip);
        ctx.apply_bindings(&self.faces.bind);
        ctx.apply_uniforms(UniformsSource::table(&FaceU { vp, eye, res, gloc }));
        ctx.draw(0, self.faces.count, 1);
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
    let p = panel();
    // Panel face and its shoulder rim.
    let face: Vec<V3> = OUTLINE.iter().map(|&(u, v)| p.p(u, v, 0.0)).collect();
    m.polygon(&face, PANEL);
    for k in 0..OUTLINE.len() {
        let (a, b) = (OUTLINE[k], OUTLINE[(k + 1) % OUTLINE.len()]);
        m.quad(p.p(a.0, a.1, 0.0), p.p(b.0, b.1, 0.0), p.p(b.0, b.1, -0.20), p.p(a.0, a.1, -0.20), PANEL.scale(0.7));
    }
    // Glare shield: a hood from the panel top edge up toward the windscreen.
    let top: Vec<(f64, f64)> = OUTLINE[2..].iter().chain(OUTLINE[..1].iter().skip(1)).copied().collect();
    let shield = |u: f64, v: f64, t: f64| p.p(u * (1.0 - 0.06 * t), v + 0.012 + 0.05 * t, 0.045 - 0.42 * t);
    for k in 0..top.len() - 1 {
        let (a, b) = (top[k], top[k + 1]);
        for s in 0..4 {
            let (t0, t1) = (s as f64 / 4.0, (s + 1) as f64 / 4.0);
            m.quad(shield(a.0, a.1, t0), shield(b.0, b.1, t0), shield(b.0, b.1, t1), shield(a.0, a.1, t1), GLARE);
        }
        m.capsule(shield(a.0, a.1, 0.0), shield(b.0, b.1, 0.0), 0.011, 0.011, GLARE.scale(1.6), 6);
        m.quad(p.p(a.0, a.1, 0.0), p.p(b.0, b.1, 0.0), shield(b.0, b.1, 0.0), shield(a.0, a.1, 0.0), GLARE.scale(1.2));
    }

    // Displays: two portrait MFDs and the large centre pedestal display.
    display(m, &panel_at(-0.232, 0.72, 0.0), 0.175, 0.215, 0.125, 0.16, R_MFD_L, true);
    display(m, &panel_at(0.232, 0.72, 0.0), 0.175, 0.215, 0.125, 0.16, R_MFD_R, true);
    display(m, &panel_at(0.0, 0.58, 0.0), 0.195, 0.255, 0.15, 0.20, R_CPD, true);

    // Up-front control: DED over the ICP keypad, protruding under the HUD.
    let ufc = panel_at(0.0, 0.86, 0.0);
    m.block(&ufc.at(0.0, 0.0, 0.05), v3(0.118, 0.078, 0.05), BEZEL.scale(0.95));
    let ufc_face = Frame { o: ufc.p(0.0, 0.0, 0.1005), ..ufc };
    screen(m, &ufc_face.at(0.0, 0.048, 0.0), 0.20, 0.047, R_DED);
    screen(m, &ufc_face.at(0.0, -0.022, 0.0), 0.20, 0.129 * 0.72, R_ICP);

    // HUD: body on the glare shield, side posts and the tall combiner frame.
    // The body sits forward, inside the glare shield; only the glass rises above it.
    let hud_body = Frame { o: v3(0.0, 1.03, -4.15), x: V3::X, y: V3::Y, z: V3::Z };
    m.block(&hud_body, v3(0.10, 0.042, 0.12), GLARE.scale(1.5));
    m.block(&Frame { o: v3(0.0, 1.074, -4.04), ..hud_body }, v3(0.095, 0.004, 0.035), GLARE.scale(2.2));
    for &s in &[-1.0, 1.0] {
        let (b, t) = (v3(s * 0.104, 1.07, -4.02), v3(s * 0.104, 1.352, -3.958));
        m.capsule(b, t, 0.0045, 0.004, GLARE.scale(2.0), 6);
    }
    m.capsule(v3(-0.104, 1.352, -3.958), v3(0.104, 1.352, -3.958), 0.0025, 0.0025, GLARE.scale(2.0), 6);

    // Glare-shield lamps: AOA indexer left of the HUD, master caution right.
    let lamp = |u: f64| Frame { o: v3(u, 1.045, -4.035), x: V3::X, y: v3(0.0, 0.94, -0.34), z: v3(0.0, 0.34, 0.94) };
    m.block(&lamp(-0.14).at(0.0, 0.0, -0.005), v3(0.017, 0.045, 0.01), GLARE.scale(1.4));
    screen(m, &lamp(-0.14).at(0.0, 0.0, 0.006), 0.02, 0.07, R_AOA);
    m.block(&lamp(0.14).at(0.0, 0.0, -0.005), v3(0.024, 0.024, 0.01), GLARE.scale(1.4));
    screen(m, &lamp(0.14).at(0.0, 0.0, 0.006), 0.036, 0.036, R_MC);

    // Left column: RWR scope, airspeed/Mach indicator, altimeter, accelerometer.
    display(m, &panel_at(-0.40, 0.75, 0.0), 0.10, 0.10, 0.074, 0.074, R_RWR, false);
    gauge(m, &panel_at(-0.40, 0.645, 0.0), 0.034, R_DIALS[0]);
    gauge(m, &panel_at(-0.40, 0.555, 0.0), 0.034, R_DIALS[1]);
    gauge(m, &panel_at(-0.40, 0.462, 0.0), 0.031, R_GMETER);
    // Right column: standby ADI over RPM, FTIT, nozzle and oil.
    gauge(m, &panel_at(0.40, 0.75, 0.0), 0.040, R_DIALS[2]);
    for (i, y) in [0.66, 0.59, 0.52, 0.45].iter().enumerate() {
        gauge(m, &panel_at(0.40, *y, 0.0), 0.029, R_DIALS[3 + i]);
    }
    plate(m, &panel_at(0.222, 0.88, 0.0), 0.13, 0.065, R_CAUTION);
    plate(m, &panel_at(-0.222, 0.88, 0.0), 0.13, 0.065, R_ARM);
    // Small switch rows low on the panel, mostly behind the knees.
    for &s in &[-1.0, 1.0] {
        for i in 0..3 {
            let f = panel_at(s * (0.13 + i as f64 * 0.045), 0.47, 0.0);
            m.block(&f.at(0.0, 0.0, 0.006), v3(0.016, 0.022, 0.006), BEZEL);
            m.capsule(f.p(0.0, 0.0, 0.01), f.p(0.0, 0.012, 0.03), 0.003, 0.0025, METALC, 6);
        }
    }

    // Tub: floor, footwell, side walls, sills and the canopy rails.
    m.quad(v3(-0.47, 0.22, -4.40), v3(0.47, 0.22, -4.40), v3(0.47, 0.22, -2.40), v3(-0.47, 0.22, -2.40), WALL.scale(0.55));
    m.quad(v3(-0.47, 0.22, -4.40), v3(0.47, 0.22, -4.40), v3(0.47, 0.42, -4.12), v3(-0.47, 0.42, -4.12), WALL.scale(0.45));
    for &s in &[-1.0, 1.0] {
        let n = 12;
        for k in 0..n {
            let (z0, z1) = (mix(-4.30, -2.30, k as f64 / n as f64), mix(-4.30, -2.30, (k + 1) as f64 / n as f64));
            let (b0, b1) = (airframe::canopy_profile(z0).1, airframe::canopy_profile(z1).1);
            let (w0, w1) = (airframe::canopy_profile(z0).0.max(0.47), airframe::canopy_profile(z1).0.max(0.47));
            m.quad(v3(s * 0.47, 0.22, z0), v3(s * 0.47, 0.22, z1), v3(s * 0.47, b1 - 0.03, z1), v3(s * 0.47, b0 - 0.03, z0), WALL);
            m.quad(v3(s * 0.47, b0 - 0.03, z0), v3(s * 0.47, b1 - 0.03, z1), v3(s * w1, b1, z1), v3(s * w0, b0, z0), WALL.scale(1.15));
            let d = (v3(s * w1, b1, z1) - v3(s * w0, b0, z0)).norm() * 0.012;
            pilot::bar(m, v3(s * w0, b0, z0) - d, v3(s * w1, b1, z1) + d, 0.024, 0.018, V3::Y, Rgb(0.12, 0.13, 0.14));
        }

        // Side consoles with labelled panels, running forward to the stick
        // mount and the throttle quadrant.
        let base = Frame { o: v3(s * 0.38, 0.52, -3.25), x: V3::X, y: V3::Y, z: V3::Z };
        m.block(&base, v3(0.09, 0.12, 0.63), CONSOLE);
        let names = if s < 0.0 { 0 } else { 5 };
        for j in 0..5 {
            let z = -3.66 + j as f64 * 0.23;
            let top = Frame { o: v3(s * 0.38, 0.6415, z), x: V3::X, y: V3::Z * -1.0, z: V3::Y };
            let (u0, v0, u1, v1) = uv(r_console(names + j));
            let (hw, hh) = (0.082, 0.105);
            m.quad_uv(top.p(-hw, hh, 0.0), top.p(hw, hh, 0.0), top.p(hw, -hh, 0.0), top.p(-hw, -hh, 0.0), [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]);
            // No toggles under the throttle or the right arm rest.
            if (s < 0.0 && j > 0) || (s > 0.0 && j > 1) {
                for k in 0..2 {
                    let x = s * 0.38 + (k as f64 - 0.5) * 0.055;
                    m.capsule(v3(x, 0.645, z + 0.03), v3(x, 0.67, z + 0.022), 0.004, 0.003, METALC, 6);
                    m.ellipsoid(v3(x, 0.672, z + 0.021), v3(0.006, 0.006, 0.006), METALC.scale(0.9), 3, 6);
                }
            }
        }
    }
    pilot::build_static(m);

    // Canopy aft bow and bulkhead behind the seat, on the canopy's facet lines.
    let nv = airframe::CANOPY_NV;
    let bow: Vec<V3> = (0..=nv).map(|k| airframe::canopy_point(0.985, k as f64 / nv as f64)).collect();
    pilot::strap(m, &bow, 0.03, 0.026, V3::Z, Rgb(0.13, 0.15, 0.16));
    let bulk: Vec<V3> = (0..=nv).map(|k| airframe::canopy_point(0.995, k as f64 / nv as f64)).collect();
    m.polygon(&bulk, Rgb(0.12, 0.13, 0.13));
    m.facet();
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
fn mfd(cv: &mut Canvas, r: R, page: Page, a: &Aircraft, ad: &AirData, m: &Mission, time: f64) {
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
            cv.text("CRM  RWS  NORM", x0 + w / 2.0, y0 + 34.0, 1.0, GREEN, Align::Center);
            cv.text("40", x0 + 10.0, y0 + 60.0, 1.0, GREEN, Align::Left);
            let (l, rr, t, b) = (x0 + 28.0, x0 + w - 28.0, y0 + 50.0, y0 + h - 50.0);
            for i in 1..4 {
                let yy = t + (b - t) * i as f32 / 4.0;
                cv.line(l, yy, rr, yy, 1.0, rgba(0x15, 0x37, 0x31, 1.0));
            }
            cv.line(l, t, l, b, 1.0, GREEN);
            cv.line(rr, t, rr, b, 1.0, GREEN);
            let sweep = ((time * 0.24).fract() as f32).abs();
            let sx = l + 4.0 + sweep * (rr - l - 8.0);
            cv.line(sx, t + 2.0, sx, b - 2.0, 2.0, rgba(0x39, 0x79, 0x57, 1.0));
            cv.text("NO TRACK", x0 + w / 2.0, (t + b) / 2.0, 2.0, rgba(0x80, 0xb7, 0x91, 1.0), Align::Center);
            cv.text(&format!("{} C", pad(ad.kcas, 3)), l + 4.0, b + 14.0, 1.0, GREEN, Align::Left);
            cv.text(&format!("{} ALT", pad(ad.baro_ft / 100.0, 3)), rr - 4.0, b + 14.0, 1.0, GREEN, Align::Right);
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
                let yy = y0 + 48.0 + i as f32 * 36.0;
                cv.text(k, x0 + 14.0, yy, 1.0, DIM, Align::Left);
                cv.text(v, x0 + w - 14.0, yy + 12.0, 2.0, GREEN, Align::Right);
                cv.line(x0 + 12.0, yy + 24.0, x0 + w - 12.0, yy + 24.0, 1.0, rgba(0x1c, 0x34, 0x31, 1.0));
            }
            let (s, c) = if a.ab { ("AFTERBURNER", AMBER) } else { ("FLCS NORM", GREEN) };
            cv.text(s, x0 + w / 2.0, y0 + h - 40.0, 2.0, c, Align::Center);
        }
    }
}

fn draw_instruments(cv: &mut Canvas, a: &Aircraft, m: &Mission, pages: [Page; 2], time: f64) {
    let ad = AirData::of(a);
    mfd(cv, R_MFD_L, pages[0], a, &ad, m, time);
    mfd(cv, R_MFD_R, pages[1], a, &ad, m, time);
    mfd(cv, R_CPD, Page::Hsd, a, &ad, m, time);

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

    // RWR: no emitters on a navigation sortie.
    let r = R_RWR;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x02, 0x08, 0x05, 1.0));
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    for k in [18.0, 36.0, 54.0] {
        cv.circle(cx, cy, k, 1.0, rgba(0x31, 0x53, 0x40, 1.0));
    }
    cv.line(cx - 56.0, cy, cx + 56.0, cy, 1.0, rgba(0x46, 0x6d, 0x50, 1.0));
    cv.line(cx, cy - 56.0, cx, cy + 56.0, 1.0, rgba(0x46, 0x6d, 0x50, 1.0));
    cv.text("N", cx, r.y + 9.0, 1.0, GREEN, Align::Center);
    cv.text("NO EMITTER", cx, cy + 22.0, 1.0, GREEN, Align::Center);
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
    g_meter(cv, R_GMETER, &ad);
    adi(cv, R_DIALS[2], a);
    dial(cv, R_DIALS[3], a.rpm, 0.0, 110.0, "RPM", &pad(a.rpm, 2), 10);
    dial(cv, R_DIALS[4], a.ftit, 200.0, 1200.0, "FTIT", &pad(a.ftit, 3), 9);
    dial(cv, R_DIALS[5], a.nozzle, 0.0, 100.0, "NOZ", &pad(a.nozzle, 2), 11);
    dial(cv, R_DIALS[6], a.oil_psi, 0.0, 100.0, "OIL", &pad(a.oil_psi, 2), 11);

    // Caution light panel.
    let r = R_CAUTION;
    cv.rect(r.x, r.y, r.w, r.h, rgba(0x20, 0x27, 0x29, 1.0));
    let labels = [("FLCS", a.alpha_deg > 25.0), ("FUEL LOW", a.fuel < 450.0), ("ENGINE", a.fuel <= 0.0), ("OIL", a.oil_psi < 20.0), ("ALT LOW", ad.radar_ft < 300.0), ("SPD BRK", a.brake > 0.1)];
    for (i, (s, lit)) in labels.iter().enumerate() {
        let (x, y) = (r.x + (i % 2) as f32 * 128.0, r.y + (i / 2) as f32 * 42.6);
        cv.rect(x + 3.0, y + 3.0, 122.0, 36.0, if *lit { rgba(0xd4, 0x96, 0x38, 1.0) } else { rgba(0x0c, 0x14, 0x14, 1.0) });
        cv.text(s, x + 64.0, y + 21.0, 2.0, if *lit { rgba(0x1c, 0x14, 0x05, 1.0) } else { rgba(0x6b, 0x75, 0x69, 1.0) }, Align::Center);
    }

    // Master arm / laser panel.
    let r = R_ARM;
    hardware(cv, r, "");
    cv.text("MASTER ARM", r.x + 64.0, r.y + 22.0, 1.0, WHITE, Align::Center);
    for (i, s) in ["ARM", "SIM", "SAFE"].iter().enumerate() {
        let y = r.y + 46.0 + i as f32 * 22.0;
        cv.text(s, r.x + 80.0, y, 1.0, rgba(0xb6, 0xc6, 0xb5, 1.0), Align::Left);
        if i == 2 {
            cv.line(r.x + 38.0, y, r.x + 64.0, y, 4.0, WHITE);
        }
    }
    cv.text("LASER ARM", r.x + 190.0, r.y + 22.0, 1.0, WHITE, Align::Center);
    cv.text("OFF", r.x + 190.0, r.y + 90.0, 1.0, rgba(0x96, 0xa5, 0xa0, 1.0), Align::Center);
    cv.line(r.x + 190.0, r.y + 48.0, r.x + 190.0, r.y + 76.0, 4.0, WHITE);
    cv.text("ALT REL", r.x + 128.0, r.y + r.h - 16.0, 1.0, rgba(0x96, 0xa5, 0xa0, 1.0), Align::Center);

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
    cv.text("COLD RIVER", r.x + r.w / 2.0, r.y + 18.0, 2.0, ink, Align::Center);
    for (i, s) in ["CHECK FUEL", "10 STEERPOINTS", "H AIRBRAKE", "K CONFIG", "ALT + MOUSE LOOK", "C CENTER VIEW", "V CHASE VIEW"].iter().enumerate() {
        cv.text(s, r.x + 10.0, r.y + 46.0 + i as f32 * 24.0, 1.0, ink, Align::Left);
    }

    // Console panels.
    let names = ["FUEL", "EPU / JFS", "UHF / VHF", "AUDIO", "EXT LIGHTS", "POWER", "FLCS", "SENSOR PWR", "AVIONICS", "ECS"];
    for (i, n) in names.iter().enumerate() {
        let r = r_console(i);
        hardware(cv, r, "");
        cv.text(n, r.x + r.w / 2.0, r.y + 16.0, 1.0, rgba(0xc7, 0xcf, 0xc5, 1.0), Align::Center);
        for k in 0..3 {
            let x = r.x + 26.0 + k as f32 * 38.0;
            cv.stroke_rect(x - 13.0, r.y + 38.0, 26.0, 46.0, 1.0, rgba(0x77, 0x83, 0x7c, 1.0));
            cv.text(["ON", "NRM", "AUT"][k], x, r.y + 31.0, 1.0, WHITE, Align::Center);
            cv.text("OFF", x, r.y + 92.0, 1.0, rgba(0x9d, 0xaa, 0xa0, 1.0), Align::Center);
        }
        if i == 2 {
            cv.rect(r.x + 14.0, r.y + 108.0, r.w - 28.0, 26.0, rgba(0x07, 0x14, 0x10, 1.0));
            cv.text("305.000", r.x + r.w / 2.0, r.y + 121.0, 2.0, GREEN, Align::Center);
        } else {
            cv.line(r.x + 14.0, r.y + 112.0, r.x + r.w - 14.0, r.y + 112.0, 1.0, rgba(0x6f, 0x7b, 0x72, 1.0));
            cv.text("TEST  RESET", r.x + r.w / 2.0, r.y + 128.0, 1.0, rgba(0xaf, 0xb9, 0xb0, 1.0), Align::Center);
        }
    }
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
pub fn draw_hud(cv: &mut Canvas, a: &Aircraft, m: &Mission, cam: &Camera, body: &BodyProjector) {
    let glass: Vec<[f32; 2]> = HUD_GLASS.iter().filter_map(|&p| body.project(p)).collect();
    if glass.len() < 4 {
        return;
    }
    cv.set_clip(Some(glass));
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
    // Boxed readout: text anchored at a HUD point, frame drawn in pixels around it.
    let boxed = |cv: &mut Canvas, s: &str, x: f64, y: f64, align: Align| {
        if let Some(p) = at(x, y) {
            let tw = Canvas::text_width(s, big);
            let x0 = match align {
                Align::Left => p[0],
                Align::Center => p[0] - tw * 0.5,
                Align::Right => p[0] - tw,
            };
            cv.text(s, p[0], p[1], big, col, align);
            let pad = 2.0 * big;
            cv.stroke_rect(x0 - pad - 1.0, p[1] - 3.5 * big - pad, tw + 2.0 * pad + 2.0, 7.0 * big + 2.0 * pad, lw, col);
        }
    };
    let half = 70.0;
    let ad = AirData::of(a);
    let alt_ft = ad.baro_ft;
    // Left: load factor, the KCAS box, then Mach, angle of attack, peak G and
    // the master mode. Right: the barometric altitude box (altimeter on QNH),
    // radar altitude, vertical velocity and steering to the next gate.
    ht(cv, &format!("G {:.1}", ad.g), -half, -62.0, small, Align::Left);
    boxed(cv, &pad(ad.kcas, 3), -half, 0.0, Align::Left);
    ht(cv, "C", -half, 13.0, small, Align::Left);
    boxed(cv, &pad(alt_ft, 5), half, 0.0, Align::Right);
    ht(cv, "B", half, 13.0, small, Align::Right);
    ht(cv, &format!("M {:.2}", ad.mach), -half, 34.0, small, Align::Left);
    ht(cv, &format!("AOA {:.1}", a.alpha_deg), -half, 46.0, small, Align::Left);
    ht(cv, &format!("GMAX {:.1}", ad.g_max), -half, 58.0, small, Align::Left);
    ht(cv, "NAV", -half, 70.0, small, Align::Left);
    ht(cv, &format!("R {}", pad(ad.radar_ft, 5)), half, 34.0, small, Align::Right);
    ht(cv, &format!("VV {:+}", (ad.vs_fpm / 10.0).round() as i64 * 10), half, 46.0, small, Align::Right);
    let gate = m.current_gate();
    let (dist, ttg) = steering(a, m);
    ht(cv, &format!("{:.1}>{:02}", dist / 1852.0, (m.gate_index + 1).min(m.gates.len())), half, 58.0, small, Align::Right);
    ht(cv, &clock(ttg), half, 70.0, small, Align::Right);
    // Heading tape along the bottom of the field of view.
    let base = (a.heading / 5.0).round() * 5.0;
    for k in -5..=5 {
        let n = wrap(base + k as f64 * 5.0, 360.0);
        let diff = wrap180(n - a.heading);
        if diff.abs() > 13.0 {
            continue;
        }
        let major = (n.round() as i64) % 10 == 0;
        hl(cv, diff * 3.0, 100.0, diff * 3.0, if major { 106.0 } else { 103.0 });
        if major {
            ht(cv, &pad((n / 10.0).round() % 36.0, 2), diff * 3.0, 93.0, small, Align::Center);
        }
    }
    hl(cv, 0.0, 108.0, -2.5, 112.0);
    hl(cv, 0.0, 108.0, 2.5, 112.0);
    ht(cv, &pad(a.heading, 3), 0.0, 119.0, small, Align::Center);
    // Speed and altitude scales flank the flight path.
    for i in -3..=3 {
        let speed = (ad.kcas / 10.0).round() * 10.0 + i as f64 * 10.0;
        let dy = (ad.kcas - speed) * 0.9;
        if dy.abs() < 24.0 {
            hl(cv, -half - 6.0, dy, -half - 2.0, dy);
        }
        let alt = (alt_ft / 100.0).round() * 100.0 + i as f64 * 100.0;
        let ay = (alt_ft - alt) * 0.09;
        if ay.abs() < 24.0 {
            hl(cv, half + 6.0, ay, half + 2.0, ay);
        }
    }
    // Gun cross at the top of the field.
    hl(cv, -8.0, -55.0, -3.0, -55.0);
    hl(cv, 3.0, -55.0, 8.0, -55.0);
    hl(cv, 0.0, -60.0, 0.0, -50.0);

    // Inertial pitch ladder.
    let hd = a.heading * DEG;
    for pitch in (-85i32..=85).step_by(5) {
        let e = pitch as f64 * DEG;
        let dir = |az: f64| {
            let hh = hd + az * DEG;
            v3(hh.sin() * e.cos(), e.sin(), -hh.cos() * e.cos())
        };
        if world_dir(dir(0.0)).is_none() {
            continue;
        }
        for &s in &[-1.0, 1.0] {
            let (i0, i1) = if pitch == 0 { (2.0, 9.0) } else { (2.0, 5.0) };
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
            if let Some(q) = world_dir(dir(6.0)) {
                cv.text(&format!("{}", pitch.abs()), q[0] + 2.0, q[1], small, col, Align::Left);
            }
        }
    }
    // Flight-path marker.
    if let Some(f) = world_dir(a.vel.norm()) {
        let s = big * 2.6;
        cv.circle(f[0], f[1], s, lw, col);
        cv.line(f[0] - s * 2.4, f[1], f[0] - s, f[1], lw, col);
        cv.line(f[0] + s, f[1], f[0] + s * 2.4, f[1], lw, col);
        cv.line(f[0], f[1] - s, f[0], f[1] - s * 2.0, lw, col);
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
    cv.set_clip(None);
}

/// Camera in body axes, for the cockpit pass and the combiner projection.
pub fn body_camera(cam: &Camera, a: &Aircraft) -> Camera {
    let inv = a.q.conj();
    Camera { eye: inv.rotate(cam.eye - a.pos), forward: inv.rotate(cam.forward), right: inv.rotate(cam.right), up: inv.rotate(cam.up), fov: cam.fov }
}

pub fn eye_world(a: &Aircraft) -> V3 {
    a.pos + a.q.rotate(EYE)
}
