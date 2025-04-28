use std::{fs::File, io::BufReader, path::PathBuf};

use clap::Parser;
use esm::DataVersion;

mod esm;
mod world_gen;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// .esm file to load world data from and convert into Minecraft .mca files.
    file : PathBuf,

    /// ESM Data Version to use.
    #[arg(value_enum)]
    data_version : DataVersion,
}

fn main() {
    let args = Args::parse();

    let skyrim = File::open(args.file).unwrap();

    let mut buf_reader = BufReader::new(skyrim);
    esm::ESMReader::read(args.data_version, &mut buf_reader).unwrap();
}
