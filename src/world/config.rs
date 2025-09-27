use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::planet::PlanetConfig;

use super::defaults;

/// Resource tracking the current air temperature at player position.
#[derive(Resource, Default)]
pub struct CurrentTemperature {
    pub fahrenheit: f32,
    pub celsius: f32,
    pub(crate) last_chunk_x: i32,
    pub(crate) last_chunk_z: i32,
}

impl CurrentTemperature {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            fahrenheit: 70.0,
            celsius: 21.0,
            last_chunk_x: i32::MAX,
            last_chunk_z: i32::MAX,
        }
    }

    pub fn update(&mut self, fahrenheit: f32) {
        self.fahrenheit = fahrenheit;
        self.celsius = (fahrenheit - 32.0) * 5.0 / 9.0;
    }
}

#[derive(Resource, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WorldGenConfig {
    pub seed: u64,
    pub planet_size: u32,
    pub sea_level: f32,
    pub ocean_depth: f32,
    pub deep_ocean_depth: f32,
    pub continent_threshold: f32,
    pub continent_power: f32,
    pub continent_bias: f32,
    pub continent_count: u32,
    pub continent_radius: f32,
    pub continent_edge_power: f32,
    pub continent_frequency: f64,
    pub continent_belt_width: f32,
    pub continent_repulsion_strength: f32,
    pub continent_drift_gain: f32,
    pub continent_drift_belt_gain: f32,
    pub detail_frequency: f64,
    pub detail_amplitude: f32,
    pub micro_detail_scale: f32,
    pub micro_detail_amplitude: f32,
    pub micro_detail_roughness: f32,
    pub micro_detail_land_blend: f32,
    pub mountain_frequency: f64,
    pub mountain_height: f32,
    pub mountain_threshold: f32,
    pub mountain_range_count: u32,
    pub mountain_range_width: f32,
    pub mountain_range_strength: f32,
    pub mountain_range_spur_chance: f32,
    pub mountain_range_spur_strength: f32,
    pub mountain_range_roughness: f32,
    pub mountain_erosion_iterations: u32,
    pub mountain_convergence_boost: f32,
    pub mountain_divergence_penalty: f32,
    pub mountain_shear_boost: f32,
    pub mountain_arc_threshold: f32,
    pub mountain_arc_strength: f32,
    pub mountain_arc_width_factor: f32,
    pub moisture_frequency: f64,
    pub equator_temp_c: f32,
    pub pole_temp_c: f32,
    pub lapse_rate_c_per_block: f32,
    pub temperature_variation: f32,
    pub highland_bonus: f32,
    pub island_frequency: f64,
    pub island_threshold: f32,
    pub island_height: f32,
    pub island_falloff: f32,
    pub hydrology_resolution: u32,
    pub hydrology_rainfall: f32,
    pub hydrology_rainfall_variance: f32,
    pub hydrology_rainfall_frequency: f64,
    pub hydrology_rainfall_contrast: f32,
    pub hydrology_rainfall_dry_factor: f32,
    pub hydrology_iterations: u32,
    pub hydrology_time_step: f32,
    pub hydrology_infiltration_rate: f32,
    pub hydrology_baseflow: f32,
    pub hydrology_erosion_rate: f32,
    pub hydrology_deposition_rate: f32,
    pub hydrology_sediment_capacity: f32,
    pub hydrology_bankfull_depth: f32,
    pub hydrology_floodplain_softening: f32,
    pub hydrology_minimum_slope: f32,
    pub hydrology_shoreline_radius: f32,
    pub hydrology_shoreline_max_height: f32,
    pub hydrology_shoreline_smoothing: u32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        use defaults::*;

        Self {
            seed: SEED,
            planet_size: PLANET_SIZE,
            sea_level: SEA_LEVEL,
            ocean_depth: OCEAN_DEPTH,
            deep_ocean_depth: DEEP_OCEAN_DEPTH,
            continent_threshold: CONTINENT_THRESHOLD,
            continent_power: CONTINENT_POWER,
            continent_bias: CONTINENT_BIAS,
            continent_count: CONTINENT_COUNT,
            continent_radius: CONTINENT_RADIUS,
            continent_edge_power: CONTINENT_EDGE_POWER,
            continent_frequency: CONTINENT_FREQUENCY,
            continent_belt_width: CONTINENT_BELT_WIDTH,
            continent_repulsion_strength: CONTINENT_REPULSION_STRENGTH,
            continent_drift_gain: CONTINENT_DRIFT_GAIN,
            continent_drift_belt_gain: CONTINENT_DRIFT_BELT_GAIN,
            detail_frequency: DETAIL_FREQUENCY,
            detail_amplitude: DETAIL_AMPLITUDE,
            micro_detail_scale: MICRO_DETAIL_SCALE,
            micro_detail_amplitude: MICRO_DETAIL_AMPLITUDE,
            micro_detail_roughness: MICRO_DETAIL_ROUGHNESS,
            micro_detail_land_blend: MICRO_DETAIL_LAND_BLEND,
            mountain_frequency: MOUNTAIN_FREQUENCY,
            mountain_height: MOUNTAIN_HEIGHT,
            mountain_threshold: MOUNTAIN_THRESHOLD,
            mountain_range_count: MOUNTAIN_RANGE_COUNT,
            mountain_range_width: MOUNTAIN_RANGE_WIDTH,
            mountain_range_strength: MOUNTAIN_RANGE_STRENGTH,
            mountain_range_spur_chance: MOUNTAIN_RANGE_SPUR_CHANCE,
            mountain_range_spur_strength: MOUNTAIN_RANGE_SPUR_STRENGTH,
            mountain_range_roughness: MOUNTAIN_RANGE_ROUGHNESS,
            mountain_erosion_iterations: MOUNTAIN_EROSION_ITERATIONS,
            mountain_convergence_boost: MOUNTAIN_CONVERGENCE_BOOST,
            mountain_divergence_penalty: MOUNTAIN_DIVERGENCE_PENALTY,
            mountain_shear_boost: MOUNTAIN_SHEAR_BOOST,
            mountain_arc_threshold: MOUNTAIN_ARC_THRESHOLD,
            mountain_arc_strength: MOUNTAIN_ARC_STRENGTH,
            mountain_arc_width_factor: MOUNTAIN_ARC_WIDTH_FACTOR,
            moisture_frequency: MOISTURE_FREQUENCY,
            equator_temp_c: EQUATOR_TEMP_C,
            pole_temp_c: POLE_TEMP_C,
            lapse_rate_c_per_block: LAPSE_RATE_C_PER_BLOCK,
            temperature_variation: TEMPERATURE_VARIATION,
            highland_bonus: HIGHLAND_BONUS,
            island_frequency: ISLAND_FREQUENCY,
            island_threshold: ISLAND_THRESHOLD,
            island_height: ISLAND_HEIGHT,
            island_falloff: ISLAND_FALLOFF,
            hydrology_resolution: HYDROLOGY_RESOLUTION,
            hydrology_rainfall: HYDROLOGY_RAINFALL,
            hydrology_rainfall_variance: HYDROLOGY_RAINFALL_VARIANCE,
            hydrology_rainfall_frequency: HYDROLOGY_RAINFALL_FREQUENCY,
            hydrology_rainfall_contrast: HYDROLOGY_RAINFALL_CONTRAST,
            hydrology_rainfall_dry_factor: HYDROLOGY_RAINFALL_DRY_FACTOR,
            hydrology_iterations: HYDROLOGY_ITERATIONS,
            hydrology_time_step: HYDROLOGY_TIME_STEP,
            hydrology_infiltration_rate: HYDROLOGY_INFILTRATION_RATE,
            hydrology_baseflow: HYDROLOGY_BASEFLOW,
            hydrology_erosion_rate: HYDROLOGY_EROSION_RATE,
            hydrology_deposition_rate: HYDROLOGY_DEPOSITION_RATE,
            hydrology_sediment_capacity: HYDROLOGY_SEDIMENT_CAPACITY,
            hydrology_bankfull_depth: HYDROLOGY_BANKFULL_DEPTH,
            hydrology_floodplain_softening: HYDROLOGY_FLOODPLAIN_SOFTENING,
            hydrology_minimum_slope: HYDROLOGY_MINIMUM_SLOPE,
            hydrology_shoreline_radius: HYDROLOGY_SHORELINE_RADIUS,
            hydrology_shoreline_max_height: HYDROLOGY_SHORELINE_MAX_HEIGHT,
            hydrology_shoreline_smoothing: HYDROLOGY_SHORELINE_SMOOTHING,
        }
    }
}

impl WorldGenConfig {
    pub fn from_planet_config(config: &PlanetConfig) -> Self {
        let planet_size = config.size_chunks as u32 * 32;

        // Standard world size for frequency calculations (16384 blocks = 512 chunks)
        const STANDARD_WORLD_SIZE: f32 = 16384.0;

        // Frequency scaling factor - inverse relationship for scale-invariant features
        // Larger worlds need higher frequencies to maintain same physical feature size
        let frequency_scale = planet_size.max(1) as f64 / STANDARD_WORLD_SIZE as f64;

        // SCALE-DEPENDENT: Features that scale with world size
        // Continent count scales logarithmically - more continents on larger worlds but not linearly
        let continent_count = ((planet_size as f32).log2() * 0.4)
            .max(3.0)
            .min(20.0)
            .round() as u32;
        // Continent radius scales proportionally to maintain map appearance
        let continent_radius = 0.23 * (planet_size as f32 / STANDARD_WORLD_SIZE).sqrt();
        // Major river count scales with world area
        Self {
            seed: config.seed,
            planet_size,
            sea_level: config.sea_level,

            // SCALE-INVARIANT: Physical ocean dimensions always constant in blocks
            ocean_depth: 50.0,       // Continental shelf depth ~50 blocks (50m)
            deep_ocean_depth: 200.0, // Deep ocean ~200 blocks (200m) - scaled for gameplay

            // SCALE-DEPENDENT: Continental distribution
            continent_threshold: 0.18,
            continent_power: 0.95,
            continent_bias: 0.34,
            continent_count,  // Logarithmic scaling
            continent_radius, // Proportional scaling
            continent_edge_power: 1.2,
            continent_frequency: 0.45 * frequency_scale, // More continents, same physical size
            continent_belt_width: defaults::continent::CONTINENT_BELT_WIDTH,
            continent_repulsion_strength: defaults::continent::CONTINENT_REPULSION_STRENGTH,
            continent_drift_gain: defaults::continent::CONTINENT_DRIFT_GAIN,
            continent_drift_belt_gain: defaults::continent::CONTINENT_DRIFT_BELT_GAIN,

            // SCALE-INVARIANT: Terrain detail (hills, valleys)
            // Target: hills should be ~50-200 blocks wide regardless of world size
            // Since we use UV space (0-1), we need to scale frequency by world size
            // to maintain constant physical feature size
            detail_frequency: (planet_size as f64 / 100.0), // Hills ~100 blocks wide
            detail_amplitude: 12.0,                         // Hills always 12 blocks tall
            micro_detail_scale: defaults::MICRO_DETAIL_SCALE, // Adds sub-20 block terrain break-up
            micro_detail_amplitude: defaults::MICRO_DETAIL_AMPLITUDE,
            micro_detail_roughness: defaults::MICRO_DETAIL_ROUGHNESS,
            micro_detail_land_blend: defaults::MICRO_DETAIL_LAND_BLEND,

            // SCALE-INVARIANT: Mountain dimensions
            // Target: mountains should be ~200-800 blocks wide regardless of world size
            mountain_frequency: (planet_size as f64 / 400.0), // Mountains ~400 blocks wide
            mountain_height: 250.0, // Mountains rise 250 blocks (250m) - more realistic
            mountain_threshold: 0.42,
            mountain_range_count: {
                let scale = (planet_size as f32 / STANDARD_WORLD_SIZE).max(0.25);
                let base = 12.0 * scale.powf(0.75);
                base.round().clamp(4.0, 40.0) as u32
            },
            mountain_range_width: 300.0,
            mountain_range_strength: 2.2,
            mountain_range_spur_chance: 0.6,
            mountain_range_spur_strength: 1.5,
            mountain_range_roughness: 1.25,
            mountain_erosion_iterations: defaults::mountain::MOUNTAIN_EROSION_ITERATIONS,
            mountain_convergence_boost: defaults::mountain::MOUNTAIN_CONVERGENCE_BOOST,
            mountain_divergence_penalty: defaults::mountain::MOUNTAIN_DIVERGENCE_PENALTY,
            mountain_shear_boost: defaults::mountain::MOUNTAIN_SHEAR_BOOST,
            mountain_arc_threshold: defaults::mountain::MOUNTAIN_ARC_THRESHOLD,
            mountain_arc_strength: defaults::mountain::MOUNTAIN_ARC_STRENGTH,
            mountain_arc_width_factor: defaults::mountain::MOUNTAIN_ARC_WIDTH_FACTOR,

            // SCALE-INVARIANT: Biome transitions
            moisture_frequency: (planet_size as f64 / 300.0), // Biome patches ~300 blocks wide

            // Climate (scale-independent)
            equator_temp_c: 28.0,
            pole_temp_c: -30.0,
            lapse_rate_c_per_block: 0.008,
            temperature_variation: 3.0,

            // SCALE-INVARIANT: Highland/plateau heights
            highland_bonus: 20.0, // Highlands always 20 blocks above base

            // SCALE-INVARIANT: Island dimensions
            island_frequency: (planet_size as f64 / 100.0), // Islands ~100 blocks wide
            island_threshold: 0.55,
            island_height: 30.0, // Islands always rise 30 blocks max
            island_falloff: 2.8,

            // Hydrology
            hydrology_resolution: ((planet_size as f32 / 16.0).sqrt() as u32)
                .max(256)
                .min(4096),
            hydrology_rainfall: 1.1,
            hydrology_rainfall_variance: 0.6,
            hydrology_rainfall_frequency: (planet_size as f64 / 220.0), // Rain patterns ~220 blocks wide
            hydrology_rainfall_contrast: defaults::HYDROLOGY_RAINFALL_CONTRAST,
            hydrology_rainfall_dry_factor: defaults::HYDROLOGY_RAINFALL_DRY_FACTOR,
            hydrology_iterations: (72.0
                * (planet_size as f32 / STANDARD_WORLD_SIZE).clamp(0.5, 2.0))
            .round()
            .clamp(24.0, 144.0) as u32,
            hydrology_time_step: 0.32,
            hydrology_infiltration_rate: 0.52,
            hydrology_baseflow: 0.006,
            hydrology_erosion_rate: 0.15,
            hydrology_deposition_rate: 0.32,
            hydrology_sediment_capacity: 0.5,
            hydrology_bankfull_depth: 15.0,
            hydrology_floodplain_softening: 6.0,
            hydrology_minimum_slope: 0.0005,
            hydrology_shoreline_radius: 96.0,
            hydrology_shoreline_max_height: 18.0,
            hydrology_shoreline_smoothing: 2,
        }
    }
}
