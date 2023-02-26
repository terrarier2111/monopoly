use serde::{Deserialize, Serialize};

pub const PROPERTIES: usize = 28; // 22 normal, 4 stations, 2 special
pub const MAX_HOUSES: usize = 5;

pub struct DefinedProperty {
    pub frame: PropertyFrame,
    pub houses: usize,
    pub owner: Option<usize>,
}

impl DefinedProperty {

    pub fn calculate_price(&self, moves: usize) -> usize {
        match &self.frame.ty {
            PropertyType::Normal { .. } | PropertyType::Station => self.frame.rents[self.houses].unwrap(),
            PropertyType::Special => self.frame.rents[0].unwrap() * moves,
        }
    }

}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyFrame {
    pub id: usize,
    pub name: String,
    pub buy_price: usize,
    pub rents: [Option<usize>; 1 + MAX_HOUSES],
    pub ty: PropertyType,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PropertyType {
    Normal {
        associates: [Option<usize>; 2], // the ids of the associates
    },
    Station,
    Special,
}
