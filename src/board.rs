use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use crate::property::{PropertyFrame, PropertyType};
use serde::{Deserialize, Serialize};

pub const TILES: usize = 40;

pub struct Board {
    pub tiles: [Tile; TILES],
    pub index: BoardIndex,
}

const BOARD_PATH: &str = "./config/board.json";

pub fn load_board() -> Board {
    if Path::new(BOARD_PATH).exists() {
        let mut file = File::open(BOARD_PATH).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        let tiles: Vec<Tile> = serde_json::from_str(&*buf).unwrap();
        let tiles = tiles.try_into().unwrap();
        let index = BoardIndex::new(&tiles);
        Board {
            tiles,
            index,
        }
    } else {
        let mut file = File::create(BOARD_PATH).unwrap();
        file.write_all(serde_json::to_string(&Vec::from(Board::default().tiles)).unwrap().as_ref()).unwrap();
        Board::default()
    }
}


struct SerdeBoard {
    tiles: [Tile; TILES],
}

impl Default for Board {
    fn default() -> Self {
        let tiles = [Tile::Start { name: "Start".to_string() },
            Tile::Property { property: PropertyFrame {
                id: 0,
                name: "DarkBlue1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(1), None] },
            } },
            Tile::DrawCard {
                kind: CardKind::Community,
            },
            Tile::Property { property: PropertyFrame {
                id: 1,
                name: "DarkBlue2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(0), None] },
            } },
            Tile::Pay { name: "Pay1".to_string(), amount: 0 },
            Tile::Property { property: PropertyFrame {
                id: 2,
                name: "Station1".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Station,
            } },
            Tile::Property { property: PropertyFrame {
                id: 3,
                name: "LightBlue1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(4), Some(5)] },
            } },
            Tile::DrawCard {
                kind: CardKind::Chance,
            },
            Tile::Property { property: PropertyFrame {
                id: 4,
                name: "LightBlue2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(3), Some(5)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 5,
                name: "LightBlue3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(3), Some(4)] },
            } },
            Tile::Jail { name: "Jail".to_string() },
            Tile::Property { property: PropertyFrame {
                id: 6,
                name: "Violet1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(8), Some(9)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 7,
                name: "Special1".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Special,
            } },
            Tile::Property { property: PropertyFrame {
                id: 8,
                name: "Violet2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(6), Some(9)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 9,
                name: "Violet3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(6), Some(8)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 10,
                name: "Station2".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Station,
            } },
            Tile::Property { property: PropertyFrame {
                id: 11,
                name: "Brown1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(12), Some(13)] },
            } },
            Tile::DrawCard {
                kind: CardKind::Community,
            },
            Tile::Property { property: PropertyFrame {
                id: 12,
                name: "Brown2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(11), Some(13)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 13,
                name: "Brown3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(11), Some(12)] },
            } },
            Tile::Parking {
                name: "Parking".to_string(),
            },
            Tile::Property { property: PropertyFrame {
                id: 14,
                name: "Red1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(15), Some(16)] },
            } },
            Tile::DrawCard {
                kind: CardKind::Chance,
            },
            Tile::Property { property: PropertyFrame {
                id: 15,
                name: "Red2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(14), Some(16)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 16,
                name: "Red3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(14), Some(15)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 17,
                name: "Station3".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Station,
            } },
            Tile::Property { property: PropertyFrame {
                id: 18,
                name: "Yellow1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(19), Some(21)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 19,
                name: "Yellow2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(18), Some(21)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 20,
                name: "Special2".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Normal { associates: [Some(11), Some(12)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 21,
                name: "Yellow3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(22), Some(23)] },
            } },
            Tile::GoToJail {
                name: "Go to jail".to_string(),
            },
            Tile::Property { property: PropertyFrame {
                id: 22,
                name: "Green1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(23), Some(24)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 23,
                name: "Green2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(22), Some(24)] },
            } },
            Tile::DrawCard {
                kind: CardKind::Community,
            },
            Tile::Property { property: PropertyFrame {
                id: 24,
                name: "Green3".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(22), Some(23)] },
            } },
            Tile::Property { property: PropertyFrame {
                id: 25,
                name: "Station4".to_string(),
                buy_price: 0,
                rents: [Some(0), None, None, None, None, None],
                ty: PropertyType::Station,
            } },
            Tile::DrawCard {
                kind: CardKind::Chance,
            },
            Tile::Property { property: PropertyFrame {
                id: 26,
                name: "OtherBlue1".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(27), None] },
            } },
            Tile::Pay {
                name: "Pay2".to_string(),
                amount: 0,
            },
            Tile::Property { property: PropertyFrame {
                id: 27,
                name: "OtherBlue2".to_string(),
                buy_price: 0,
                rents: [Some(0); 6],
                ty: PropertyType::Normal { associates: [Some(26), None] },
            } },
        ];
        let index = BoardIndex::new(&tiles);
        Self {
            tiles,
            index,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Tile {
    Parking {
        name: String,
    },
    Start {
        name: String,
    },
    Jail {
        name: String,
    },
    GoToJail {
        name: String,
    },
    Property {
        property: PropertyFrame,
    },
    Pay {
        name: String,
        amount: usize,
    },
    DrawCard {
        kind: CardKind,
    },
}

impl Tile {

    pub fn kind(&self) -> TileKind {
        match self {
            Tile::Parking { .. } => TileKind::Parking,
            Tile::Start { .. } => TileKind::Start,
            Tile::Jail { .. } => TileKind::Jail,
            Tile::GoToJail { .. } => TileKind::GoToJail,
            Tile::Property { .. } => TileKind::Property,
            Tile::Pay { .. } => TileKind::Pay,
            Tile::DrawCard { .. } => TileKind::DrawCard,
        }
    }

}

#[derive(Copy, Clone, PartialEq)]
pub enum TileKind {
    Parking,
    Start,
    Jail,
    GoToJail,
    Property,
    Pay,
    DrawCard,
}

#[derive(Debug, Serialize, Deserialize)]
#[repr(usize)]
pub enum CardKind {
    Chance = 0,
    Community = 1,
}

pub struct BoardIndex {
    pub jail: usize,
    pub start: usize,
}

impl BoardIndex {

    pub fn new(board: &[Tile; 40]) -> Self {
        let mut jail_idx = None;
        let mut start_idx = None;
        for x in board.iter().enumerate() {
            if x.1.kind() == TileKind::Jail {
                if jail_idx.replace(x.0).is_some() {
                    panic!("There may only be 1 jail!");
                }
            }
            if x.1.kind() == TileKind::Start {
                if start_idx.replace(x.0).is_some() {
                    panic!("There may only be 1 start!");
                }
            }
        }
        Self {
            jail: jail_idx.expect("No jail was found on the board."),
            start: start_idx.expect("No start was found on the board."),
        }
    }

}
