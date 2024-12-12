use std::{collections::HashMap, fs::File, path::Path, sync::RwLock};

use serde::Serialize;

use crate::esm::{Cell, Land};

#[derive(Serialize)]
#[serde(rename_all="PascalCase")]
pub struct Block {
	pub name : String,

	pub properties : HashMap<String, String>
}

#[derive(Serialize)]
pub struct BlockState {
	pub palette : Vec<Block>,
	pub data : Option<Vec<i64>>
}

impl BlockState {
	pub fn new_from_palette(palette : Vec<Block>) -> BlockState {
		if palette.len() >= 16 {
			panic!("Palettes of length 16 or greater are not currently supported.");
		}

		let dat = vec![0; 256];

		BlockState {
			palette,
			data: Some(dat)
		}
	}

	pub fn draw_height(&mut self, idx : i8, x : usize, z : usize, start_y : usize, end_y : usize) {
		let dat = self.data.as_mut().unwrap();

		let z_shift = (z % 4) * 4;

		for y in start_y..(end_y + 1) {
			let xy_idx = x + (y * 16);
			let curr = dat[xy_idx];

			let zeroed = curr & !(0b1111 << z_shift);
			dat[xy_idx] = zeroed | ((idx as i64 & 0b1111) << z_shift);
			println!("{xy_idx} {z_shift} {} {}", curr, dat[xy_idx]);
		}
	}

	pub fn fill_layer(&mut self, idx : i8, y : usize) {
		let dat = self.data.as_mut().unwrap();

		let idx_64 = idx as i64;

		let mut full = 0;

		for i in 0..16 {
			full |= idx_64 << (i * 4);
		}

		for x in 0..16 {
			let xy_idx = x + (y * 16);
			dat[xy_idx] = full;
		}
	}
}

#[derive(Serialize)]
pub struct Biomes {
	pub palette : Vec<String>,
}

#[derive(Serialize)]
pub struct Section {
	#[serde(rename="Y")]
	pub y : i8,
	
	pub block_states : BlockState,

	pub biomes : Biomes,
}

/// fastanvil doesn't contain an implementation that's good enough for us.
/// 
/// Luckily, fastnbt can handle serialization for us.
#[derive(Serialize)]
#[serde(rename_all="PascalCase")]
pub struct Chunk {
	pub data_version : i32,

	#[serde(rename="xPos")]
	pub x_pos : i32,
	#[serde(rename="zPos")]
	pub z_pos : i32,
	#[serde(rename="yPos")]
	pub y_pos : i32,
	
	pub status : String,

	#[serde(rename="sections")]
	pub sections : Vec<Section>,
}

const MIN_Y : i32 = -1024;

impl Default for Chunk {
	fn default() -> Self {
		let mut bottom_block = BlockState::new_from_palette(vec![
			Block { 
				name: "minecraft:air".into(),
				properties: HashMap::new() 
			},
			Block{
				name: "minecraft:bedrock".into(),
				properties: HashMap::new()
			}
		]);
		bottom_block.fill_layer(1, 0);
		
		Self {
			data_version: 4189,

			x_pos: 0,
			y_pos: -1024,
			z_pos: 0,

			status: String::from("minecraft:full"),

			sections: vec![
				Section {
					y: (MIN_Y >> 4) as i8,
					block_states: bottom_block,
					biomes: Biomes {
						palette: vec!["minecraft:plains".into()],
					}
				}
			]
		}
	}
}

pub fn parse_land(land : Land) {
	// Order of operations:
	// Deduce region ranges and chunk ranges from Cell coordinates.
	// Write height data to these chunk ranges.

	// A region is 32 x 32 chunks.
	// We say a Minecraft block is 64 Skyrim Units.
	// So therefore one cell is 4 x 4 minecraft chunks.
	// Therefore, we can cram about 8 x 8 cells into one region.

	// So our current region position is:
	// floor(Cell Position/8)
	let curr_region_x = land.cell.x / 8; 
	let curr_region_y = land.cell.y / 8;

	let region_name = format!("r.{curr_region_x}.{curr_region_y}.mca");
	let region_path = Path::new(&region_name);

	// Our handy units mean we can only be in one region at a given time:
	let region = if region_path.exists() {
		let read = File::open(region_path).unwrap();
		fastanvil::Region::from_stream(read).unwrap()
	} else {
		let new_region = File::create(region_path).unwrap();
		fastanvil::Region::new(new_region).unwrap()
	};

	// Cells are comprised of 4 x 4 chunks, so we skip to the relevant starting chunk:
	let chunk_start_x = (land.cell.x % 8) * 4;
	let chunk_start_y = (land.cell.y % 8) * 4;

	// let default_sections = SectionTower::<Section> {
	// 	sections: vec![],
	// 	map: vec![],
	// 	-1024,
	// 	1024
	// };
	

	// let chunks : [CurrentJavaChunk; 4] = [CurrentJavaChunk {
	// 	data_version: 4189,
	// 	sections: SectionTower::<Section>{
	// 		sections: vec![],

	// 	}
	// }];

	for v in land.height_gradient {
		
	}
}