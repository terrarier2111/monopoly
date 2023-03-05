#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(once_cell)]

use std::fs;
use std::fs::File;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::Rng;
use wgpu::{Features, TextureFormat};
use wgpu_biolerless::{DeviceRequirements, StateBuilder};
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::WindowBuilder;
use crate::action_cards::ActionCard;
use crate::atlas::Atlas;
use crate::board::{Board, Tile};
use crate::model::Model;
use crate::player::{Character, load_characters, Player};
use crate::property::{DefinedProperty, PROPERTIES};
use crate::render::{Camera, CameraController, ModeledInstance, Renderer};
use crate::screen_sys::ScreenSystem;
use crate::screens::login;
use crate::ui::ClickKind;

mod player;
mod property;
mod action_cards;
mod board;
mod ui;
mod render;
mod atlas;
mod screen_sys;
mod screens;
mod utils;
mod model;

fn main() {
    if !Path::new("./config/").exists() {
        fs::create_dir("./config/").unwrap();
    }
    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("Schul-monopoly")
        .build(&event_loop)
        .unwrap();
    let mut req = DeviceRequirements::default();
    req.features |= Features::PUSH_CONSTANTS;
    req.limits.max_push_constant_size = 16;
    let state = Arc::new(pollster::block_on(
        StateBuilder::new().window(&window).device_requirements(req).build(),
    ).unwrap());
    let renderer = Arc::new(Renderer::new(state.clone(), &window).unwrap());

    let game = Arc::new(Game::new(renderer.clone()));

    game.screen_sys.push_screen(Box::new(login::Login::new(Arc::new(Mutex::new(game.characters.clone())))));

    let mut mouse_pos = (0.0, 0.0);
    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(_) => {}
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            game.camera_controller.lock().unwrap().process_events(event);
            match event {
                WindowEvent::Resized(size) => {
                    if !state.resize(*size) {
                        println!("Couldn't resize!");
                    } else {
                        game.renderer.dimensions.set(size.width, size.height);
                    }
                }
                WindowEvent::Moved(_) => {}
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Destroyed => {}
                WindowEvent::DroppedFile(_) => {}
                WindowEvent::HoveredFile(_) => {}
                WindowEvent::HoveredFileCancelled => {}
                WindowEvent::ReceivedCharacter(_) => {}
                WindowEvent::Focused(_) => {}
                WindowEvent::KeyboardInput { .. } => {}
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::CursorMoved { position, .. } => {
                    let (width, height) = game.renderer.dimensions.get();
                    mouse_pos = (position.x / width as f64, 1.0 - position.y / height as f64);
                    game.screen_sys.on_mouse_hover(&game, mouse_pos);
                }
                WindowEvent::CursorEntered { .. } => {}
                WindowEvent::CursorLeft { .. } => {}
                WindowEvent::MouseWheel { .. } => {}
                WindowEvent::MouseInput { button, state, .. } => {
                    if button == &MouseButton::Left {
                        game.screen_sys.on_mouse_click(&game, mouse_pos, if state == &ElementState::Pressed {
                            ClickKind::PressDown
                        } else {
                            ClickKind::Release
                        });
                    }
                }
                WindowEvent::TouchpadPressure { .. } => {}
                WindowEvent::AxisMotion { .. } => {}
                WindowEvent::Touch(_) => {}
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    if !state.resize(**new_inner_size) {
                        println!("Couldn't resize!");
                    }
                }
                WindowEvent::ThemeChanged(_) => {}
                WindowEvent::Ime(_) => {}
                WindowEvent::Occluded(_) => {}
                WindowEvent::TouchpadMagnify { .. } => {}
                WindowEvent::SmartMagnify { .. } => {}
                WindowEvent::TouchpadRotate { .. } => {}
            }
        },
        Event::DeviceEvent { .. } => {}
        Event::UserEvent(_) => {}
        Event::Suspended => {}
        Event::Resumed => {}
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            // FIXME: perform redraw
            let models = game.screen_sys.tick(&game, &window);
            let mut camera = game.camera.lock().unwrap();
            game.camera_controller.lock().unwrap().update_camera(&mut camera);
            renderer.render(models, vec![], game.atlas.clone(), &camera);
        }
        Event::RedrawEventsCleared => {}
        Event::LoopDestroyed => {}
        _ => {}
    })
}

const INITIAL_CURRENCY: usize = 400; // TODO: make this configurable!

pub struct Game {
    pub players: Mutex<Vec<Player>>,
    pub properties: [Mutex<DefinedProperty>; PROPERTIES],
    pub cards: Vec<ActionCard>,
    pub card_stacks: [Mutex<Vec<usize>>; 2],
    pub curr_player: AtomicUsize,
    pub board: Mutex<Board>,
    pub game_state: Mutex<GameState>,
    pub screen_sys: Arc<ScreenSystem>,
    pub renderer: Arc<Renderer>,
    pub atlas: Arc<Atlas>,
    pub characters: Vec<Character>,
    pub models: Mutex<Vec<ModeledInstance>>,
    pub camera: Mutex<Camera>,
    pub camera_controller: Mutex<CameraController>,
}

impl Game {

    pub fn new(renderer: Arc<Renderer>) -> Self {
        let board = board::load_board();
        let mut players = vec![];

        let mut properties = MaybeUninit::uninit_array();
        let mut idx = 0;
        for tile in board.tiles.iter() {
            if let Tile::Property { property } = tile {
                properties[idx].write(Mutex::new(DefinedProperty {
                    frame: property.clone(),
                    houses: 0,
                    owner: None,
                }));
                idx += 1;
            }
        }
        let cards = action_cards::load_cards();
        let mut first_card_stack = vec![];
        for _ in 0..(cards.len() / 2) {
            first_card_stack.push(rand::thread_rng().gen_range(0..(cards.len())));
        }
        let mut second_card_stack = vec![];
        for x in 0..cards.len() {
            if !first_card_stack.contains(&x) {
                second_card_stack.push(x);
            }
        }

        let atlas = Arc::new(Atlas::new(renderer.state.clone(), (1024, 1024), TextureFormat::Rgba8Unorm));
        let camera = Mutex::new(Camera::new(&renderer.state));

        Self {
            players: Mutex::new(players),
            properties: unsafe { MaybeUninit::array_assume_init(properties) },
            cards,
            card_stacks: [Mutex::new(first_card_stack), Mutex::new(second_card_stack)],
            curr_player: AtomicUsize::new(0),
            board: Mutex::new(board),
            game_state: Mutex::new(GameState::Login),
            screen_sys: Arc::new(ScreenSystem::new()),
            renderer,
            atlas,
            characters: load_characters(),
            models: Mutex::new(vec![]),
            camera,
            camera_controller: Mutex::new(CameraController::new(0.2)),
        }
    }

    pub fn tick(&self) {
        let curr_player = self.curr_player.load(Ordering::Acquire);
        let players = self.players.lock().unwrap().len();
        if players != 0 {
            self.curr_player.store((curr_player + 1) % players, Ordering::Release);
        }

    }

    pub fn add_player(&self, char_id: usize) {
        let mut players = self.players.lock().unwrap();
        let len = players.len();
        players.push(Player {
            name: String::new(), // FIXME: implement text fields to enable players to choose names.
            currency: INITIAL_CURRENCY,
            id: len,
            character_id: char_id,
            properties: vec![],
            position: self.board.lock().unwrap().index.start,
            jail_free_cards: 0,
            jail_free_throws: 0,
            wait: 0,
        });
    }

}

#[derive(Copy, Clone, PartialEq)]
pub enum GameState {
    Login,
    InGame,
    Finished,
}
