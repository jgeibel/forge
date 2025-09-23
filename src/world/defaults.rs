pub mod core {
    pub const SEED: u64 = 0;
    pub const PLANET_SIZE: u32 = 16384;
    pub const SEA_LEVEL: f32 = 64.0;
}

pub mod ocean {
    pub const OCEAN_DEPTH: f32 = 24.0;
    pub const DEEP_OCEAN_DEPTH: f32 = 40.0;
}

pub mod continent {
    pub const CONTINENT_THRESHOLD: f32 = 0.14000002_f32;
    pub const CONTINENT_POWER: f32 = 1.0;
    pub const CONTINENT_BIAS: f32 = 0.34;
    pub const CONTINENT_COUNT: u32 = 12_u32;
    pub const CONTINENT_RADIUS: f32 = 0.24;
    pub const CONTINENT_EDGE_POWER: f32 = 1.2;
    pub const CONTINENT_FREQUENCY: f64 = 2.0000000208616258_f64;
}

pub mod terrain {
    pub const DETAIL_FREQUENCY: f64 = 7.0;
    pub const DETAIL_AMPLITUDE: f32 = 8.0;
    pub const HIGHLAND_BONUS: f32 = 20_f32;
}

pub mod mountain {
    pub const MOUNTAIN_FREQUENCY: f64 = 2.5;
    pub const MOUNTAIN_HEIGHT: f32 = 160_f32;
    pub const MOUNTAIN_THRESHOLD: f32 = 0.48;
    pub const MOUNTAIN_RANGE_COUNT: u32 = 14_u32;
    pub const MOUNTAIN_RANGE_WIDTH: f32 = 300.0;
    pub const MOUNTAIN_RANGE_STRENGTH: f32 = 2.2;
    pub const MOUNTAIN_RANGE_SPUR_CHANCE: f32 = 0.45_f32;
    pub const MOUNTAIN_RANGE_SPUR_STRENGTH: f32 = 1.5;
    pub const MOUNTAIN_RANGE_ROUGHNESS: f32 = 1.6999996_f32;
}

pub mod climate {
    pub const MOISTURE_FREQUENCY: f64 = 2.6;
    pub const EQUATOR_TEMP_C: f32 = 30.0;
    pub const POLE_TEMP_C: f32 = -25.0;
    pub const LAPSE_RATE_C_PER_BLOCK: f32 = 0.008;
    pub const TEMPERATURE_VARIATION: f32 = 3.0;
}

pub mod island {
    pub const ISLAND_FREQUENCY: f64 = 7.5999999940395355_f64;
    pub const ISLAND_THRESHOLD: f32 = 0.08_f32;
    pub const ISLAND_HEIGHT: f32 = 50_f32;
    pub const ISLAND_FALLOFF: f32 = 1.8000007_f32;
}

pub mod hydrology {
    pub const HYDROLOGY_RESOLUTION: u32 = 1536;
    pub const HYDROLOGY_RAINFALL: f32 = 1.4;
    pub const HYDROLOGY_RAINFALL_VARIANCE: f32 = 0.4;
    pub const HYDROLOGY_RAINFALL_FREQUENCY: f64 = 0.8;
    pub const HYDROLOGY_MAJOR_RIVER_COUNT: u32 = 12_u32;
    pub const HYDROLOGY_MAJOR_RIVER_BOOST: f32 = 7.5_f32;
}

pub mod river {
    pub const RIVER_FLOW_THRESHOLD: f32 = 120.0;
    pub const RIVER_DEPTH_SCALE: f32 = 0.06;
    pub const RIVER_MAX_DEPTH: f32 = 22.0;
    pub const RIVER_SURFACE_RATIO: f32 = 0.65;
}

pub mod lake {
    pub const LAKE_FLOW_THRESHOLD: f32 = 140.0;
    pub const LAKE_DEPTH: f32 = 6.0;
    pub const LAKE_SHORE_BLEND: f32 = 3.0;
}

pub use climate::*;
pub use continent::*;
pub use core::*;
pub use hydrology::*;
pub use island::*;
pub use lake::*;
pub use mountain::*;
pub use ocean::*;
pub use river::*;
pub use terrain::*;
