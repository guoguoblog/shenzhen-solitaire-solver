extern crate rand;
use self::rand::{thread_rng, Rng};
use std::rc::Rc;

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum Suit {
    Black,
    Green,
    Red,
}

pub trait TerminalPrintable {
    fn print(&self) -> String;
}

#[derive(Debug)]
enum Card {
    JokerCard,
    DragonCard{suit: Suit},
    NumberCard{suit: Suit, rank: u8},
    /// Dummy "card" representing an immovable stack of dragons in a free cell.
    DragonStack,
}
impl Card {
    fn can_hold(&self, card: Rc<Card>) -> bool {
        match (&*self, &*card) {
            (
                &Card::NumberCard{suit: self_suit, rank: self_rank},
                &Card::NumberCard{suit: card_suit, rank: card_rank},
            ) if self_suit != card_suit && self_rank == card_rank + 1 => {
                true
            },
            _ => false,
        }
    }
}
impl TerminalPrintable for Card {
    fn print(&self) -> String {
        match self {
            Card::JokerCard => String::from("J"),
            Card::DragonCard{suit} => term_color(*suit, "D".to_string()),
            Card::NumberCard{suit, rank} => term_color(*suit, rank.to_string()),
            Card::DragonStack => String::from("X"),
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
        match (self, &**card) {
            (CardCell::JokerCell{..}, &Card::JokerCard) =>
                Some(CardCell::JokerCell{has_joker: true}),
            (CardCell::GoalCell{top_card: None}, &Card::NumberCard{rank: 1, ..}) =>
                Some(CardCell::GoalCell{top_card: Some(card.clone())}),
            (CardCell::GoalCell{top_card: Some(ref top_card)}, &Card::NumberCard{suit, rank}) =>
                match **top_card {
                    Card::NumberCard{suit: top_suit, rank: top_rank}
                    if top_suit == suit && top_rank + 1 == rank =>
                        Some(CardCell::GoalCell{top_card: Some(card.clone())}),
                    _ => None,
                }
            _ => None,
        }
    }

    /// Returns a clone of this stack with the passed card stack on top, or None if the card stack
    /// does not fit.
    ///
    /// Assumes `cards` is properly formed, ie not empty and all NumberCards, in descending order,
    /// with no matching Suit across consecutive cards.
    fn accept_stack(&self, cards: &[Rc<Card>]) -> Option<Self> {
        if let CardCell::GameCell{card_stack} = self {
            // if let Some(rc_card) = card_stack.last() {
            //     match (&**rc_card, &**cards.first().expect("cards must be nonempty")) {
            //         (&Card::NumberCard{..}, &Card::NumberCard{..}) => {
            //             println!("gr8");
            //         },
            //         _ => (),
            //     }
            // }
            let mut new_stack = card_stack.clone();
            new_stack.extend_from_slice(cards);
            Some(CardCell::GameCell{card_stack: new_stack})
        }
        else {panic!("Only GameCells may accept stacks.");}
    }

    fn top(&self) -> Option<Rc<Card>> {
        match self {
            CardCell::GoalCell{top_card: Some(ref card)} => Some(card.clone()),
            CardCell::FreeCell{card: Some(ref card)} => Some(card.clone()),
            CardCell::GameCell{card_stack} => card_stack.last().map(|rc_card| rc_card.clone()),
            _ => None,
        }
    }

    fn pop(&self) -> Self {
        match self {
            CardCell::GameCell{card_stack} => {
                let mut new_stack = card_stack.clone();
                new_stack.pop();
                CardCell::GameCell{card_stack: new_stack}
            },
            CardCell::FreeCell{card: _} => CardCell::FreeCell{card: None},
            _ => panic!("May not take cards from this cell type"),
        }
    }

    fn pop_n(&self, n: usize) -> Self {
        match self {
            CardCell::GameCell{card_stack} => {
                CardCell::GameCell{card_stack: card_stack[..card_stack.len() - n].to_vec()}
            },
            _ => panic!("May not take multiple cards from non-GameCells"),
        }
    }

    fn iter_stack(&self) -> Vec<Rc<Card>> {
        match &self {
            CardCell::GameCell{card_stack} => {
                let mut result: Vec<Rc<Card>> = Vec::new();
                let mut iter = card_stack.iter().rev();
                let mut last_card: Rc<Card>;
                if let Some(rc_card) = iter.next() {
                    last_card = rc_card.clone();
                    result.push(last_card.clone());  // TODO: can we avoid one of these clones??
                }
                else {
                    return result;
                }
                for rc_card in iter {
                    if rc_card.can_hold(last_card) {
                        last_card = rc_card.clone();
                        result.push(last_card.clone());
                    }
                    else {
                        break;
                    }
                }
                result.reverse();
                result
            },
            _ => panic!("iter_stack is only for GameCells")
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
            *source = Rc::new(source.pop());
            return true;
        }
        false
    }

    /// Move a stack from one game cell to another by index, and return the resulting board.
    ///
    /// This function only handles moving stacks when it is unambiguous how many cards will be
    /// moved, ie a DragonCard to an empty cell or a stack of NumberCards to another stack of
    /// NumberCards. To move a stack of NumberCards to an empty cell, see `move_n_cards`.
    ///
    /// If this function is unable to accomodate a move or the move is illegal, None is returned.
    pub fn move_stack(&self, source: usize, dest: usize) -> Option<Board> {
        let top_dest_rank = match *self.game_cells[dest].top()? {
            Card::NumberCard{rank, ..} => rank,
            _ => return None,
        };
        let mut board = self.clone();
        let stack = board.game_cells[source].iter_stack();
        let top_source_rank = match *(*stack.last()?) {
            Card::NumberCard{rank, ..} => rank,
            _ => return None,
        };
        let cards_to_grab = top_dest_rank.saturating_sub(top_source_rank) as usize;
        if 0 >= cards_to_grab || cards_to_grab > stack.len() {
            return None;
        }
        board.game_cells[source] = Rc::new(board.game_cells[source].pop_n(cards_to_grab));
        let substack = &stack[stack.len() - cards_to_grab..];
        board.game_cells[dest] = Rc::new(board.game_cells[dest].accept_stack(substack)?);
        Some(board)
    }

    /// Helper function to `stack_dragons`: remove all exposed dragons of the given suit from
    /// the board. Returns true if all four dragons are removed.
    ///
    /// Note that this leaves the board in an impossible state!
    fn remove_dragons(&mut self, suit: Suit) -> bool {
        let mut count = 0;
        for mut cell in self.game_cells.iter_mut().chain(self.free_cells.iter_mut()) {
            match cell.top() {
                Some(rc_card) => match *rc_card {
                    Card::DragonCard{suit: dsuit} if dsuit == suit => {
                        *cell = Rc::new(cell.pop());
                        count += 1;
                        if count == 4 {
                            return true
                        }
                    },
                    _ => (),
                },
                _ => (),
            }
        }
        false
    }

    /// Stack exposed dragons of the given suit into an open free cell, and return the resulting
    /// board.
    ///
    /// If not all dragons are exposed or no free cell is open, returns None instead.
    pub fn stack_dragons(&self, suit: Suit) -> Option<Board> {
        // TODO: this function would love some tests.
        // TODO: learn how to write tests in rust
        let mut board = self.clone();
        if !board.remove_dragons(suit) {
            return None
        }
        let mut found = false;
        for mut cell in board.free_cells.iter_mut() {
            if cell.top().is_none() {
                *cell = Rc::new(CardCell::FreeCell{card: Some(Rc::new(Card::DragonStack))});
                // Can't just return here, because we're already
                // borrowing `board` to iterate it, I guess. 🤮
                found = true;
                break;
            }
        }
        if found {Some(board)}
        else {None}
    }

    pub fn is_solved(&self) -> bool {
        for cell in self.game_cells.iter() {
            if let Some(_) = cell.top() {
                return false;
            }
        }
        true
    }

    /// The maximum rank of number card that is safe to auto-move to the goal.
    fn auto_safe_rank(&self) -> u8 {
        self.goal_cells.iter().map(|cell| match cell.top() {
            Some(rc) => match *rc {
                Card::NumberCard{rank, ..} => rank,
                _ => unreachable!(),  // no other card type should be in a goal cell
            },
            None => 0,
        }).min().expect("goal cells is a sized array you goof.") + 2
    }

    /// Perform moves which are always safe to do and return the resulting board.
    pub fn do_automoves(&self) -> Board {
        let mut board = self.clone();
        let mut progress = true;
        let mut safe_rank = self.auto_safe_rank();

        while progress {
            progress = false;
            for mut cell in board.game_cells.iter_mut() {
                progress = match cell.top() {
                    Some(rc_card) => match *rc_card {
                        Card::JokerCard => Board::move_card(&mut cell, &mut board.joker_cell),
                        Card::NumberCard{rank, ..} if rank <= safe_rank => {
                            let mut did = false;
                            for mut goal in board.goal_cells.iter_mut() {
                                if Board::move_card(&mut cell, &mut goal) {
                                    did = true;
                                    break
                                }
                            }
                            did
                        }
                        _ => false,
                    }
                    None => false,
                } || progress;
            }
            safe_rank = board.auto_safe_rank();
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


#[cfg(test)]
mod tests {
    use std::mem;
    use super::*;

    fn assert_vec_rc_ptr_eq<T>(left: &Vec<Rc<T>>, right: &Vec<Rc<T>>) {
        assert_eq!(left.len(), right.len());
        for (left_elem, right_elem) in left.iter().zip(right) {
            assert!(Rc::ptr_eq(left_elem, right_elem));
        }
    }

    fn empty_board() -> Board {
        Board {
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
            game_cells: [
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
                Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
            ],
        }
    }

    fn add_game_card(board: &mut Board, card: Card, column: usize) -> Rc<Card> {
        // Indiana Jones the cell from the array.
        let mut rc_game_cell = mem::replace(
            &mut board.game_cells[column],
            Rc::new(CardCell::GameCell{card_stack: Vec::new()}),
        );
        let rc_card = Rc::new(card);
        match Rc::get_mut(&mut rc_game_cell) {
            Some(CardCell::GameCell{card_stack}) => card_stack.push(rc_card.clone()),
            _ => panic!("Non-GameCell in game_cell slot!?"),
        }
        // Set it back, now that we've mutated it.
        // Don't need to Indiana Jones, because we put the temp cell into the toilet 🚽
        board.game_cells[column] = rc_game_cell;
        rc_card
    }

    fn get_card_stack_vec(board: &Board, column: usize) -> &Vec<Rc<Card>> {
        match &*board.game_cells[column] {
            &CardCell::GameCell{ref card_stack} => card_stack,
            _ => panic!("Non-GameCell in game_cell slot!?"),
        }
    }

    #[test]
    /// Ensure a joker on the game board is automoved to the goal.
    fn automove_jokers() {
        let board = {
            let mut board = empty_board();
            add_game_card(&mut board, Card::JokerCard, 3);
            board
        };
        assert_eq!(get_card_stack_vec(&board, 3).len(), 1);
        let new_board = board.do_automoves();
        match *new_board.joker_cell {
            CardCell::JokerCell{has_joker} => assert!(has_joker),
            _ => panic!("Non-JokerCell in joker_cell slot?"),
        }
        assert_eq!(get_card_stack_vec(&new_board, 3).len(), 0);
    }

    #[test]
    /// Ensure 1s on the game board are automoved to the goal.
    fn automove_1() {
        let green_1;
        let board = {
            let mut board = empty_board();
            green_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 1}, 5);
            board
        };
        assert_eq!(get_card_stack_vec(&board, 5).len(), 1);
        let new_board = board.do_automoves();
        match &*new_board.goal_cells[0] {
            CardCell::GoalCell{top_card: Some(top_card)} =>
                assert!(Rc::ptr_eq(&top_card, &green_1)),
            CardCell::GoalCell{top_card: None} => panic!("Missing card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
        assert_eq!(get_card_stack_vec(&new_board, 5).len(), 0);
    }

    #[test]
    /// Ensure automove will grab many cards in a single `do_automoves`, even if they start covered
    fn automove_many() {
        let mut board = empty_board();
        let red_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 2}, 3);
        let red_9 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 9}, 3);
        let black_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 2}, 3);
        let black_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 1}, 4);
        let red_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 1}, 4);
        let green_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 2}, 4);
        let green_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 1}, 4);

        let new_board = board.do_automoves();

        assert_eq!(get_card_stack_vec(&new_board, 4).len(), 0);
        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 3),
            &vec![red_2, red_9],
        );
        match &*new_board.goal_cells[0] {
            CardCell::GoalCell{top_card: Some(top_card)} =>
                assert!(Rc::ptr_eq(&top_card, &green_2)),
            CardCell::GoalCell{top_card: None} => panic!("Missing card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
        match &*new_board.goal_cells[1] {
            CardCell::GoalCell{top_card: Some(top_card)} =>
                assert!(Rc::ptr_eq(&top_card, &red_1)),
            CardCell::GoalCell{top_card: None} => panic!("Missing card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
        match &*new_board.goal_cells[2] {
            CardCell::GoalCell{top_card: Some(top_card)} =>
                assert!(Rc::ptr_eq(&top_card, &black_2)),
            CardCell::GoalCell{top_card: None} => panic!("Missing card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
    }

    #[test]
    /// Ensure the automove only makes safe moves, such that no card is moved to the goal area
    /// if any other card on the board may need to be placed on it.
    fn automove_limit() {
        let mut board = empty_board();
        let black_3 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 3}, 3);
        let red_9 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 9}, 3);
        let black_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 2}, 3);
        let black_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 1}, 4);
        let green_3 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 3}, 4);
        let green_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 2}, 4);
        let green_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 1}, 4);

        let new_board = board.do_automoves();

        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 4),
            &vec![black_1, green_3],
        );
        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 3),
            &vec![black_3, red_9, black_2],
        );
        match &*new_board.goal_cells[0] {
            CardCell::GoalCell{top_card: Some(top_card)} =>
                assert!(Rc::ptr_eq(&top_card, &green_2)),
            CardCell::GoalCell{top_card: None} => panic!("Missing card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
        match &*new_board.goal_cells[1] {
            CardCell::GoalCell{top_card: None} => (),
            CardCell::GoalCell{top_card: Some(_)} => panic!("Unexpected card"),
            _ => panic!("Non-GoalCell in goal_cell slot?"),
        }
    }

    #[test]
    /// Ensure you can move a stack to another stack.
    fn move_stack() {
        let mut board = empty_board();
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 7}, 0);
        let green_6 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 6}, 0);
        let red_5 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 5}, 0);
        let black_4 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 4}, 0);
        let black_8 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 8}, 1);
        let black_7 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 7}, 1);

        assert_eq!(get_card_stack_vec(&board, 0).len(), 4);
        assert_eq!(get_card_stack_vec(&board, 1).len(), 2);
        let new_board = match board.move_stack(0, 1) {
            Some(new_board) => new_board,
            None => panic!("did not move stack"),
        };
        assert_eq!(get_card_stack_vec(&new_board, 0).len(), 1);
        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 1),
            &vec![black_8, black_7, green_6, red_5, black_4],
        );
    }

    #[test]
    /// Ensure you can't move a stack onto a smaller stack.
    fn smaller_move_stack() {
        let mut board = empty_board();
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 6}, 0);
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 9}, 1);

        assert!(board.move_stack(1, 0).is_none());
    }
}
