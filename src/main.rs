use core::str;
use std::{fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}, path::Path};

use fastanvil::{Chunk, CurrentJavaChunk, Region};

pub trait DataHeader : Sized {
    fn header_size() -> u32;
    fn read(reader : &mut BufReader<File>) -> Result<Self, std::io::Error>;
    fn skip_data(&self, reader : &mut BufReader<File>) -> std::io::Result<()>;
}

#[derive(Debug)]
pub struct RecordHeader {
    pub ty : String,
    pub data_size : u32,
    pub flags : u32,
    pub id : u32,
    pub timestamp: u16,
    pub version_control: u16,
    pub internal_version : u16,
    pub misc : u16,
}

impl DataHeader for RecordHeader {
    fn header_size() -> u32 {
        24
    }

    fn read(reader : &mut BufReader<File>) -> Result<Self, std::io::Error> {
        let mut buf : [u8; 4] = [0; 4];
        let mut buf16 : [u8; 2] = [0; 2];

        let ty : String;
        reader.read_exact(&mut buf)?;
        ty = str::from_utf8(&buf).unwrap().into();

        let data_size : u32;
        reader.read_exact(&mut buf)?;
        data_size = u32::from_le_bytes(buf);

        let flags : u32;
        reader.read_exact(&mut buf)?;
        flags = u32::from_le_bytes(buf);

        let id : u32;
        reader.read_exact(&mut buf)?;
        id = u32::from_le_bytes(buf);

        let timestamp : u16;
        reader.read_exact(&mut buf16)?;
        timestamp = u16::from_le_bytes(buf16);
        
        let version_control : u16;
        reader.read_exact(&mut buf16)?;
        version_control = u16::from_le_bytes(buf16);

        let internal_version : u16;
        reader.read_exact(&mut buf16)?;
        internal_version = u16::from_le_bytes(buf16);

        let misc : u16;
        reader.read_exact(&mut buf16)?;
        misc = u16::from_le_bytes(buf16);

        Ok(RecordHeader {
            ty,
            data_size,
            flags,
            id,
            timestamp,
            version_control,
            internal_version,
            misc
        })
    }

    fn skip_data(&self, reader : &mut BufReader<File>) -> std::io::Result<()> {
        reader.seek_relative(self.data_size.into())
    }
}

#[derive(Debug)]
pub struct GroupHeader {
    pub ty : String,
    pub total_size : u32,
    pub label : [u8; 4],
    pub group_ty : i32,
    pub timestamp : u16,
    pub version_control : u16,
    pub misc : u32
}

impl DataHeader for GroupHeader {
    fn header_size() -> u32 {
        24
    }

    fn read(reader : &mut BufReader<File>) -> Result<Self, std::io::Error> {
        let mut buf : [u8; 4] = [0; 4];
        let mut buf16 : [u8; 2] = [0; 2];

        let ty : String;
        reader.read_exact(&mut buf)?;
        ty = str::from_utf8(&buf).unwrap().into();

        let total_size : u32;
        reader.read_exact(&mut buf)?;
        total_size = u32::from_le_bytes(buf);

        let label : [u8; 4];
        reader.read_exact(&mut buf)?;
        label = buf.clone();

        let group_ty : i32;
        reader.read_exact(&mut buf)?;
        group_ty = i32::from_le_bytes(buf);

        let timestamp : u16;
        reader.read_exact(&mut buf16)?;
        timestamp = u16::from_le_bytes(buf16);
        
        let version_control : u16;
        reader.read_exact(&mut buf16)?;
        version_control = u16::from_le_bytes(buf16);

        let misc : u32;
        reader.read_exact(&mut buf)?;
        misc = u32::from_le_bytes(buf);

        Ok(GroupHeader {
            ty,
            total_size,
            label,
            group_ty,
            timestamp,
            version_control,
            misc
        })
    }

    fn skip_data(&self, reader : &mut BufReader<File>) -> std::io::Result<()> {
        reader.seek_relative((self.total_size - GroupHeader::header_size()).into())
    }
}

#[derive(Debug)]
pub struct FieldHeader {
    pub ty : String,
    pub size : u16
}

impl DataHeader for FieldHeader {
    fn header_size() -> u32 {
        6
    }

    fn read(reader : &mut BufReader<File>) -> Result<Self, std::io::Error> {
        let mut buf : [u8; 4] = [0; 4];
        let mut buf16 : [u8; 2] = [0; 2];

        let ty : String;
        reader.read_exact(&mut buf)?;
        ty = str::from_utf8(&buf).unwrap().into();

        let size : u16;
        reader.read_exact(&mut buf16)?;
        size = u16::from_le_bytes(buf16);

        Ok(FieldHeader {
            ty,
            size
        })
    }

    fn skip_data(&self, reader : &mut BufReader<File>) -> std::io::Result<()> {
        reader.seek_relative(self.size.into())
    }
}

pub struct Group {

}

fn grab_world_children(buf_reader : &mut BufReader<File>) -> Result<GroupHeader, std::io::Error> {
    let tes4 = RecordHeader::read(buf_reader)?;

    assert_eq!(tes4.ty, "TES4");

    buf_reader.seek(SeekFrom::Current(tes4.data_size.into()))?;

    let mut group : GroupHeader;

    loop {
        group = GroupHeader::read(buf_reader)?;
        let label = str::from_utf8(&group.label).unwrap();
        if label == "WRLD" {
            break;
        }
        group.skip_data(buf_reader)?;
    }
    
    let world_record = RecordHeader::read(buf_reader)?;

    assert_eq!(world_record.ty, "WRLD");

    // We know this is EDID
    let edid = FieldHeader::read(buf_reader)?;

    let mut world_string_buf : Vec<u8> = Vec::new();
    buf_reader.read_until(b'\0', &mut world_string_buf)?;
    let world_string = String::from_utf8(world_string_buf).unwrap();
    assert_eq!(world_string, String::from("Tamriel\0"));

    buf_reader.seek_relative((world_record.data_size - u32::from(edid.size) - FieldHeader::header_size()).into())?;

    // World Children group:
    let group = GroupHeader::read(buf_reader)?;

    Ok(group)
}

fn main() {
    let skyrim = File::open("Skyrim.esm").unwrap();

    let mut buf_reader = BufReader::new(skyrim);

    let group = grab_world_children(&mut buf_reader);
    
    println!("{:?}", group);


    // println!("{:?}", chunk.block(0, -1023, 0));
}
