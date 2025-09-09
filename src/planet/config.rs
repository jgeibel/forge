use bevy::prelude::*;

// Planet size presets (in chunks)
pub enum PlanetSize {
    Tiny,    // 2048x2048 blocks (64x64 chunks)
    Small,   // 4096x4096 blocks (128x128 chunks)
    Medium,  // 8192x8192 blocks (256x256 chunks)
    Default, // 16384x16384 blocks (512x512 chunks)
    Large,   // 32768x32768 blocks (1024x1024 chunks)
    Huge,    // 65536x65536 blocks (2048x2048 chunks)
}

impl PlanetSize {
    pub fn chunks(&self) -> i32 {
        match self {
            PlanetSize::Tiny => 64,
            PlanetSize::Small => 128,
            PlanetSize::Medium => 256,
            PlanetSize::Default => 512,
            PlanetSize::Large => 1024,
            PlanetSize::Huge => 2048,
        }
    }
    
    pub fn blocks(&self) -> i32 {
        self.chunks() * 32
    }
}

// Default planet dimensions in chunks
pub const PLANET_SIZE_CHUNKS: i32 = 512;  // 512x512 chunks = 16384x16384 blocks
pub const PLANET_HEIGHT_CHUNKS: i32 = 8;  // 8 chunks tall = 256 blocks

// Planet dimensions in blocks
pub const PLANET_SIZE_BLOCKS: i32 = PLANET_SIZE_CHUNKS * 32;  // 16384 blocks
pub const PLANET_HEIGHT_BLOCKS: i32 = PLANET_HEIGHT_CHUNKS * 32;  // 256 blocks

// World generation constants
pub const SEA_LEVEL: f32 = 64.0;  // Sea level height
pub const BEDROCK_LAYERS: i32 = 3;  // Bottom 3 layers are unbreakable
pub const MAX_ALTITUDE: f32 = 256.0;  // Maximum world height

#[derive(Resource, Clone)]
pub struct PlanetConfig {
    pub size_chunks: i32,
    pub height_chunks: i32,
    pub seed: u64,  // Changed to u64 for better seed range
    pub name: String,
    pub sea_level: f32,
}

impl Default for PlanetConfig {
    fn default() -> Self {
        Self {
            size_chunks: PLANET_SIZE_CHUNKS,
            height_chunks: PLANET_HEIGHT_CHUNKS,
            seed: 12345,
            name: "Terra".to_string(),
            sea_level: SEA_LEVEL,
        }
    }
}