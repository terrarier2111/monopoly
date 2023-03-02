use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use crate::render::{Renderer, TexTriple, TexTy};
use crate::screen_sys::Screen;
use crate::ui::{Button, Color, ColorBox, Coloring, Container, Tex, TextBox, TextSection};
use crate::{Game, ScreenSystem, ui};
use std::sync::{Arc, Mutex, RwLock};
use image::{EncodableLayout, GenericImageView};
use rand::Rng;
use wgpu::{Sampler, SamplerDescriptor, TextureAspect, TextureDimension, TextureFormat, TextureViewDescriptor};
use wgpu_biolerless::TextureBuilder;
use wgpu_glyph::{HorizontalAlign, Layout, Text, VerticalAlign};
use crate::player::Character;
use crate::utils::DARK_GRAY_UI;

#[derive(Clone)]
pub struct Login {
    container: Arc<Container>,
    chars: Arc<Mutex<Vec<Character>>>,
}

impl Login {
    pub fn new(chars: Arc<Mutex<Vec<Character>>>) -> Self {
        Self {
            container: Arc::new(Container::new()),
            chars,
        }
    }
}

impl Screen for Login {
    fn on_active(&mut self, game: &Arc<Game>) {
        let entry_offset = 1.0 / (self.chars.lock().unwrap().len() + 3) as f32;
        for char in self.chars.lock().unwrap().iter().enumerate() {
            let mut buf = image::open(&char.1.model_path).unwrap();
            // let mut buf = image::load_from_memory(include_bytes!("../../config/joda.jpg")).unwrap();
            let buf = Arc::new(buf.into_rgba8());
            let tex = game.renderer.state.create_texture(TextureBuilder::new().data(buf.as_bytes())
                .format(TextureFormat::Rgba8UnormSrgb/*TextureFormat::Rgba8Uint*/).texture_dimension(TextureDimension::D2).dimensions(buf.dimensions()));
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
                        }))/*TexTy::Atlas(game.atlas.alloc(char.1.model_path.clone(), buf.dimensions(), buf.as_bytes()))*//*TexTy::Simple()*/,
                        grayscale_conv: false,
                    })/*Coloring::Color([
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                    ])*/,
                    TextSection {
                        layout: Layout::default_single_line().v_align(VerticalAlign::Bottom/*Bottom*//*VerticalAlign::Center*/).h_align(HorizontalAlign::Left),
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
                    /*match button.inner_box.coloring {
                        Coloring::Color(mut color) => {
                            color[0].r += 0.1;
                            color[1].r += 0.1;
                            color[2].r += 0.1;
                            color[3].r += 0.1;
                            color[4].r += 0.1;
                            color[5].r += 0.1;
                        }
                        Coloring::Tex(_) => {}
                    }
                    button.inner_box.pos.0 += 0.1;*/

                })),
                Some(buf)
                )))));
        }
        /*self.container.add(Arc::new(RwLock::new(Box::new(ColorBox {
            pos: (0.25, 0.25),
            width: 0.5,
            height: 0.5,
            coloring: Coloring::Color([
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
            ]),
        }))));
        self.container.add(Arc::new(RwLock::new(Box::new(TextBox {
            pos: (0.0, 0.0),
            width: 0.5,
            height: 0.5,
            coloring: Coloring::Color([
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 0.2,
                },
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.2,
                },
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 0.2,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 0.2,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 0.2,
                },
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 0.2,
                },
            ]),
            text: TextSection { layout: Default::default(), text: vec![Text::new("Teste").with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(500.0)] }
        }))));*/
    }

    fn on_deactive(&mut self, _game: &Arc<Game>) {}

    fn tick(&mut self, _game: &Arc<Game>) {}

    fn is_closable(&self) -> bool {
        false
    }

    fn is_tick_always(&self) -> bool {
        true
    }

    fn container(&self) -> &Arc<Container> {
        &self.container
    }

    fn clone_screen(&self) -> Box<dyn Screen> {
        Box::new(self.clone())
    }

}
