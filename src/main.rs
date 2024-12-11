use std::{fs::File, io::{BufReader, Read, Write}};

use world_gen::Chunk;

mod esm;
mod world_gen;

fn main() {
    let out_buf = std::io::Cursor::new(Vec::new());
    let mut r = fastanvil::Region::new(out_buf).unwrap();
    r.write_chunk(0, 0, &fastnbt::to_bytes(&Chunk::default()).unwrap()).unwrap();

    let inner_stream = r.into_inner().unwrap();
    File::create("r.0.0.mca").unwrap().write_all(&inner_stream.into_inner()).unwrap();
    // let a = File::open("r.0.0-ref.mca").unwrap();
    // let mut r = fastanvil::Region::from_stream(a).unwrap();
    // let b = fastnbt::from_bytes::<fastanvil::CurrentJavaChunk>(&r.read_chunk(0, 0).unwrap().unwrap()).unwrap();
    // println!("{:?}", b.sections.unwrap());
    // let skyrim = File::open("Skyrim.esm").unwrap();

    // let mut buf_reader = BufReader::new(skyrim);
    // esm::read_skyrim(&mut buf_reader).unwrap();
}
