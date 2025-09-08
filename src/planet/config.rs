use bevy::prelude::*;

// Planet dimensions in chunks
pub const PLANET_SIZE_CHUNKS: i32 = 64;  // 64x64 chunks = 2048x2048 blocks
pub const PLANET_HEIGHT_CHUNKS: i32 = 8;  // 8 chunks tall = 256 blocks

// Planet dimensions in blocks
pub const PLANET_SIZE_BLOCKS: i32 = PLANET_SIZE_CHUNKS * 32;  // 2048 blocks
pub const PLANET_HEIGHT_BLOCKS: i32 = PLANET_HEIGHT_CHUNKS * 32;  // 256 blocks

// Bedrock layer
pub const BEDROCK_LAYERS: i32 = 3;  // Bottom 3 layers are unbreakable

// Maximum altitude player can reach
pub const MAX_ALTITUDE: f32 = 256.0;  // Same as world height

#[derive(Resource, Clone)]
pub struct PlanetConfig {
    pub size_chunks: i32,
    pub height_chunks: i32,
    pub seed: u32,
    pub name: String,
}

impl Default for PlanetConfig {
    fn default() -> Self {
        Self {
            size_chunks: PLANET_SIZE_CHUNKS,
            height_chunks: PLANET_HEIGHT_CHUNKS,
            seed: 12345,
            name: "Terra".to_string(),
        }
    }
}