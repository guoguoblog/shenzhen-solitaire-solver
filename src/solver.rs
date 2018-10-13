use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap};
use std::hash::Hash;
use std::rc::Rc;

use ::board::{Board, Card, CardCellIndex, CardCell, MoveStackError, Suit};

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

fn counter<T, I>(iter: I) -> HashMap<T, u32> where
    T: Hash + Eq,
    I: Iterator<Item=T>,
{
    let mut result = HashMap::new();
    for elem in iter {
        let c = result.entry(elem).or_insert(0);
        *c += 1;
    }
    result
}

#[derive(Eq, PartialEq)]
struct AStarState {
    fscore: u32,
    board: Rc<Board>,
}

impl Ord for AStarState {
    fn cmp(&self, other: &AStarState) -> Ordering {
        other.fscore.cmp(&self.fscore)
    }
}

impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &AStarState) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


/// "hscore". An ~optimistic guess of how many moves it'll take to solve.
///
/// Considers automoves as moves, thus this heuristic is not
/// technically admissable. However it should still prevent making
/// unnecessary moves.
fn estimated_moves_to_solve(board: &Board) -> u32 {
    // Count how many cards are missing from the goal cells.
    let ungoaled_numcards: u32 = board.goal_cells().iter().map(|goal_cell|
        match goal_cell.top() {
            Some(rc) => match *rc {
                Card::NumberCard{rank, ..} => 9 - rank as u32,
                _ => unreachable!(),  // no other card type should be in a goal cell
            },
            None => 9,
        }
    ).sum();

    // Count how many dragon suits still need to be grouped.
    let ungrouped_dragon_suits: u32 = board.free_cells().iter().map(|cell| match cell.top() {
        Some(rc) => match *rc {
            Card::DragonStack => 0,
            _ => 1,
        },
        None => 1,
    }).sum();

    // Count how many dragons are trapped under dragons of the same suit.
    // These will require a move to separate em before they can be grouped.
    // If we know all of our dragons are already grouped we skip this
    // check entirely.
    let trapped_dragons: u32 = if ungrouped_dragon_suits == 0 {0} else {
        board.game_cells().iter().map(|game_cell| match &**game_cell {
            &CardCell::GameCell{ref card_stack} => {
                let rust_pls: u32 = counter(
                    card_stack.iter().filter_map(|rc|
                        match **rc {
                            Card::DragonCard{suit} => Some(suit),
                            _ => None,
                        }
                    )
                ).values().map(|num| num - 1).sum();
                rust_pls
            },
            _ => unreachable!(),  // should only be gamecells
        }).sum()
    };

    ungoaled_numcards + trapped_dragons + ungrouped_dragon_suits
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

pub fn solve(board: &Board) -> Option<Vec<Board>> {
    Some(solve_rc(board)?.into_iter().map(|board|
        Rc::try_unwrap(board).unwrap_or_else(|_| panic!("Didn't drop all the refs :(((("))
    ).collect())
}

// A*ly search
pub fn solve_rc(board: &Board) -> Option<VecDeque<Rc<Board>>> {
    let board = Rc::new(board.clone());
    let mut open_set = BinaryHeap::new();
    open_set.push(AStarState{
        fscore: estimated_moves_to_solve(&*board),
        board: board.clone(),
    });
    let mut path: HashMap<Rc<Board>, Rc<Board>> = HashMap::new();
    let mut closed_set = HashSet::new();
    let mut gscores: HashMap<Rc<Board>, u32> = HashMap::new();  // actual cost of getting here.
    gscores.insert(board.clone(), 0);  // it "actually" took no moves to start with this board.

    while let Some(AStarState{board, ..}) = open_set.pop() {
        if board.is_solved() {
            return Some(reconstruct_path(path, board));
        }

        closed_set.insert(board.clone());

        // we're trying to minimize moves, and each move is equally
        // costly, so this is a constant `1`.
        // We're also able to hoist this math outta the neighbor loop.
        let gscore: u32 = gscores.get(&*board).expect("why aint the board in here") + 1;

        for next_board in next_states(&board) {
            let next_board = Rc::new(next_board);
            if closed_set.contains(&*next_board) {
                continue;
            }

            if let Some(score) = gscores.get(&next_board) {
                if score < &gscore {
                    continue;
                }
            }

            path.insert(next_board.clone(), board.clone());
            gscores.insert(next_board.clone(), gscore);
            open_set.push(AStarState{
                fscore: estimated_moves_to_solve(&*next_board) + gscore,
                board: next_board,  // safe to give on last line of loop
            });
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Ensure we make the obvious moves when the game is near the end.
    fn fast_win() {
        // My goodness rust needs named arguments
        let board = Board::new(
            vec![
                Some(Card::DragonCard{suit: Suit::Green}),
                Some(Card::DragonStack),
                Some(Card::DragonStack),
            ],
            true,
            vec![
                Some(Card::NumberCard{suit: Suit::Red, rank: 9}),
                Some(Card::NumberCard{suit: Suit::Black, rank: 4}),
                Some(Card::NumberCard{suit: Suit::Green, rank: 1}),
            ],
            vec![
                Vec::new(),
                vec![Card::NumberCard{suit: Suit::Green, rank: 4}],
                vec![
                    Card::NumberCard{suit: Suit::Black, rank: 9},
                    Card::NumberCard{suit: Suit::Green, rank: 8},
                    Card::NumberCard{suit: Suit::Black, rank: 7},
                    Card::NumberCard{suit: Suit::Green, rank: 6},
                ],
                vec![
                    Card::NumberCard{suit: Suit::Black, rank: 5},
                    Card::NumberCard{suit: Suit::Green, rank: 3},
                ],
                vec![
                    Card::DragonCard{suit: Suit::Green},
                    Card::NumberCard{suit: Suit::Green, rank: 2},
                    Card::DragonCard{suit: Suit::Green},
                ],
                vec![Card::DragonCard{suit: Suit::Green}],
                Vec::new(),
                vec![
                    Card::NumberCard{suit: Suit::Green, rank: 9},
                    Card::NumberCard{suit: Suit::Black, rank: 8},
                    Card::NumberCard{suit: Suit::Green, rank: 7},
                    Card::NumberCard{suit: Suit::Black, rank: 6},
                    Card::NumberCard{suit: Suit::Green, rank: 5},
                ],
            ],
        );
        assert_eq!(solve(&board).expect("couldn't even solve").len(), 3);
    }

    #[test]
    fn very_easy() {
        // XXD  J 999
        //  --D--D--
        //    D
        let board = Board::new(
            vec![
                Some(Card::DragonStack),
                Some(Card::DragonStack),
                Some(Card::DragonCard{suit: Suit::Green}),
            ],
            true,
            vec![
                Some(Card::NumberCard{suit: Suit::Red, rank: 9}),
                Some(Card::NumberCard{suit: Suit::Black, rank: 9}),
                Some(Card::NumberCard{suit: Suit::Green, rank: 9}),
            ],
            vec![
                Vec::new(),
                Vec::new(),
                vec![
                    Card::DragonCard{suit: Suit::Green},
                    Card::DragonCard{suit: Suit::Green},
                ],
                Vec::new(),
                Vec::new(),
                vec![Card::DragonCard{suit: Suit::Green}],
                Vec::new(),
                Vec::new(),
            ],
        );

        assert_eq!(solve(&board).expect("couldn't even solve").len(), 3);
    }
}
