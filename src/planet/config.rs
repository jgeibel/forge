use bevy::prelude::*;

// Planet size presets (in chunks)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetSize {
    Tiny,        // 2048x2048 blocks (64x64 chunks)
    Small,       // 4096x4096 blocks (128x128 chunks)
    Medium,      // 8192x8192 blocks (256x256 chunks)
    Default,     // 16384x16384 blocks (512x512 chunks) - old default
    Large,       // 32768x32768 blocks (1024x1024 chunks)
    Huge,        // 65536x65536 blocks (2048x2048 chunks)
    Realistic,   // 524288x524288 blocks (16384x16384 chunks) ~524km circumference
    Continental, // 2097152x2097152 blocks (65536x65536 chunks) ~2097km circumference
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
            PlanetSize::Realistic => 16384,
            PlanetSize::Continental => 65536,
        }
    }

    pub fn blocks(&self) -> i32 {
        self.chunks() * 32
    }

    pub fn circumference_km(&self) -> f32 {
        // Assuming 1 block = 1 meter
        self.blocks() as f32 / 1000.0
    }
}

// Default planet dimensions in chunks - now using Realistic size
pub const PLANET_SIZE_CHUNKS: i32 = 16384; // 16384x16384 chunks = 524288x524288 blocks
pub const PLANET_HEIGHT_CHUNKS: i32 = 8; // 8 chunks tall = 256 blocks

// Planet dimensions in blocks
pub const PLANET_SIZE_BLOCKS: i32 = PLANET_SIZE_CHUNKS * 32; // 524288 blocks
pub const PLANET_HEIGHT_BLOCKS: i32 = PLANET_HEIGHT_CHUNKS * 32; // 256 blocks

// World generation constants
pub const SEA_LEVEL: f32 = 64.0; // Sea level height
pub const BEDROCK_LAYERS: i32 = 3; // Bottom 3 layers are unbreakable
pub const MAX_ALTITUDE: f32 = 256.0; // Maximum world height

#[derive(Resource, Clone)]
pub struct PlanetConfig {
    pub size_chunks: i32,
    pub height_chunks: i32,
    pub seed: u64, // Changed to u64 for better seed range
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
