extern crate rand;
use self::rand::{thread_rng, Rng};
use std::rc::Rc;

#[derive(Copy, Clone)]
#[derive(Debug)]
enum Suit {
    Black,
    Green,
    Red,
}

pub trait TerminalPrintable {
    fn print(&self) -> String;
}

enum Card {
    JokerCard,
    DragonCard{suit: Suit},
    NumberCard{suit: Suit, rank: u8},
}
impl TerminalPrintable for Card {
    fn print(&self) -> String {
        match self {
            Card::JokerCard => String::from("J"),
            Card::DragonCard{suit} => term_color(*suit, "D".to_string()),
            Card::NumberCard{suit, rank} => term_color(*suit, rank.to_string()),
        }
    }
}

enum CardCell {
    JokerCell{has_joker: bool},
    FreeCell{card: Option<Rc<Card>>},
    GameCell{card_stack: Vec<Rc<Card>>},
    GoalCell{top_card: Option<Rc<Card>>},
}
impl CardCell {
    fn accept(&self, card: &Rc<Card>) -> Option<Self> {
        match self {
            CardCell::JokerCell{..} => match **card {
                Card::JokerCard => Some(CardCell::JokerCell{has_joker: true}),
                _ => None,
            },
            _ => None
        }
    }

    fn top(&self) -> Option<Rc<Card>> {
        match self {
            CardCell::GoalCell{top_card: Some(ref card)} => Some(card.clone()),
            CardCell::FreeCell{card: Some(ref card)} => Some(card.clone()),
            CardCell::GameCell{card_stack} => card_stack.last().map(|rc_card| rc_card.clone()),
            _ => None,
        }
    }

    fn pop(&self) -> Option<Self> {
        match self {
            CardCell::GameCell{card_stack} => {
                let mut new_stack: Vec<_> = card_stack.iter().map(|rc_card| rc_card.clone()).collect();
                new_stack.pop();
                Some(CardCell::GameCell{card_stack: new_stack})
            },
            _ => None,
        }
    }
}
impl TerminalPrintable for CardCell {
    fn print(&self) -> String {
        match self {
            CardCell::JokerCell{has_joker: true} => String::from("J"),
            CardCell::JokerCell{has_joker: false} => String::from("-"),
            CardCell::FreeCell{card: None} => String::from("-"),
            CardCell::FreeCell{card: Some(ref card)} => card.print(),
            CardCell::GoalCell{top_card: None} => String::from("-"),
            CardCell::GoalCell{top_card: Some(ref top_card)} => top_card.print(),
            CardCell::GameCell{card_stack} if card_stack.len() == 0 => String::from("-"),
            CardCell::GameCell{card_stack} => {
                let mut builder = String::new();
                for card in card_stack.iter() {
                    builder.push_str(&card.print());
                    builder.push_str("\n");
                }
                builder.pop();  // remove trailing "\n"
                builder
            }
        }
    }
}


#[derive(Clone)]
pub struct Board {
    joker_cell: Rc<CardCell>,
    free_cells: [Rc<CardCell>; 3],
    goal_cells: [Rc<CardCell>; 3],
    game_cells: [Rc<CardCell>; 8],
}

impl Board {
    pub fn deal() -> Board {
        let mut deck = create_deck();
        thread_rng().shuffle(&mut deck);

        let mut stacks = distribute(deck, 8).into_iter();

        Board{
            joker_cell: Rc::new(CardCell::JokerCell{has_joker: false}),
            free_cells: [
                Rc::new(CardCell::FreeCell{card: None}),
                Rc::new(CardCell::FreeCell{card: None}),
                Rc::new(CardCell::FreeCell{card: None}),
            ],
            goal_cells: [
                Rc::new(CardCell::GoalCell{top_card: None}),
                Rc::new(CardCell::GoalCell{top_card: None}),
                Rc::new(CardCell::GoalCell{top_card: None}),
            ],
            // TODO: I hate this, but I'm tired of fighting rust over it.
            // Maybe revisit this someday:
            // https://llogiq.github.io/2016/04/28/arraymap.html
            game_cells: [
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
                Rc::new(CardCell::GameCell{card_stack: stacks.next().unwrap()}),
            ],
        }
    }

    fn move_card(source: &mut Rc<CardCell>, dest: &mut Rc<CardCell>) -> bool {
        if let Some(new_cell) = dest.accept(&source.top().expect("me am play gods")) {
            *dest = Rc::new(new_cell);
            *source = Rc::new(source.pop().expect("cripes I should really be checkin these"));
            return true;
        }
        false
    }

    // fn move_stack(&mut self, source: GameCell, dest: GameCell, num_cards: u8)

    pub fn do_automoves(&self) -> Board {
        let mut board = self.clone();
        // Board::move_card(&mut board.game_cells[7], &mut board.joker_cell);
        let mut progress = true;

        while progress {
            progress = false;
            for mut cell in board.game_cells.iter_mut() {
                progress = match cell.top() {
                    Some(rc_card) => match *rc_card {
                        Card::JokerCard => Board::move_card(&mut cell, &mut board.joker_cell),
                        _ => false,
                    }
                    None => false,
                } || progress;
            }
        }
        board
    }
}

impl TerminalPrintable for Board {
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

fn create_deck() -> Vec<Rc<Card>> {
    let mut vec: Vec<Rc<Card>> = Vec::with_capacity(40);
    for suit in vec![Suit::Black, Suit::Green, Suit::Red] {
        for _ in 0..4 {
            vec.push(Rc::new(Card::DragonCard{suit}));
        }
        for rank in 1..10 {
            vec.push(Rc::new(Card::NumberCard{suit, rank}));
        }
    }
    vec.push(Rc::new(Card::JokerCard{}));
    return vec
}

fn distribute<T>(vec: Vec<T>, n: usize) -> Vec<Vec<T>> {
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
