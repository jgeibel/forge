use bevy::prelude::*;
use noise::Seedable;
use crate::planet::config::PlanetConfig;
use crate::block::BlockType;
use super::biomes::Biome;
use super::terrain::TerrainGenerator;
use super::features::FeatureGenerator;

#[derive(Resource, Clone)]
pub struct WorldGenConfig {
    pub seed: u64,
    pub planet_size: u32,
    pub sea_level: f32,
    
    // Continental generation
    pub continental_frequency: f64,
    pub continental_octaves: usize,
    
    // Mountain generation
    pub mountain_frequency: f64,
    pub mountain_scale: f32,
    
    // Cave generation
    pub cave_density: f32,
    pub cave_threshold: f64,
    
    // Ore generation
    pub ore_richness: f32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            planet_size: 16384,
            sea_level: 64.0,
            
            // Continental generation - very low frequency for large land masses
            continental_frequency: 0.0003,
            continental_octaves: 4,
            
            // Mountain generation - medium frequency for mountain ranges
            mountain_frequency: 0.002,
            mountain_scale: 48.0,
            
            // Cave generation
            cave_density: 0.5,
            cave_threshold: 0.55,
            
            // Ore generation
            ore_richness: 1.0,
        }
    }
}

impl WorldGenConfig {
    pub fn from_planet_config(config: &PlanetConfig) -> Self {
        let planet_size = config.size_chunks as u32 * 32;
        
        // Scale frequencies based on planet size
        // Use full linear scaling for frequencies (not sqrt) to maintain continent sizes
        let size_scale = planet_size as f64 / 16384.0;
        
        Self {
            seed: config.seed,
            planet_size,
            sea_level: config.sea_level,
            
            // Scale continental frequency with planet size
            // Frequency needs to scale linearly with size to maintain feature proportions
            continental_frequency: 0.0003 / size_scale,
            continental_octaves: match planet_size {
                0..=4096 => 3,
                4097..=16384 => 4,
                16385..=32768 => 5,
                _ => 6,
            },
            
            mountain_frequency: 0.002 / size_scale,
            mountain_scale: 48.0 * (size_scale as f32).min(2.0),
            
            cave_density: 0.5,
            cave_threshold: 0.55,
            ore_richness: 1.0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct WorldGenerator {
    terrain_gen: TerrainGenerator,
    feature_gen: FeatureGenerator,
    config: WorldGenConfig,
}

impl Default for WorldGenerator {
    fn default() -> Self {
        let config = WorldGenConfig::default();
        Self::new(config)
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        let terrain_gen = TerrainGenerator::new(config.seed, &config);
        let feature_gen = FeatureGenerator::new(config.seed, &config);
        
        Self {
            terrain_gen,
            feature_gen,
            config,
        }
    }
    
    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        let config = WorldGenConfig::from_planet_config(planet_config);
        Self::new(config)
    }
    
    /// Generate the base height at a world position (wrapping around planet edges)
    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        // Wrap coordinates for seamless planet
        let wrapped_x = world_x.rem_euclid(self.config.planet_size as f32);
        let wrapped_z = world_z.rem_euclid(self.config.planet_size as f32);
        
        self.terrain_gen.get_height(wrapped_x as f64, wrapped_z as f64)
    }
    
    /// Get the biome at a world position
    pub fn get_biome(&self, world_x: f32, world_z: f32) -> Biome {
        let wrapped_x = world_x.rem_euclid(self.config.planet_size as f32);
        let wrapped_z = world_z.rem_euclid(self.config.planet_size as f32);
        let height = self.get_height(world_x, world_z);
        
        self.terrain_gen.get_biome(wrapped_x as f64, wrapped_z as f64, height)
    }
    
    /// Generate a block at a specific world position
    pub fn get_block(&self, world_x: f32, world_y: f32, world_z: f32) -> BlockType {
        let height = self.get_height(world_x, world_z);
        let biome = self.get_biome(world_x, world_z);
        
        // Bedrock layer
        if world_y < 3.0 {
            return BlockType::Bedrock;
        }
        
        // Below sea level but above terrain - water or ice
        if world_y < self.config.sea_level && world_y > height {
            // Check if water should be frozen based on biome
            if matches!(biome, Biome::FrozenOcean | Biome::IcePlains | Biome::IceSpikes) {
                // Ice layer on top, deeper water may be liquid
                if world_y >= self.config.sea_level - 3.0 {
                    return BlockType::Ice;
                } else if world_y >= self.config.sea_level - 8.0 {
                    // Transition layer - packed ice
                    return BlockType::PackedIce;
                }
            }
            return BlockType::Water;
        }
        
        // Above terrain - air, but check for snow cover in cold biomes
        if world_y > height {
            // Add snow layers on top of blocks in cold biomes
            if world_y <= height + 1.0 && world_y > height {
                if matches!(biome, Biome::Tundra | Biome::IcePlains | Biome::Taiga | Biome::SnowyMountains) {
                    // Snow layer on ground
                    if height >= self.config.sea_level {
                        return BlockType::Snow;
                    }
                }
            }
            return BlockType::Air;
        }
        
        // Check for caves
        if self.feature_gen.is_cave(world_x as f64, world_y as f64, world_z as f64) {
            return BlockType::Air;
        }
        
        // Surface blocks based on biome
        if world_y >= height - 1.0 && world_y <= height {
            return biome.surface_block();
        }
        
        // Subsurface blocks (dirt layer)
        if world_y >= height - 4.0 {
            return biome.subsurface_block();
        }
        
        // Deep underground - stone with ores
        let ore = self.feature_gen.get_ore(world_x as f64, world_y as f64, world_z as f64);
        ore.unwrap_or(BlockType::Stone)
    }
    
    /// Check if a position should have a cave
    pub fn is_cave(&self, world_x: f32, world_y: f32, world_z: f32) -> bool {
        self.feature_gen.is_cave(world_x as f64, world_y as f64, world_z as f64)
    }
    
    /// Get air temperature at a position in Fahrenheit
    pub fn get_air_temperature(&self, world_x: f32, world_y: f32, world_z: f32) -> f32 {
        // Use chunk center for consistent temperature within chunk
        let chunk_x = (world_x / 32.0).floor() * 32.0 + 16.0;
        let chunk_z = (world_z / 32.0).floor() * 32.0 + 16.0;
        
        // Wrap coordinates for seamless planet
        let wrapped_x = chunk_x.rem_euclid(self.config.planet_size as f32);
        let wrapped_z = chunk_z.rem_euclid(self.config.planet_size as f32);
        
        // Get base temperature from climate (0-1 range)
        let base_temp = self.terrain_gen.climate.get_temperature(wrapped_x as f64, wrapped_z as f64);
        
        // Convert to Fahrenheit (-40°F to 104°F range)
        // 0.0 = -40°F (polar), 1.0 = 104°F (equatorial)
        let mut temp_f = -40.0 + (base_temp * 144.0);
        
        // Apply altitude adjustment (about 3.5°F per 10 blocks above sea level)
        if world_y > self.config.sea_level {
            let altitude_above_sea = world_y - self.config.sea_level;
            temp_f -= altitude_above_sea * 0.35;
        }
        
        // Slight warming effect when underwater/near water
        if world_y < self.config.sea_level {
            temp_f += 5.0; // Water moderates temperature
        }
        
        // Clamp to reasonable range
        temp_f.clamp(-60.0, 120.0)
    }
}