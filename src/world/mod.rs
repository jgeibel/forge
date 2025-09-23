use bevy::prelude::*;
use image::{ImageBuffer, Rgba};
use noise::{NoiseFn, Perlin};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::PlanetConfig;

pub mod defaults;
pub mod metadata;

/// Resource tracking the current air temperature at player position.
#[derive(Resource, Default)]
pub struct CurrentTemperature {
    pub fahrenheit: f32,
    pub celsius: f32,
    last_chunk_x: i32,
    last_chunk_z: i32,
}

impl CurrentTemperature {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Biome {
    DeepOcean,
    Ocean,
    FrozenOcean,
    Beach,
    Desert,
    Savanna,
    TropicalRainforest,
    TemperateGrassland,
    TemperateForest,
    BorealForest,
    Tundra,
    Snow,
    Mountain,
    SnowyMountain,
    IceCap,
}

impl Biome {
    pub fn surface_block(&self) -> BlockType {
        match self {
            Biome::DeepOcean | Biome::Ocean => BlockType::Sand,
            Biome::FrozenOcean | Biome::IceCap => BlockType::Ice,
            Biome::Beach | Biome::Desert => BlockType::Sand,
            Biome::Savanna
            | Biome::TropicalRainforest
            | Biome::TemperateGrassland
            | Biome::TemperateForest
            | Biome::BorealForest => BlockType::Grass,
            Biome::Tundra | Biome::Snow | Biome::SnowyMountain => BlockType::Snow,
            Biome::Mountain => BlockType::Stone,
        }
    }

    pub fn subsurface_block(&self) -> BlockType {
        match self {
            Biome::DeepOcean | Biome::Ocean | Biome::Beach => BlockType::Sand,
            Biome::FrozenOcean | Biome::IceCap => BlockType::PackedIce,
            Biome::Desert => BlockType::Sand,
            Biome::Savanna
            | Biome::TropicalRainforest
            | Biome::TemperateGrassland
            | Biome::TemperateForest
            | Biome::BorealForest => BlockType::Dirt,
            Biome::Tundra | Biome::Snow | Biome::SnowyMountain => BlockType::PackedIce,
            Biome::Mountain => BlockType::Stone,
        }
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
    pub detail_frequency: f64,
    pub detail_amplitude: f32,
    pub mountain_frequency: f64,
    pub mountain_height: f32,
    pub mountain_threshold: f32,
    pub mountain_range_count: u32,
    pub mountain_range_width: f32,
    pub mountain_range_strength: f32,
    pub mountain_range_spur_chance: f32,
    pub mountain_range_spur_strength: f32,
    pub mountain_range_roughness: f32,
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
    pub hydrology_major_river_count: u32,
    pub hydrology_major_river_boost: f32,
    pub river_flow_threshold: f32,
    pub river_depth_scale: f32,
    pub river_max_depth: f32,
    pub river_surface_ratio: f32,
    pub lake_flow_threshold: f32,
    pub lake_depth: f32,
    pub lake_shore_blend: f32,
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
            detail_frequency: DETAIL_FREQUENCY,
            detail_amplitude: DETAIL_AMPLITUDE,
            mountain_frequency: MOUNTAIN_FREQUENCY,
            mountain_height: MOUNTAIN_HEIGHT,
            mountain_threshold: MOUNTAIN_THRESHOLD,
            mountain_range_count: MOUNTAIN_RANGE_COUNT,
            mountain_range_width: MOUNTAIN_RANGE_WIDTH,
            mountain_range_strength: MOUNTAIN_RANGE_STRENGTH,
            mountain_range_spur_chance: MOUNTAIN_RANGE_SPUR_CHANCE,
            mountain_range_spur_strength: MOUNTAIN_RANGE_SPUR_STRENGTH,
            mountain_range_roughness: MOUNTAIN_RANGE_ROUGHNESS,
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
            hydrology_major_river_count: HYDROLOGY_MAJOR_RIVER_COUNT,
            hydrology_major_river_boost: HYDROLOGY_MAJOR_RIVER_BOOST,
            river_flow_threshold: RIVER_FLOW_THRESHOLD,
            river_depth_scale: RIVER_DEPTH_SCALE,
            river_max_depth: RIVER_MAX_DEPTH,
            river_surface_ratio: RIVER_SURFACE_RATIO,
            lake_flow_threshold: LAKE_FLOW_THRESHOLD,
            lake_depth: LAKE_DEPTH,
            lake_shore_blend: LAKE_SHORE_BLEND,
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
        let major_river_count = ((planet_size as f32 / STANDARD_WORLD_SIZE).sqrt() * 10.0)
            .max(5.0)
            .min(100.0)
            .round() as u32;

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

            // SCALE-INVARIANT: Terrain detail (hills, valleys)
            // Target: hills should be ~50-200 blocks wide regardless of world size
            // Since we use UV space (0-1), we need to scale frequency by world size
            // to maintain constant physical feature size
            detail_frequency: (planet_size as f64 / 100.0), // Hills ~100 blocks wide
            detail_amplitude: 12.0,                         // Hills always 12 blocks tall

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
            hydrology_rainfall: 1.4,
            hydrology_rainfall_variance: 0.4,
            hydrology_rainfall_frequency: (planet_size as f64 / 200.0), // Rain patterns ~200 blocks wide
            hydrology_major_river_count: major_river_count,             // Scale-dependent
            hydrology_major_river_boost: 6.0,

            // SCALE-INVARIANT: River physical dimensions
            river_flow_threshold: 120.0,
            river_depth_scale: 0.06,
            river_max_depth: 18.0, // Rivers always max 18 blocks deep
            river_surface_ratio: 0.65,

            // SCALE-INVARIANT: Lake dimensions
            lake_flow_threshold: 140.0,
            lake_depth: 15.0, // Lakes average 15 blocks deep (15m) - more realistic
            lake_shore_blend: 5.0, // Shore transition 5 blocks for gradual slope
        }
    }
}

#[derive(Resource, Clone)]
pub struct WorldGenerator {
    config: WorldGenConfig,
    continent_noise: Perlin,
    detail_noise: Perlin,
    mountain_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
    island_noise: Perlin,
    hydrology_rain_noise: Perlin,
    continent_sites: Vec<ContinentSite>,
    mountain_ranges: MountainRangeMap,
    hydrology: HydrologyMap,
}

#[derive(Clone)]
struct ContinentSite {
    position: Vec2,
    ridge_angle: f32,
}

#[derive(Clone, Copy)]
struct TerrainComponents {
    base_height: f32,
}

#[derive(Clone, Copy, Default)]
struct HydrologySample {
    channel_depth: f32,
    water_level: f32,
    river_intensity: f32,
    lake_intensity: f32,
    rainfall: f32,
    major_river: f32,
}

#[derive(Clone)]
struct MountainRangeMap {
    width: usize,
    height: usize,
    data: Vec<f32>,
}

#[derive(Clone, Copy)]
struct RangeParams {
    spur_chance: f32,
    spur_strength: f32,
    roughness: f32,
}

impl MountainRangeMap {
    fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            data: vec![0.0],
        }
    }

    fn generate(config: &WorldGenConfig) -> Self {
        let planet_size = config.planet_size.max(1) as f32;
        let mut resolution = (planet_size / 32.0).round() as usize;
        if resolution < 128 {
            resolution = 128;
        }
        if resolution > 4096 {
            resolution = 4096;
        }

        let width = resolution;
        let height = resolution;
        let mut map = Self {
            width,
            height,
            data: vec![0.0; width * height],
        };

        let count = config.mountain_range_count as usize;
        if count == 0 || map.data.is_empty() {
            return map;
        }

        let mut rng = StdRng::seed_from_u64(config.seed.wrapping_add(17));
        let base_half_width =
            (config.mountain_range_width.max(8.0) / planet_size * 0.5).clamp(0.002, 0.25);
        let base_strength = config.mountain_range_strength.max(0.0);
        let range_params = RangeParams {
            spur_chance: config.mountain_range_spur_chance.clamp(0.0, 1.0),
            spur_strength: config.mountain_range_spur_strength.clamp(0.0, 2.0),
            roughness: config.mountain_range_roughness.clamp(0.0, 2.5),
        };
        let roughness_noise = Perlin::new(config.seed.wrapping_add(91) as u32);

        for _ in 0..count {
            let mut points = Vec::new();
            let mut current = Vec2::new(rng.gen::<f32>(), rng.gen::<f32>());
            let mut heading = rng.gen::<f32>() * TAU;

            let segments = rng.gen_range(6..12);
            let total_length = rng.gen_range(0.18..0.42);
            let step = total_length / segments as f32;
            points.push(current);

            for _ in 0..segments {
                let bend = (rng.gen::<f32>() - 0.5) * 0.4;
                heading = (heading + bend).rem_euclid(TAU);
                let lateral = (rng.gen::<f32>() - 0.5) * 0.35 * step;
                let forward = Vec2::new(heading.cos(), heading.sin());
                let normal = Vec2::new(-forward.y, forward.x);
                current += forward * step + normal * lateral;
                current.x = current.x.rem_euclid(1.0);
                current.y = current.y.rem_euclid(1.0);
                points.push(current);
            }

            let width_variation = rng.gen_range(0.75..1.35);
            let strength_variation = rng.gen_range(0.7..1.3);
            let half_width = (base_half_width * width_variation).clamp(0.002, 0.3);
            let strength = base_strength * strength_variation;
            map.paint_range(
                &points,
                half_width,
                strength,
                &roughness_noise,
                range_params,
                &mut rng,
                true,
            );
        }

        map.normalize();
        map
    }

    fn sample(&self, u: f32, v: f32) -> f32 {
        if self.data.is_empty() || self.width == 0 || self.height == 0 {
            return 0.0;
        }

        let x = u.rem_euclid(1.0) * self.width as f32;
        let y = v.rem_euclid(1.0) * self.height as f32;

        let x0 = x.floor() as isize;
        let y0 = y.floor() as isize;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        let v0 = v00 + (v10 - v00) * tx;
        let v1 = v01 + (v11 - v01) * tx;
        (v0 + (v1 - v0) * ty).clamp(0.0, 1.0)
    }

    fn paint_range(
        &mut self,
        points: &[Vec2],
        half_width: f32,
        strength: f32,
        roughness_noise: &Perlin,
        params: RangeParams,
        rng: &mut StdRng,
        allow_spurs: bool,
    ) {
        if points.len() < 2 || half_width <= 0.0 || strength <= 0.0 {
            return;
        }

        let spawn_chance = params.spur_chance.clamp(0.0, 1.0);

        for window in points.windows(2) {
            let start = window[0];
            let end = window[1];
            self.paint_segment(start, end, half_width, strength, roughness_noise, params);

            if allow_spurs && spawn_chance > f32::EPSILON && params.spur_strength > 0.0 {
                if rng.gen::<f32>() < spawn_chance {
                    if let Some(spur_points) =
                        self.generate_spur(start, end, half_width, params, rng)
                    {
                        let spur_half = (half_width * 0.6).clamp(0.001, half_width);
                        let spur_strength =
                            strength * params.spur_strength * rng.gen_range(0.6..1.35);
                        self.paint_range(
                            &spur_points,
                            spur_half,
                            spur_strength,
                            roughness_noise,
                            params,
                            rng,
                            false,
                        );
                    }
                }
            }
        }
    }

    fn paint_segment(
        &mut self,
        start: Vec2,
        end: Vec2,
        half_width: f32,
        strength: f32,
        roughness_noise: &Perlin,
        params: RangeParams,
    ) {
        let dx = torus_delta(start.x, end.x);
        let dy = torus_delta(start.y, end.y);
        let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
        let steps = (distance * self.width as f32 * 2.4).ceil() as usize;
        let tangent = Vec2::new(dx, dy).normalize_or_zero();
        let lateral = Vec2::new(-tangent.y, tangent.x);
        let rough_freq = 4.0 + params.roughness * 6.0;

        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let point = Vec2::new(
                (start.x + dx * t).rem_euclid(1.0),
                (start.y + dy * t).rem_euclid(1.0),
            );

            let noise_value = if params.roughness > 0.01 {
                torus_noise(roughness_noise, point.x, point.y, rough_freq, t)
            } else {
                0.0
            };

            let width_mod = (1.0 + noise_value * params.roughness * 0.5).clamp(0.35, 2.8);
            let strength_mod = (1.0 + noise_value * params.roughness * 0.4).clamp(0.3, 2.6);
            let local_half = (half_width * width_mod).clamp(0.0005, 0.35);
            let local_strength = strength * strength_mod;

            self.splat(point, local_half, local_strength);

            if params.roughness > 0.2 && tangent.length_squared() > 0.0 {
                let along_offset = (noise_value * 0.5 + 0.5) * local_half * 0.6;
                let side_offset = (noise_value * 0.5) * local_half * 0.5;

                let crest_point = wrap_vec2(point + tangent * along_offset);
                self.splat(crest_point, local_half * 0.55, local_strength * 0.55);

                let spur_point = wrap_vec2(point + lateral * side_offset);
                self.splat(spur_point, local_half * 0.45, local_strength * 0.4);
            }
        }
    }

    fn generate_spur(
        &self,
        start: Vec2,
        end: Vec2,
        half_width: f32,
        params: RangeParams,
        rng: &mut StdRng,
    ) -> Option<Vec<Vec2>> {
        let dx = torus_delta(start.x, end.x);
        let dy = torus_delta(start.y, end.y);
        let base = Vec2::new(dx, dy);
        let base_length = base.length();
        if base_length <= f32::EPSILON {
            return None;
        }

        let dir = base / base_length;
        let normal = Vec2::new(-dir.y, dir.x);
        if normal.length_squared() <= f32::EPSILON {
            return None;
        }

        let anchor_t = rng.gen_range(0.15..0.85);
        let anchor = Vec2::new(
            (start.x + dx * anchor_t).rem_euclid(1.0),
            (start.y + dy * anchor_t).rem_euclid(1.0),
        );

        let mut heading = normal * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        heading = heading.normalize_or_zero();
        if heading.length_squared() <= f32::EPSILON {
            return None;
        }

        let spur_segments = rng.gen_range(3..6);
        let rough_factor = params.roughness.max(0.2);
        let base_length = (half_width * rng.gen_range(1.8..3.6)).max(0.005);
        let step = (base_length / spur_segments as f32).max(0.002);

        let mut points = Vec::with_capacity(spur_segments + 1);
        points.push(anchor);
        let mut current = anchor;

        for _ in 0..spur_segments {
            let bend = (rng.gen::<f32>() - 0.5) * 0.6 * rough_factor;
            heading = rotate_vec2(heading, bend);
            let mix = rng.gen_range(-0.35..0.35);
            heading = (heading + dir * mix).normalize_or_zero();
            if heading.length_squared() <= f32::EPSILON {
                break;
            }

            current = wrap_vec2(current + heading * step);
            points.push(current);
        }

        if points.len() > 2 {
            Some(points)
        } else {
            None
        }
    }

    fn splat(&mut self, center: Vec2, half_width: f32, strength: f32) {
        let radius = (half_width * self.width as f32 * 3.0).ceil() as i32;
        if radius <= 0 {
            return;
        }

        let cx = (center.x * self.width as f32).floor() as i32;
        let cy = (center.y * self.height as f32).floor() as i32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = wrap_index(cx + dx, self.width as i32);
                let y = wrap_index(cy + dy, self.height as i32);

                let sample_u = (x as f32 + 0.5) / self.width as f32;
                let sample_v = (y as f32 + 0.5) / self.height as f32;
                let du = torus_distance(center.x, sample_u);
                let dv = torus_distance(center.y, sample_v);
                let dist = (du * du + dv * dv).sqrt();
                if dist > half_width * 3.0 {
                    continue;
                }

                let norm = (dist / half_width).min(3.0);
                let falloff = (-norm * norm * 0.7).exp();
                let idx = y as usize * self.width + x as usize;
                self.data[idx] += falloff * strength;
            }
        }
    }

    fn normalize(&mut self) {
        let mut max_value = 0.0_f32;
        for value in &self.data {
            if *value > max_value {
                max_value = *value;
            }
        }

        if max_value <= f32::EPSILON {
            self.data.fill(0.0);
            return;
        }

        for value in &mut self.data {
            *value = (*value / max_value).clamp(0.0, 1.0);
        }
    }

    fn get(&self, x: isize, y: isize) -> f32 {
        let xi = wrap_index_isize(x, self.width as isize) as usize;
        let yi = wrap_index_isize(y, self.height as isize) as usize;
        self.data[yi * self.width + xi]
    }
}

#[derive(Clone)]
struct HydrologyMap {
    width: usize,
    height: usize,
    planet_size: f32,
    sea_level: f32,
    river_max_depth: f32,
    lake_depth: f32,
    river_depth: Vec<f32>,
    water_level: Vec<f32>,
    river_mask: Vec<f32>,
    lake_mask: Vec<f32>,
    rainfall: Vec<f32>,
    major_path: Vec<f32>,
    major_strength: Vec<f32>,
}

impl HydrologyMap {
    fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            planet_size: 1.0,
            sea_level: 0.0,
            river_max_depth: 0.0,
            lake_depth: 0.0,
            river_depth: vec![0.0_f32],
            water_level: vec![0.0_f32],
            river_mask: vec![0.0_f32],
            lake_mask: vec![0.0_f32],
            rainfall: vec![0.0_f32],
            major_path: vec![0.0_f32],
            major_strength: vec![0.0_f32],
        }
    }

    fn generate(generator: &WorldGenerator) -> Self {
        let config = &generator.config;
        let width = config.hydrology_resolution.max(32) as usize;
        let height = width;
        let planet_size = config.planet_size as f32;
        let sea_level = config.sea_level;
        let count = width * height;

        let mut heights = vec![0.0_f32; count];
        let mut rainfall_map = vec![0.0_f32; count];

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let world_x = u * planet_size;
                let world_z = v * planet_size;
                let components = generator.terrain_components(world_x, world_z);
                let idx = y * width + x;
                heights[idx] = components.base_height;
                rainfall_map[idx] = generator.raw_rainfall(world_x, world_z);
            }
        }

        let mut downstream: Vec<Option<usize>> = vec![None; count];
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let current_height = heights[idx];
                if current_height <= sea_level {
                    continue;
                }

                let mut lowest_height = current_height;
                let mut target = None;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let neighbor =
                            Self::wrap_index(width, height, x as isize + dx, y as isize + dy);
                        let neighbor_height = heights[neighbor];
                        if neighbor_height < lowest_height {
                            lowest_height = neighbor_height;
                            target = Some(neighbor);
                        }
                    }
                }

                downstream[idx] = target;
            }
        }

        let mut upstream: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (idx, &target) in downstream.iter().enumerate() {
            if let Some(target) = target {
                upstream[target].push(idx);
            }
        }

        let mut order: Vec<usize> = (0..count).collect();
        order.sort_unstable_by(|a, b| {
            heights[*b]
                .partial_cmp(&heights[*a])
                .unwrap_or(Ordering::Equal)
        });

        let river_threshold = config.river_flow_threshold.max(0.0);
        let depth_scale = config.river_depth_scale.max(0.0);
        let max_depth = config.river_max_depth.max(0.01);
        let surface_ratio = config.river_surface_ratio.clamp(0.1, 1.0);
        let lake_threshold = config.lake_flow_threshold.max(0.0);
        let lake_depth = config.lake_depth.max(0.0);

        let mut flow_accum = vec![0.0_f32; count];
        let mut river_depth = vec![0.0_f32; count];
        let mut water_level = vec![0.0_f32; count];
        let mut river_mask = vec![0.0_f32; count];
        let mut lake_mask = vec![0.0_f32; count];
        let mut major_path_mask = vec![0.0_f32; count];
        let mut major_strength = vec![0.0_f32; count];

        if config.hydrology_major_river_count > 0 && config.hydrology_major_river_boost > 0.0 {
            // Try to generate 3x more rivers than requested, keep the best ones that reach ocean
            let attempts = (config.hydrology_major_river_count * 3).min(count as u32) as usize;
            let desired = config.hydrology_major_river_count.min(count as u32) as usize;
            if desired > 0 {
                // Find the highest points on land as river sources
                // Adaptive threshold based on actual terrain heights
                let max_height = heights.iter().cloned().fold(0.0_f32, f32::max);
                let height_range = max_height - sea_level;
                let mountain_threshold = sea_level + height_range * 0.3; // Top 70% of elevation range
                let mut candidates: Vec<(usize, f32)> = (0..count)
                    .filter(|idx| heights[*idx] > mountain_threshold)
                    .map(|idx| {
                        // Prioritize highest peaks
                        let elevation = heights[idx] - sea_level;
                        let score = elevation * elevation;
                        (idx, score)
                    })
                    .collect();
                candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

                let spacing = (width.min(height) as f32 * 0.2).max(5.0);
                let mut selected: Vec<usize> = Vec::new();
                let num_candidates = candidates.len();
                for (idx, _) in candidates {
                    if selected.len() >= attempts {
                        // Try more candidates
                        break;
                    }
                    let x = idx % width;
                    let y = idx / width;
                    let too_close = selected.iter().any(|&other| {
                        let ox = other % width;
                        let oy = other / width;
                        let dx = (x as isize - ox as isize).abs() as usize;
                        let dy = (y as isize - oy as isize).abs() as usize;
                        let wrap_dx = dx.min(width - dx);
                        let wrap_dy = dy.min(height - dy);
                        let distance_sq = (wrap_dx * wrap_dx + wrap_dy * wrap_dy) as f32;
                        distance_sq.sqrt() < spacing
                    });
                    if too_close {
                        continue;
                    }
                    selected.push(idx);
                }

                if !selected.is_empty() {
                    // Reduce minimum length requirement - even short rivers are better than none
                    let min_length = 3; // Very short threshold to allow more rivers
                    let rainfall_boost = config.hydrology_rainfall
                        * config.hydrology_major_river_boost.max(0.0)
                        * 0.75;

                    info!("Attempting {} river paths from {} highland candidates (want {} rivers, threshold: {:.1})",
                          selected.len(), num_candidates, desired, mountain_threshold);
                    let mut valid_rivers = 0;
                    let mut successful_paths = Vec::new();
                    let num_attempts = selected.len();

                    for seed in selected {
                        let mut path = Vec::new();
                        let mut current = seed;
                        let mut visited = vec![false; count];
                        let mut guard = 0;

                        // Force rivers to find a path to ocean by carving through obstacles
                        while guard < count * 2 {
                            // Allow longer searches
                            if visited[current] {
                                // Hit a loop - break out
                                break;
                            }
                            visited[current] = true;
                            path.push(current);

                            if heights[current] <= sea_level {
                                // Reached the ocean!
                                break;
                            }

                            // Find the best neighbor - prioritize reaching ocean
                            let mut best_score = f32::MAX;
                            let mut best_next = None;

                            let x = current % width;
                            let y = current / width;

                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = ((x as isize + dx as isize + width as isize)
                                        % width as isize)
                                        as usize;
                                    let ny = ((y as isize + dy as isize + height as isize)
                                        % height as isize)
                                        as usize;
                                    let neighbor = ny * width + nx;

                                    if visited[neighbor] {
                                        continue;
                                    }

                                    // Score based on:
                                    // 1. Height (lower is better)
                                    // 2. Distance to edge (closer is better as edges often have ocean)
                                    let height_score = heights[neighbor];
                                    let edge_dist =
                                        (nx.min(width - nx - 1).min(ny).min(height - ny - 1))
                                            as f32;
                                    let score = height_score + edge_dist * 0.05; // Slight preference for edges

                                    if score < best_score {
                                        best_score = score;
                                        best_next = Some(neighbor);
                                    }
                                }
                            }

                            if let Some(next) = best_next {
                                current = next;
                            } else {
                                // No unvisited neighbors - stuck
                                break;
                            }
                            guard += 1;
                        }

                        let land_length =
                            path.iter().filter(|&&idx| heights[idx] > sea_level).count();

                        // Debug: log all path attempts
                        let reached_ocean =
                            path.last().map_or(false, |&idx| heights[idx] <= sea_level);
                        info!("River path from height {:.1}: {} cells total, {} on land, reached ocean: {}",
                              heights[seed] - sea_level, path.len(), land_length, reached_ocean);

                        // CRITICAL: Only accept rivers that actually reach the ocean!
                        if !reached_ocean {
                            warn!("River path got stuck - not reaching ocean!");
                            continue;
                        }

                        if land_length < min_length {
                            continue;
                        }

                        valid_rivers += 1;
                        successful_paths.push((seed, path.clone()));

                        // Stop if we have enough successful rivers
                        if valid_rivers >= desired {
                            break;
                        }
                    }

                    // Now process only the successful paths (up to desired count)
                    for (river_idx, (seed, path)) in
                        successful_paths.iter().take(desired).enumerate()
                    {
                        info!(
                            "Processing successful major river {} from height {:.1}",
                            river_idx + 1,
                            heights[*seed] - sea_level
                        );

                        // Calculate strength for this river based on its length and boost
                        let path_length_factor = (path.len() as f32 / 50.0).min(2.0).max(1.0);
                        let base_flow = river_threshold * 5.0 * path_length_factor;
                        let strength = config.hydrology_major_river_boost.max(1.0) * base_flow;

                        // Mark the entire path and make it WIDE for visibility
                        for (i, &idx) in path.iter().enumerate() {
                            // Make major rivers 3-5 cells wide for visibility
                            let x = idx % width;
                            let y = idx / width;

                            // Mark a wider path
                            for dy in -2i32..=2i32 {
                                for dx in -2i32..=2i32 {
                                    // Create a diamond shape for the river
                                    if dx.abs() + dy.abs() > 2 {
                                        continue;
                                    }

                                    let nx = ((x as isize + dx as isize + width as isize)
                                        % width as isize)
                                        as usize;
                                    let ny = ((y as isize + dy as isize + height as isize)
                                        % height as isize)
                                        as usize;
                                    let wide_idx = ny * width + nx;

                                    // Mark this as part of a major river
                                    major_path_mask[wide_idx] = 1.0;
                                    river_mask[wide_idx] = 1.0;

                                    // Set significant depth
                                    let depth_factor = if dx == 0 && dy == 0 { 1.0 } else { 0.7 };
                                    river_depth[wide_idx] =
                                        river_depth[wide_idx].max(max_depth * 0.6 * depth_factor);

                                    // Ensure water level is set
                                    water_level[wide_idx] = water_level[wide_idx].max(sea_level);
                                }
                            }

                            // Accumulate flow as we go downstream
                            let position_factor = 1.0 + (i as f32 / 3.0);
                            let current_strength = strength * position_factor;
                            major_strength[idx] = major_strength[idx].max(current_strength);
                            flow_accum[idx] += current_strength;
                        }

                        // Add rainfall to the watershed of this river
                        if rainfall_boost > 0.0 {
                            let mut stack = vec![*seed];
                            let mut visited = vec![false; count];
                            while let Some(idx) = stack.pop() {
                                if visited[idx] {
                                    continue;
                                }
                                visited[idx] = true;
                                rainfall_map[idx] += rainfall_boost;
                                for &up in &upstream[idx] {
                                    if heights[up] > sea_level {
                                        stack.push(up);
                                    }
                                }
                            }
                        }
                    }

                    info!(
                        "Created {} valid major rivers with paths to ocean (attempted {})",
                        valid_rivers.min(desired),
                        num_attempts
                    );
                } else {
                    warn!(
                        "No major river candidates found! Mountain threshold: {:.1}",
                        mountain_threshold
                    );
                }
            }
        }

        for &idx in &order {
            let terrain_height = heights[idx];
            if terrain_height <= sea_level {
                continue;
            }

            let rainfall = rainfall_map[idx].max(0.0);
            let major = major_strength[idx];
            // Major rivers get massive water boost to ensure they're visible
            let major_contribution = if major_path_mask[idx] > 0.0 {
                major * 5.0 + river_threshold * 2.0 // Ensure major rivers always exceed threshold
            } else {
                major
            };
            let water = flow_accum[idx] + rainfall + major_contribution;
            let major_scale = if major_path_mask[idx] > 0.0 {
                (1.0 + config.hydrology_major_river_boost.max(0.0)).max(1.5)
            } else {
                1.0
            };
            let local_threshold = if major_path_mask[idx] > 0.0 {
                1.0 // Major rivers always form channels
            } else {
                river_threshold
            };

            if let Some(down) = downstream[idx] {
                flow_accum[down] += water;
                if water >= local_threshold {
                    let depth = if major_path_mask[idx] > 0.0 {
                        // Major rivers carve much deeper channels
                        let major_depth =
                            max_depth * (1.0 + config.hydrology_major_river_boost * 0.5);
                        ((water * depth_scale * 2.0) * major_scale).min(major_depth)
                    } else {
                        ((water * depth_scale) * major_scale).min(max_depth)
                    };
                    river_depth[idx] = river_depth[idx].max(depth);
                    let bed = terrain_height - river_depth[idx];
                    let surface = bed + river_depth[idx] * surface_ratio;
                    water_level[idx] = water_level[idx].max(surface.max(sea_level));
                    river_mask[idx] = 1.0;
                }
            } else if water >= lake_threshold {
                river_depth[idx] = river_depth[idx].max(lake_depth);
                let bed = terrain_height - river_depth[idx];
                let surface = terrain_height.max(sea_level);
                water_level[idx] = water_level[idx].max(surface.max(bed));
                lake_mask[idx] = 1.0;
            }
        }

        Self {
            width,
            height,
            planet_size,
            sea_level,
            river_max_depth: max_depth,
            lake_depth,
            river_depth,
            water_level,
            river_mask,
            lake_mask,
            rainfall: rainfall_map,
            major_path: major_path_mask,
            major_strength,
        }
    }

    fn wrap_index(width: usize, height: usize, x: isize, y: isize) -> usize {
        let w = width as isize;
        let h = height as isize;
        let ix = ((x % w) + w) % w;
        let iy = ((y % h) + h) % h;
        (iy as usize) * width + ix as usize
    }

    fn sample(&self, world_x: f32, world_z: f32) -> HydrologySample {
        if self.width == 0 || self.height == 0 {
            return HydrologySample::default();
        }

        let u = (world_x / self.planet_size).rem_euclid(1.0);
        let v = (world_z / self.planet_size).rem_euclid(1.0);

        let fx = u * self.width as f32;
        let fy = v * self.height as f32;

        let x0 = fx.floor() as isize;
        let y0 = fy.floor() as isize;
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let bilinear = |values: &[f32]| {
            let v00 = values[Self::wrap_index(self.width, self.height, x0, y0)];
            let v10 = values[Self::wrap_index(self.width, self.height, x0 + 1, y0)];
            let v01 = values[Self::wrap_index(self.width, self.height, x0, y0 + 1)];
            let v11 = values[Self::wrap_index(self.width, self.height, x0 + 1, y0 + 1)];
            lerp_f32(lerp_f32(v00, v10, tx), lerp_f32(v01, v11, tx), ty)
        };

        let mut depth = bilinear(&self.river_depth).max(0.0);
        let mut water_level = bilinear(&self.water_level);
        let river_mask = bilinear(&self.river_mask).clamp(0.0, 1.0);
        let lake_mask = bilinear(&self.lake_mask).clamp(0.0, 1.0);
        let rainfall = bilinear(&self.rainfall).max(0.0);
        let major = bilinear(&self.major_path).clamp(0.0, 1.0);
        let _major_strength = bilinear(&self.major_strength).max(0.0);

        if water_level <= 0.0 {
            water_level = self.sea_level;
        }

        let coverage = river_mask.max(lake_mask);
        depth *= coverage;
        water_level = lerp_f32(self.sea_level, water_level, coverage);

        let river_intensity = if depth > 0.01 {
            (depth / self.river_max_depth).clamp(0.0, 1.0) * river_mask
        } else {
            0.0
        };

        let lake_intensity = if depth > 0.01 {
            (depth / self.lake_depth.max(0.01)).clamp(0.0, 1.0) * lake_mask
        } else {
            0.0
        };

        HydrologySample {
            channel_depth: depth,
            water_level,
            river_intensity,
            lake_intensity,
            rainfall,
            major_river: major.max(0.0), // Return the actual value, not binary
        }
    }
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self::new(WorldGenConfig::default())
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        let seed = config.seed as u32;
        let continent_noise = Perlin::new(seed);
        let detail_noise = Perlin::new(seed.wrapping_add(1));
        let mountain_noise = Perlin::new(seed.wrapping_add(2));
        let moisture_noise = Perlin::new(seed.wrapping_add(3));
        let temperature_noise = Perlin::new(seed.wrapping_add(4));
        let island_noise = Perlin::new(seed.wrapping_add(5));
        let hydrology_rain_noise = Perlin::new(seed.wrapping_add(6));

        let continent_sites = generate_continent_sites(config.seed, config.continent_count.max(1));

        let mut generator = Self {
            config,
            continent_noise,
            detail_noise,
            mountain_noise,
            moisture_noise,
            temperature_noise,
            island_noise,
            hydrology_rain_noise,
            continent_sites,
            mountain_ranges: MountainRangeMap::empty(),
            hydrology: HydrologyMap::empty(),
        };

        generator.mountain_ranges = MountainRangeMap::generate(&generator.config);
        generator.hydrology = HydrologyMap::generate(&generator);
        generator
    }

    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        Self::new(WorldGenConfig::from_planet_config(planet_config))
    }

    pub fn config(&self) -> &WorldGenConfig {
        &self.config
    }

    pub fn planet_size(&self) -> u32 {
        self.config.planet_size
    }

    fn terrain_components(&self, world_x: f32, world_z: f32) -> TerrainComponents {
        let (u, v) = self.normalized_uv(world_x, world_z);

        // Force ocean borders at left/right edges for seamless wrapping
        let border_width = 0.03; // 3% of map width on each side
        let ocean_border_factor = if u < border_width {
            // Left edge - fade to ocean
            (u / border_width).clamp(0.0, 1.0)
        } else if u > (1.0 - border_width) {
            // Right edge - fade to ocean
            ((1.0 - u) / border_width).clamp(0.0, 1.0)
        } else {
            1.0 // No ocean border modification in the middle
        };

        let continent = self.fractal_periodic(
            &self.continent_noise,
            u,
            v,
            self.config.continent_frequency,
            4,
            2.0,
            0.45,
        );

        let continent_mask = ((continent + 1.0) * 0.5).powf(self.config.continent_power as f64);
        let mut land_factor = ((continent_mask as f32)
            - (self.config.continent_threshold - self.config.continent_bias))
            .max(0.0)
            / (1.0 - self.config.continent_threshold);
        land_factor = land_factor.clamp(0.0, 1.0);

        let site_mask = self.continent_site_mask(u as f32, v as f32);
        land_factor = (land_factor * site_mask * ocean_border_factor as f32).clamp(0.0, 1.0);

        let ocean_factor = 1.0 - land_factor;
        let sea_level = self.config.sea_level;
        let deep_floor = sea_level - self.config.deep_ocean_depth;
        let shallow_floor = sea_level - self.config.ocean_depth;

        let ocean_height = lerp_f32(
            deep_floor,
            shallow_floor,
            (continent_mask as f32).clamp(0.0, 1.0),
        );

        // Use world-space noise for consistent hill sizes across all world sizes
        // Multiple octaves for natural-looking terrain
        let detail_scale = 50.0; // Base hill size in blocks
        let detail1 = self.world_noise(&self.detail_noise, world_x, world_z, detail_scale) as f32;
        let detail2 = self.world_noise(
            &self.detail_noise,
            world_x + 1000.0,
            world_z + 1000.0,
            detail_scale * 2.0,
        ) as f32
            * 0.5;
        let detail3 = self.world_noise(
            &self.detail_noise,
            world_x + 2000.0,
            world_z + 2000.0,
            detail_scale * 4.0,
        ) as f32
            * 0.25;
        let detail =
            (detail1 + detail2 + detail3) / 1.75 * self.config.detail_amplitude * land_factor;

        // Use world-space noise for consistent mountain sizes
        let mountain_scale = 200.0; // Base mountain size in blocks
        let mountain1 = self.world_noise(&self.mountain_noise, world_x, world_z, mountain_scale);
        let mountain2 = self.world_noise(
            &self.mountain_noise,
            world_x + 5000.0,
            world_z + 5000.0,
            mountain_scale * 2.0,
        ) * 0.5;
        let mountain_raw = (mountain1 + mountain2) / 1.5;

        let mountain_mask = ((mountain_raw + 1.0) * 0.5).powf(1.8);
        let mountain_bonus = if mountain_mask as f32 > self.config.mountain_threshold {
            (mountain_mask as f32 - self.config.mountain_threshold)
                / (1.0 - self.config.mountain_threshold)
        } else {
            0.0
        };
        let ridge_factor = self.continent_ridge_factor(u as f32, v as f32);
        let range_factor = self
            .mountain_ranges
            .sample(u as f32, v as f32)
            .clamp(0.0, 1.0);
        let land_weight = land_factor.powf(0.65);
        let base_mountain = (mountain_bonus * ridge_factor + land_factor * 0.1).clamp(0.0, 1.0)
            * self.config.mountain_height
            * land_factor;
        let range_bonus = range_factor
            * self.config.mountain_height
            * self.config.mountain_range_strength
            * land_weight;
        let mountains = base_mountain + range_bonus;

        let interior_mask = land_factor.powf(1.4);
        let range_highlands = range_factor * self.config.highland_bonus * 0.6 * interior_mask;
        let highlands = ((ridge_factor * 0.9 + interior_mask * 0.4).clamp(0.0, 1.0)
            * self.config.highland_bonus
            * interior_mask)
            + range_highlands;

        let land_height = sea_level + detail + highlands + mountains + land_factor * 16.0;
        let island_raw = self.fractal_periodic(
            &self.island_noise,
            u,
            v,
            self.config.island_frequency,
            3,
            2.3,
            0.55,
        );
        let island_mask = ((island_raw + 1.0) * 0.5) as f32;
        let island_strength = ((island_mask - self.config.island_threshold)
            / (1.0 - self.config.island_threshold))
            .max(0.0)
            .clamp(0.0, 1.0);
        let ocean_only = ocean_factor.powf(self.config.island_falloff.max(0.1));
        let island_bonus = island_strength * ocean_only * self.config.island_height;

        let base_height = ocean_height * ocean_factor + land_height * land_factor + island_bonus;

        TerrainComponents { base_height }
    }

    fn raw_rainfall(&self, world_x: f32, world_z: f32) -> f32 {
        let base = self.config.hydrology_rainfall.max(0.0);
        if base <= 0.0 {
            return 0.0;
        }

        let variance = self.config.hydrology_rainfall_variance.clamp(0.0, 3.0);
        let (u, v) = self.normalized_uv(world_x, world_z);
        let noise = self.fractal_periodic(
            &self.hydrology_rain_noise,
            u,
            v,
            self.config.hydrology_rainfall_frequency.max(0.05),
            3,
            2.1,
            0.55,
        ) as f32;
        if variance <= f32::EPSILON {
            return base;
        }

        let humidity = self.sample_moisture(world_x, world_z) * 2.0 - 1.0;
        let noise = noise.clamp(-1.0, 1.0);
        let combined = (humidity * 0.6 + noise * 0.4).clamp(-1.0, 1.0);
        let multiplier = (1.0 + combined * variance).max(0.0);
        base * multiplier
    }

    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let hydro = self.sample_hydrology(world_x, world_z, components.base_height);
        let mut height = components.base_height - hydro.channel_depth;
        if hydro.lake_intensity > 0.0 {
            let shore_level = (hydro.water_level - self.config.lake_shore_blend).min(height);
            height = height.min(shore_level);
        }
        height.max(4.0)
    }

    fn sample_hydrology(&self, world_x: f32, world_z: f32, base_height: f32) -> HydrologySample {
        let mut sample = self.hydrology.sample(world_x, world_z);

        if base_height <= self.config.sea_level {
            sample.channel_depth = 0.0;
            sample.water_level = self.config.sea_level;
            sample.river_intensity = 0.0;
            sample.lake_intensity = 0.0;
            return sample;
        }

        if sample.channel_depth > 0.0 {
            let max_carve = (base_height - 4.0).max(0.0);
            sample.channel_depth = sample.channel_depth.min(max_carve);
        }

        if sample.water_level <= self.config.sea_level {
            sample.water_level = base_height - sample.channel_depth;
            sample.water_level = sample.water_level.max(self.config.sea_level);
        }

        sample
    }

    pub fn get_biome(&self, world_x: f32, world_z: f32) -> Biome {
        let height = self.get_height(world_x, world_z);
        let temperature_c = self.sample_temperature_c(world_x, world_z, height);
        let moisture = self.sample_moisture(world_x, world_z);

        self.classify_biome_at_position(world_x, world_z, height, temperature_c, moisture)
    }

    pub fn get_moisture(&self, world_x: f32, world_z: f32) -> f32 {
        self.sample_moisture(world_x, world_z)
    }

    pub fn get_temperature_c(&self, world_x: f32, world_z: f32) -> f32 {
        let height = self.get_height(world_x, world_z);
        self.sample_temperature_c(world_x, world_z, height)
    }

    pub fn get_water_level(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        if sample.water_level > self.config.sea_level {
            sample.water_level
        } else if components.base_height <= self.config.sea_level {
            self.config.sea_level
        } else {
            self.config.sea_level
        }
    }

    pub fn river_intensity(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        sample.river_intensity.max(sample.lake_intensity)
    }

    pub fn major_river_factor(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        sample.major_river
    }

    pub fn rainfall_intensity(&self, world_x: f32, world_z: f32) -> f32 {
        let sample = self.hydrology.sample(world_x, world_z);
        if self.hydrology.width <= 1 || self.hydrology.height <= 1 {
            self.raw_rainfall(world_x, world_z)
        } else {
            sample.rainfall
        }
    }

    pub fn get_block(&self, world_x: f32, world_y: f32, world_z: f32) -> BlockType {
        if world_y < 2.0 {
            return BlockType::Bedrock;
        }

        let height = self.get_height(world_x, world_z);
        let biome = self.get_biome(world_x, world_z);
        let water_surface = self.get_water_level(world_x, world_z);

        if world_y as f32 > height {
            if (world_y as f32) <= water_surface {
                return match biome {
                    Biome::FrozenOcean | Biome::IceCap => BlockType::Ice,
                    _ => BlockType::Water,
                };
            }
            return BlockType::Air;
        }

        if world_y >= height - 1.0 {
            return biome.surface_block();
        }

        if world_y >= height - 4.0 {
            return biome.subsurface_block();
        }

        BlockType::Stone
    }

    pub fn get_air_temperature(&self, world_x: f32, world_y: f32, world_z: f32) -> f32 {
        let temp_c = self.sample_temperature_c(world_x, world_z, world_y);
        celsius_to_fahrenheit(temp_c)
    }

    pub fn export_planet_preview<P: AsRef<Path>>(
        &self,
        width: u32,
        height: u32,
        path: P,
    ) -> image::ImageResult<()> {
        let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        let planet_size = self.config.planet_size as f32;

        let mut land_pixels = 0u64;
        let mut mountain_pixels = 0u64;

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let world_x = u * planet_size;
            let world_z = v * planet_size;
            let height_value = self.get_height(world_x, world_z);
            let biome = self.get_biome(world_x, world_z);
            let color = self.preview_color(world_x, world_z, biome, height_value);
            *pixel = Rgba(color);

            if height_value >= self.config.sea_level {
                land_pixels += 1;
            }
            if height_value
                >= self.config.sea_level
                    + self.config.highland_bonus * 0.45
                    + self.config.mountain_height * 0.22
            {
                mountain_pixels += 1;
            }
        }

        let total_pixels = (width as u64) * (height as u64);
        let land_ratio = land_pixels as f64 / total_pixels as f64;
        let mountain_ratio = mountain_pixels as f64 / total_pixels as f64;

        info!(
            "Preview summary: {:.1}% land, {:.1}% highlands",
            land_ratio * 100.0,
            mountain_ratio * 100.0
        );

        image.save(path)
    }

    fn normalized_uv(&self, world_x: f32, world_z: f32) -> (f64, f64) {
        let size = self.config.planet_size.max(1) as f32;
        let u = (world_x / size).rem_euclid(1.0) as f64;
        let v = (world_z / size).rem_euclid(1.0) as f64;
        (u, v)
    }

    fn periodic_noise(&self, noise: &Perlin, u: f64, v: f64, cycles: f64) -> f64 {
        const TAU: f64 = std::f64::consts::PI * 2.0;
        let theta = (u * cycles) * TAU;
        let phi = (v * cycles) * TAU;
        noise.get([theta.sin(), theta.cos(), phi.sin(), phi.cos()])
    }

    // Direct world coordinate noise sampling for scale-invariant features
    fn world_noise(&self, noise: &Perlin, world_x: f32, world_z: f32, scale: f32) -> f64 {
        // Sample noise directly at world coordinates
        // scale determines feature size in blocks
        let x = world_x as f64 / scale as f64;
        let z = world_z as f64 / scale as f64;

        // Use 4D noise to maintain tileable behavior across world boundaries
        let planet_size = self.config.planet_size as f64;
        const TAU: f64 = std::f64::consts::PI * 2.0;
        let theta = (world_x as f64 / planet_size) * TAU;
        let phi = (world_z as f64 / planet_size) * TAU;

        // Mix world-space and periodic coordinates for best of both
        noise.get([
            theta.sin() + x * 0.1,
            theta.cos() + x * 0.1,
            phi.sin() + z * 0.1,
            phi.cos() + z * 0.1,
        ])
    }

    fn fractal_periodic(
        &self,
        noise: &Perlin,
        u: f64,
        v: f64,
        base_cycles: f64,
        octaves: usize,
        lacunarity: f64,
        gain: f64,
    ) -> f64 {
        let mut frequency = base_cycles.max(0.0001);
        let mut amplitude = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;

        for _ in 0..octaves {
            sum += self.periodic_noise(noise, u, v, frequency) * amplitude;
            norm += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }

        if norm == 0.0 {
            0.0
        } else {
            sum / norm
        }
    }

    fn sample_moisture(&self, world_x: f32, world_z: f32) -> f32 {
        let (u, v) = self.normalized_uv(world_x, world_z);
        let moisture = self.fractal_periodic(
            &self.moisture_noise,
            u,
            v,
            self.config.moisture_frequency,
            3,
            2.2,
            0.55,
        );
        ((moisture + 1.0) * 0.5) as f32
    }

    fn sample_temperature_c(&self, world_x: f32, world_z: f32, height: f32) -> f32 {
        let size = self.config.planet_size.max(1) as f32;
        let latitude = ((world_z / size).rem_euclid(1.0) - 0.5).abs();
        let lat_factor = (1.0 - latitude * 2.0).clamp(-1.0, 1.0);

        let base_temp = lerp_f32(
            self.config.pole_temp_c,
            self.config.equator_temp_c,
            (lat_factor + 1.0) * 0.5,
        );

        let elevation_above_sea = (height - self.config.sea_level).max(0.0);
        let lapse = elevation_above_sea * self.config.lapse_rate_c_per_block;

        let (u, v) = self.normalized_uv(world_x, world_z);
        let variation = self.fractal_periodic(&self.temperature_noise, u, v, 2.5, 3, 2.0, 0.6)
            as f32
            * self.config.temperature_variation;

        base_temp - lapse + variation
    }

    fn classify_beach_biome(
        &self,
        world_x: f32,
        world_z: f32,
        height: f32,
        temp_c: f32,
    ) -> Option<Biome> {
        let sea_level = self.config.sea_level;
        let elevation_above_sea = height - sea_level;

        // Expanded elevation range for beaches - from slightly underwater to higher ground
        if elevation_above_sea < -2.0 || elevation_above_sea > 12.0 {
            return None;
        }

        // Calculate distance to deep water and terrain slope
        let (distance_to_water, avg_slope) =
            self.calculate_coastal_properties(world_x, world_z, height);

        // No beach if no water found within reasonable distance
        if distance_to_water > 150.0 {
            return None;
        }

        // Calculate maximum beach width based on slope and elevation
        // More generous slope tolerance for beach formation
        let slope_factor = (1.0 - avg_slope.min(0.8) / 0.8).max(0.0); // Allow steeper slopes
        let elevation_factor = if elevation_above_sea < 1.0 {
            1.0 // Full beach potential near sea level
        } else if elevation_above_sea < 4.0 {
            0.8 // Still good beach potential up to 4 blocks
        } else {
            (1.0 - (elevation_above_sea - 4.0) / 8.0).max(0.0) // Gradual falloff
        };

        // Moderate beach width - up to 40 blocks in ideal conditions
        let max_beach_width = 40.0 * slope_factor * elevation_factor;

        // Lower threshold - allow beaches even on moderate slopes
        if max_beach_width < 3.0 {
            return None;
        }

        // Check if we're within beach zone
        if distance_to_water <= max_beach_width {
            // Add some randomness - not all shores become beaches
            let beach_probability =
                self.calculate_beach_probability(world_x, world_z, slope_factor);

            // Moderate probability threshold - selective beach formation
            if beach_probability > 0.4 {
                return Some(if temp_c < 0.0 {
                    Biome::Snow
                } else {
                    Biome::Beach
                });
            }
        }

        None
    }

    fn calculate_coastal_properties(&self, world_x: f32, world_z: f32, height: f32) -> (f32, f32) {
        let sea_level = self.config.sea_level;
        let mut min_distance = f32::MAX;
        let sample_count = 16;
        let mut height_samples = Vec::new();

        // Sample in expanding circles to find water and measure slope
        for radius in [3.0, 6.0, 10.0, 20.0, 40.0, 80.0] {
            let mut found_water_at_radius = false;

            for i in 0..sample_count {
                let angle = (i as f32) * std::f32::consts::TAU / (sample_count as f32);
                let check_x = world_x + angle.cos() * radius;
                let check_z = world_z + angle.sin() * radius;

                let components = self.terrain_components(check_x, check_z);
                height_samples.push(components.base_height);

                // More lenient water detection - just below sea level counts
                if components.base_height < sea_level - 0.5 {
                    min_distance = min_distance.min(radius);
                    found_water_at_radius = true;
                }
            }

            // If we found water at this radius, we can stop expanding
            if found_water_at_radius && radius >= 10.0 {
                break;
            }
        }

        // Calculate average slope from height variations
        let avg_slope = if height_samples.len() > 1 {
            let mut total_slope = 0.0;
            let mut sample_count = 0;

            for (i, &sample_height) in height_samples.iter().enumerate() {
                let height_diff = (sample_height - height).abs();
                let distance = if i < 16 {
                    3.0
                } else if i < 32 {
                    6.0
                } else if i < 48 {
                    10.0
                } else if i < 64 {
                    20.0
                } else if i < 80 {
                    40.0
                } else {
                    80.0
                };

                if distance > 0.0 {
                    total_slope += height_diff / distance;
                    sample_count += 1;
                }
            }

            if sample_count > 0 {
                total_slope / sample_count as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        (min_distance, avg_slope)
    }

    fn calculate_beach_probability(&self, world_x: f32, world_z: f32, slope_factor: f32) -> f32 {
        // Use lower frequency noise for more continuous beaches
        let (u, v) = self.normalized_uv(world_x, world_z);
        let beach_noise = self.fractal_periodic(
            &self.detail_noise, // Reuse existing noise
            u,
            v,
            self.config.detail_frequency * 0.5, // Lower frequency for larger, more continuous patches
            2,
            2.0,
            0.5,
        ) as f32;

        // Combine slope factor with noise - but give slope more weight
        // Better slopes = higher base probability
        let base_probability = slope_factor * 0.85; // Increased from 0.7
        let noise_influence = (beach_noise + 1.0) * 0.5 * 0.3; // Reduced from 0.6 to 0.3

        (base_probability + noise_influence).clamp(0.0, 1.0)
    }

    fn classify_biome_at_position(
        &self,
        world_x: f32,
        world_z: f32,
        height: f32,
        temp_c: f32,
        moisture: f32,
    ) -> Biome {
        let sea_level = self.config.sea_level;
        let deep_ocean_cutoff = sea_level - self.config.deep_ocean_depth;
        let shallow_ocean_cutoff = sea_level - 1.5;

        if height < deep_ocean_cutoff {
            return if temp_c <= -2.0 {
                Biome::FrozenOcean
            } else {
                Biome::DeepOcean
            };
        }

        if height < shallow_ocean_cutoff {
            return if temp_c <= -2.0 {
                Biome::FrozenOcean
            } else {
                Biome::Ocean
            };
        }

        // Check for beach biome using the new system
        if let Some(beach_biome) = self.classify_beach_biome(world_x, world_z, height, temp_c) {
            return beach_biome;
        }

        let elevation = height - sea_level;

        let mountain_limit = self.config.highland_bonus * 0.6 + self.config.mountain_height * 0.35;

        if elevation > mountain_limit {
            return if temp_c < -5.0 {
                Biome::SnowyMountain
            } else {
                Biome::Mountain
            };
        }

        if temp_c < -15.0 {
            return Biome::IceCap;
        }
        if temp_c < -5.0 {
            return Biome::Snow;
        }
        if temp_c < 0.0 {
            return Biome::Tundra;
        }

        if temp_c < 8.0 {
            return if moisture < 0.35 {
                Biome::BorealForest
            } else {
                Biome::TemperateForest
            };
        }

        if temp_c < 18.0 {
            if moisture < 0.25 {
                return Biome::TemperateGrassland;
            } else if moisture < 0.6 {
                return Biome::TemperateForest;
            } else {
                return Biome::TropicalRainforest;
            }
        }

        if temp_c < 26.0 {
            if moisture < 0.2 {
                return Biome::Desert;
            } else if moisture < 0.45 {
                return Biome::Savanna;
            } else {
                return Biome::TropicalRainforest;
            }
        }

        if moisture < 0.15 {
            Biome::Desert
        } else if moisture < 0.45 {
            Biome::Savanna
        } else {
            Biome::TropicalRainforest
        }
    }

    fn continent_site_mask(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let radius = self.config.continent_radius.max(0.01);
        let radius_sq = radius * radius;
        let edge_power = self.config.continent_edge_power.max(0.1);
        let mut best = 0.0_f32;

        for site in &self.continent_sites {
            let du = torus_distance(u, site.position.x);
            let dv = torus_distance(v, site.position.y);
            let dist_sq = du * du + dv * dv;

            if dist_sq <= radius_sq {
                let influence = 1.0 - (dist_sq / radius_sq);
                best = best.max(influence);
            }
        }

        if best == 0.0 {
            0.0
        } else {
            best.powf(edge_power)
        }
    }

    fn continent_ridge_factor(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let radius = self.config.continent_radius.max(0.01);
        let ridge_width = (radius * 0.3).max(0.02);
        let mut strongest = 0.0_f32;

        for site in &self.continent_sites {
            let du = torus_distance(u, site.position.x);
            let dv = torus_distance(v, site.position.y);

            let dist_sq = du * du + dv * dv;
            if dist_sq > radius * radius {
                continue;
            }

            let cos_a = site.ridge_angle.cos();
            let sin_a = site.ridge_angle.sin();
            let along = du * cos_a + dv * sin_a;
            let across = -du * sin_a + dv * cos_a;

            let longitudinal = (1.0 - (along.abs() / radius)).max(0.0);
            let transverse = (1.0 - (across.abs() / ridge_width)).max(0.0);
            strongest = strongest.max(longitudinal * transverse);
        }

        strongest.clamp(0.0, 1.0)
    }

    pub fn preview_color(&self, world_x: f32, world_z: f32, biome: Biome, height: f32) -> [u8; 4] {
        let sea_level = self.config.sea_level;
        let water_depth = (sea_level - height).max(0.0);

        let base = match biome {
            Biome::DeepOcean => {
                let t = (water_depth / self.config.deep_ocean_depth).clamp(0.0, 1.0);
                lerp_color([12, 36, 92], [2, 9, 28], t)
            }
            Biome::Ocean => {
                let t = (water_depth / self.config.ocean_depth).clamp(0.0, 1.0);
                lerp_color([30, 90, 180], [8, 48, 128], t)
            }
            Biome::FrozenOcean | Biome::IceCap => [210, 230, 240],
            Biome::Beach => [216, 200, 160],
            Biome::Desert => [236, 212, 120],
            Biome::Savanna => [198, 182, 96],
            Biome::TropicalRainforest => [44, 118, 56],
            Biome::TemperateGrassland => [100, 176, 80],
            Biome::TemperateForest => [70, 140, 72],
            Biome::BorealForest => [60, 120, 104],
            Biome::Tundra => [150, 160, 150],
            Biome::Snow => [240, 240, 245],
            Biome::Mountain => [130, 130, 130],
            Biome::SnowyMountain => [232, 236, 242],
        };

        let min_height = sea_level - self.config.deep_ocean_depth;
        let max_height = sea_level + self.config.mountain_height + 64.0;
        let normalized = ((height - min_height) / (max_height - min_height)).clamp(0.0, 1.0);
        let shade = 0.6 + normalized * 0.4;

        let [r, g, b] = base;
        let mut color = [
            ((r as f32) * shade).min(255.0) as u8,
            ((g as f32) * shade).min(255.0) as u8,
            ((b as f32) * shade).min(255.0) as u8,
            255,
        ];

        let river_intensity = self.river_intensity(world_x, world_z);
        let major_river = self.major_river_factor(world_x, world_z);

        // Show major rivers in a distinct darker blue color
        if major_river > 0.1 {
            // Lower threshold for visibility
            // Major rivers are dark blue and fully opaque
            color[0] = 5;
            color[1] = 30;
            color[2] = 100;
        } else if river_intensity > 0.02 {
            let river_color = [20.0, 90.0, 210.0];
            let blend = river_intensity.clamp(0.0, 1.0);
            color[0] = lerp_f32(color[0] as f32, river_color[0], blend) as u8;
            color[1] = lerp_f32(color[1] as f32, river_color[1], blend) as u8;
            color[2] = lerp_f32(color[2] as f32, river_color[2], blend) as u8;
        }

        color
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTemperature>()
            .add_systems(Startup, setup_world_generator)
            .add_systems(
                Update,
                update_temperature.run_if(in_state(GameState::Playing)),
            );
    }
}

fn setup_world_generator(mut commands: Commands, planet_config: Res<PlanetConfig>) {
    let world_gen = WorldGenerator::from_planet_config(&planet_config);

    if let Ok(env_value) = std::env::var("FORGE_EXPORT_WORLD_MAP") {
        let output_path: PathBuf = if env_value.trim().is_empty() {
            PathBuf::from("target/world_preview.png")
        } else {
            PathBuf::from(env_value.trim())
        };

        let map_size = std::env::var("FORGE_WORLD_MAP_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|size| *size > 0)
            .unwrap_or(1024);
        let map_height = (map_size / 2).max(1);

        match world_gen.export_planet_preview(map_size, map_height, &output_path) {
            Ok(()) => info!(
                "Exported world preview image to {:?} ({}x{})",
                output_path, map_size, map_height
            ),
            Err(error) => warn!(
                "Failed to export world preview image to {:?}: {}",
                output_path, error
            ),
        }
    }

    commands.insert_resource(world_gen);
}

fn update_temperature(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    world_gen: Res<WorldGenerator>,
    mut temperature: ResMut<CurrentTemperature>,
) {
    let Ok(transform) = camera_query.get_single() else {
        return;
    };

    let pos = transform.translation;
    let chunk_x = (pos.x / 32.0).floor() as i32;
    let chunk_z = (pos.z / 32.0).floor() as i32;

    if chunk_x != temperature.last_chunk_x || chunk_z != temperature.last_chunk_z {
        let temp_f = world_gen.get_air_temperature(pos.x, pos.y, pos.z);
        temperature.update(temp_f);
        temperature.last_chunk_x = chunk_x;
        temperature.last_chunk_z = chunk_z;
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn celsius_to_fahrenheit(c: f32) -> f32 {
    c * 9.0 / 5.0 + 32.0
}

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        lerp_f32(a[0] as f32, b[0] as f32, t) as u8,
        lerp_f32(a[1] as f32, b[1] as f32, t) as u8,
        lerp_f32(a[2] as f32, b[2] as f32, t) as u8,
    ]
}

fn generate_continent_sites(seed: u64, count: u32) -> Vec<ContinentSite> {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = count.max(1);
    let grid_len = (n as f32).sqrt().ceil() as u32;
    let cell_size = 1.0 / grid_len as f32;
    let jitter = cell_size * 0.6;

    let mut sites = Vec::with_capacity(n as usize);
    let mut index = 0u32;

    let offset_u = rng.gen::<f32>() * cell_size;
    let offset_v = rng.gen::<f32>() * cell_size;

    for row in 0..grid_len {
        for col in 0..grid_len {
            if index >= n {
                break;
            }
            index += 1;
            let base_u = (col as f32 + offset_u).rem_euclid(grid_len as f32) * cell_size;
            let base_v = (row as f32 + offset_v).rem_euclid(grid_len as f32) * cell_size;
            let jitter_u = (rng.gen::<f32>() - 0.5) * jitter;
            let jitter_v = (rng.gen::<f32>() - 0.5) * jitter;
            let u = (base_u + jitter_u).rem_euclid(1.0);
            let v = (base_v + jitter_v).rem_euclid(1.0);
            let angle = rng.gen::<f32>() * TAU;
            sites.push(ContinentSite {
                position: Vec2::new(u, v),
                ridge_angle: angle,
            });
        }
        if index >= n {
            break;
        }
    }

    sites
}

fn wrap_vec2(v: Vec2) -> Vec2 {
    Vec2::new(v.x.rem_euclid(1.0), v.y.rem_euclid(1.0))
}

fn rotate_vec2(vec: Vec2, radians: f32) -> Vec2 {
    let (sin_a, cos_a) = radians.sin_cos();
    Vec2::new(vec.x * cos_a - vec.y * sin_a, vec.x * sin_a + vec.y * cos_a)
}

fn torus_noise(noise: &Perlin, u: f32, v: f32, cycles: f32, extra: f32) -> f32 {
    if cycles <= f32::EPSILON {
        return 0.0;
    }

    let cycles = cycles.max(0.01) as f64;
    let theta = (u as f64 * cycles) * std::f64::consts::TAU;
    let phi = (v as f64 * cycles) * std::f64::consts::TAU;
    let extra_angle = (extra as f64) * std::f64::consts::TAU;

    noise.get([
        theta.sin(),
        theta.cos(),
        phi.sin() + extra_angle.sin() * 0.35,
        phi.cos() + extra_angle.cos() * 0.35,
    ]) as f32
}

fn wrap_index(value: i32, size: i32) -> i32 {
    let mut result = value % size;
    if result < 0 {
        result += size;
    }
    result
}

fn wrap_index_isize(value: isize, size: isize) -> isize {
    let mut result = value % size;
    if result < 0 {
        result += size;
    }
    result
}

fn torus_delta(a: f32, b: f32) -> f32 {
    let mut diff = b - a;
    if diff > 0.5 {
        diff -= 1.0;
    } else if diff < -0.5 {
        diff += 1.0;
    }
    diff
}

fn torus_distance(a: f32, b: f32) -> f32 {
    let diff = (a - b).abs().fract();
    if diff > 0.5 {
        1.0 - diff
    } else {
        diff
    }
}
