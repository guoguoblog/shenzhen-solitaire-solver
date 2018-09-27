use std::cmp::{Ordering, min};
use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap};
use std::rc::Rc;

// use ::display::display_board;
use ::board::{Board, Card, CardCellIndex, MoveStackError, Suit};

const SOURCE_SLOTS: &[CardCellIndex] = &[
    CardCellIndex::FreeCellIndex(0),
    CardCellIndex::FreeCellIndex(1),
    CardCellIndex::FreeCellIndex(2),
    CardCellIndex::GameCellIndex(0),
    CardCellIndex::GameCellIndex(1),
    CardCellIndex::GameCellIndex(2),
    CardCellIndex::GameCellIndex(3),
    CardCellIndex::GameCellIndex(4),
    CardCellIndex::GameCellIndex(5),
    CardCellIndex::GameCellIndex(6),
    CardCellIndex::GameCellIndex(7),
];
const DEST_SLOTS: &[CardCellIndex] = &[
    CardCellIndex::GoalCellIndex(0),
    CardCellIndex::GoalCellIndex(1),
    CardCellIndex::GoalCellIndex(2),
    CardCellIndex::FreeCellIndex(0),
    CardCellIndex::FreeCellIndex(1),
    CardCellIndex::FreeCellIndex(2),
    CardCellIndex::GameCellIndex(0),
    CardCellIndex::GameCellIndex(1),
    CardCellIndex::GameCellIndex(2),
    CardCellIndex::GameCellIndex(3),
    CardCellIndex::GameCellIndex(4),
    CardCellIndex::GameCellIndex(5),
    CardCellIndex::GameCellIndex(6),
    CardCellIndex::GameCellIndex(7),
];

#[derive(Eq, PartialEq)]
struct State {
    score: u32,
    board: Rc<Board>,
}

impl State {
    fn new(board: Rc<Board>) -> State {
        State {
            score: board_heuristic(&*board),
            board: board,
        }
    }
}

impl Ord for State {
    fn cmp(&self, other: &State) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &State) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn board_heuristic(board: &Board) -> u32 {
    let safe_rank = board.auto_safe_rank();
    let goal_score: u32 = board.goal_cells().iter().map(|cell| match cell.top() {
        Some(rc) => match *rc {
            Card::NumberCard{rank, ..} => min(rank, safe_rank) as u32,
            _ => unreachable!(),  // no other card type should be in a goal cell
        },
        None => 0,
    }).sum();
    let dragon_score: u32 = board.free_cells().iter().map(|cell| match cell.top() {
        Some(rc) => match *rc {
            Card::DragonStack => 9,
            _ => 0,
        },
        None => 0,
    }).sum();
    goal_score + dragon_score
}

fn get_valid_dests(board: &Board) -> Vec<&CardCellIndex> {
    let mut seen_free_cell = false;
    let mut seen_free_game_cell = false;

    DEST_SLOTS.iter().filter(|slot| {
        let top_card = board.get_cell(slot).top();
        match slot {
            // Only consider one empty cell, and don't consider occupied cells.
            CardCellIndex::FreeCellIndex(_) => {
                if top_card.is_none() {
                    if seen_free_cell {
                        false
                    }
                    else {
                        seen_free_cell = true;
                        true
                    }
                }
                else {
                    false
                }
            },
            // Only consider one empty cell.
            CardCellIndex::GameCellIndex(_) => {
                match top_card {
                    Some(card) => if let Card::NumberCard{..} = &*card {true} else {false},
                    None => {
                        if seen_free_game_cell {
                            false
                        }
                        else {
                            seen_free_game_cell = true;
                            true
                        }
                    },
                }
            },
            // Never consider an empty cell (because the automove will take care of it)
            CardCellIndex::GoalCellIndex(_) => top_card.is_some(),
        }
    }).collect()
}


pub fn next_states(board: &Board) -> Vec<Board> {
    let mut states = Vec::new();
    // Group dragons
    for suit in vec![Suit::Black, Suit::Green, Suit::Red] {
        if let Some(new_board) = board.stack_dragons(suit) {
            states.push(new_board.do_automoves());
        }
    }
    // Just try all moves.
    // We can do a little preprocessing on clearly invalid source and dest slots
    // before doing n * m comparisons.
    let source_slots = SOURCE_SLOTS.iter().filter(|slot| {
        let top_card = board.get_cell(slot).top();
        match top_card {
            None => false,
            Some(card) => if let Card::DragonStack = &*card {false} else {true},
        }
    });
    let dest_slots = get_valid_dests(board);

    // only need this reassign for the debug statement below
    // let source_slots: Vec<_> = source_slots.collect();
    // println!("{} → {} = {}", source_slots.len(), dest_slots.len(), source_slots.len() * dest_slots.len());

    for source_slot in source_slots {
        for dest_slot in dest_slots.iter() {
            match board.move_stack(source_slot, dest_slot) {
                Ok(new_board) => states.push(new_board.do_automoves()),
                Err(MoveStackError::AmbiguousMove(max_height)) =>
                    for height in 1..=max_height as usize {
                        if let Some(new_board) = board.move_n_cards(source_slot, dest_slot, height) {
                            states.push(new_board.do_automoves());
                        }
                    }
                Err(MoveStackError::InvalidMove) => (),
            }
        }
    }
    states
}

// BFSly search
pub fn solve(board: &Board) -> Option<Vec<Board>> {
    Some(solve_rc(board)?.into_iter().map(|board|
        Rc::try_unwrap(board).unwrap_or_else(|_| panic!("Didn't drop all the refs :(((("))
    ).collect())
}


fn solve_rc(board: &Board) -> Option<VecDeque<Rc<Board>>> {
    let board = Rc::new(board.clone());
    let mut open = BinaryHeap::new();
    open.push(State::new(board.clone()));
    let mut path: HashMap<Rc<Board>, Rc<Board>> = HashMap::new();
    let mut seen = HashSet::new();
    seen.insert(board);

    while let Some(State{board, ..}) = open.pop() {
        if board.is_solved() {
            return Some(reconstruct_path(path, board));
        }

        // println!("{}", display_board(&board));
        for next_board in next_states(&board) {
            let next_board = Rc::new(next_board);
            if !seen.insert(next_board.clone()) {
                continue;
            }
            path.insert(next_board.clone(), board.clone());
            open.push(State::new(next_board));
        }
    }

    None
}


fn reconstruct_path(mut path: HashMap<Rc<Board>, Rc<Board>>, board: Rc<Board>) -> VecDeque<Rc<Board>> {
    let mut result: VecDeque<Rc<Board>> = VecDeque::new();
    result.push_front(board.clone());
    // Would be great to `while let Some(board) = path.remove(&board)` here,
    // but the `let` rebinds the name `board` to a too-small scope, shadowing
    // this outer `board`.
    let mut board = board;
    loop {
        match path.remove(&board) {
            Some(b) => board = b,
            None => break,
        }
        result.push_front(board.clone());
    }
    result
}
