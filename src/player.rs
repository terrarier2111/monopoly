use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};

pub struct Player {
    pub name: String,
    pub currency: usize,
    pub id: usize,
    pub character_id: usize,
    pub properties: Vec<usize>,
    pub position: usize,
    pub jail_free_cards: usize,
    pub jail_free_throws: usize,
    pub wait: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub id: usize,
    pub model_path: String,
}

const CHARACTER_PATH: &str = "./characters.json";

pub fn load_characters() -> Vec<Character> {
    if Path::new(CHARACTER_PATH).exists() {
        let mut file = File::open(CHARACTER_PATH).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        serde_json::from_str(&*buf).unwrap()
    } else {
        let mut file = File::create(CHARACTER_PATH).unwrap();
        let characters = vec![];
        file.write_all(serde_json::to_string(&characters).unwrap().as_ref()).unwrap();
        characters
    }
}
