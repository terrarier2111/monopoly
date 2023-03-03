use crate::atlas::{Atlas, AtlasAlloc, AtlasId};
use bytemuck_derive::Pod;
use bytemuck_derive::Zeroable;
use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::size_of;
use std::process::abort;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use dashmap::DashMap;
use wgpu::{BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType, BlendState, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites, LoadOp, Operations, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPipeline, Sampler, SamplerBindingType, ShaderSource, ShaderStages, Texture, TextureFormat, TextureSampleType, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use wgpu::util::StagingBelt;
use wgpu_biolerless::{
    FragmentShaderState, ModuleSrc, PipelineBuilder, ShaderModuleSources, State, VertexShaderState,
    WindowSize,
};
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section};
use winit::window::Window;
use crate::utils::LIGHT_GRAY_GPU;

pub struct Renderer {
    pub state: Arc<State>,
    atlas_pipeline: RenderPipeline,
    tex_pipeline: RenderPipeline,
    color_pipeline: RenderPipeline,
    tex_bind_group_layout: BindGroupLayout,
    pub dimensions: Dimensions,
    glyphs: Mutex<Vec<GlyphInfo>>,
}

pub struct GlyphInfo {
    pub brush: Mutex<GlyphBrush<()>>,
    pub format: TextureFormat,
    staging_belt: Mutex<StagingBelt>,
}

impl GlyphInfo {
    pub fn new(brush: GlyphBrush<()>, format: TextureFormat) -> Self {
        Self {
            brush: Mutex::new(brush),
            format,
            staging_belt: Mutex::new(StagingBelt::new(1024)),
        }
    }
}

impl Renderer {
    pub fn new(state: Arc<State>, window: &Window) -> anyhow::Result<Self> {
        let mut glyphs = vec![];
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!(
            "PlayfairDisplayRegular.ttf"
        ))?;

        glyphs.push(GlyphInfo {
            brush: Mutex::new(GlyphBrushBuilder::using_font(font).build(&state.device(), state.format())),
            format: state.format(),
            staging_belt: Mutex::new(StagingBelt::new(1024)),
        });

        let bgl = state.create_bind_group_layout(&[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                view_dimension: TextureViewDimension::D2,
                sample_type: TextureSampleType::Float { filterable: true },
            },
            count: None,
        }, BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        }]);

        let (width, height) = window.window_size();
        Ok(Self {
            atlas_pipeline: Self::atlas_pipeline(&state),
            tex_pipeline: Self::tex_pipeline(&state),
            color_pipeline: Self::color_pipeline(&state),
            state,
            dimensions: Dimensions::new(width, height),
            glyphs: Mutex::new(glyphs),
            tex_bind_group_layout: bgl,
        })
    }

    pub fn render(
        &self,
        models: Vec<Model>,
        atlas: Arc<Atlas>, /*atlases: Arc<Mutex<Vec<Arc<Atlas>>>>*/
    ) {
        self.state
            .render(
                |view, mut encoder, state| {
                    /*for atlas in atlases.lock().unwrap().iter() {
                        atlas.update(&mut encoder);
                    }*/
                    atlas.update(&mut encoder);
                    let mut atlas_models: HashMap<AtlasId, Vec<AbsoluteTextureVertex>> = HashMap::new();
                    let mut color_models = vec![];
                    let mut texture_models = vec![];
                    for model in models {
                        match model.color_src.clone() { // FIXME: try getting rid of this clone!
                            ColorSource::PerVert => {
                                color_models.extend(model.vertices.into_iter().map(
                                    |vert| match vert {
                                        Vertex::Color { pos, color } => ColorVertex { pos, color },
                                        Vertex::Texture { .. } => unreachable!(),
                                    },
                                ));
                            }
                            ColorSource::Atlas(atlas) => {
                                // FIXME: make different atlases work!
                                let vertices = model.vertices.into_iter().map(|vert| match vert {
                                    Vertex::Color { .. } => unreachable!(),
                                    Vertex::Texture { pos, alpha, uv, color_scale_factor, grayscale_conv } => {
                                        AbsoluteTextureVertex { pos, alpha, uv: match uv {
                                            UvKind::Absolute(abs) => abs,
                                            UvKind::Relative(_) => unreachable!(),
                                        }, color_scale_factor,
                                            meta: {
                                                let mut meta = 0;
                                                if grayscale_conv {
                                                    meta |= GRAYSCALE_CONV_FLAG;
                                                }
                                                meta
                                            },
                                        }
                                    }
                                });
                                if let Some(mut models) = atlas_models.get_mut(&atlas.id()) {
                                    models.extend(vertices);
                                } else {
                                    atlas_models
                                        .insert(atlas.id(), vertices.collect::<Vec<AbsoluteTextureVertex>>());
                                }
                            }
                            ColorSource::Tex(tex) => {
                                // println!("tex_debug: {:?}", tex.tex.size());
                                let vertices = model.vertices.into_iter().map(|vert| match vert {
                                    Vertex::Color { .. } => unreachable!(),
                                    Vertex::Texture { pos, alpha, uv, color_scale_factor, grayscale_conv } => {
                                        RelativeTextureVertex { pos, alpha, uv: match uv {
                                            UvKind::Absolute(_) => unreachable!(),
                                            UvKind::Relative(rel) => rel,
                                        }, color_scale_factor,
                                            meta: {
                                                let mut meta = 0;
                                                if grayscale_conv {
                                                    meta |= GRAYSCALE_CONV_FLAG;
                                                }
                                                meta
                                            },
                                        }
                                    }
                                });
                                texture_models.push((tex, vertices.collect::<Vec<_>>()));
                            }
                        }
                    }
                    let color_buffer =
                        state.create_buffer(color_models.as_slice(), BufferUsages::VERTEX);
                    {
                        let attachments = [Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(LIGHT_GRAY_GPU),
                                store: true,
                            },
                        })];
                        let mut render_pass =
                            state.create_render_pass(&mut encoder, &attachments, None);
                        // let buffer = state.create_buffer(atlas_models.as_slice(), BufferUsages::VERTEX);
                        // render_pass.set_vertex_buffer(0, buffer.slice(..));

                        render_pass.set_vertex_buffer(0, color_buffer.slice(..));
                        render_pass.set_pipeline(&self.color_pipeline);
                        render_pass.draw(0..(color_models.len() as u32), 0..1);
                    }

                    // println!("tex models: {}", texture_models.len());
                    for (idx, texture_models) in texture_models.iter().enumerate() {
                        let texture_buffer =
                            state.create_buffer(texture_models.1.as_slice(), BufferUsages::VERTEX);

                        let bg = state.create_bind_group(&self.tex_bind_group_layout, &[BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&texture_models.0.view),
                        }, BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&texture_models.0.sampler),
                        }]);
                        {
                            let attachments = [Some(RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: Operations {
                                    load: LoadOp::Load,
                                    store: true,
                                },
                            })];
                            let mut render_pass =
                                state.create_render_pass(&mut encoder, &attachments, None);
                            // let buffer = state.create_buffer(atlas_models.as_slice(), BufferUsages::VERTEX);
                            // render_pass.set_vertex_buffer(0, buffer.slice(..));
                            println!("idx: {} models: {}", idx, texture_models.1.len());

                            render_pass.set_vertex_buffer(0, texture_buffer.slice(..));
                            render_pass.set_bind_group(0, &bg, &[]);
                            render_pass.set_pipeline(&self.tex_pipeline);
                            render_pass.draw(0..(texture_models.1.len() as u32), 0..1);
                        }
                    }
                    for glyph in self.glyphs.lock().unwrap().iter() {
                        let mut staging_belt = glyph.staging_belt.lock().unwrap();
                        let (width, height) = self.dimensions.get();
                        glyph.brush.lock().unwrap().draw_queued(&state.device(), &mut staging_belt, &mut encoder, view, width, height).unwrap();
                        staging_belt.finish();
                    }
                    encoder
                },
                &TextureViewDescriptor::default(),
            )
            .unwrap();
        for glyph in self.glyphs.lock().unwrap().iter() {
            glyph.staging_belt.lock().unwrap().recall();
        }
    }

    fn color_pipeline(state: &State) -> RenderPipeline {
        PipelineBuilder::new()
            .vertex(VertexShaderState {
                entry_point: "main_vert",
                buffers: &[ColorVertex::desc()],
            })
            .fragment(FragmentShaderState {
                entry_point: "main_frag",
                targets: &[Some(ColorTargetState {
                    format: state.format(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            })
            .shader_src(ShaderModuleSources::Single(ModuleSrc::Source(
                ShaderSource::Wgsl(include_str!("ui_color.wgsl").into()),
            )))
            .layout(&state.create_pipeline_layout(&[], &[]))
            .build(state)
    }

    fn atlas_pipeline(state: &State) -> RenderPipeline {
        PipelineBuilder::new()
            .vertex(VertexShaderState {
                entry_point: "main_vert",
                buffers: &[AbsoluteTextureVertex::desc()],
            })
            .fragment(FragmentShaderState {
                entry_point: "main_frag",
                targets: &[Some(ColorTargetState {
                    format: state.format(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            })
            .shader_src(ShaderModuleSources::Single(ModuleSrc::Source(
                ShaderSource::Wgsl(include_str!("ui_atlas.wgsl").into()),
            )))
            .layout(&state.create_pipeline_layout(
                &[&state.create_bind_group_layout(&[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ])],
                &[],
            ))
            .build(state)
    }

    fn tex_pipeline(state: &State) -> RenderPipeline {
        PipelineBuilder::new()
            .vertex(VertexShaderState {
                entry_point: "main_vert",
                buffers: &[RelativeTextureVertex::desc()],
            })
            .fragment(FragmentShaderState {
                entry_point: "main_frag",
                targets: &[Some(ColorTargetState {
                    format: state.format(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            })
            .shader_src(ShaderModuleSources::Single(ModuleSrc::Source(
                ShaderSource::Wgsl(include_str!("ui_tex.wgsl").into()),
            )))
            .layout(&state.create_pipeline_layout(
                &[&state.create_bind_group_layout(&[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ])],
                &[],
            ))
            .build(state)
    }

    pub fn add_glyph(&self, glyph_info: GlyphInfo) -> usize {
        let mut glyphs = self.glyphs.lock().unwrap();
        let len = glyphs.len();
        glyphs.push(glyph_info);
        len
    }

    pub fn queue_glyph(&self, glyph_id: usize, section: Section) {
        self.glyphs.lock().unwrap()[glyph_id].brush.lock().unwrap().queue(section);
    }
}

#[derive(Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub color_src: ColorSource,
}

#[derive(Clone)]
pub enum ColorSource {
    PerVert,
    Atlas(Arc<Atlas>),
    Tex(Arc<TexTriple>),
}

pub enum TexTy {
    Atlas(Arc<AtlasAlloc>),
    Simple(Arc<TexTriple>),
}

#[derive(Copy, Clone)]
pub enum Vertex {
    Color {
        pos: [f32; 2],
        color: [f32; 4],
    },
    Texture {
        pos: [f32; 2],
        alpha: f32,
        uv: UvKind,
        color_scale_factor: f32,
        grayscale_conv: bool,
    },
}

#[derive(Copy, Clone)]
pub enum UvKind {
    Absolute((u32, u32)),
    Relative((f32, f32)),
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct ColorVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

impl ColorVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<ColorVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

const GRAYSCALE_CONV_FLAG: u32 = 1 << 0;

#[derive(Zeroable, Copy, Clone)]
#[repr(C)]
struct AbsoluteTextureVertex {
    pos: [f32; 2],
    uv: (u32, u32),
    alpha: f32,
    color_scale_factor: f32,
    meta: u32,
}

unsafe impl bytemuck::Pod for AbsoluteTextureVertex {}

impl AbsoluteTextureVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<AbsoluteTextureVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[derive(Zeroable, Copy, Clone)]
#[repr(C)]
struct RelativeTextureVertex {
    pos: [f32; 2],
    uv: (f32, f32),
    alpha: f32,
    color_scale_factor: f32,
    meta: u32,
}

unsafe impl bytemuck::Pod for RelativeTextureVertex {}

impl RelativeTextureVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<RelativeTextureVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 5]>() as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Uint32,
                },
            ],
        }
    }
}

pub struct Dimensions {
    inner: AtomicU64,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: AtomicU64::new(width as u64 | ((height as u64) << 32)),
        }
    }

    pub fn get(&self) -> (u32, u32) {
        let val = self.inner.load(Ordering::Acquire);
        (val as u32, (val >> 32) as u32)
    }

    pub fn set(&self, width: u32, height: u32) {
        let val = width as u64 | ((height as u64) << 32);
        self.inner.store(val, Ordering::Release);
    }
}

pub trait Renderable {
    fn render(&self, sender: Sender<Vec<Vertex>> /*, screen_dims: (u32, u32)*/);
}

pub struct TexTriple {
    pub tex: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}
