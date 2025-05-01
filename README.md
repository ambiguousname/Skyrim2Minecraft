# How to Run
1. Clone the repo. (`git clone https://github.com/ambiguousname/Skyrim2Minecraft.git`)
2. Copy `Skyrim.esm` or `Oblivion.esm` from your game's `Data` folder and copy it to this folder.
3. [Install Rust](https://rustup.rs/)
4. Run `cargo run Skyrim.esm skyrim` (or `cargo run Oblivion.esm oblivion`) in the terminal.
5. Copy the `datapacks` and `region` folders into the root of your [Minecraft Java save](https://minecraft.wiki/w/World).
6. Note that this program assumes that it can generate blocks anywhere from y-level `-592` to `624`, so you may need to use a datapack to increase the y-limit of the overworld! Or just [download the custom world](https://drive.proton.me/urls/HVWNDC03T0#VKyc404VoD40). 

Please note that this program has only been tested with Skyrim: Special Edition and Oblivion: Game of the Year Edition (2009), it may not work on older versions.

# Possible Improvements
- Water Height mapping
- Coloring terrain
	- VCLR data
	- Texture sampling
- Averaging heights
	- Add slabs
- Add stairs based on vertex normals
- Improve speed of chunk writing (it's definitely a bottleneck, but need to profile)