use core::str;
use std::{fs::File, io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom}};

use flate2::read::ZlibDecoder;

pub trait DataHeader : Sized {
    fn header_size() -> u32;
    fn read(reader : &mut (impl Read + Seek)) -> Result<Self, std::io::Error>;
    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()>;
}

#[derive(Debug)]
struct RecordHeader {
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

    fn read(reader : &mut (impl Read + Seek)) -> Result<Self, std::io::Error> {
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

    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
        reader.seek_relative(self.data_size.into())
    }
}

#[derive(Debug)]
struct GroupHeader {
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

    fn read(reader : &mut (impl Read + Seek)) -> Result<Self, std::io::Error> {
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

    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
        reader.seek_relative((self.total_size - GroupHeader::header_size()).into())
    }
}

#[derive(Debug)]
struct FieldHeader {
    pub ty : String,
    pub size : u16
}

impl DataHeader for FieldHeader {
    fn header_size() -> u32 {
        6
    }

    fn read(reader : &mut (impl Read + Seek)) -> Result<Self, std::io::Error> {
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

    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
        reader.seek_relative(self.size.into())
    }
}

#[derive(Debug)]
struct Land {
	offset_height : f32,
	height_gradient : Vec<u8>,
}

struct Cell {
	x : i32,
	y : i32
}

fn read_land(cell : Cell, land : RecordHeader, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
	let mut buf : [u8; 4] = [0; 4];

    reader.read_exact(&mut buf)?;
    let decrypted_size = u32::from_le_bytes(buf);

    // Subtract the 4 bytes we just read:
    let compressed_chunk = reader.take((land.data_size as u64) - (4 as u64));

    let mut out_land = Vec::with_capacity(decrypted_size.try_into().unwrap());

    ZlibDecoder::new(compressed_chunk).read_to_end(&mut out_land)?;

    let mut land_cursor = Cursor::new(out_land);

	let mut left_to_read = decrypted_size;

	while left_to_read > 0 {
		let field = FieldHeader::read(&mut land_cursor)?;

		if field.ty == "VHGT" {
			land_cursor.read_exact(&mut buf)?;

			// Based on https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/LAND
			let offset_height = f32::from_le_bytes(buf);

			let mut height_gradient = Vec::with_capacity(1089);

			let mut byte : [u8; 1] = [0; 1];

			for i in 0..1089 {
				land_cursor.read_exact(&mut byte)?;

				let height_byte = u8::from_le_bytes(byte);

				height_gradient.push(height_byte);
			}

			let l = Land {
				offset_height,
				height_gradient
			};

			println!("{l:?}");
			
			break;
		} else {
			field.skip_data(&mut land_cursor)?;
		}

		left_to_read -= field.size as u32 + FieldHeader::header_size();
	}

	Ok(())
}

fn read_cell_refs(cell : Cell, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
    let cell_child_grp = GroupHeader::read(reader)?;
    assert_eq!(cell_child_grp.ty, "GRUP");

    // Now we find the temporary children group (where LAND is always stored)

    let mut temp_child = GroupHeader::read(reader)?;
    if temp_child.group_ty != 9 {
        temp_child.skip_data(reader)?;

        temp_child = GroupHeader::read(reader)?;
    }
    
    assert_eq!(temp_child.ty, "GRUP");

    let mut left_to_read = temp_child.total_size - GroupHeader::header_size();

    while left_to_read > 0 {
        let record_header = RecordHeader::read(reader)?;

        if record_header.ty == "LAND" {
			read_land(cell, record_header, reader).unwrap();
            reader.seek_relative(left_to_read as i64)?;
            break;
        }

        left_to_read -= (record_header.data_size as u32) + RecordHeader::header_size();
    }
    Ok(())
}

fn read_cell(cell : RecordHeader, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
    // Assume the cell is encrypted:
    let mut buf : [u8; 4] = [0; 4];
    reader.read_exact(&mut buf)?;
    let decrypted_size = u32::from_le_bytes(buf);

    let x : i32;
    let y : i32;

    // Subtract the 4 bytes we just read:
    let compressed_chunk = reader.take((cell.data_size as u64) - (4 as u64));

    let mut out_cell = Vec::with_capacity(decrypted_size.try_into().unwrap());

    ZlibDecoder::new(compressed_chunk).read_to_end(&mut out_cell)?;

    let mut cell_cursor = Cursor::new(out_cell);

    loop {
        let field = FieldHeader::read(&mut cell_cursor)?;
        // Cell location:
        if field.ty == "XCLC" {
            let mut buf : [u8; 4] = [0; 4];

            cell_cursor.read_exact(&mut buf)?;
            x = i32::from_le_bytes(buf);

            cell_cursor.read_exact(&mut buf)?;
            y = i32::from_le_bytes(buf);
            break;
        } else {
            field.skip_data(&mut cell_cursor)?;
            continue;
        }
    }
    read_cell_refs(Cell {x, y}, reader)?;
    
    Ok(())
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

pub fn read_skyrim(reader : &mut BufReader<File>) -> std::io::Result<()> {
	let _world_group = grab_world_children(reader)?;

    // Read the first cell and its children:
    let first_world_cell = RecordHeader::read(reader)?;
    read_cell(first_world_cell, reader)?;

    loop {
        let _block = GroupHeader::read(reader)?;
        loop {
            let _subblock = GroupHeader::read(reader)?;

            let cell = RecordHeader::read(reader)?;
            read_cell(cell, reader)?;
            break;
        }
        break;
    }

	Ok(())
}