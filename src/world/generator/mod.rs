use bevy::prelude::*;
use image::{ImageBuffer, Rgba};
use noise::{NoiseFn, Perlin};
use std::path::{Path, PathBuf};

use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::PlanetConfig;

use super::biome::Biome;
use super::config::{CurrentTemperature, WorldGenConfig};

mod continents;
mod hydrology;
mod mountains;
mod util;

use continents::{generate_continent_sites, ContinentSite};
use hydrology::{HydrologyMap, HydrologySample};
use mountains::MountainRangeMap;
use util::{celsius_to_fahrenheit, lerp_color, lerp_f32};

/// Logical phases in the world generation pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldGenPhase {
    Core,
    Continents,
    Terrain,
    Mountains,
    Climate,
    Islands,
    Hydrology,
    Finalize,
}

pub trait WorldGenProgress {
    fn on_phase(&mut self, phase: WorldGenPhase);
}

struct NoopProgress;

impl WorldGenProgress for NoopProgress {
    fn on_phase(&mut self, _phase: WorldGenPhase) {}
}

impl<F> WorldGenProgress for F
where
    F: FnMut(WorldGenPhase),
{
    fn on_phase(&mut self, phase: WorldGenPhase) {
        self(phase);
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

#[derive(Clone, Copy)]
struct TerrainComponents {
    base_height: f32,
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self::new(WorldGenConfig::default())
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        Self::with_progress(config, NoopProgress)
    }

    pub fn with_progress<P>(config: WorldGenConfig, mut progress: P) -> Self
    where
        P: WorldGenProgress,
    {
        progress.on_phase(WorldGenPhase::Core);

        let seed = config.seed as u32;
        let continent_noise = Perlin::new(seed);
        let detail_noise = Perlin::new(seed.wrapping_add(1));
        let mountain_noise = Perlin::new(seed.wrapping_add(2));
        let moisture_noise = Perlin::new(seed.wrapping_add(3));
        let temperature_noise = Perlin::new(seed.wrapping_add(4));
        let island_noise = Perlin::new(seed.wrapping_add(5));
        let hydrology_rain_noise = Perlin::new(seed.wrapping_add(6));

        let mut generator = Self {
            config,
            continent_noise,
            detail_noise,
            mountain_noise,
            moisture_noise,
            temperature_noise,
            island_noise,
            hydrology_rain_noise,
            continent_sites: Vec::new(),
            mountain_ranges: MountainRangeMap::empty(),
            hydrology: HydrologyMap::empty(),
        };

        progress.on_phase(WorldGenPhase::Continents);
        generator.continent_sites = generate_continent_sites(
            generator.config.seed,
            generator.config.continent_count.max(1),
        );

        progress.on_phase(WorldGenPhase::Terrain);
        generator.initialize_terrain_phase();

        progress.on_phase(WorldGenPhase::Mountains);
        generator.mountain_ranges = MountainRangeMap::generate(&generator.config);

        progress.on_phase(WorldGenPhase::Climate);
        generator.initialize_climate_phase();

        progress.on_phase(WorldGenPhase::Islands);
        generator.initialize_island_phase();

        progress.on_phase(WorldGenPhase::Hydrology);
        generator.hydrology = HydrologyMap::generate(&generator);

        progress.on_phase(WorldGenPhase::Finalize);
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

    fn initialize_terrain_phase(&mut self) {
        // Placeholder for terrain preprocessing; retained for future expansion.
    }

    fn initialize_climate_phase(&mut self) {
        // Currently climate is computed procedurally at sample time.
        // This hook exists to support future cached climate data.
    }

    fn initialize_island_phase(&mut self) {
        // Islands rely on procedural noise at sample time, so no precomputation yet.
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
