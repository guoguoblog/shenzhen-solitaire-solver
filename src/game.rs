extern crate getch;

use std::rc::Rc;

use ::board::{Board, CardCell, Card};
use ::display::{display_card, display_cell};
use ::util;

/// Formats a string literal as a "selected" marker in the game ui.
///
/// In practice, this turns the string yellow.
macro_rules! selector_color {
    ($text: tt) => {
        concat!("\x1b[38;5;11m", $text, "\x1b[0m")
    }
}

enum GameMode {
    SelectSource,
    SelectDestination{cursor: u8},
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

        match self.mode {
            GameMode::SelectDestination{cursor: cursor @ 1...3} =>
                top_row[cursor as usize - 1] = selector_color!("v"),
            _ => (),
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

        let mut strings: Vec<_> = self.board.game_cells().iter().map(|cell| display_cell(cell)).collect();
        match self.cursor {
            7...14 => strings[self.cursor as usize - 7].push_str("\n^"),
            _ => (),
        }
        match self.mode {
            GameMode::SelectDestination{cursor: cursor @ 7...14} =>
                strings[cursor as usize - 7].push_str(concat!("\n", selector_color!("^"))),
            _ => (),
        }
        s.push_str(&util::join_vertical(strings));
        println!("{}", s);
    }

    pub fn play(&mut self) {
        self.print();
        self.board = self.board.do_automoves();
        self.print();
        let g = getch::Getch::new();
        let mut chr: u8 = 0;
        while chr != 3 {
            chr = match g.getch() {
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
                // if selection is none don't select
                if self.cell_at(self.cursor).top().is_some() {
                    self.mode = GameMode::SelectDestination{cursor: self.cursor};
                }
            },
            GameMode::SelectDestination{cursor} => {
                if self.cursor < 7 {unimplemented!();}
                match self.board.move_stack(cursor as usize - 7, self.cursor as usize - 7) {
                    Some(board) => {
                        self.board = board.do_automoves();
                    }
                    None => (),
                }
                self.mode = GameMode::SelectSource;
            }
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
        self.cursor = match self.cursor {
            7 => 2,
            8...10 => 3,
            11...13 => 4,
            14 => 5,
            _ => return,
        };
    }

    fn move_cursor_left(&mut self) {
        self.cursor = match self.cursor {
            1 | 7 => return,
            _ => self.cursor - 1
        };
    }

    fn move_cursor_right(&mut self) {
        self.cursor = match self.cursor {
            6 | 14 => return,
            _ => self.cursor + 1
        };
    }

    fn move_cursor_down(&mut self) {
        self.cursor = match self.cursor {
            1 => 7,
            2 | 3 => self.cursor + 5,
            4 | 5 => self.cursor + 9,
            6 => 14,
            _ => return,
        };
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
