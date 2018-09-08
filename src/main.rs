extern crate rand;
use rand::{thread_rng, Rng};

const GREEN_DRAGON: &str = "發";
const RED_DRAGON: &str = "中";
const BLACK_DRAGON: &str = "▯";

#[derive(Copy, Clone)]
enum Suit {
    Black,
    Green,
    Red,
}

trait TerminalPrintable {
    fn print(&self) -> String;
}
trait Card: TerminalPrintable {}

struct JokerCard ();
impl Card for JokerCard {}
impl TerminalPrintable for JokerCard {
    fn print(&self) -> String {
        String::from("J")
    }
}

struct DragonCard {
    suit: Suit,
}
impl Card for DragonCard {}
impl TerminalPrintable for DragonCard {
    fn print(&self) -> String {
        term_color(self.suit, match self.suit {
            Suit::Green => GREEN_DRAGON,
            Suit::Red => RED_DRAGON,
            Suit::Black => BLACK_DRAGON,
        }.to_string())
    }
}

struct NumberCard {
    suit: Suit,
    value: u8,
}
impl Card for NumberCard {}
impl TerminalPrintable for NumberCard {
    fn print(&self) -> String {
        term_color(self.suit, self.value.to_string())
    }
}

trait Cell: TerminalPrintable {}

struct FreeCell<'a> {
    card: Option<&'a Card>,
}
impl<'a> Cell for FreeCell<'a> {}
impl<'a> TerminalPrintable for FreeCell<'a> {
    fn print(&self) -> String {
        String::from("F")
    }
}

struct GameCell<'a> {
    card_stack: Vec<&'a Card>,
}
impl<'a> Cell for GameCell<'a> {}
impl<'a> TerminalPrintable for GameCell<'a> {
    fn print(&self) -> String {
        String::from("C")
    }
}

struct GoalCell<'a> {
    top_card: Option<&'a Card>,
}
impl<'a> Cell for GoalCell<'a> {}
impl<'a> TerminalPrintable for GoalCell<'a> {
    fn print(&self) -> String {
        String::from("G")
    }
}

struct JokerCell {
    has_joker: bool
}
impl Cell for JokerCell {}
impl TerminalPrintable for JokerCell {
    fn print(&self) -> String {
        if self.has_joker {
            String::from("J")
        } else {
            String::from("-")
        }
    }
}

struct Board<'a> {
    joker_cell: JokerCell,
    free_cells: [FreeCell<'a>; 3],
    goal_cells: [GoalCell<'a>; 3],
    game_cells: [GameCell<'a>; 8],
}

impl<'a> Board<'a> {
    fn deal() -> Board<'a> {
        let mut deck = create_deck();
        thread_rng().shuffle(&mut deck);
        Board{
            joker_cell: JokerCell{has_joker: false},
            free_cells: [FreeCell{card: None}, FreeCell{card: None}, FreeCell{card: None}],
            goal_cells: [
                GoalCell{top_card: None},
                GoalCell{top_card: None},
                GoalCell{top_card: None},
            ],
            game_cells: [
                GameCell{card_stack: vec![]}, GameCell{card_stack: vec![]},
                GameCell{card_stack: vec![]}, GameCell{card_stack: vec![]},
                GameCell{card_stack: vec![]}, GameCell{card_stack: vec![]},
                GameCell{card_stack: vec![]}, GameCell{card_stack: vec![]},
            ],
        }
    }
}

impl<'a> TerminalPrintable for Board<'a> {
    fn print(&self) -> String {
        let mut s = String::new();
        for cell in self.free_cells.iter() {
            s.push_str(&cell.print());
        }
        s.push_str("  ");
        s.push_str(&self.joker_cell.print());
        s.push_str(" ");
        for cell in self.goal_cells.iter() {
            s.push_str(&cell.print());
        }

        return s;
    }
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

fn create_deck<'a>() -> Vec<Box<Card>> {
    let mut vec: Vec<Box<Card>> = Vec::new();
    for suit in vec![Suit::Black, Suit::Green, Suit::Red] {
        for _ in 0..4 {
            vec.push(Box::new(DragonCard{suit}));
        }
        for value in 1..9 {
            vec.push(Box::new(NumberCard{suit, value}));
        }
    }
    vec.push(Box::new(JokerCard{}));
    return vec
}


fn main() {
    let b = Board::deal();
    println!("{}", b.print());
}
