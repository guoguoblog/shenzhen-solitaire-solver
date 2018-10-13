#[macro_use]
extern crate indoc;

#[macro_use]
mod display;
mod game;
mod board;
mod util;
mod solver;


fn print_usage(exe: &str) {
    println!("usage: {} {{play,solve}} [seed]", exe);
}

fn main() {
    let exe = std::env::args().nth(0).expect("Could not find executable name");

    let (b, seed) = match std::env::args().nth(2) {
        Some(seed_str) => {
            let seed = board::Seed::from_string(&seed_str);
            (board::Board::deal_seeded(&seed), seed)
        },
        None => board::Board::deal()
    };

    match std::env::args().nth(1).as_ref().map(|cmd| cmd.as_str()) {
        Some("play") => {
            println!("{}\n", seed);
            game::Game::print_controls();
            game::Game::new(b).play();
        }
        Some("solve") => {
            println!("{}", seed);
            println!("{}", display::display_board(&b));
            let b2 = b.do_automoves();

            let states = solver::solve(&b2).expect("no answer");
            for board in states {
                println!("{}", display::display_board(&board));
            }
        }
        None => print_usage(&exe),
        Some(cmd) => {
            print_usage(&exe);
            println!(
                "{}: error: argument cmd: invalid choice: '{}' (choose from 'play', 'solve')",
                &exe, cmd,
            );
        }
    }

}
