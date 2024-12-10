use std::{fs::File, io::BufReader};

use fastanvil;

mod esm;

fn main() {
    let skyrim = File::open("Skyrim.esm").unwrap();

    let mut buf_reader = BufReader::new(skyrim);
    esm::read_skyrim(&mut buf_reader).unwrap();
}
