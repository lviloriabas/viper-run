//! Immediate-mode 2D batcher on the GPU: lines, fills, arcs and pixel text.
//! Used for the HUD, MFD pages, gauges and menus. Optional convex clipping
//! keeps HUD symbology inside the physical combiner glass.
use crate::font;
use crate::gfx;
use miniquad::*;

pub type Col = [f32; 4];

pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Col {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}
pub fn with_alpha(c: Col, a: f32) -> Col {
    [c[0], c[1], c[2], c[3] * a]
}

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy)]
struct V {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    c: Col,
}

impl V {
    fn lerp(a: &V, b: &V, t: f32) -> V {
        let m = |p: f32, q: f32| p + (q - p) * t;
        V { x: m(a.x, b.x), y: m(a.y, b.y), u: m(a.u, b.u), v: m(a.v, b.v), c: [m(a.c[0], b.c[0]), m(a.c[1], b.c[1]), m(a.c[2], b.c[2]), m(a.c[3], b.c[3])] }
    }
}

const FLOATS: usize = 8;

pub struct Canvas {
    pipeline: Pipeline,
    bindings: Bindings,
    cap: usize,
    data: Vec<f32>,
    clip: Option<Vec<[f32; 2]>>,
    /// Affine transform: x' = a x + c y + e, y' = b x + d y + f.
    xf: [f32; 6],
    solid: (f32, f32),
}

#[repr(C)]
struct Uniforms {
    size: [f32; 2],
}

const VS: &str = r#"
in vec2 pos; in vec2 uv; in vec4 col;
uniform vec2 u_size;
out vec2 v_uv; out vec4 v_col;
void main() {
    v_uv = uv; v_col = col;
    gl_Position = vec4(pos.x / u_size.x * 2.0 - 1.0, 1.0 - pos.y / u_size.y * 2.0, 0.0, 1.0);
}"#;
const FS: &str = r#"
in vec2 v_uv; in vec4 v_col;
uniform sampler2D u_font;
out vec4 frag;
void main() { frag = v_col * texture(u_font, v_uv); }"#;

impl Canvas {
    pub fn new(ctx: &mut dyn RenderingBackend, font_tex: TextureId, cap: usize) -> Canvas {
        let shader = gfx::shader(ctx, VS, FS, &["u_font"], &[("u_size", UniformType::Float2)]);
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2), VertexAttribute::new("uv", VertexFormat::Float2), VertexAttribute::new("col", VertexFormat::Float4)],
            shader,
            PipelineParams { color_blend: Some(gfx::alpha_blend()), alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))), ..Default::default() },
        );
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<f32>(cap * FLOATS));
        let idx: Vec<u32> = (0..cap as u32).collect();
        let ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&idx));
        Canvas {
            pipeline,
            bindings: Bindings { vertex_buffers: vec![vb], index_buffer: ib, images: vec![font_tex] },
            cap,
            data: Vec::with_capacity(cap * FLOATS),
            clip: None,
            xf: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            solid: font::solid_uv(),
        }
    }

    /// Draws everything batched so far into the current pass.
    pub fn flush(&mut self, ctx: &mut dyn RenderingBackend, w: f32, h: f32) {
        let n = (self.data.len() / FLOATS).min(self.cap);
        if n == 0 {
            return;
        }
        ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(&self.data[..n * FLOATS]));
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(UniformsSource::table(&Uniforms { size: [w, h] }));
        ctx.draw(0, n as i32, 1);
        self.data.clear();
    }

    // ---------------------------------------------------------- state
    pub fn set_clip(&mut self, poly: Option<Vec<[f32; 2]>>) {
        self.clip = poly.map(|mut p| {
            // Normalise to counter-clockwise in y-down screen space.
            let area: f32 = (0..p.len()).map(|i| {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                a[0] * b[1] - b[0] * a[1]
            }).sum();
            if area < 0.0 {
                p.reverse();
            }
            p
        });
    }
    pub fn set_transform(&mut self, x: f32, y: f32, angle: f32) {
        let (s, c) = angle.sin_cos();
        self.xf = [c, s, -s, c, x, y];
    }
    pub fn reset_transform(&mut self) {
        self.xf = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }

    // ---------------------------------------------------------- core
    fn push(&mut self, v: &V) {
        self.data.extend_from_slice(&[v.x, v.y, v.u, v.v, v.c[0], v.c[1], v.c[2], v.c[3]]);
    }

    fn tri_v(&mut self, a: V, b: V, c: V) {
        let t = |v: V| {
            let m = &self.xf;
            V { x: m[0] * v.x + m[2] * v.y + m[4], y: m[1] * v.x + m[3] * v.y + m[5], ..v }
        };
        let (a, b, c) = (t(a), t(b), t(c));
        if self.data.len() / FLOATS + 9 > self.cap {
            return;
        }
        match &self.clip {
            None => {
                self.push(&a);
                self.push(&b);
                self.push(&c);
            }
            Some(poly) => {
                let mut pts = vec![a, b, c];
                let n = poly.len();
                for i in 0..n {
                    let (p, q) = (poly[i], poly[(i + 1) % n]);
                    let side = |v: &V| (q[0] - p[0]) * (v.y - p[1]) - (q[1] - p[1]) * (v.x - p[0]);
                    let mut out = Vec::with_capacity(pts.len() + 2);
                    for k in 0..pts.len() {
                        let (cur, nxt) = (&pts[k], &pts[(k + 1) % pts.len()]);
                        let (sc, sn) = (side(cur), side(nxt));
                        if sc >= 0.0 {
                            out.push(*cur);
                        }
                        if (sc >= 0.0) != (sn >= 0.0) {
                            out.push(V::lerp(cur, nxt, sc / (sc - sn)));
                        }
                    }
                    pts = out;
                    if pts.len() < 3 {
                        return;
                    }
                }
                for k in 1..pts.len() - 1 {
                    let (p0, p1, p2) = (pts[0], pts[k], pts[k + 1]);
                    self.push(&p0);
                    self.push(&p1);
                    self.push(&p2);
                }
            }
        }
    }

    fn sv(&self, x: f32, y: f32, c: Col) -> V {
        V { x, y, u: self.solid.0, v: self.solid.1, c }
    }

    // ---------------------------------------------------------- shapes
    pub fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], col: Col) {
        let (a, b, c) = (self.sv(a[0], a[1], col), self.sv(b[0], b[1], col), self.sv(c[0], c[1], col));
        self.tri_v(a, b, c);
    }

    pub fn quad(&mut self, p: [[f32; 2]; 4], col: Col) {
        self.tri(p[0], p[1], p[2], col);
        self.tri(p[0], p[2], p[3], col);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, col: Col) {
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], col);
    }

    pub fn rect_grad(&mut self, x: f32, y: f32, w: f32, h: f32, top: Col, bottom: Col) {
        let (a, b) = (self.sv(x, y, top), self.sv(x + w, y, top));
        let (c, d) = (self.sv(x + w, y + h, bottom), self.sv(x, y + h, bottom));
        self.tri_v(a, b, c);
        self.tri_v(a, c, d);
    }

    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32, lw: f32, col: Col) {
        self.rect(x, y, w, lw, col);
        self.rect(x, y + h - lw, w, lw, col);
        self.rect(x, y + lw, lw, h - 2.0 * lw, col);
        self.rect(x + w - lw, y + lw, lw, h - 2.0 * lw, col);
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32, col: Col) {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 1e-4 {
            return;
        }
        let (nx, ny) = (-dy / l * w * 0.5, dx / l * w * 0.5);
        self.quad([[x1 + nx, y1 + ny], [x2 + nx, y2 + ny], [x2 - nx, y2 - ny], [x1 - nx, y1 - ny]], col);
    }

    pub fn dashed(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32, dash: f32, gap: f32, col: Col) {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 1e-4 {
            return;
        }
        let mut t = 0.0;
        while t < l {
            let t2 = (t + dash).min(l);
            self.line(x1 + dx * t / l, y1 + dy * t / l, x1 + dx * t2 / l, y1 + dy * t2 / l, w, col);
            t += dash + gap;
        }
    }

    pub fn polyline(&mut self, pts: &[[f32; 2]], w: f32, closed: bool, col: Col) {
        let n = pts.len();
        if n < 2 {
            return;
        }
        let segs = if closed { n } else { n - 1 };
        for i in 0..segs {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            self.line(a[0], a[1], b[0], b[1], w, col);
        }
    }

    pub fn arc(&mut self, x: f32, y: f32, r: f32, a0: f32, a1: f32, w: f32, col: Col) {
        let segs = (((a1 - a0).abs() * r.max(4.0)).sqrt() * 2.2).clamp(8.0, 96.0) as usize;
        let (ro, ri) = (r + w * 0.5, (r - w * 0.5).max(0.0));
        for i in 0..segs {
            let t0 = a0 + (a1 - a0) * i as f32 / segs as f32;
            let t1 = a0 + (a1 - a0) * (i + 1) as f32 / segs as f32;
            let (c0, s0, c1, s1) = (t0.cos(), t0.sin(), t1.cos(), t1.sin());
            self.quad([[x + c0 * ro, y + s0 * ro], [x + c1 * ro, y + s1 * ro], [x + c1 * ri, y + s1 * ri], [x + c0 * ri, y + s0 * ri]], col);
        }
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32, w: f32, col: Col) {
        self.arc(x, y, r, 0.0, std::f32::consts::TAU, w, col);
    }

    pub fn disc(&mut self, x: f32, y: f32, r: f32, col: Col) {
        let segs = ((r.max(2.0)).sqrt() * 6.0).clamp(10.0, 72.0) as usize;
        for i in 0..segs {
            let t0 = std::f32::consts::TAU * i as f32 / segs as f32;
            let t1 = std::f32::consts::TAU * (i + 1) as f32 / segs as f32;
            self.tri([x, y], [x + t0.cos() * r, y + t0.sin() * r], [x + t1.cos() * r, y + t1.sin() * r], col);
        }
    }

    // ---------------------------------------------------------- text
    /// Width of a line; `{A}`, `{LB}` and the other controller tokens count
    /// as the button glyphs `text` draws for them.
    pub fn text_width(s: &str, scale: f32) -> f32 {
        let units: f32 = spans(s).map(|sp| match sp {
            Span::Text(t) => t.chars().count() as f32 * font::ADVANCE,
            Span::Pad(b) => b.width() + 1.0,
        }).sum();
        if units == 0.0 {
            0.0
        } else {
            (units - 1.0) * scale
        }
    }

    /// Pixel text; `y` is the vertical centre of the line. Controller tokens
    /// in braces draw as the pad's buttons: coloured face buttons, shoulder
    /// and trigger caps, sticks, the D-pad and the menu and view buttons.
    pub fn text(&mut self, s: &str, x: f32, y: f32, scale: f32, col: Col, align: Align) {
        if !s.contains('{') {
            return self.plain_text(s, x, y, scale, col, align);
        }
        let w = Canvas::text_width(s, scale);
        let mut px = match align {
            Align::Left => x,
            Align::Center => x - w * 0.5,
            Align::Right => x - w,
        };
        for sp in spans(s) {
            match sp {
                Span::Text(t) => {
                    self.plain_text(t, px, y, scale, col, Align::Left);
                    px += t.chars().count() as f32 * font::ADVANCE * scale;
                }
                Span::Pad(b) => {
                    self.pad_button(b, px.round(), y, scale, (col[3] * 1.6).min(1.0));
                    px += (b.width() + 1.0) * scale;
                }
            }
        }
    }

    /// One controller button glyph, 9 units tall, left edge at x. Glyphs stay
    /// bolder than the dimmed hint text round them.
    fn pad_button(&mut self, b: PadButton, x: f32, y: f32, k: f32, alpha: f32) {
        let a = |c: Col| with_alpha(c, alpha);
        let ink = a(rgba(0x0e, 0x12, 0x10, 1.0));
        let body = a(rgba(0xd4, 0xda, 0xd6, 1.0));
        let dark = a(rgba(0x26, 0x2c, 0x2a, 1.0));
        let top = (y - 4.5 * k).round();
        let w = b.width() * k;
        let (cx, cy) = (x + w * 0.5, top + 4.5 * k);
        let pill = |cv: &mut Canvas, y0: f32, h: f32, col: Col| {
            let r = h * 0.5;
            cv.rect(x + r, y0, w - h, h, col);
            cv.disc(x + r, y0 + r, r, col);
            cv.disc(x + w - r, y0 + r, r, col);
        };
        match b {
            PadButton::Face(ch, c) => {
                self.disc(cx, cy, 4.6 * k, a(c));
                self.plain_text(ch, cx, cy, k, ink, Align::Center);
            }
            PadButton::Bumper(l) => {
                pill(self, top + 1.0 * k, 7.0 * k, body);
                self.plain_text(l, cx, cy, k, ink, Align::Center);
            }
            PadButton::Trigger(l) => {
                let r = 3.0 * k;
                self.rect(x, top + r, w, 9.0 * k - r, body);
                self.rect(x + r, top, w - 2.0 * r, r, body);
                self.disc(x + r, top + r, r, body);
                self.disc(x + w - r, top + r, r, body);
                self.plain_text(l, cx, cy + 0.5 * k, k, ink, Align::Center);
            }
            PadButton::Stick(l, click) => {
                let sx = x + 4.5 * k;
                self.disc(sx, cy, 4.6 * k, dark);
                self.circle(sx, cy, 3.9 * k, 1.2 * k, body);
                self.plain_text(l, sx, cy, k, body, Align::Center);
                if click {
                    let ax = x + 11.0 * k;
                    self.rect(ax - 0.5 * k, cy - 3.5 * k, 1.0 * k, 4.0 * k, body);
                    self.tri([ax - 2.0 * k, cy + 0.5 * k], [ax + 2.0 * k, cy + 0.5 * k], [ax, cy + 3.5 * k], body);
                }
            }
            PadButton::Dpad(lit) => {
                let arm = 3.0 * k;
                let grey = a(rgba(0x5a, 0x62, 0x5f, 1.0));
                self.rect(cx - arm * 0.5, top, arm, 9.0 * k, grey);
                self.rect(x, cy - arm * 0.5, 9.0 * k, arm, grey);
                // up, down, left, right
                let arms = [(cx - arm * 0.5, top), (cx - arm * 0.5, top + 6.0 * k), (x, cy - arm * 0.5), (x + 6.0 * k, cy - arm * 0.5)];
                for (i, &(ax, ay)) in arms.iter().enumerate() {
                    if lit & (1 << i) != 0 {
                        self.rect(ax, ay, arm, arm, a(rgba(0xf4, 0xf8, 0xf0, 1.0)));
                    }
                }
            }
            PadButton::Menu => {
                pill(self, top + 1.0 * k, 7.0 * k, body);
                for i in 0..3 {
                    self.rect(cx - 2.5 * k, top + (2.5 + 2.0 * i as f32) * k, 5.0 * k, 1.0 * k, ink);
                }
            }
            PadButton::View => {
                pill(self, top + 1.0 * k, 7.0 * k, body);
                self.stroke_rect(cx - 2.5 * k, top + 2.5 * k, 3.5 * k, 3.0 * k, k, ink);
                self.rect(cx - 1.0 * k, top + 4.0 * k, 3.5 * k, 3.0 * k, body);
                self.stroke_rect(cx - 1.0 * k, top + 4.0 * k, 3.5 * k, 3.0 * k, k, ink);
            }
        }
    }

    fn plain_text(&mut self, s: &str, x: f32, y: f32, scale: f32, col: Col, align: Align) {
        let n = s.chars().count() as f32;
        let w = if n == 0.0 { 0.0 } else { (n * font::ADVANCE - 1.0) * scale };
        let mut px = match align {
            Align::Left => x,
            Align::Center => x - w * 0.5,
            Align::Right => x - w,
        };
        let mut py = y - font::GLYPH_H * scale * 0.5;
        if self.xf[..4] == [1.0, 0.0, 0.0, 1.0] && self.xf[4].fract() == 0.0 && self.xf[5].fract() == 0.0 && scale.fract() == 0.0 {
            px = px.round();
            py = py.round();
        }
        for ch in s.chars() {
            if ch != ' ' {
                let (u0, v0, u1, v1) = font::glyph_uv(font::cell_for(ch));
                let (gw, gh) = (font::GLYPH_W * scale, font::GLYPH_H * scale);
                let a = V { x: px, y: py, u: u0, v: v0, c: col };
                let b = V { x: px + gw, y: py, u: u1, v: v0, c: col };
                let c = V { x: px + gw, y: py + gh, u: u1, v: v1, c: col };
                let d = V { x: px, y: py + gh, u: u0, v: v1, c: col };
                self.tri_v(a, b, c);
                self.tri_v(a, c, d);
            }
            px += font::ADVANCE * scale;
        }
    }

    /// Word-wrapped paragraph; returns the height used.
    pub fn paragraph(&mut self, s: &str, x: f32, y: f32, max_w: f32, scale: f32, leading: f32, col: Col) -> f32 {
        let max_chars = ((max_w / scale + 1.0) / font::ADVANCE).floor().max(1.0) as usize;
        let mut line = String::new();
        let mut yy = y;
        let line_h = font::GLYPH_H * scale * leading;
        for word in s.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > max_chars {
                self.text(&line, x, yy + line_h * 0.5, scale, col, Align::Left);
                yy += line_h;
                line.clear();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            self.text(&line, x, yy + line_h * 0.5, scale, col, Align::Left);
            yy += line_h;
        }
        yy - y
    }
}

/// A controller button that pixel text can show in place of its name.
#[derive(Clone, Copy)]
enum PadButton {
    Face(&'static str, Col),
    Bumper(&'static str),
    Trigger(&'static str),
    Stick(&'static str, bool),
    /// Lit arms as bits: up, down, left, right.
    Dpad(u8),
    Menu,
    View,
}

impl PadButton {
    fn parse(t: &str) -> Option<PadButton> {
        Some(match t {
            "A" => PadButton::Face("A", rgba(0x6c, 0xc2, 0x3e, 1.0)),
            "B" => PadButton::Face("B", rgba(0xe8, 0x4c, 0x3d, 1.0)),
            "X" => PadButton::Face("X", rgba(0x3f, 0x8e, 0xe8, 1.0)),
            "Y" => PadButton::Face("Y", rgba(0xf2, 0xc2, 0x32, 1.0)),
            "LB" => PadButton::Bumper("LB"),
            "RB" => PadButton::Bumper("RB"),
            "LT" => PadButton::Trigger("LT"),
            "RT" => PadButton::Trigger("RT"),
            "LS" => PadButton::Stick("L", false),
            "RS" => PadButton::Stick("R", false),
            "L3" => PadButton::Stick("L", true),
            "R3" => PadButton::Stick("R", true),
            "DPAD" => PadButton::Dpad(0b1111),
            "DU" => PadButton::Dpad(0b0001),
            "DD" => PadButton::Dpad(0b0010),
            "DL" => PadButton::Dpad(0b0100),
            "DLR" => PadButton::Dpad(0b1100),
            "START" => PadButton::Menu,
            "BACK" => PadButton::View,
            _ => return None,
        })
    }

    /// Width in font units.
    fn width(self) -> f32 {
        match self {
            PadButton::Face(..) | PadButton::Dpad(_) => 9.0,
            PadButton::Stick(_, click) => if click { 13.0 } else { 9.0 },
            PadButton::Bumper(_) => 15.0,
            PadButton::Trigger(_) => 13.0,
            PadButton::Menu | PadButton::View => 12.0,
        }
    }
}

enum Span<'a> {
    Text(&'a str),
    Pad(PadButton),
}

/// Splits a line into plain runs and `{TOKEN}` buttons; an unknown token
/// stays as text, braces and all.
fn spans(s: &str) -> impl Iterator<Item = Span<'_>> {
    let mut rest = s;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        if let Some(body) = rest.strip_prefix('{') {
            if let Some(end) = body.find('}') {
                if let Some(b) = PadButton::parse(&body[..end]) {
                    rest = &body[end + 1..];
                    return Some(Span::Pad(b));
                }
            }
        }
        let first = rest.chars().next().map_or(1, char::len_utf8);
        let cut = rest[first..].find('{').map_or(rest.len(), |i| i + first);
        let (t, r) = rest.split_at(cut);
        rest = r;
        Some(Span::Text(t))
    })
}
