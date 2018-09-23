#[macro_use]
extern crate indoc;

mod game;
mod display;
mod board;
mod util;

fn main() {
    game::Game::print_controls();
    game::Game::deal().play();
}
