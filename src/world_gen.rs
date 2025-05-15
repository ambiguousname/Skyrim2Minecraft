use std::{collections::HashMap, fs::{File, OpenOptions}, hash::Hash, io::Seek, path::Path};

use serde::Serialize;

use file_guard::Lock;
use crate::esm::Land;

#[derive(Serialize, Debug)]
#[serde(rename_all="PascalCase")]
pub struct Block {
	pub name : String,

	pub properties : HashMap<String, String>
}

#[derive(Serialize, Debug)]
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

		let z_shift = (x % 16) * 4;

		for y in start_y..end_y {
			let zy_idx = z + (y * 16);
			let curr = dat[zy_idx];

			let zeroed = curr & !(0b1111 << z_shift);
			dat[zy_idx] = zeroed | ((idx as i64 & 0b1111) << z_shift);
		}
	}

	pub fn fill_layer(&mut self, idx : i8, y : usize) {
		let dat = self.data.as_mut().unwrap();

		let idx_64 = idx as i64;

		let mut full = 0;

		for i in 0..16 {
			full |= idx_64 << (i * 4);
		}

		for z in 0..16 {
			let zy_idx = z + (y * 16);
			dat[zy_idx] = full;
		}
	}
}

#[derive(Serialize, Debug)]
pub struct Biomes {
	pub palette : Vec<String>,
}

impl Default for Biomes {
	fn default() -> Self {
		Self {
			palette: vec!["minecraft:plains".into()]
		}
	}
}

#[derive(Serialize, Debug)]
pub struct Section {
	#[serde(rename="Y")]
	pub y : i8,
	
	pub block_states : BlockState,

	pub biomes : Biomes,
}

/// fastanvil doesn't contain an implementation that's good enough for us.
/// 
/// Luckily, fastnbt can handle serialization for us.
#[derive(Serialize, Debug)]
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

const MIN_Y : i32 = -592;

impl Chunk {
	pub fn into_arr(_ : usize) -> Chunk {
		Chunk::default()
	}

	pub fn default_palette() -> Vec<Block> {
		vec![
			Block { 
				name: "minecraft:air".into(),
				properties: HashMap::new() 
			},
			Block{
				name: "minecraft:bedrock".into(),
				properties: HashMap::new()
			},
			Block{
				name: "minecraft:stone".into(),
				properties: HashMap::new()
			},
			Block {
				name: "minecraft:water".into(),
				properties: HashMap::new()
			}
		]
	}

	pub fn draw_height(&mut self, x : usize, z : usize, start_height : f32, end_height : f32, idx : i8) {
		let mut i = start_height;
		while i < end_height.floor() {
			let curr_y = i as i32;

			let next_idx : usize = (((curr_y - MIN_Y) >> 4) as i8).try_into().expect(&format!("Could not convert index {curr_y}."));
			let matching_section = self.sections.get_mut(next_idx);
			
			let section = if let Some(s) = matching_section {
				s
			} else {
				// Add sections until we hit the target height:
				let start = self.sections.last().expect("Could not get last section.").y;
				
				let start_idx = self.sections.len() - 1;
				for j in start_idx..next_idx {
					let y = start + ((j - start_idx) as i8) + 1;
					
					self.sections.push(Section{
						y,
						block_states: BlockState::new_from_palette(Self::default_palette()),
						biomes: Biomes::default()
					});
				}
				self.sections.last_mut().unwrap()
			};

			let height_start = curr_y.rem_euclid(16) as usize;
			let height_draw = std::cmp::min(16 - height_start, (end_height - i).abs().round_ties_even() as usize);

			section.block_states.draw_height(idx, x, z, height_start, height_start + height_draw);
			i += height_draw as f32;
		}
	}
}

impl Default for Chunk {
	fn default() -> Self {
		let mut bottom_block = BlockState::new_from_palette(Self::default_palette());
		bottom_block.fill_layer(1, 0);
		
		Self {
			data_version: 4189,

			x_pos: 0,
			y_pos: MIN_Y,
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

pub fn parse_land(land : Land, out_folder : &Path) {
	// Order of operations:
	// Deduce region ranges and chunk ranges from Cell coordinates.
	// Write height data to these chunk ranges.

	// A region is 32 x 32 chunks.
	// We say a Minecraft block is 64 Skyrim Units.
	// So therefore one cell is 4 x 4 minecraft chunks.
	// Therefore, we can cram about 8 x 8 cells into one region.

	// So our current region position is:
	// floor(Cell Position/8)
	// TODO: Not sure this is giving us negative regions?
	let curr_region_x = land.cell.x.div_euclid(8);
	let curr_region_y = land.cell.y.div_euclid(8);

	// Cells are comprised of 4 x 4 chunks, so we skip to the relevant starting chunk.
	// We need this in chunk coordinates relative to the world origin (0, 0).
	// Per cubicmetre, -Z is North (and -X is West).
	let chunk_start_x  = land.cell.x * 4;
	let chunk_start_z = land.cell.y * 4;

	let mut chunks : [Chunk; 16] = core::array::from_fn(Chunk::into_arr);

	let mut row_offset : f32 = 0.0;
	let mut curr_offset = land.offset_height;

	// TODO: Is this conversion right?
	let water_height = land.cell.water_height.map(|h| { h/64.0 });

	for (i, v) in land.height_gradient.iter().enumerate() {
		let r = i / 33;
		let c = i % 33;

		// Each vertex is 128 units apart, or 2 blocks apart.
		// There are 32 vertices in a row/col, and those are split over 4 chunks.
		// So we have 8 vertices per chunk.
		let curr_chunk_x = (c / 8) % 4;
		let curr_chunk_z = (r / 8) % 4;

		// println!("{r},{c} {curr_chunk_z},{curr_chunk_x}");

		let chunk = &mut chunks[curr_chunk_x + curr_chunk_z * 4];
		// Too lazy to come up with a better way to do this, so we always set the x_pos:
		chunk.x_pos = curr_chunk_x as i32 + chunk_start_x;
		chunk.z_pos = curr_chunk_z as i32 + chunk_start_z;

		let vert_height = *v as f32;

		if c == 0 {
			row_offset = 0.0;
			curr_offset += vert_height;
		} else {
			row_offset += vert_height;
		}

		// Conversion is: (height * 8)/(64) (Vert units -> Skyrim Units -> Minecraft Units).
		// But it's just easier to divide by 8.
		let block_height = (row_offset + curr_offset)/(8.0);

		// TODO: We currently drop the last vertex because we don't account for it. We treat each vertex as having influence over blocks 2 x 2in front of it.
		// An area of influence would probably be better.
		if c == 32 || r == 32 {
			continue;
		}

		let block_x = (c % 8) * 2;
		let block_z = (r % 8) * 2;

		let min_yf = MIN_Y as f32;
		let start_height = min_yf + 1.0;
		let end_height = block_height + 1.0;

		// Vertices are two blocks apart, so we write in a 2 x 2 block grid:
		// Shifting everything up by one to avoid overwriting bedrock.
		chunk.draw_height(block_x, block_z, start_height, end_height, 2);
		chunk.draw_height(block_x + 1, block_z, start_height, end_height, 2);
		chunk.draw_height(block_x, block_z + 1, start_height, end_height, 2);
		chunk.draw_height(block_x + 1, block_z + 1, start_height, end_height, 2);

		// if let Some(h) = water_height {
		// 	if h > end_height {
		// 		// FIXME: Not sure we account for slight block offsets like this, see line 168:
		// 		let start = end_height + 1.0;
		// 		chunk.draw_height(block_x, block_z, start, h, 3);
		// 		chunk.draw_height(block_x + 1, block_z, start, h, 3);
		// 		chunk.draw_height(block_x, block_z + 1, start, h, 3);
		// 		chunk.draw_height(block_x + 1, block_z + 1, start, h, 3);
		// 	}
		// }
	}

	let region_name = format!("r.{curr_region_x}.{curr_region_y}.mca");
	let region_path = out_folder.join(region_name);
	
	let region_exists = region_path.exists();

	let mut file =	if region_exists {
		OpenOptions::new().read(true).write(true).open(region_path).unwrap()
	} else {
		OpenOptions::new().read(true).write(true).create(true).open(region_path).unwrap()
	};

	
	let mut lock = file_guard::lock(&mut file, Lock::Exclusive, 0, usize::MAX).expect("Could not lock file.");

	{
		let f = &mut lock as &mut File;
		
		// Our handy units mean we can only be in one region at a given time:
		let mut region = if region_exists {
			fastanvil::Region::from_stream(f).unwrap()
		} else {
			fastanvil::Region::new(f).unwrap()
		};

		for c in chunks {
			region.write_chunk((c.x_pos).rem_euclid(32) as usize, (c.z_pos).rem_euclid(32) as usize, &fastnbt::to_bytes(&c).unwrap()).unwrap();
		}
	}
}