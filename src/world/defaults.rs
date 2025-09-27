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
    pub const MOUNTAIN_FREQUENCY: f64 = 2.5;
    pub const MOUNTAIN_HEIGHT: f32 = 160_f32;
    pub const MOUNTAIN_THRESHOLD: f32 = 0.48;
    pub const MOUNTAIN_RANGE_COUNT: u32 = 14_u32;
    pub const MOUNTAIN_RANGE_WIDTH: f32 = 300.0;
    pub const MOUNTAIN_RANGE_STRENGTH: f32 = 2.2;
    pub const MOUNTAIN_RANGE_SPUR_CHANCE: f32 = 0.45_f32;
    pub const MOUNTAIN_RANGE_SPUR_STRENGTH: f32 = 1.5;
    pub const MOUNTAIN_RANGE_ROUGHNESS: f32 = 1.6999996_f32;
    pub const MOUNTAIN_EROSION_ITERATIONS: u32 = 3;
    pub const MOUNTAIN_CONVERGENCE_BOOST: f32 = 0.65;
    pub const MOUNTAIN_DIVERGENCE_PENALTY: f32 = 0.4;
    pub const MOUNTAIN_SHEAR_BOOST: f32 = 0.12;
    pub const MOUNTAIN_ARC_THRESHOLD: f32 = 0.25;
    pub const MOUNTAIN_ARC_STRENGTH: f32 = 0.4;
    pub const MOUNTAIN_ARC_WIDTH_FACTOR: f32 = 0.45;
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
    pub const HYDROLOGY_ITERATIONS: u32 = 72_u32;
    pub const HYDROLOGY_TIME_STEP: f32 = 0.32_f32;
    pub const HYDROLOGY_INFILTRATION_RATE: f32 = 0.52_f32;
    pub const HYDROLOGY_BASEFLOW: f32 = 0.006_f32;
    pub const HYDROLOGY_EROSION_RATE: f32 = 0.15_f32;
    pub const HYDROLOGY_DEPOSITION_RATE: f32 = 0.32_f32;
    pub const HYDROLOGY_SEDIMENT_CAPACITY: f32 = 0.5_f32;
    pub const HYDROLOGY_BANKFULL_DEPTH: f32 = 14.0_f32;
    pub const HYDROLOGY_FLOODPLAIN_SOFTENING: f32 = 6.0_f32;
    pub const HYDROLOGY_MINIMUM_SLOPE: f32 = 0.0005_f32;
    pub const HYDROLOGY_SHORELINE_RADIUS: f32 = 96.0_f32;
    pub const HYDROLOGY_SHORELINE_MAX_HEIGHT: f32 = 18.0_f32;
    pub const HYDROLOGY_SHORELINE_SMOOTHING: u32 = 2_u32;
}
pub use climate::*;
pub use continent::*;
pub use core::*;
pub use hydrology::*;
pub use island::*;
pub use mountain::*;
pub use ocean::*;
pub use terrain::*;
