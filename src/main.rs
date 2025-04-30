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

    #[arg(short, long)]
    out_path : Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let skyrim = File::open(args.file).unwrap();

    let out_dir = args.out_path.unwrap_or(PathBuf::from("./region"));
    
	if !out_dir.exists() {
		std::fs::create_dir_all(&out_dir).expect("Could not create gen directory.");
	}

    // Clean out .mca in the target directory, so we don't have weird overlaps with previously written data:
    for path in std::fs::read_dir(&out_dir).expect(&format!("Could not read directory {:?}", out_dir)) {
        let p = path.expect(&format!("Could not read directory entry")).path();
        let extension = p.extension().expect(&format!("Could not get extension of {:?}", p));
        if extension == "mca" {
            std::fs::remove_file(&p).expect(&format!("Could not remove file {p:?}."));
        }
    }

    let pth = out_dir.as_path();

    let mut buf_reader = BufReader::new(skyrim);
    esm::ESMReader::read(args.data_version, &mut buf_reader, pth);
}
