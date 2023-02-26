use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use crate::render::Renderer;
use crate::screen_sys::Screen;
use crate::ui::{Button, Color, ColorBox, Coloring, Container, TextBox, TextSection};
use crate::{Game, ScreenSystem};
use std::sync::{Arc, Mutex, RwLock};
use rand::Rng;
use wgpu_glyph::{HorizontalAlign, Layout, Text, VerticalAlign};
use crate::utils::DARK_GRAY_UI;

#[derive(Clone)]
pub struct Login {
    container: Arc<Container>,
}

impl Login {
    pub fn new(accounts: Arc<Mutex<Vec<>>>) -> Self {
        Self {
            container: Arc::new(Container::new()),
        }
    }
}

impl Screen for Login {
    fn on_active(&mut self, game: &Arc<Game>) {
        let entry_offset = 1.0 / ENTRIES_ON_PAGE as f32;
        for entry in game.config.fav_servers.iter().enumerate() {
            self.container.add(Arc::new(RwLock::new(Box::new(Button {
                inner_box: TextBox {
                    pos: (0.0, 1.0 - ((entry.0 + 1) as f32 * entry_offset)),
                    width: 0.2,
                    height: 0.1,
                    coloring: Coloring::Color([
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                        DARK_GRAY_UI,
                    ]),
                    text: TextSection {
                        layout: Layout::default_single_line().v_align(VerticalAlign::Bottom/*Bottom*//*VerticalAlign::Center*/).h_align(HorizontalAlign::Left),
                        text: vec![Text::default().with_scale(30.0)],
                        texts: vec![entry.1.name.clone()],
                    }
                },
                data: None,
                on_click: Arc::new(Box::new(|button, client| {
                    println!("test!!");
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

                }))
            }))));
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

    fn tick(&mut self, _game: &Arc<Game>, _delta: f64) {}

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