use core::str;
use std::{fs::File, io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom}};

use clap::ValueEnum;
use flate2::read::ZlibDecoder;
use indicatif::{ProgressBar, ProgressStyle};

use crate::world_gen::parse_land;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DataVersion {
    Skyrim,
    Oblivion
}

pub struct ESMReader<'a> {
    version : DataVersion,
    reader : &'a mut BufReader<File>,
}

impl<'a> ESMReader<'a> {

    fn grab_world_children(&mut self) -> Result<GroupHeader, std::io::Error> {
        let tes4 = RecordHeader::read(self.reader, self.version)?;
    
        assert_eq!(tes4.ty, "TES4");
    
        self.reader.seek(SeekFrom::Current(tes4.data_size.into()))?;
    
        let mut group : GroupHeader;
    
        loop {
            group = GroupHeader::read(self.reader, self.version)?;
            let label = str::from_utf8(&group.label).unwrap();
            if label == "WRLD" {
                break;
            }
            group.skip_data(self.reader)?;
        }
        
        let world_record = RecordHeader::read(self.reader, self.version)?;
    
        assert_eq!(world_record.ty, "WRLD");
    
        // We know this is EDID
        let edid = FieldHeader::read(self.reader, self.version)?;
    
        let mut world_string_buf : Vec<u8> = Vec::new();
        self.reader.read_until(b'\0', &mut world_string_buf)?;
        let world_string = String::from_utf8(world_string_buf).unwrap();
        assert_eq!(world_string, String::from("Tamriel\0"));
    
        self.reader.seek_relative((world_record.data_size - u32::from(edid.size) - FieldHeader::header_size(self.version)).into())?;
        
        // World Children group:
        let group = GroupHeader::read(self.reader, self.version)?;
    
        Ok(group)
    }

    pub fn read(version : DataVersion, reader : &'a mut BufReader<File>) {
        let mut esm_reader = Self {
            version,
            reader
        };

        let world_group = esm_reader.grab_world_children().expect("Could not grab world children.");

        // If we're in Oblivion, the ROAD record is first:
        if matches!(esm_reader.version, DataVersion::Oblivion) {
            let road = RecordHeader::read(esm_reader.reader, esm_reader.version).expect("Could not read road header.");
            road.skip_data(esm_reader.reader).expect("Could not skip ROAD record data.");
        }
    
        // Read the first cell and its children:
        let first_world_cell = RecordHeader::read(esm_reader.reader, esm_reader.version).expect("Could not read record header.");
    
        let (cell_total_read, _) = ESMReader::read_cell(esm_reader.reader, esm_reader.version, first_world_cell).expect("Could not read cell.");
    
        let mut world_bytes_left = world_group.total_size - (GroupHeader::header_size(esm_reader.version) + cell_total_read);
    
        let bar = ProgressBar::new(11186);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:100} {msg}").unwrap());
    
        while world_bytes_left > 0 {
            let block = GroupHeader::read(esm_reader.reader, esm_reader.version).expect("Could not read group header.");
    
            let mut block_left_to_read = block.total_size - GroupHeader::header_size(esm_reader.version);
            
            while block_left_to_read > 0 {
                let subblock = GroupHeader::read(esm_reader.reader, esm_reader.version).expect("Could not read group header.");
    
                let mut subblock_left_to_read = subblock.total_size - GroupHeader::header_size(esm_reader.version);
    
                while subblock_left_to_read > 0 {
                    let cell = RecordHeader::read(esm_reader.reader, esm_reader.version).expect("Could not read record header.");
    
                    // TODO: Don't think this is async, cells don't have a record of their length.
                    // rayon::spawn(move || { 
                    let (left, c) = ESMReader::read_cell(esm_reader.reader, esm_reader.version, cell).expect("Could not read cell.");
                    // });
                    
                    bar.inc(1);
                    bar.set_message(format!("{},{}", c.x, c.y));

                    subblock_left_to_read -= left;
                }
    
                block_left_to_read -= subblock.total_size;
            }
    
            world_bytes_left -= block.total_size;
        }
    }

    /// Returns bytes read.
    fn read_cell(reader : &mut (impl Read + Seek), version : DataVersion, cell : RecordHeader) -> std::io::Result<(u32, Cell)> {
        let mut chunk = reader.take(cell.data_size as u64);

        // If the cell is compressed:
        let mut r = if cell.flags & 0x00040000 == 0x00040000 {
            let mut buf : [u8; 4] = [0; 4];
            chunk.read_exact(&mut buf)?;

            let decrypted_size = u32::from_le_bytes(buf);
        
            let mut out_cell = vec![0; decrypted_size as usize];
        
            ZlibDecoder::new(chunk).read_to_end(&mut out_cell)?;
        
            Cursor::new(out_cell)
        } else {
            let mut out = Vec::with_capacity(cell.data_size as usize);
            chunk.read_to_end(&mut out)?;
            Cursor::new(out)
        };
        
        let x : i32;
        let y : i32;

        loop {
            let field = FieldHeader::read(&mut r, version)?;
            // Cell location:
            if field.ty == "XCLC" {
                let mut buf : [u8; 4] = [0; 4];

                r.read_exact(&mut buf)?;
                x = i32::from_le_bytes(buf);

                r.read_exact(&mut buf)?;
                y = i32::from_le_bytes(buf);
                break;
            } else {
                field.skip_data(&mut r)?;
                continue;
            }
        }
        let total_read = ESMReader::read_cell_refs(reader, version, Cell {x, y}).expect("Could not read cell refs.") + cell.data_size + RecordHeader::header_size(version);
        
        Ok((total_read, Cell{x, y}))
    }

    /// Returns bytes read.
    fn read_cell_refs(reader : &mut (impl Read + Seek), version : DataVersion, cell : Cell) -> std::io::Result<u32> {
        let cell_child_grp = GroupHeader::read(reader, version)?;
        assert_eq!(cell_child_grp.ty, "GRUP");

        // Now we find the temporary children group (where LAND is always stored)

        let mut temp_child = GroupHeader::read(reader, version)?;
        if temp_child.group_ty != 9 {
            temp_child.skip_data(reader)?;

            temp_child = GroupHeader::read(reader, version)?;
        }
        
        assert_eq!(temp_child.ty, "GRUP");

        let mut left_to_read = temp_child.total_size - GroupHeader::header_size(version);

        while left_to_read > 0 {
            let record_header = RecordHeader::read(reader, version)?;
            if record_header.ty == "LAND" {
                Land::read(reader, version, cell.clone(), &record_header)?;
            } else {
                record_header.skip_data(reader)?;
            }

            left_to_read -= (record_header.data_size as u32) + RecordHeader::header_size(version);
        }
        Ok(cell_child_grp.total_size)
    }
}

pub trait DataHeader : Sized {
    fn header_size(version : DataVersion) -> u32;
    fn read(reader : &mut (impl Read + Seek), version : DataVersion) -> Result<Self, std::io::Error>;
    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()>;
}

#[derive(Debug, Default)]
struct RecordHeader {
    pub ty : String,
    pub data_size : u32,
    pub flags : u32,
    pub id : u32,
    pub timestamp: Option<u16>,
    pub version_control: u32,
    pub internal_version : Option<u16>,
    pub misc : Option<u16>,
}

impl DataHeader for RecordHeader {
    fn header_size(version : DataVersion) -> u32 {
        match version {
            DataVersion::Skyrim => 24,
            DataVersion::Oblivion => 20
        }
    }

    fn read(reader : &mut (impl Read + Seek), version : DataVersion) -> Result<Self, std::io::Error> {
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

        
        let timestamp : Option<u16>;
        let version_control : u32;
        let internal_version : Option<u16>;
        let misc : Option<u16>;

        match version {
            DataVersion::Skyrim => {
                reader.read_exact(&mut buf16)?;
                timestamp = Some(u16::from_le_bytes(buf16));
                
                reader.read_exact(&mut buf16)?;
                version_control = u16::from_le_bytes(buf16) as u32;
        
                reader.read_exact(&mut buf16)?;
                internal_version = Some(u16::from_le_bytes(buf16));
        
                reader.read_exact(&mut buf16)?;
                misc = Some(u16::from_le_bytes(buf16));
            },
            DataVersion::Oblivion => {
                reader.read_exact(&mut buf)?;
                version_control = u32::from_le_bytes(buf);

                timestamp = None;
                internal_version = None;
                misc = None;
            }
        };

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
    pub header_size : u32,
    pub total_size : u32,
    pub label : [u8; 4],
    pub group_ty : i32,
    pub timestamp : Option<u16>,
    pub version_control : u32,
    pub misc : Option<u32>
}

impl DataHeader for GroupHeader {
    fn header_size(version : DataVersion) -> u32 {
        match version {
            DataVersion::Skyrim => 24,
            DataVersion::Oblivion => 20
        }
    }

    fn read(reader : &mut (impl Read + Seek), version : DataVersion) -> Result<Self, std::io::Error> {
        let mut buf : [u8; 4] = [0; 4];
        let mut buf16 : [u8; 2] = [0; 2];

        let header_size = match version {
            DataVersion::Oblivion => 20,
            DataVersion::Skyrim => 24,
        };

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

        
        let timestamp : Option<u16>;
        let version_control : u32;
        let misc : Option<u32>;

        match version {
            DataVersion::Skyrim => {
                reader.read_exact(&mut buf16)?;
                timestamp = Some(u16::from_le_bytes(buf16));
                
                reader.read_exact(&mut buf16)?;
                version_control = u16::from_le_bytes(buf16) as u32;
        
                reader.read_exact(&mut buf)?;
                misc = Some(u32::from_le_bytes(buf));
            },
            DataVersion::Oblivion => {
                reader.read_exact(&mut buf)?;
                version_control = u32::from_le_bytes(buf);

                timestamp = None;
                misc = None;
            }
        }

        Ok(GroupHeader {
            ty,
            total_size,
            header_size,
            label,
            group_ty,
            timestamp,
            version_control,
            misc
        })
    }

    fn skip_data(&self, reader : &mut (impl Read + Seek)) -> std::io::Result<()> {
        reader.seek_relative((self.total_size - self.header_size).into())
    }
}

#[derive(Debug)]
struct FieldHeader {
    pub ty : String,
    pub size : u16
}

impl DataHeader for FieldHeader {
    fn header_size(_version : DataVersion) -> u32 {
        6
    }

    // Data version does not matter for fields.
    fn read(reader : &mut (impl Read + Seek), _version : DataVersion) -> Result<Self, std::io::Error> {
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
#[derive(Clone, Debug)]
pub struct Cell {
	pub x : i32,
	pub y : i32
}

#[derive(Debug)]
pub struct Land {
    pub cell : Cell,
	pub offset_height : f32,
	pub height_gradient : Vec<i8>,
}

impl Land {
    pub(self) fn read(reader : &mut (impl Read + Seek), version : DataVersion, cell : Cell, land : &RecordHeader) -> std::io::Result<()> {
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
            let field = FieldHeader::read(&mut land_cursor, version)?;
    
            if field.ty == "VHGT" {
                land_cursor.read_exact(&mut buf)?;
    
                // Based on https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/LAND
                let offset_height = f32::from_le_bytes(buf);
    
                let mut height_gradient = Vec::with_capacity(1089);
    
                let mut byte : [u8; 1] = [0; 1];
    
                for _ in 0..1089 {
                    land_cursor.read_exact(&mut byte)?;
    
                    let height_byte = i8::from_le_bytes(byte);
    
                    height_gradient.push(height_byte);
                }
    
                parse_land(Land {
                    cell,
                    offset_height,
                    height_gradient
                });
                break;
            } else {
                field.skip_data(&mut land_cursor)?;
            }
    
            left_to_read -= field.size as u32 + FieldHeader::header_size(version);
        }
    
        Ok(())
    }
}