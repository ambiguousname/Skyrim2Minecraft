use std::{fs::File, path::Path, sync::RwLock};

use fastanvil;

use crate::esm::{Cell, Land};

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

	let chunk_start_x = land.cell.x ;

	// let curr_region_x = &mut m.region_x;
	// let curr_region_y = &mut m.region_y;

	
	for v in land.height_gradient {
		
	}
}