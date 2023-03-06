use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use crate::render::{Instance, ModelColoring, ModeledInstance, Renderer, TexTriple, TexTy};
use crate::screen_sys::Screen;
use crate::ui::{Button, Color, ColorBox, Coloring, Container, Tex, TextBox, TextSection};
use crate::{Game, ScreenSystem, ui};
use std::sync::{Arc, Mutex, RwLock};
use cgmath::{Deg, Quaternion, Rotation3, Vector3};
use image::{EncodableLayout, GenericImageView};
use rand::Rng;
use wgpu::{Sampler, SamplerDescriptor, TextureAspect, TextureDimension, TextureFormat, TextureViewDescriptor};
use wgpu_biolerless::TextureBuilder;
use wgpu_glyph::{HorizontalAlign, Layout, Text, VerticalAlign};
use crate::player::Character;
use crate::utils::DARK_GRAY_UI;

#[derive(Clone)]
pub struct InGame {
    container: Arc<Container>,
    board_id: usize,
}

impl InGame {
    pub fn new() -> Self {
        Self {
            container: Arc::new(Container::new()),
            board_id: 0,
        }
    }
}

impl Screen for InGame {
    fn init(&mut self, game: &Arc<Game>) {
        let mut buf = image::open("./resources/board.jpg").unwrap();
        let buf = Arc::new(buf.into_rgba8());
        let tex = game.renderer.state.create_texture(TextureBuilder::new().data(buf.as_bytes())
            .format(TextureFormat::Rgba8UnormSrgb).texture_dimension(TextureDimension::D2).dimensions(buf.dimensions()));
        let view = tex.create_view(&TextureViewDescriptor::default());
        let tex = Arc::new(TexTriple {
            tex,
            view,
            sampler: game.renderer.state.device().create_sampler(&SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
        });
        self.board_id = game.renderer.add_model(crate::model::rectangle_model(&game.renderer.state, (0.0, 0.0), 1.0, 1.0), ModelColoring::Tex(tex));
    }

    fn on_active(&mut self, _game: &Arc<Game>) {
        /*let mut buf = image::open(&char.1.model_path).unwrap();
        let buf = Arc::new(buf.into_rgba8());
        let tex = game.renderer.state.create_texture(TextureBuilder::new().data(buf.as_bytes())
            .format(TextureFormat::Rgba8UnormSrgb).texture_dimension(TextureDimension::D2).dimensions(buf.dimensions()));
        let view = tex.create_view(&TextureViewDescriptor::default());
        self.container.add(Arc::new(RwLock::new(Box::new(Button::new(
            TextBox::new(
                (((char.0 + 1) as f32 * entry_offset), 1.0 - entry_offset * 1.5),
                0.1,
                0.2,
                Coloring::Tex(Tex {
                    ty: TexTy::Simple(Arc::new(TexTriple {
                        tex,
                        view,
                        sampler: game.renderer.state.device().create_sampler(&SamplerDescriptor {
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Linear,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        }),
                    })),
                    grayscale_conv: false,
                }),
                TextSection {
                    layout: Layout::default_single_line().v_align(VerticalAlign::Bottom).h_align(HorizontalAlign::Left),
                    text: vec![Text::default().with_scale(30.0)],
                    texts: vec![char.1.name.clone()],
                }
            ),
            Arc::new(Box::new(|button, game| {
                println!("test!!");
                match &mut button.inner_box.coloring {
                    Coloring::Color(_) => {}
                    Coloring::Tex(tex) => {
                        tex.grayscale_conv = true;
                    }
                }

            })),
            Some(buf)
        )))));*/

        /*let mut buf = image::open("./resources/board.jpg").unwrap();
        let buf = Arc::new(buf.into_rgba8());
        let tex = game.renderer.state.create_texture(TextureBuilder::new().data(buf.as_bytes())
            .format(TextureFormat::Rgba8UnormSrgb).texture_dimension(TextureDimension::D2).dimensions(buf.dimensions()));
        let view = tex.create_view(&TextureViewDescriptor::default());
        self.container.add(Arc::new(RwLock::new(Box::new(ColorBox {
            pos: (0.0, 0.0),
            width: 1.0,
            height: 1.0,
            coloring: Coloring::Tex(Tex {
                ty: TexTy::Simple(Arc::new(TexTriple {
                    tex,
                    view,
                    sampler: game.renderer.state.device().create_sampler(&SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Nearest,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        ..Default::default()
                    }),
                })),
                grayscale_conv: false,
            }),
        }))));*/
    }

    fn on_deactive(&mut self, _game: &Arc<Game>) {}

    fn tick(&mut self, game: &Arc<Game>) {
        game.models.lock().unwrap().push(ModeledInstance {
            model_id: self.board_id,
            instance: Instance { position: Vector3::unit_z() , rotation: Quaternion::from_angle_x(Deg(0.0)) },
        });
        println!("adding model!");
    }

    fn is_closable(&self) -> bool {
        false
    }

    fn is_tick_always(&self) -> bool {
        false
    }

    fn container(&self) -> &Arc<Container> {
        &self.container
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }

}
