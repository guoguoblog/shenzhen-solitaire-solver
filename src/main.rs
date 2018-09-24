#[macro_use]
extern crate indoc;

#[macro_use]
mod display;
mod game;
mod board;
mod util;

fn main() {
    game::Game::print_controls();
    game::Game::deal().play();
}
