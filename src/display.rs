use ::board::{Suit, Card, CardCell, Board};
use ::util;


pub fn display_card(card: &Card) -> String {
    match card {
        Card::JokerCard => String::from("J"),
        Card::DragonCard{suit} => term_color(*suit, "D".to_string()),
        Card::NumberCard{suit, rank} => term_color(*suit, rank.to_string()),
        Card::DragonStack => String::from("X"),
    }
}


pub fn display_cell(card_cell: &CardCell) -> String {
    match card_cell {
        CardCell::JokerCell{has_joker: true} => String::from("J"),
        CardCell::JokerCell{has_joker: false} => String::from("-"),
        CardCell::FreeCell{card: None} => String::from("-"),
        CardCell::FreeCell{card: Some(ref card)} => display_card(card),
        CardCell::GoalCell{top_card: None} => String::from("-"),
        CardCell::GoalCell{top_card: Some(ref top_card)} => display_card(top_card),
        CardCell::GameCell{card_stack} if card_stack.len() == 0 => String::from("-"),
        CardCell::GameCell{card_stack} => {
            let mut builder = String::new();
            for card in card_stack.iter() {
                builder.push_str(&display_card(card));
                builder.push_str("\n");
            }
            builder.pop();  // remove trailing "\n"
            builder
        }
    }
}

pub fn clear() {
    print!("{}[2J", 27 as char);
}

fn display_board(board: &Board) -> String {
    let mut s = String::new();
    for cell in board.free_cells().iter() {
        s.push_str(&display_cell(cell));
    }
    s.push_str("  ");
    s.push_str(&display_cell(board.joker_cell()));
    s.push_str(" ");
    for cell in board.goal_cells().iter() {
        s.push_str(&display_cell(cell));
    }
    s.push_str("\n");

    let strings: Vec<_> = board.game_cells().iter().map(|cell| display_cell(cell)).collect();
    s.push_str(&util::join_vertical(strings));

    return s;
}

fn term_color(suit: Suit, text: String) -> String {
    format!(
        "\x1b[38;5;{}m{}\x1b[0m",
        match suit {
            Suit::Black => 0,
            Suit::Green => 2,
            Suit::Red => 1,
        },
        text,
    )
}

