#[macro_use]
extern crate indoc;

#[macro_use]
mod display;
mod game;
mod board;
mod util;
mod solver;

fn main() {
    // game::Game::print_controls();
    // game::Game::deal().play();

    let b = board::Board::deal();
    println!("{}", display::display_board(&b));
    let b2 = b.do_automoves();

    let states = solver::solve(&b2).expect("no answer");
    for board in states {
        println!("{}", display::display_board(&board));
    }
}
