use std::{env, fs};

use psf2::Font;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("usage: {} <font.psf> [text]", args[0]);
    }
    let data = fs::read(&args[1]).unwrap();
    let font = Font::new(data).unwrap();
    let text = match args.get(2) {
        Some(x) => &x,
        None => "demo",
    };
    for c in text.chars() {
        let glyph = match font.get(c) {
            Some(x) => x,
            None => {
                eprintln!("missing glyph: {}", c);
                continue;
            }
        };
        for row in glyph.rows() {
            for column in row {
                let x = match column {
                    true => '█',
                    false => ' ',
                };
                print!("{}", x);
            }
            println!();
        }
    }
}
