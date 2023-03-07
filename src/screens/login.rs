use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::Path;
use crate::render::{Renderer, TexTriple, TexTy};
use crate::screen_sys::Screen;
use crate::ui::{Button, Color, ColorBox, Coloring, Container, Tex, TextBox, TextSection};
use crate::{Game, GameState, ScreenSystem, ui};
use std::sync::{Arc, Mutex, RwLock};
use image::{EncodableLayout, GenericImageView, RgbaImage};
use rand::Rng;
use wgpu::{Sampler, SamplerDescriptor, TextureAspect, TextureDimension, TextureFormat, TextureViewDescriptor};
use wgpu_biolerless::TextureBuilder;
use wgpu_glyph::{HorizontalAlign, Layout, Text, VerticalAlign};
use crate::player::Character;
use crate::screens::in_game::InGame;
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
        let local = Path::new("./config/eiffelturm.jpg");
        println!("{:?}", local.canonicalize().unwrap());
        let entry_offset = 1.0 / (self.chars.lock().unwrap().len() + 3) as f32;
        for char in self.chars.lock().unwrap().iter().enumerate() {
            println!("path: {}", char.1.model_path);
            let mut buf = image::open(Path::new(&char.1.model_path).canonicalize().unwrap()).unwrap();
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
                Arc::new(Box::new(|button: &mut Button<'_, (Arc<RgbaImage>, usize)>, game| {
                    if let Coloring::Tex(tex) = &mut button.inner_box.coloring {
                        if !tex.grayscale_conv {
                            game.add_player(button.data.as_mut().unwrap().1);
                        }
                        tex.grayscale_conv = true;
                    }
                })),
                Some((buf, char.1.id))
            )))));
        }
        let mut buf = image::open("./resources/eiffelturm.jpg").unwrap();
        let buf = Arc::new(buf.into_rgba8());
        let tex = game.renderer.state.create_texture(TextureBuilder::new().data(buf.as_bytes())
            .format(TextureFormat::Rgba8UnormSrgb).texture_dimension(TextureDimension::D2).dimensions(buf.dimensions()));
        let view = tex.create_view(&TextureViewDescriptor::default());
        self.container.add(Arc::new(RwLock::new(Box::new(Button::new(
            TextBox::new(
                (0.35, entry_offset * 1.5),
                0.3,
                buf.height() as f32 / (buf.width() as f32 / 0.3),
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
                    text: vec![],
                    texts: vec![],
                }
            ),
            Arc::new(Box::new(|button: &mut Button<'_, Arc<RgbaImage>>, game| {
                println!("start game!");
                *game.game_state.lock().unwrap() = GameState::InGame;
                game.screen_sys.push_screen(Box::new(InGame::new()));

            })),
            Some(buf)
        )))));
    }

    fn on_deactive(&mut self, _game: &Arc<Game>) {}

    fn tick(&mut self, _game: &Arc<Game>) {}

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
