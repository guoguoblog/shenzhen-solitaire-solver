mod board;
use board::TerminalPrintable;

fn main() {
    let mut b = board::Board::deal();
    println!("{}", b.print());
    b = b.do_automoves();
    println!("{}", b.print());
}
