use crate::atlas::{Atlas, AtlasAlloc, AtlasId};
use bytemuck_derive::Pod;
use bytemuck_derive::Zeroable;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::process::abort;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::time::Duration;
use cgmath::{Deg, InnerSpace, Matrix4, perspective, Point3, Quaternion, Rad, SquareMatrix, Vector3};
use dashmap::DashMap;
use swap_arc::SwapArc;
use wgpu::{BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType, BufferUsages, Color, ColorTargetState, ColorWrites, DepthStencilState, IndexFormat, LoadOp, Operations, PushConstantRange, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPipeline, Sampler, SamplerBindingType, ShaderSource, ShaderStages, Texture, TextureDimension, TextureFormat, TextureSampleType, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use wgpu::util::StagingBelt;
use wgpu_biolerless::{FragmentShaderState, ModuleSrc, PipelineBuilder, RawTextureBuilder, ShaderModuleSources, State, TextureBuilder, VertexShaderState, WindowSize};
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::window::Window;
use crate::model::{ModelColorVertex, ModelTexVertex, Vertex as MVV};
use crate::utils::LIGHT_GRAY_GPU;
use std::f32::consts::FRAC_PI_2;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Renderer {
    pub state: Arc<State>,
    atlas_pipeline: RenderPipeline,
    tex_ui_pipeline: RenderPipeline,
    color_ui_pipeline: RenderPipeline,
    color_model_pipeline: RenderPipeline,
    tex_model_pipeline: RenderPipeline,
    tex_bind_group_layout: BindGroupLayout,
    camera_bind_group_layout: BindGroupLayout,
    pub model_bind_group_layout: BindGroupLayout,
    pub dimensions: Dimensions,
    glyphs: Mutex<Vec<GlyphInfo>>,
    models: Mutex<Vec<UploadedModel>>,
    depth_tex: SwapArc<TexTriple>,
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

        let camera_bind_group_layout = state.create_bind_group_layout(&[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }]);

        let model_bind_group_layout = state.create_bind_group_layout(&[
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
        ]);

        let depth_tex = TexTriple::create_depth_texture(&state);
        let (width, height) = window.window_size();
        Ok(Self {
            atlas_pipeline: Self::atlas_ui_pipeline(&state),
            tex_ui_pipeline: Self::tex_ui_pipeline(&state),
            color_ui_pipeline: Self::color_ui_pipeline(&state),
            color_model_pipeline: Self::color_model_pipeline(&state, &camera_bind_group_layout),
            tex_model_pipeline: Self::tex_model_pipeline(&state, &model_bind_group_layout, &camera_bind_group_layout),
            state,
            dimensions: Dimensions::new(width, height),
            glyphs: Mutex::new(glyphs),
            tex_bind_group_layout: bgl,
            models: Mutex::new(vec![]),
            camera_bind_group_layout,
            model_bind_group_layout,
            depth_tex: SwapArc::new(Arc::new(depth_tex)),
        })
    }

    pub fn resize(&self, _size: (u32, u32)) {
        self.depth_tex.store(Arc::new(TexTriple::create_depth_texture(&self.state)));
    }

    pub fn add_model(&self, model: crate::model::Model, coloring: ModelColoring) -> usize {
        let mut models = self.models.lock().unwrap();
        let bind_group = match &coloring {
            ModelColoring::Direct(_) => None,
            ModelColoring::Tex(tex) => {
                let bg = self.state.create_bind_group(&self.model_bind_group_layout, &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&tex.view),
                }, BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&tex.sampler),
                }]);
                Some(bg)
            }
        };
        models.push(UploadedModel {
            model,
            coloring,
            bind_group,
        });
        models.len() - 1
    }

    pub fn render(
        &self,
        ui_models: Vec<Model>,
        instances: Vec<ModeledInstance>,
        atlas: Arc<Atlas>, /*atlases: Arc<Mutex<Vec<Arc<Atlas>>>>*/
        camera: &Camera,
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
                    for model in ui_models {
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

                    // setup a buffer before creating the render pass in order to help the
                    // compiler understand that the textures are living long enough.
                    let mut tex_buffer = vec![];

                    for texture_models in texture_models.iter() {
                        let texture_buffer =
                            state.create_buffer(texture_models.1.as_slice(), BufferUsages::VERTEX);

                        let bg = state.create_bind_group(&self.tex_bind_group_layout, &[BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&texture_models.0.view),
                        }, BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&texture_models.0.sampler),
                        }]);
                        tex_buffer.push((texture_buffer, bg));
                    }
                    {
                        let mut texture_models = texture_models.iter();
                        let mut tex_buffer = tex_buffer.iter();
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
                        render_pass.set_pipeline(&self.color_ui_pipeline);
                        render_pass.draw(0..(color_models.len() as u32), 0..1);

                        // println!("tex models: {}", texture_models.len());
                        render_pass.set_pipeline(&self.tex_ui_pipeline);
                        for buf in tex_buffer {
                            let model = texture_models.next().unwrap();
                            render_pass.set_vertex_buffer(0, buf.0.slice(..));
                            render_pass.set_bind_group(0, &buf.1, &[]);
                            render_pass.draw(0..(model.1.len() as u32), 0..1);
                        }
                    }

                    let projection = Projection::new(state.raw_inner_surface_config().width, state.raw_inner_surface_config().height, Deg(90.0/*45.0*/), 0.1, 100.0);

                    let mut camera_uniform = CameraUniform::new();
                    camera_uniform.update_view_proj(camera, &projection);

                    let camera_buffer = state.create_buffer(
                        &[camera_uniform],
                        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    );
                    let camera_bind_group = state.create_bind_group(
                        &self.camera_bind_group_layout,
                        &[BindGroupEntry {
                            binding: 0,
                            resource: camera_buffer.as_entire_binding(),
                        }],
                    );

                    let mut diff_instances = HashSet::new();

                    let models = self.models.lock().unwrap();
                    let mut instance_buffer = vec![vec![]; models.len()];
                    for instance in instances.iter() {
                        instance_buffer[instance.model_id].push(instance.instance.to_raw());
                        diff_instances.insert(instance.model_id);
                    }

                    let mut instance_gpu_buffs = vec![];
                    for instance in instance_buffer.iter() {
                        // FIXME: don't actually create empty buffers for models with no instances!
                        let buf = self.state.create_buffer(instance, BufferUsages::VERTEX);
                        instance_gpu_buffs.push(buf);
                    }

                    {
                        let tex = self.depth_tex.load();
                        let attachment = Some(RenderPassDepthStencilAttachment {
                            view: &tex.view,
                            depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: true }),
                            stencil_ops: None,
                        });
                        let attachments = [Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Load,
                                store: true,
                            },
                        })];
                        let mut render_pass =
                            state.create_render_pass(&mut encoder, &attachments, attachment);
                        // FIXME: try using the same render pass as for UI!

                        // println!("tex models: {}", texture_models.len());
                        render_pass.set_bind_group(0, &camera_bind_group, &[]); // camera bind group
                        for model_id in diff_instances.into_iter() {
                            let model = models.get(model_id).unwrap();
                            /*match &model.coloring {
                                ModelColoring::Direct(color) => {
                                    render_pass.set_pipeline(&self.color_model_pipeline);
                                    render_pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::cast_slice(color));
                                }
                                ModelColoring::Tex(_) => {
                                    render_pass.set_pipeline(&self.tex_model_pipeline);
                                    render_pass.set_bind_group(1, model.bind_group.as_ref().unwrap(), &[]); // texture bind group
                                }
                            }*/
                            render_pass.set_pipeline(&self.tex_model_pipeline);
                            for mesh in model.model.meshes.iter() {
                                println!("idx: {}", model_id);
                                println!("drawing mesh {} : {}", instance_buffer.get(model_id).unwrap().len(), mesh.num_elements);
                                println!("materials: {}", model.model.materials.len());
                                render_pass.set_bind_group(1, &model.model.materials[mesh.material].bind_group, &[]);
                                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                render_pass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint32/*IndexFormat::Uint16*/);
                                render_pass.set_vertex_buffer(1, instance_gpu_buffs.get(model_id).unwrap().slice(..));
                                render_pass.draw_indexed(0..mesh.num_elements, 0, 0..(instance_buffer.get(model_id).unwrap().len() as u32));
                            }
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

    fn color_ui_pipeline(state: &State) -> RenderPipeline {
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

    fn atlas_ui_pipeline(state: &State) -> RenderPipeline {
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

    fn tex_ui_pipeline(state: &State) -> RenderPipeline {
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

    fn tex_model_pipeline(state: &State, bgl: &BindGroupLayout, camera_layout: &BindGroupLayout) -> RenderPipeline {
        PipelineBuilder::new()
            .vertex(VertexShaderState {
                entry_point: "main_vert",
                buffers: &[ModelTexVertex::desc(), InstanceRaw::desc()],
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
                ShaderSource::Wgsl(include_str!("model_texture.wgsl").into()),
            )))
            .layout(&state.create_pipeline_layout(&[camera_layout, bgl], &[]))
            .depth_stencil(DepthStencilState {
                format: TexTriple::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
            .build(state)
    }

    fn color_model_pipeline(state: &State, camera_layout: &BindGroupLayout) -> RenderPipeline {
        PipelineBuilder::new()
            .vertex(VertexShaderState {
                entry_point: "main_vert",
                buffers: &[ModelColorVertex::desc(), InstanceRaw::desc()],
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
                ShaderSource::Wgsl(include_str!("model_color.wgsl").into()),
            )))
            .layout(&state.create_pipeline_layout(&[camera_layout], &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..16,
            }]))
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

struct UploadedModel {
    model: crate::model::Model,
    coloring: ModelColoring,
    bind_group: Option<BindGroup>,
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

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    >(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            Vector3::unit_y(),
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into()
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition {
                                             y: scroll,
                                             ..
                                         }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        if self.rotate_horizontal > 0.0 {
            println!("camera pos {:?} horiz(yaw): {:?} vert(pitch): {:?}", camera.position, self.rotate_horizontal, self.rotate_vertical);
        }
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let scrollward = Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            println!("safe guard 1 | {:?}", camera.pitch);
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
            println!("safe guard 2");
        }
        println!("yaw {:?} pitch {:?}", camera.yaw, camera.pitch);
    }
}

#[derive(Clone)]
pub struct ModeledInstance {
    pub model_id: usize,
    pub instance: Instance,
}

pub enum ModelColoring {
    Direct([f32; 4]),
    Tex(Arc<TexTriple>),
}

#[derive(Clone)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
                VertexAttribute {
                    offset: size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 4,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 5,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 12]>() as BufferAddress,
                    shader_location: 6,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl TexTriple {

    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_depth_texture(state: &State) -> Self {
        let texture = state.create_raw_texture(RawTextureBuilder::new().texture_dimension(TextureDimension::D2)
            .format(Self::DEPTH_FORMAT).dimensions((state.raw_inner_surface_config().width, state.raw_inner_surface_config().height)).usages(wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING));

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = state.device().create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { tex: texture, view, sampler }
    }

}
