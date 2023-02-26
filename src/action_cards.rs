use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use rand::Rng;
use serde::{Deserialize, Serialize};

const ACTION_CARDS_PATH: &str = "./action_cards.json";

pub fn load_cards() -> Vec<ActionCard> {
    if Path::new(ACTION_CARDS_PATH).exists() {
        let mut file = File::open(ACTION_CARDS_PATH).unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        serde_json::from_str(&*buf).unwrap()
    } else {
        let mut file = File::create(ACTION_CARDS_PATH).unwrap();
        let cards = vec![ActionCard {
            text: "Go to jail".to_string(),
            action: Action::GoToJail,
        }, ActionCard {
            text: "Pay 2$".to_string(),
            action: Action::DirectCurrency { amount: -2, },
        }, ActionCard {
            text: "Get 2$".to_string(),
            action: Action::DirectCurrency { amount: 2, },
        }, ActionCard {
            text: "Pay everybody 2".to_string(),
            action: Action::DistributeCurrency { amount: -2 },
        }, ActionCard {
            text: "Everybody pays you 2".to_string(),
            action: Action::DistributeCurrency { amount: 2 },
        }, ActionCard {
            text: "Wait 1 round".to_string(),
            action: Action::Wait { rounds: 1 },
        }, ActionCard {
            text: "Go 2 tiles back".to_string(),
            action: Action::MoveRelative { amount: -2 },
        }, ActionCard {
            text: "Go 2 tiles forward".to_string(),
            action: Action::MoveRelative { amount: 2 },
        }, ActionCard {
            text: "Go to the first tile".to_string(),
            action: Action::MoveAbsolute { tile: 0 },
        }, ActionCard {
            text: "Jail free card".to_string(),
            action: Action::JailFree,
        },];
        file.write_all(serde_json::to_string(&cards).unwrap().as_ref()).unwrap();
        cards
    }
}

#[derive(Serialize, Deserialize)]
pub struct ActionCard {
    pub text: String,
    pub action: Action,
}

#[derive(Serialize, Deserialize)]
pub enum Action {
    // currency is exchanged between the player and the bank
    DirectCurrency {
        amount: isize,
    },
    // currency is exchanged between players
    DistributeCurrency {
        amount: isize,
    },
    MoveRelative {
        amount: isize,
    },
    MoveAbsolute {
        tile: usize,
    },
    Wait {
        rounds: usize,
    },
    GoToJail,
    JailFree,
}

pub struct CardStack(Vec<usize>);

impl CardStack {

    #[inline]
    pub fn new(cards: Vec<usize>) -> Self {
        Self(cards)
    }

    pub fn draw(&self) -> usize {
        self.0[rand::thread_rng().gen_range(0..(self.0.len()))]
    }

}
