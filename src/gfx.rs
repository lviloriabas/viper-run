//! Shader compilation helpers and small GPU utilities.
use miniquad::*;
use std::sync::OnceLock;

static HEADER: OnceLock<&'static str> = OnceLock::new();

/// Picks the newest GLSL dialect the context accepts. Every shader in the game
/// is written in the common subset of GLSL 1.30, 1.50, 3.30 and ES 3.00.
pub fn init_header(ctx: &dyn RenderingBackend) {
    let g = ctx.info().glsl_support;
    let h = if g.v330 {
        "#version 330\n"
    } else if g.v150 {
        "#version 150\n"
    } else if g.v130 {
        "#version 130\n"
    } else {
        "#version 300 es\nprecision highp float;\n"
    };
    let _ = HEADER.set(h);
}

pub fn shader(ctx: &mut dyn RenderingBackend, vs: &str, fs: &str, images: &[&str], uniforms: &[(&str, UniformType)]) -> ShaderId {
    let header = HEADER.get().copied().unwrap_or("#version 330\n");
    let vs = format!("{header}{vs}");
    let fs = format!("{header}{fs}");
    let meta = ShaderMeta {
        images: images.iter().map(|s| s.to_string()).collect(),
        uniforms: UniformBlockLayout { uniforms: uniforms.iter().map(|(n, t)| UniformDesc::new(n, *t)).collect() },
    };
    match ctx.new_shader(ShaderSource::Glsl { vertex: &vs, fragment: &fs }, meta) {
        Ok(s) => s,
        Err(e) => panic!("shader compilation failed: {e:?}\n--- vertex\n{vs}\n--- fragment\n{fs}"),
    }
}

/// Note: miniquad's GL backend disables the depth test whenever a pipeline does
/// not write depth, so blended pipelines here keep depth writes on and are
/// drawn back to front instead.
pub fn alpha_blend() -> BlendState {
    BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))
}

pub fn additive() -> BlendState {
    BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One)
}

/// Colour + depth render target.
pub struct Target {
    pub pass: RenderPass,
    pub color: TextureId,
    pub w: u32,
    pub h: u32,
}

impl Target {
    pub fn new(ctx: &mut dyn RenderingBackend, w: u32, h: u32, filter: FilterMode) -> Target {
        let color = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::RGBA8,
            min_filter: filter,
            mag_filter: filter,
            ..Default::default()
        });
        let depth = ctx.new_render_texture(TextureParams { width: w, height: h, format: TextureFormat::Depth32, ..Default::default() });
        let pass = ctx.new_render_pass(color, Some(depth));
        Target { pass, color, w, h }
    }
    /// Frees the pass; miniquad deletes its attached textures with it.
    pub fn delete(&self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_render_pass(self.pass);
    }
}

pub fn immutable_vb<T>(ctx: &mut dyn RenderingBackend, data: &[T]) -> BufferId {
    ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(data))
}

pub fn immutable_ib<T>(ctx: &mut dyn RenderingBackend, data: &[T]) -> BufferId {
    ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(data))
}

/// Sequential u32 indices, for non-indexed triangle soups.
pub fn sequential_ib(ctx: &mut dyn RenderingBackend, n: usize) -> BufferId {
    let idx: Vec<u32> = (0..n as u32).collect();
    immutable_ib(ctx, &idx)
}
