use std::{fs::{File, OpenOptions}, io::{BufReader, Read, Write}};

use world_gen::Chunk;

mod esm;
mod world_gen;

fn main() {
    // let mut r = fastanvil::Region::new(OpenOptions::new().read(true).write(true).open("r.0.0.mca").unwrap()).unwrap();
    // let mut c = Chunk::default();
    // c.draw_height(0, 0, -1024.0, -800.0, 2);
    // r.write_chunk(0, 0, &fastnbt::to_bytes(&c).unwrap()).unwrap();
    // let a = File::open("r.0.0-ref.mca").unwrap();
    // let mut r = fastanvil::Region::from_stream(a).unwrap();
    // let b = fastnbt::from_bytes::<fastanvil::CurrentJavaChunk>(&r.read_chunk(0, 0).unwrap().unwrap()).unwrap();
    // println!("{:?}", b.sections.unwrap());
    let skyrim = File::open("Skyrim.esm").unwrap();

    let mut buf_reader = BufReader::new(skyrim);
    esm::read_skyrim(&mut buf_reader).unwrap();
}
