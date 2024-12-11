use std::{fs::File, io::BufReader};

mod esm;
mod world_gen;

fn main() {
    let skyrim = File::open("Skyrim.esm").unwrap();

    let mut buf_reader = BufReader::new(skyrim);
    esm::read_skyrim(&mut buf_reader).unwrap();
}
