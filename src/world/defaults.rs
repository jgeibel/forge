pub mod core {
    pub const SEED: u64 = 0;
    pub const PLANET_SIZE: u32 = 16384;
    pub const SEA_LEVEL: f32 = 64.0;
}
pub mod ocean {
    pub const OCEAN_DEPTH: f32 = 26_f32;
    pub const DEEP_OCEAN_DEPTH: f32 = 42_f32;
}
pub mod continent {
    pub const CONTINENT_THRESHOLD: f32 = 0.14000002_f32;
    pub const CONTINENT_POWER: f32 = 1.0;
    pub const CONTINENT_BIAS: f32 = 0.34;
    pub const CONTINENT_COUNT: u32 = 12_u32;
    pub const CONTINENT_RADIUS: f32 = 0.24;
    pub const CONTINENT_EDGE_POWER: f32 = 1.2;
    pub const CONTINENT_FREQUENCY: f64 = 2.0000000208616258_f64;
    pub const CONTINENT_BELT_WIDTH: f32 = 0.22;
    pub const CONTINENT_REPULSION_STRENGTH: f32 = 0.08;
    pub const CONTINENT_DRIFT_GAIN: f32 = 0.18;
    pub const CONTINENT_DRIFT_BELT_GAIN: f32 = 0.55;
}
pub mod terrain {
    pub const DETAIL_FREQUENCY: f64 = 7.0;
    pub const DETAIL_AMPLITUDE: f32 = 8.0;
    pub const MICRO_DETAIL_SCALE: f32 = 12.0;
    pub const MICRO_DETAIL_AMPLITUDE: f32 = 10.0;
    pub const MICRO_DETAIL_ROUGHNESS: f32 = 0.7;
    pub const MICRO_DETAIL_LAND_BLEND: f32 = 0.4;
    pub const HIGHLAND_BONUS: f32 = 20_f32;
}
pub mod mountain {
    pub const MOUNTAIN_FREQUENCY: f64 = 2.6;
    pub const MOUNTAIN_HEIGHT: f32 = 260_f32;
    pub const MOUNTAIN_THRESHOLD: f32 = 0.48;
    pub const MOUNTAIN_RANGE_COUNT: u32 = 18_u32;
    pub const MOUNTAIN_RANGE_WIDTH: f32 = 420.0;
    pub const MOUNTAIN_RANGE_STRENGTH: f32 = 2.6;
    pub const MOUNTAIN_RANGE_SPUR_CHANCE: f32 = 0.55_f32;
    pub const MOUNTAIN_RANGE_SPUR_STRENGTH: f32 = 1.8;
    pub const MOUNTAIN_RANGE_ROUGHNESS: f32 = 1.9_f32;
    pub const MOUNTAIN_EROSION_ITERATIONS: u32 = 4;
    pub const MOUNTAIN_CONVERGENCE_BOOST: f32 = 0.75;
    pub const MOUNTAIN_DIVERGENCE_PENALTY: f32 = 0.45;
    pub const MOUNTAIN_SHEAR_BOOST: f32 = 0.14;
    pub const MOUNTAIN_ARC_THRESHOLD: f32 = 0.23;
    pub const MOUNTAIN_ARC_STRENGTH: f32 = 0.45;
    pub const MOUNTAIN_ARC_WIDTH_FACTOR: f32 = 0.5;
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
    pub const HYDROLOGY_RESOLUTION: u32 = 1280;
    pub const HYDROLOGY_RAINFALL: f32 = 1.1_f32;
    pub const HYDROLOGY_RAINFALL_VARIANCE: f32 = 0.6_f32;
    pub const HYDROLOGY_RAINFALL_FREQUENCY: f64 = 0.74_f64;
    pub const HYDROLOGY_RAINFALL_CONTRAST: f32 = 1.6_f32;
    pub const HYDROLOGY_RAINFALL_DRY_FACTOR: f32 = 0.08_f32;
    pub const HYDROLOGY_RIVER_DENSITY: f32 = 0.12_f32;
    pub const HYDROLOGY_RIVER_WIDTH_SCALE: f32 = 1.0_f32;
    pub const HYDROLOGY_RIVER_DEPTH_SCALE: f32 = 12.0_f32;
    pub const HYDROLOGY_MEANDER_STRENGTH: f32 = 0.6_f32;
    pub const HYDROLOGY_POND_DENSITY: f32 = 0.35_f32;
    pub const HYDROLOGY_POND_MIN_RADIUS: f32 = 6.0_f32;
    pub const HYDROLOGY_POND_MAX_RADIUS: f32 = 18.0_f32;
    pub const HYDROLOGY_ESTUARY_LENGTH: f32 = 420.0_f32;
    pub const HYDROLOGY_FLOODPLAIN_RADIUS: f32 = 32.0_f32;
    pub const HYDROLOGY_COASTAL_BLEND: f32 = 0.7_f32;
    pub const HYDROLOGY_MAJOR_RIVER_COUNT: u32 = 24;
    pub const HYDROLOGY_MAJOR_RIVER_MIN_FLOW: f32 = 0.015_f32;
    pub const HYDROLOGY_MAJOR_RIVER_DEPTH_BOOST: f32 = 1.8_f32;
}
pub use climate::*;
pub use continent::*;
pub use core::*;
pub use hydrology::*;
pub use island::*;
pub use mountain::*;
pub use ocean::*;
pub use terrain::*;
