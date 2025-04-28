use std::{fs::File, io::BufReader, path::Path};

mod esm;
mod world_gen;

fn main() {
    let args : Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: Skyrim2Minecraft <.esm filepath>");
        return;
    }

    let pth = Path::new(&args[1]);
    if !pth.exists() {
        println!("{} does not exist.", pth.display());
        return;
    }

    let skyrim = File::open(pth).unwrap();

    let mut buf_reader = BufReader::new(skyrim);
    esm::ESMReader::read(esm::DataVersion::Skyrim, &mut buf_reader).unwrap();
}
