use bevy::prelude::*;
use image::{ImageBuffer, Rgba};
use noise::{NoiseFn, Perlin};
use std::path::{Path, PathBuf};

use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::PlanetConfig;

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

#[derive(Resource, Clone)]
pub struct WorldGenConfig {
    pub seed: u64,
    pub planet_size: u32,
    pub sea_level: f32,
    pub ocean_depth: f32,
    pub deep_ocean_depth: f32,
    pub continent_threshold: f32,
    pub continent_power: f32,
    pub continent_bias: f32,
    pub continent_frequency: f64,
    pub detail_frequency: f64,
    pub detail_amplitude: f32,
    pub mountain_frequency: f64,
    pub mountain_height: f32,
    pub mountain_threshold: f32,
    pub moisture_frequency: f64,
    pub equator_temp_c: f32,
    pub pole_temp_c: f32,
    pub lapse_rate_c_per_block: f32,
    pub temperature_variation: f32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            planet_size: 16384,
            sea_level: 64.0,
            ocean_depth: 24.0,
            deep_ocean_depth: 40.0,
            continent_threshold: 0.25,
            continent_power: 1.0,
            continent_bias: 0.22,
            continent_frequency: 0.6,
            detail_frequency: 7.0,
            detail_amplitude: 8.0,
            mountain_frequency: 2.8,
            mountain_height: 52.0,
            mountain_threshold: 0.5,
            moisture_frequency: 2.6,
            equator_temp_c: 30.0,
            pole_temp_c: -25.0,
            lapse_rate_c_per_block: 0.008,
            temperature_variation: 3.0,
        }
    }
}

impl WorldGenConfig {
    pub fn from_planet_config(config: &PlanetConfig) -> Self {
        let planet_size = config.size_chunks as u32 * 32;
        let size_scale = planet_size.max(1) as f32 / 16384.0;

        Self {
            seed: config.seed,
            planet_size,
            sea_level: config.sea_level,
            ocean_depth: 16.0 * size_scale.clamp(0.7, 1.3),
            deep_ocean_depth: 24.0 * size_scale.clamp(0.8, 1.5),
            continent_threshold: 0.2,
            continent_power: 0.95,
            continent_bias: 0.25,
            continent_frequency: 0.55,
            detail_frequency: 7.5,
            detail_amplitude: 15.0,
            mountain_frequency: 2.0,
            mountain_height: 80.0 * size_scale.clamp(0.8, 1.4),
            mountain_threshold: 0.48,
            moisture_frequency: 2.7,
            equator_temp_c: 28.0,
            pole_temp_c: -30.0,
            lapse_rate_c_per_block: 0.008,
            temperature_variation: 3.0,
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

        Self {
            config,
            continent_noise,
            detail_noise,
            mountain_noise,
            moisture_noise,
            temperature_noise,
        }
    }

    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        Self::new(WorldGenConfig::from_planet_config(planet_config))
    }

    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        let (u, v) = self.normalized_uv(world_x, world_z);

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
        let land_factor = ((continent_mask as f32)
            - (self.config.continent_threshold - self.config.continent_bias))
            .max(0.0)
            / (1.0 - self.config.continent_threshold);
        let land_factor = land_factor.clamp(0.0, 1.0);

        let ocean_factor = 1.0 - land_factor;
        let sea_level = self.config.sea_level;
        let deep_floor = sea_level - self.config.deep_ocean_depth;
        let shallow_floor = sea_level - self.config.ocean_depth;

        let ocean_height = lerp_f32(
            deep_floor,
            shallow_floor,
            (continent_mask as f32).clamp(0.0, 1.0),
        );

        let detail = self.fractal_periodic(
            &self.detail_noise,
            u,
            v,
            self.config.detail_frequency,
            3,
            2.5,
            0.4,
        ) as f32
            * self.config.detail_amplitude
            * land_factor;

        let mountain_raw = self.fractal_periodic(
            &self.mountain_noise,
            u,
            v,
            self.config.mountain_frequency,
            4,
            2.1,
            0.5,
        );
        let mountain_mask = ((mountain_raw + 1.0) * 0.5).powf(1.8);
        let mountain_bonus = if mountain_mask as f32 > self.config.mountain_threshold {
            (mountain_mask as f32 - self.config.mountain_threshold)
                / (1.0 - self.config.mountain_threshold)
        } else {
            0.0
        };
        let mountains = mountain_bonus.clamp(0.0, 1.0) * self.config.mountain_height * land_factor;

        let land_height = sea_level + detail + mountains + land_factor * 18.0;
        let height = ocean_height * ocean_factor + land_height * land_factor;

        height.max(4.0)
    }

    pub fn get_biome(&self, world_x: f32, world_z: f32) -> Biome {
        let height = self.get_height(world_x, world_z);
        let temperature_c = self.sample_temperature_c(world_x, world_z, height);
        let moisture = self.sample_moisture(world_x, world_z);

        self.classify_biome(height, temperature_c, moisture)
    }

    pub fn get_block(&self, world_x: f32, world_y: f32, world_z: f32) -> BlockType {
        if world_y < 2.0 {
            return BlockType::Bedrock;
        }

        let height = self.get_height(world_x, world_z);
        let biome = self.get_biome(world_x, world_z);

        if world_y > height {
            if world_y <= self.config.sea_level {
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
            let color = self.preview_color(biome, height_value);
            *pixel = Rgba(color);

            if height_value >= self.config.sea_level {
                land_pixels += 1;
            }
            if height_value >= self.config.sea_level + self.config.mountain_height * 0.6 {
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

    fn classify_biome(&self, height: f32, temp_c: f32, moisture: f32) -> Biome {
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

        if height < sea_level + 2.0 {
            return if temp_c < 0.0 {
                Biome::Snow
            } else {
                Biome::Beach
            };
        }

        let elevation = height - sea_level;

        if elevation > self.config.mountain_height * 0.8 {
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

    fn preview_color(&self, biome: Biome, height: f32) -> [u8; 4] {
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
        [
            ((r as f32) * shade).min(255.0) as u8,
            ((g as f32) * shade).min(255.0) as u8,
            ((b as f32) * shade).min(255.0) as u8,
            255,
        ]
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
