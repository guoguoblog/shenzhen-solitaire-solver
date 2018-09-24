extern crate getch;

use std::rc::Rc;

use ::board::{Board, CardCell, Card, CardCellIndex, MoveStackError};
use ::display::{display_cell, display_highlighted_cell, dim, no_dim};
use ::util;

#[derive(Debug)]
enum GameMode {
    SelectSource,
    SelectDestination{cursor: u8},
    ChooseStackHeight{cursor: u8, height: u8, max_height: u8},
}

/// Human-playable board representation.
pub struct Game {
    board: Board,
    cursor: u8,
    mode: GameMode,
}
impl Game {
    pub fn new(board: Board) -> Game {
        Game{
            board,
            cursor: 11,
            mode: GameMode::SelectSource
        }
    }

    pub fn deal() -> Game {
        Game::new(Board::deal())
    }

    fn print(&self) {
        let mut s = String::new();

        let mut top_row = [" "; 10];
        match self.cursor {
            1...3 => top_row[self.cursor as usize - 1] = "v",
            4...6 => top_row[self.cursor as usize + 3] = "v",
            _ => (),
        }

        let should_dim;
        match self.mode {
            GameMode::SelectDestination{cursor: cursor @ 1...3} => {
                top_row[cursor as usize - 1] = selector_color!("v");
                should_dim = false;
            },
            GameMode::ChooseStackHeight{..} => should_dim = true,
            _ => should_dim = false,
        }
        s.push_str(&top_row.join(""));
        s.push_str("\n");

        for cell in self.board.free_cells().iter() {
            s.push_str(&display_cell(cell));
        }
        s.push_str("  ");
        s.push_str(&display_cell(self.board.joker_cell()));
        s.push_str(" ");
        for cell in self.board.goal_cells().iter() {
            s.push_str(&display_cell(cell));
        }
        s.push_str("\n");

        let mut strings: Vec<_> = self.board.game_cells().iter().enumerate().map(|(i, cell)|
            match self.mode {
                GameMode::ChooseStackHeight{height, cursor, ..} if cursor as usize - 7 == i => {
                    display_highlighted_cell(cell, height)},
                _ => display_cell(cell),
            }
        ).collect();
        match self.cursor {
            7...14 => strings[self.cursor as usize - 7].push_str(
                &format!("\n{}", no_dim("^".to_string(), should_dim))
            ),
            _ => (),
        }
        match self.mode {
            GameMode::SelectDestination{cursor: cursor @ 7...14} |
            GameMode::ChooseStackHeight{cursor: cursor @ 7...14, ..} =>
                strings[cursor as usize - 7].push_str(
                    &format!("\n{}", no_dim(selector_color!("^").to_string(), should_dim))
                ),
            _ => (),
        }
        s.push_str(&util::join_vertical(strings));
        if should_dim {
            s = dim(s);
        }
        println!("{}", s);
    }

    pub fn play(&mut self) {
        self.print();
        self.board = self.board.do_automoves();
        self.print();
        let g = getch::Getch::new();
        while !self.board.is_solved() {
            let chr = match g.getch() {
                Ok(value) => value,
                Err(msg) => {
                    println!("Ok guess we're done ({})", msg);
                    return;
                }
            };
            match chr as char {
                '?' => Game::print_controls(),
                'g' | 'G' => self.stack_dragons(),
                'w' | 'W' => self.move_cursor_up(),
                'a' | 'A' => self.move_cursor_left(),
                's' | 'S' => self.move_cursor_down(),
                'd' | 'D' => self.move_cursor_right(),
                ' ' => self.select(),
                _ => (), // println!("{}", chr),
            }
            self.print();
        }
        println!("You wiiiin");
    }

    fn select(&mut self) {
        match self.mode {
            GameMode::SelectSource => {
                // Can't select goal cells
                if 4 <= self.cursor && self.cursor <= 6 {return;}
                // if selection is none or a DragonStack, don't select
                if let Some(rc_card) = self.cell_at(self.cursor).top() {
                    match &*rc_card {
                        Card::DragonStack => (),
                        _ => self.mode = GameMode::SelectDestination{cursor: self.cursor},
                    }
                }
            },
            GameMode::SelectDestination{cursor} => {
                let new_board = self.board.move_stack(
                    &Game::cursor_to_cci(cursor),
                    &Game::cursor_to_cci(self.cursor),
                );
                self.mode = match new_board {
                    Ok(board) => {
                        self.board = board.do_automoves();
                        GameMode::SelectSource
                    },
                    Err(MoveStackError::InvalidMove) => GameMode::SelectSource,
                    Err(MoveStackError::AmbiguousMove(max_height)) =>
                        GameMode::ChooseStackHeight{
                            cursor: cursor,
                            height: max_height,
                            max_height: max_height,
                        },
                }
            },
            GameMode::ChooseStackHeight{cursor, height, ..} => {
                let new_board = self.board.move_n_cards(
                    cursor as usize - 7,
                    self.cursor as usize - 7,
                    height as usize,
                );
                if let Some(board) = new_board {
                    self.board = board.do_automoves();
                }
                self.mode = GameMode::SelectSource
            }
        }
    }

    fn cursor_to_cci(cursor: u8) -> CardCellIndex {
        match cursor {
            1...3 => CardCellIndex::FreeCellIndex(cursor as usize - 1),
            4...6 => CardCellIndex::GoalCellIndex(cursor as usize - 4),
            7...14 => CardCellIndex::GameCellIndex(cursor as usize - 7),
            _ => panic!(format!("Invalid cursor value {}", cursor)),
        }
    }

    fn stack_dragons(&mut self) {
        match self.cell_at(self.cursor).top() {
            Some(rc_card) => match &*rc_card {
                &Card::DragonCard{suit} => {
                    match self.board.stack_dragons(suit) {
                        Some(board) => {
                            self.board = board.do_automoves();
                        },
                        None => (),
                    }
                },
                _ => (),
            }
            None => (),
        }
    }

    fn move_cursor_up(&mut self) {
        match self.mode {
            GameMode::ChooseStackHeight{height, cursor, max_height} =>
                if height < max_height {
                    self.mode = GameMode::ChooseStackHeight{
                        height: height + 1,
                        cursor: cursor,
                        max_height: max_height,
                    }
                },
            GameMode::SelectSource | GameMode::SelectDestination{..} =>
                self.cursor = match self.cursor {
                    7 => 2,
                    8...10 => 3,
                    11...13 => 4,
                    14 => 5,
                    _ => return,
                },
        }
    }

    fn move_cursor_left(&mut self) {
        match self.mode {
            GameMode::SelectSource | GameMode::SelectDestination{..} =>
                self.cursor = match self.cursor {
                    1 | 7 => return,
                    _ => self.cursor - 1
                },
            GameMode::ChooseStackHeight{..} => (),
        }
    }

    fn move_cursor_right(&mut self) {
        match self.mode {
            GameMode::SelectSource | GameMode::SelectDestination{..} =>
                self.cursor = match self.cursor {
                    6 | 14 => return,
                    _ => self.cursor + 1
                },
            GameMode::ChooseStackHeight{..} => (),
        }
    }

    fn move_cursor_down(&mut self) {
        match self.mode {
            GameMode::ChooseStackHeight{height, cursor, max_height} =>
                if height > 1 {
                    self.mode = GameMode::ChooseStackHeight{
                        height: height - 1,
                        cursor: cursor,
                        max_height: max_height,
                    }
                },
            GameMode::SelectSource | GameMode::SelectDestination{..} =>
                self.cursor = match self.cursor {
                    1 => 7,
                    2 | 3 => self.cursor + 5,
                    4 | 5 => self.cursor + 9,
                    6 => 14,
                    _ => return,
                },
        }
    }

    pub fn print_controls() {
        println!("{}", indoc!("
            Controls:
            - WASD to move the cursor (until I can figure out how to support the arrow keys)
            - Space to select or place a card
            - G to group the selected dragons
            - ? to show these controls
        "));
    }

    fn cell_at(&self, cursor: u8) -> &Rc<CardCell> {
        match cursor {
            1...3 => &self.board.free_cells()[cursor as usize - 1],
            4...6 => &self.board.goal_cells()[cursor as usize - 4],
            7...14 => &self.board.game_cells()[cursor as usize - 7],
            other => panic!(format!("unexpected cursor value {}", other)),
        }
    }
}
