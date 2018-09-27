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


pub fn display_highlighted_card(card: &Card) -> String {
    match card {
        &Card::NumberCard{suit, rank} => term_highlight(suit, rank.to_string()),
        _ => panic!("Only NumberCards may be highlighted"),
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
        CardCell::GameCell{card_stack} if card_stack.is_empty() => String::from("-"),
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

pub fn display_highlighted_cell(card_cell: &CardCell, height: u8) -> String {
    match card_cell {
        CardCell::GameCell{card_stack} if !card_stack.is_empty() => {
            let mut builder = String::new();
            // if rust were a little more like python, this might look something like:

            // let mut iter = card_stack.iter();
            // for card in iter.take(5) {
            //     builder.push(display_card(card));
            // }
            // for card in iter {
            //     builder.push(display_highlighted_card(card));
            // }

            // As-is, I can't figure out how to get rust to consume an iterator twice.
            // ᖍ(シ)ᖌ
            let pivot = card_stack.len() - height as usize;

            for card in &card_stack[..pivot] {
                builder.push_str(&display_card(card));
                builder.push_str("\n");
            }
            for card in &card_stack[pivot..] {
                builder.push_str(&display_highlighted_card(card));
                builder.push_str("\n");
            }
            builder.pop();  // remove trailing "\n"
            builder
        }
        _ => panic!("may only be called on a GameCell with cards."),
    }
}

pub fn clear() {
    print!("{}[2J", 27 as char);
}

pub fn display_board(board: &Board) -> String {
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
        "\x1b[{}m{}\x1b[39m",
        match suit {
            Suit::Black => 90,
            Suit::Green => 32,
            Suit::Red => 31,
        },
        text,
    )
}

fn term_highlight(suit: Suit, text: String) -> String {
    format!(
        "\x1b[22m\x1b[{}m{}\x1b[39m\x1b[2m",
        match suit {
            Suit::Black => 90,
            Suit::Green => 32,
            Suit::Red => 31,
        },
        text,
    )
}


pub fn dim(text: String) -> String {
    format!("\x1b[2m{}\x1b[22m", text)
}

pub fn no_dim(text: String, should_dim: bool) -> String {
    if should_dim {format!("\x1b[22m{}\x1b[2m", text)}
    else {text}
}

/// Formats a string literal as a "selected" marker in the game ui.
///
/// In practice, this turns the string yellow.
macro_rules! selector_color {
    ($text: tt) => {
        concat!("\x1b[38;5;11m", $text, "\x1b[0m")
    }
}
