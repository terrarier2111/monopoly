use crate::atlas::UV;
use crate::render::{ColorSource, Model, TexTriple, TexTy, Vertex};
use crate::screen_sys::ScreenSystem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use atomicfloat::AtomicF64;
use fontdue::{Font, FontSettings};
use wgpu::{Sampler, Texture, TextureView};
use wgpu_glyph::{BuiltInLineBreaker, Extra, Layout, Section, Text};
use crate::{Game, Renderer};

pub trait Component: Send + Sync {
    fn build_model(&self) -> Model;

    // fn is_inbounds(&self, pos: (f32, f32)) -> bool; // FIXME: is this one better?

    fn do_render(&self, _game: &Arc<Game>) {}

    fn pos(&self) -> (f32, f32);

    fn dims(&self) -> (f32, f32);

    fn on_click(&mut self, game: &Arc<Game>, click_kind: ClickKind);

    fn on_click_outside(&mut self, game: &Arc<Game>);

    fn on_scroll(&mut self, game: &Arc<Game>);

    fn on_hover(&mut self, game: &Arc<Game>, mode: HoverMode);
}

#[derive(Copy, Clone, PartialEq)]
pub enum ClickKind {
    PressDown,
    Release,
}

#[derive(Copy, Clone)]
pub enum HoverMode {
    Enter,
    Exit,
}

pub struct UIComponent {
    inner: Arc<InnerUIComponent>,
}

impl UIComponent {
    pub fn build_model(&self) -> Model {
        self.inner.build_model()
    }

    pub fn on_click(&self, game: &Arc<Game>, click_kind: ClickKind) {
        self.inner.inner.write().unwrap().on_click(game, click_kind);
        self.inner.make_dirty();
    }

    pub fn on_click_outside(&self, game: &Arc<Game>) {
        self.inner.inner.write().unwrap().on_click_outside(game);
        self.inner.make_dirty();
    }

    pub fn is_inbounds(&self, pos: (f32, f32)) -> bool {
        let inner = self.inner.inner.read().unwrap();
        let dims = inner.dims();
        let inner_pos = inner.pos();
        // println!("pos: {:?}", pos);
        let bounds = (inner_pos.0 + dims.0, inner_pos.1 + dims.1);
        // println!("higher than comp start ({:?}): {}", inner_pos, (pos.0 >= inner_pos.0 && pos.1 >= inner_pos.1));
        // println!("lower than comp end: ({:?}): {}", bounds, (pos.0 <= bounds.0 && pos.1 <= bounds.1));
        (pos.0 >= inner_pos.0 && pos.1 >= inner_pos.1) && (pos.0 <= bounds.0 && pos.1 <= bounds.1)
    }
}

pub struct InnerUIComponent {
    inner: Arc<RwLock<Box<dyn Component>>>, // FIXME: should we prefer a Mutex over a Rwlock?
    precomputed_model: Mutex<Model>,
    dirty: AtomicBool,
}

impl InnerUIComponent {
    fn build_model(&self) -> Model {
        if self.dirty.fetch_and(false, Ordering::AcqRel) {
            let model = self.inner.write().unwrap().build_model();
            *self.precomputed_model.lock().unwrap() = model.clone();
            model
        } else {
            self.precomputed_model.lock().unwrap().clone()
        }
    }

    pub fn make_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }
}

#[derive(Copy, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {

    pub fn scale(mut self, factor: f32) -> Self {
        self.r *= factor;
        self.b *= factor;
        self.g *= factor;
        self
    }

    pub fn into_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

pub struct Tex {
    // pub alpha: f32, // FIXME: try readding this!
    pub ty: TexTy,
}

pub enum Coloring<const VERTICES: usize> {
    Color([Color; VERTICES]),
    Tex(Tex),
}

pub struct ScrollData {
    min_y: AtomicF64,
    max_y: AtomicF64,
    min_x: AtomicF64,
    max_x: AtomicF64,
    offset_x: AtomicF64,
    offset_y: AtomicF64,
}

impl Default for ScrollData {
    fn default() -> Self {
        Self {
            min_y: AtomicF64::new(0.0),
            max_y: AtomicF64::new(0.0),
            min_x: AtomicF64::new(0.0),
            max_x: AtomicF64::new(0.0),
            offset_x: AtomicF64::new(0.0),
            offset_y: AtomicF64::new(0.0),
        }
    }
}

#[derive(Default)]
pub struct Container {
    components: RwLock<Vec<UIComponent>>,
    scroll_data: ScrollData, // FIXME: use this for scroll sliders
}

impl Container {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(self: &Arc<Self>, component: Arc<RwLock<Box<dyn Component>>>) {
        let model = component.read().unwrap().build_model();
        self.components.write().unwrap().push(UIComponent {
            inner: Arc::new(InnerUIComponent {
                inner: component,
                precomputed_model: Mutex::new(model),
                dirty: AtomicBool::new(false),
            }),
        });
    }

    pub fn build_models(&self, game: &Arc<Game>) -> Vec<Model> {
        let mut models = vec![];
        for component in self.components.read().unwrap().iter() {
            models.push(component.build_model());
            component.inner.inner.read().unwrap().do_render(game);
        }
        models
    }

    pub fn on_mouse_click(&self, game: &Arc<Game>, pos: (f64, f64), click_kind: ClickKind) {
        let mut found = false;
        for component in self.components.read().unwrap().iter() {
            if !found && component.is_inbounds((pos.0 as f32, pos.1 as f32)) { // FIXME: switch to using f64 instead!
                component.on_click(game, click_kind);
                found = true;
            } else {
                component.on_click_outside(game);
            }
        }
    }
}

pub struct Button<'a, T = ()> {
    pub inner_box: TextBox<'a>,
    pub data: Option<Box<T>>,
    pub on_click: Arc<Box<dyn Fn(&mut Button, &Arc<Game>) + Send + Sync>>,
}

impl Component for Button<'_> {
    fn build_model(&self) -> Model {
        self.inner_box.build_model()
    }

    fn do_render(&self, game: &Arc<Game>) {
        self.inner_box.do_render(game)
    }

    fn pos(&self) -> (f32, f32) {
        self.inner_box.pos()
    }

    fn dims(&self) -> (f32, f32) {
        self.inner_box.dims()
    }

    fn on_click(&mut self, game: &Arc<Game>, _click_kind: ClickKind) {
        let func = self.on_click.clone();
        func(self, game);
    }

    fn on_click_outside(&mut self, _game: &Arc<Game>) {}

    fn on_scroll(&mut self, _game: &Arc<Game>) {}

    fn on_hover(&mut self, _game: &Arc<Game>, _mode: HoverMode) {}
}

pub struct ColorBox {
    pub pos: (f32, f32),
    pub width: f32,
    pub height: f32,
    pub coloring: Coloring<6>,
}

impl Component for ColorBox {
    fn build_model(&self) -> Model {
        let (x_off, y_off) = ((2.0 * self.pos.0), (2.0 * self.pos.1));
        let vertices = [
            [-1.0 + x_off, -1.0 + y_off],
            [2.0 * self.width - 1.0 + x_off, -1.0 + y_off],
            [
                2.0 * self.width - 1.0 + x_off,
                2.0 * self.height - 1.0 + y_off,
            ],
            [-1.0 + x_off, -1.0 + y_off],
            [-1.0 + x_off, 2.0 * self.height - 1.0 + y_off],
            [
                2.0 * self.width - 1.0 + x_off,
                2.0 * self.height - 1.0 + y_off,
            ],
        ];
        let vertices = match &self.coloring {
            Coloring::Color(colors) => {
                let mut ret = Vec::with_capacity(6);
                for (i, pos) in vertices.into_iter().enumerate() {
                    ret.push(Vertex::Color {
                        pos,
                        color: colors[i].into_array(),
                    });
                }
                ret
            }
            Coloring::Tex(tex) => {
                let mut ret = Vec::with_capacity(6);
                for pos in vertices {
                    ret.push(Vertex::Atlas {
                        pos,
                        alpha: 1.0, // FIXME: make this actually parameterized!
                        uv: match &tex.ty {
                            TexTy::Atlas(atlas) => atlas.uv().into_tuple(),
                        },
                    });
                }
                ret
            }
        };
        Model {
            vertices,
            color_src: match &self.coloring {
                Coloring::Color(_) => ColorSource::PerVert,
                Coloring::Tex(tex) => match &tex.ty {
                    TexTy::Atlas(atlas) => ColorSource::Atlas(atlas.atlas().clone()),
                },
            },
        }
    }

    fn pos(&self) -> (f32, f32) {
        self.pos
    }

    fn dims(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    fn on_click(&mut self, _game: &Arc<Game>, _click_kind: ClickKind) {}

    fn on_click_outside(&mut self, _game: &Arc<Game>) {}

    fn on_scroll(&mut self, _game: &Arc<Game>) {}

    fn on_hover(&mut self, _game: &Arc<Game>, _mode: HoverMode) {}
}

pub struct TextBox<'a> {
    pub pos: (f32, f32),
    pub width: f32,
    pub height: f32,
    pub coloring: Coloring<6>,
    pub text: TextSection<'a>,
    hovered: bool,
    pressed: bool,
}

impl<'a> TextBox<'a> {

    pub fn new(pos: (f32, f32), width: f32, height: f32, coloring: Coloring<6>, text: TextSection<'a>) -> Self {
        Self {
            pos,
            width,
            height,
            coloring,
            text,
            hovered: false,
            pressed: false,
        }
    }

}

impl Component for TextBox<'_> {
    fn build_model(&self) -> Model {
        let (x_off, y_off) = ((2.0 * self.pos.0), (2.0 * self.pos.1));
        let vertices = [
            [-1.0 + x_off, -1.0 + y_off],
            [2.0 * self.width - 1.0 + x_off, -1.0 + y_off],
            [
                2.0 * self.width - 1.0 + x_off,
                2.0 * self.height - 1.0 + y_off,
            ],
            [-1.0 + x_off, -1.0 + y_off],
            [-1.0 + x_off, 2.0 * self.height - 1.0 + y_off],
            [
                2.0 * self.width - 1.0 + x_off,
                2.0 * self.height - 1.0 + y_off,
            ],
        ];
        let vertices = match &self.coloring {
            Coloring::Color(colors) => {
                let mut ret = Vec::with_capacity(6);
                for (i, pos) in vertices.into_iter().enumerate() {
                    ret.push(Vertex::Color {
                        pos,
                        color: colors[i].scale(if self.hovered {
                            0.8
                        } else {
                            1.0
                        }).scale(if self.pressed {
                            0.8
                        } else {
                            1.0
                        }).into_array(),
                    });
                }
                ret
            }
            Coloring::Tex(tex) => {
                // FIXME: support darkening textures as well!
                let mut ret = Vec::with_capacity(6);
                for pos in vertices {
                    ret.push(Vertex::Atlas {
                        pos,
                        alpha: 1.0, // FIXME: make this actually parameterized!
                        uv: match &tex.ty {
                            TexTy::Atlas(atlas) => atlas.uv().into_tuple(),
                        },
                    });
                }
                ret
            }
        };
        Model {
            vertices,
            color_src: match &self.coloring {
                Coloring::Color(_) => ColorSource::PerVert,
                Coloring::Tex(tex) => match &tex.ty {
                    TexTy::Atlas(atlas) => ColorSource::Atlas(atlas.atlas().clone()),
                },
            },
        }
    }

    fn do_render(&self, game: &Arc<Game>) {
        let (width, height) = game.renderer.dimensions.get();
        game.renderer.queue_glyph(0, Section {
            screen_position: (self.pos.0 * width as f32/*(self.pos.0 - 1.0) / 2.0*/, /*0.0*/(1.0 - self.pos.1/* - self.height*/) * height as f32/*(self.pos.1 - 1.0) / 2.0*/),
            bounds: (self.width * width as f32, self.height * height as f32),
            layout: self.text.layout,
            text: self.text.text.iter().enumerate().map(|txt| {
                txt.1.with_text(&*self.text.texts[txt.0])
            }).collect::<Vec<_>>(),
        });
    }

    fn pos(&self) -> (f32, f32) {
        self.pos
    }

    fn dims(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    fn on_click(&mut self, _game: &Arc<Game>, _click_kind: ClickKind) {
        // FIXME: add release and down as a parameter and use it to handle pressed
    }

    fn on_click_outside(&mut self, _game: &Arc<Game>) {}

    fn on_scroll(&mut self, _game: &Arc<Game>) {}

    fn on_hover(&mut self, _game: &Arc<Game>, _mode: HoverMode) {
        self.hovered = true;
    }
}

pub struct TextSection<'a, X = Extra> {
    /// Built in layout, can be overridden with custom layout logic see queue_custom_layout
    pub layout: Layout<BuiltInLineBreaker>,
    /// Text to render, rendered next to one another according the layout.
    pub text: Vec<Text<'a, X>>,
    pub texts: Vec</*Arc<*/String/*>*/>,
}

pub struct InputBox<'a> {
    pub inner_box: TextBox<'a>,
    active: bool,
}

impl Component for InputBox<'_> {
    fn build_model(&self) -> Model {
        // FIXME: handle inner active!
        self.inner_box.build_model()
    }

    fn do_render(&self, game: &Arc<Game>) {
        self.inner_box.do_render(game)
    }

    fn pos(&self) -> (f32, f32) {
        self.inner_box.pos
    }

    fn dims(&self) -> (f32, f32) {
        (self.inner_box.width, self.inner_box.height)
    }

    fn on_click(&mut self, _game: &Arc<Game>, _click_kind: ClickKind) {
        self.active = true;
    }

    fn on_click_outside(&mut self, _game: &Arc<Game>) {
        self.active = false;
    }

    fn on_scroll(&mut self, _game: &Arc<Game>) {}

    fn on_hover(&mut self, _game: &Arc<Game>, _mode: HoverMode) {}
}
