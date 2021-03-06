extern crate itertools;
extern crate rand;
extern crate zero85;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use self::itertools::sorted;
use self::rand::{thread_rng, Rng, SeedableRng, StdRng};
use self::zero85::{FromZ85, ToZ85};

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Black,
    Green,
    Red,
}

#[derive(Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Card {
    JokerCard,
    DragonCard{suit: Suit},
    NumberCard{suit: Suit, rank: u8},
    /// Dummy "card" representing an immovable stack of dragons in a free cell.
    DragonStack,
}
impl Card {
    fn can_hold(&self, card: &Card) -> bool {
        match (self, card) {
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

#[derive(Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum CardCell {
    JokerCell{has_joker: bool},
    FreeCell{card: Option<Rc<Card>>},
    GameCell{card_stack: Vec<Rc<Card>>},
    GoalCell{top_card: Option<Rc<Card>>},
}
impl CardCell {
    fn accept(&self, card: &Rc<Card>) -> Option<Self> {
        match (self, &**card) {
            (_, &Card::DragonStack) => None,

            (CardCell::JokerCell{..}, &Card::JokerCard) =>
                Some(CardCell::JokerCell{has_joker: true}),

            (CardCell::FreeCell{card: None}, _) =>
                Some(CardCell::FreeCell{card: Some(card.clone())}),

            (CardCell::GoalCell{top_card: None}, &Card::NumberCard{rank: 1, ..}) =>
                Some(CardCell::GoalCell{top_card: Some(card.clone())}),

            (CardCell::GoalCell{top_card: Some(ref top_card)}, &Card::NumberCard{suit, rank}) =>
                match **top_card {
                    Card::NumberCard{suit: top_suit, rank: top_rank}
                    if top_suit == suit && top_rank + 1 == rank =>
                        Some(CardCell::GoalCell{top_card: Some(card.clone())}),
                    _ => None,
                }

            (CardCell::GameCell{..}, _) => self.accept_stack(&[card.clone()]),

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
            if let Some(rc_card) = card_stack.last() {
                let card = cards.first().expect("cards must be nonempty");
                if !rc_card.can_hold(&**card) {
                    return None;
                }
            }
            let mut new_stack = card_stack.clone();
            new_stack.extend_from_slice(cards);
            Some(CardCell::GameCell{card_stack: new_stack})
        }
        else {panic!("Only GameCells may accept stacks.");}
    }

    pub fn top(&self) -> Option<Rc<Card>> {
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
                    result.push(last_card.clone());
                }
                else {
                    return result;
                }
                for rc_card in iter {
                    if rc_card.can_hold(&*last_card) {
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

/// Enum to refer to the different card cells on a board.
#[derive(Eq, PartialEq)]
pub enum CardCellIndex {
    FreeCellIndex(usize),
    GoalCellIndex(usize),
    GameCellIndex(usize),
}

pub enum MoveStackError {
    AmbiguousMove(u8),
    InvalidMove,
}

#[derive(Clone)]
pub struct Board {
    joker_cell: Rc<CardCell>,
    free_cells: [Rc<CardCell>; 3],
    goal_cells: [Rc<CardCell>; 3],
    game_cells: [Rc<CardCell>; 8],
}

impl Board {
    pub fn joker_cell(&self) -> &Rc<CardCell> {&self.joker_cell}
    pub fn free_cells(&self) -> &[Rc<CardCell>; 3] {&self.free_cells}
    pub fn goal_cells(&self) -> &[Rc<CardCell>; 3] {&self.goal_cells}
    pub fn game_cells(&self) -> &[Rc<CardCell>; 8] {&self.game_cells}

    // pining for named arguments
    pub fn new(free_cells: Vec<Option<Card>>, joker_cell: bool, goal_cells: Vec<Option<Card>>, game_cells: Vec<Vec<Card>>) -> Board {
        let mut free_cells = free_cells.into_iter().map(|cell|
            Rc::new(CardCell::FreeCell{card: cell.map(|card| Rc::new(card))})
        );
        let mut goal_cells = goal_cells.into_iter().map(|cell|
            Rc::new(CardCell::GoalCell{top_card: cell.map(|card| Rc::new(card))})
        );
        let mut game_cells = game_cells.into_iter().map(|cell|
            Rc::new(CardCell::GameCell{card_stack: cell.into_iter().map(|card| Rc::new(card)).collect()})
        );

        Board{
            joker_cell: Rc::new(CardCell::JokerCell{has_joker: joker_cell}),
            // TODO: I hate this, but I'm tired of fighting rust over it.
            // Maybe revisit this someday:
            // https://llogiq.github.io/2016/04/28/arraymap.html
            free_cells: [
                free_cells.next().unwrap(),
                free_cells.next().unwrap(),
                free_cells.next().unwrap(),
            ],
            goal_cells: [
                goal_cells.next().unwrap(),
                goal_cells.next().unwrap(),
                goal_cells.next().unwrap(),
            ],
            game_cells: [
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
                game_cells.next().unwrap(),
            ],
        }

    }

    pub fn deal() -> (Board, Seed) {
        let seed = Seed::random();
        (Board::deal_seeded(&seed), seed)
    }

    pub fn deal_seeded(seed: &Seed) -> Board {
        let mut deck = create_deck();
        StdRng::from_seed(seed.key).shuffle(&mut deck);

        Board::new(
            vec![None, None, None], false, vec![None, None, None],
            distribute(deck, 8),
        )
    }

    fn move_card(source: &mut Rc<CardCell>, dest: &mut Rc<CardCell>) -> bool {
        if let Some(new_cell) = dest.accept(&source.top().expect("me am play gods")) {
            *dest = Rc::new(new_cell);
            *source = Rc::new(source.pop());
            return true;
        }
        false
    }

    /// Move a stack of number cards from one game cell to another, and return the resulting board.
    ///
    /// Only handles the case where both source and dest refer to a GameCell whose top card is a
    /// NumberCard. For more general card moving, see `move_stack` or `move_n_cards`.
    fn move_number_card_stack(&self, source: usize, dest: usize) -> Option<Board> {
        let top_dest_rank = match *self.game_cells[dest].top()? {
            Card::NumberCard{rank, ..} => rank,
            _ => return None,
        };
        let top_source_rank = match *self.game_cells[source].top()? {
            Card::NumberCard{rank, ..} => rank,
            _ => return None,
        };
        let cards_to_grab = top_dest_rank.saturating_sub(top_source_rank) as usize;

        // Now that we've decided how many cards to move, let's move em!
        self.move_n_cards_by_idx(source, dest, cards_to_grab)
    }

    /// Move a stack from one card cell to another, and return the resulting board.
    ///
    /// This function only handles moving stacks when it is unambiguous how many cards will be
    /// moved, eg a DragonCard to an empty cell or a stack of NumberCards to another stack of
    /// NumberCards. To move a stack of NumberCards to an empty game cell, see `move_n_cards`.
    ///
    /// If the requested move is illegal, InvalidMove is returned. If there exist more than one
    /// ways to accomplish the requested move, AmbiguousMove is returned (and `move_n_cards` should
    /// be called instead).
    pub fn move_stack(&self, source: &CardCellIndex, dest: &CardCellIndex) -> Result<Board, MoveStackError> {
        // Can't move in-place.
        if source == dest {
            return Err(MoveStackError::InvalidMove);
        }

        // We might need to special-case moving between game cells, for stacks of num cards.
        if let (
                    &CardCellIndex::GameCellIndex(source_idx),
                    &CardCellIndex::GameCellIndex(dest_idx),
                ) = (source, dest) {
            // If there's already something in the destination, better let move_number_card_stack
            // handle it (because only NumberCards can move to an occupied dest).
            if self.game_cells[dest_idx].top().is_some() {
                return match self.move_number_card_stack(source_idx, dest_idx) {
                    Some(board) => Ok(board),
                    None => Err(MoveStackError::InvalidMove),
                };
            }
            // If our "stack" of NumberCards is one deep we can just move that card.
            // Otherwise we need to handle this in `move_n_cards`
            let height = self.game_cells[source_idx].iter_stack().len();
            if height > 1 {
                return Err(MoveStackError::AmbiguousMove(height as u8));
            }
        }

        // Safe to (try to) move a single card.
        // We're not able to leverage `move_card` here, because we have no way to prove to Rust
        // that we don't want to mutably borrow the same cell twice.
        let source_cell = self.get_cell(source);
        let dest_cell = self.get_cell(dest);

        // Trying to move no card
        let source_card = &match source_cell.top() {
            Some(card) => card,
            None => return Err(MoveStackError::InvalidMove),
        };

        // Card move is invalid
        let new_dest = match dest_cell.accept(source_card) {
            Some(cell) => cell,
            None => return Err(MoveStackError::InvalidMove),
        };

        let mut board = self.clone();
        board.replace_cell(dest, new_dest);
        board.replace_cell(source, source_cell.pop());
        Ok(board)
    }

    fn replace_cell(&mut self, index: &CardCellIndex, new_cell: CardCell) {
        // might be nice to check that the cell type is right
        match index {
            &CardCellIndex::FreeCellIndex(n) => self.free_cells[n] = Rc::new(new_cell),
            &CardCellIndex::GoalCellIndex(n) => self.goal_cells[n] = Rc::new(new_cell),
            &CardCellIndex::GameCellIndex(n) => self.game_cells[n] = Rc::new(new_cell),
        }
    }

    pub fn get_cell(&self, index: &CardCellIndex) -> &Rc<CardCell> {
        match index {
            &CardCellIndex::FreeCellIndex(n) => &self.free_cells[n],
            &CardCellIndex::GoalCellIndex(n) => &self.goal_cells[n],
            &CardCellIndex::GameCellIndex(n) => &self.game_cells[n],
        }
    }

    pub fn move_n_cards(&self, source: &CardCellIndex, dest: &CardCellIndex, n: usize) -> Option<Board> {
        match (source, dest) {
            (&CardCellIndex::GameCellIndex(source_idx), &CardCellIndex::GameCellIndex(dest_idx)) =>
                self.move_n_cards_by_idx(source_idx, dest_idx, n),
            _ => panic!("`move_n_cards` may only act on game cells"),
        }
    }

    fn move_n_cards_by_idx(&self, source: usize, dest: usize, n: usize) -> Option<Board> {
        let stack = self.game_cells[source].iter_stack();
        if 0 >= n || n > stack.len() {
            return None;
        }

        let mut board = self.clone();
        board.game_cells[source] = Rc::new(board.game_cells[source].pop_n(n));
        let substack = &stack[stack.len() - n..];
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
    pub fn auto_safe_rank(&self) -> u8 {
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
            for mut cell in board.game_cells.iter_mut().chain(board.free_cells.iter_mut()) {
                progress = match cell.top() {
                    Some(rc_card) => match *rc_card {
                        Card::JokerCard => Board::move_card(cell, &mut board.joker_cell),
                        Card::NumberCard{rank, ..} if rank <= safe_rank => {
                            let mut did = false;
                            for mut goal in board.goal_cells.iter_mut() {
                                if Board::move_card(cell, goal) {
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

impl PartialEq for Board {
    fn eq(&self, rhs: &Board) -> bool {
        self.joker_cell == rhs.joker_cell &&
        sorted(self.game_cells.iter()) == sorted(rhs.game_cells.iter()) &&
        sorted(self.free_cells.iter()) == sorted(rhs.free_cells.iter()) &&
        sorted(self.goal_cells.iter()) == sorted(rhs.goal_cells.iter())
    }
}

impl Eq for Board {}

impl Hash for Board {
    fn hash<H>(&self, hasher: &mut H) where
        H: Hasher,
    {
        self.joker_cell.hash(hasher);
        sorted(self.game_cells.iter()).hash(hasher);
        sorted(self.free_cells.iter()).hash(hasher);
        sorted(self.goal_cells.iter()).hash(hasher);
    }
}

pub struct Seed {
    key: [u8; 32],
}

impl Seed {
    pub fn from_string(seed: &str) -> Seed {
        let bytes = seed.from_z85().unwrap();
        let mut array = [0; 32];
        let bytes = &bytes[..array.len()]; // panics if not enough data
        array.copy_from_slice(bytes);
        Seed {key: array}
    }

    pub fn to_string(&self) -> String {
        self.key.to_z85().unwrap()
    }

    pub fn random() -> Seed {
        Seed {key: thread_rng().gen()}
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.to_string())
    }
}

fn create_deck() -> Vec<Card> {
    let mut vec: Vec<Card> = Vec::with_capacity(40);
    for suit in vec![Suit::Black, Suit::Green, Suit::Red] {
        for _ in 0..4 {
            vec.push(Card::DragonCard{suit});
        }
        for rank in 1..10 {
            vec.push(Card::NumberCard{suit, rank});
        }
    }
    vec.push(Card::JokerCard);
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

    fn set_free_card(board: &mut Board, card: Card, column: usize) -> Rc<Card> {
        let rc_card = Rc::new(card);
        board.free_cells[column] = Rc::new(CardCell::FreeCell{card: Some(rc_card.clone())});
        rc_card
    }

    fn get_card_stack_vec(board: &Board, column: usize) -> &Vec<Rc<Card>> {
        match &*board.game_cells[column] {
            &CardCell::GameCell{ref card_stack} => card_stack,
            _ => panic!("Non-GameCell in game_cell slot!?"),
        }
    }

    fn assert_is_invalid_move<T>(result: &Result<T, MoveStackError>) {
        match result {
            Err(MoveStackError::InvalidMove) => (),
            Err(MoveStackError::AmbiguousMove{..}) =>
                panic!("Expected InvalidMove, but found AmbiguousMove"),
            Ok(_) =>
                panic!("Expected InvalidMove, but move was valid"),
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
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 1}, 4);
        let red_1 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 1}, 4);
        let green_2 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 2}, 4);
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 1}, 4);

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
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 1}, 4);

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
        let new_board = match board.move_stack(
            &CardCellIndex::GameCellIndex(0),
            &CardCellIndex::GameCellIndex(1),
        ) {
            Ok(new_board) => new_board,
            Err(_) => panic!("did not move stack"),
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

        assert_is_invalid_move(&board.move_stack(
            &CardCellIndex::GameCellIndex(1),
            &CardCellIndex::GameCellIndex(0),
        ));
    }

    #[test]
    /// Ensure you can't move a stack onto a stack of the same suit
    fn move_stack_suit_match() {
        let mut board = empty_board();
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 6}, 0);
        add_game_card(&mut board, Card::NumberCard{suit: Suit::Red, rank: 7}, 1);

        assert_is_invalid_move(&board.move_stack(
            &CardCellIndex::GameCellIndex(0),
            &CardCellIndex::GameCellIndex(1),
        ));
    }

    #[test]
    /// Ensure you can't move a DragonStack at all
    fn cant_move_dragon_stack() {
        let mut board = empty_board();
        set_free_card(&mut board, Card::DragonStack, 0);

        assert_is_invalid_move(&board.move_stack(
            &CardCellIndex::FreeCellIndex(0),
            &CardCellIndex::GameCellIndex(0),
        ));
    }

    #[test]
    /// Ensure you can stack a NumberCard from a FreeCell on a NumberCard in a GameCell.
    fn move_stack_via_free() {
        let mut board = empty_board();
        let black_4 = set_free_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 4}, 0);
        let green_5 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Green, rank: 5}, 0);

        let new_board = match board.move_stack(
            &CardCellIndex::FreeCellIndex(0),
            &CardCellIndex::GameCellIndex(0),
        ) {
            Ok(new_board) => new_board,
            Err(_) => panic!("did not move stack"),
        };

        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 0),
            &vec![green_5, black_4],
        );
    }

    #[test]
    /// Ensure you can move a number card to an empty GameCell from a FreeCell
    fn move_num_to_empty_via_free() {
        let mut board = empty_board();
        let black_4 = set_free_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 4}, 0);

        let new_board = match board.move_stack(
            &CardCellIndex::FreeCellIndex(0),
            &CardCellIndex::GameCellIndex(0),
        ) {
            Ok(new_board) => new_board,
            Err(_) => panic!("did not move stack"),
        };

        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 0),
            &vec![black_4],
        );
    }

    #[test]
    /// Ensure you can move an unstacked number card to an empty GameCell from a GameCell
    fn move_num_to_empty_via_game() {
        let mut board = empty_board();
        let black_8 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 8}, 0);
        let black_4 = add_game_card(&mut board, Card::NumberCard{suit: Suit::Black, rank: 4}, 0);

        let new_board = match board.move_stack(
            &CardCellIndex::GameCellIndex(0),
            &CardCellIndex::GameCellIndex(1),
        ) {
            Ok(new_board) => new_board,
            Err(_) => panic!("did not move stack"),
        };
        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 0),
            &vec![black_8],
        );
        assert_vec_rc_ptr_eq(
            &get_card_stack_vec(&new_board, 1),
            &vec![black_4],
        );
    }

}
