extern crate rand;
use rand::{thread_rng, Rng};
use std::fmt::Debug;

#[derive(Copy, Clone)]
#[derive(Debug)]
enum Suit {
    Black,
    Green,
    Red,
}

trait TerminalPrintable {
    fn print(&self) -> String;
}
trait Card: TerminalPrintable + Debug {}

#[derive(Debug)]
struct JokerCard ();
impl Card for JokerCard {}
impl TerminalPrintable for JokerCard {
    fn print(&self) -> String {
        String::from("J")
    }
}

#[derive(Debug)]
struct DragonCard {
    suit: Suit,
}
impl Card for DragonCard {}
impl TerminalPrintable for DragonCard {
    fn print(&self) -> String {
        term_color(self.suit, "D".to_string())
    }
}

#[derive(Debug)]
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

trait CardCell: TerminalPrintable {}

struct FreeCell<'a> {
    card: Option<&'a Card>,
}
impl<'a> CardCell for FreeCell<'a> {}
impl<'a> TerminalPrintable for FreeCell<'a> {
    fn print(&self) -> String {
        match self.card {
            Some(card) => {card.print()},
            None => String::from("-"),
        }
    }
}

struct GameCell {
   pub card_stack: Vec<Box<Card>>,
}
impl CardCell for GameCell {}
impl TerminalPrintable for GameCell {
    fn print(&self) -> String {
        if self.card_stack.len() == 0 {
            return String::from("-");
        }
        let mut builder = String::new();
        for card in self.card_stack.iter() {
            builder.push_str(&card.print());
            builder.push_str("\n");
        }
        builder.pop();  // remove trailing "\n"
        return builder;
    }
}

struct GoalCell<'a> {
    top_card: Option<&'a Card>,
}
impl<'a> CardCell for GoalCell<'a> {}
impl<'a> TerminalPrintable for GoalCell<'a> {
    fn print(&self) -> String {
        match self.top_card {
            Some(card) => {card.print()},
            None => String::from("-"),
        }
    }
}

struct JokerCell {
    has_joker: bool
}
impl CardCell for JokerCell {}
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
    game_cells: [GameCell; 8],
}

impl<'a> Board<'a> {
    fn deal() -> Board<'a> {
        let mut deck = create_deck();
        thread_rng().shuffle(&mut deck);

        let mut stacks = deal(deck, 8).into_iter();

        Board{
            joker_cell: JokerCell{has_joker: false},
            free_cells: [
                FreeCell{card: None},
                FreeCell{card: None},
                FreeCell{card: None},
            ],
            goal_cells: [
                GoalCell{top_card: None},
                GoalCell{top_card: None},
                GoalCell{top_card: None},
            ],
            // TODO: I hate this, but I'm tired of fighting rust over it.
            // Maybe revisit this someday:
            // https://llogiq.github.io/2016/04/28/arraymap.html
            game_cells: [
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
                GameCell{card_stack: stacks.next().unwrap()},
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
        s.push_str("\n");

        let strings: Vec<_> = self.game_cells.iter().map(|cell| cell.print()).collect();
        s.push_str(&join_vertical(strings));

        return s;
    }
}

fn join_vertical(strings: Vec<String>) -> String {
    let mut result = String::new();
    let columns: Vec<Vec<_>> = strings.iter().map(|str| str.split("\n").collect()).collect();
    let length = columns.iter().map(|strs| strs.len()).max().expect("input must not be empty");

    for y in 0..length {
        // TODO: doesn't fit this util really, but it's certainly the easiest place to add it
        result.push_str(" ");
        for column in columns.iter() {
            result.push_str(column.get(y).unwrap_or(&" "));
        }
        result.push_str("\n");
    }

    return result;
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

fn create_deck() -> Vec<Box<Card>> {
    let mut vec: Vec<Box<Card>> = Vec::with_capacity(40);
    for suit in vec![Suit::Black, Suit::Green, Suit::Red] {
        for _ in 0..4 {
            vec.push(Box::new(DragonCard{suit}));
        }
        for value in 1..10 {
            vec.push(Box::new(NumberCard{suit, value}));
        }
    }
    vec.push(Box::new(JokerCard{}));
    return vec
}

fn deal<T>(vec: Vec<T>, n: usize) -> Vec<Vec<T>> {
    let mut chunks : Vec<Vec<T>> = Vec::with_capacity(n);
    let chunksize = (vec.len() as f32 / n as f32).ceil() as usize;

    for _ in 0..n {
        chunks.push(Vec::new());
    }

    for (i, v) in vec.into_iter().enumerate() {
        chunks[i / chunksize].push(v);
    }
    chunks
}

fn main() {
    let b = Board::deal();
    println!("{}", b.print());
}
