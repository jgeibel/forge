pub mod continental;
pub mod heightmap;

use crate::world::noise_layers::{NoiseLayer, LayeredNoise};
use crate::world::biomes::{Biome, ClimateMap};
use crate::world::WorldGenConfig;

#[derive(Clone)]
pub struct TerrainGenerator {
    continental: continental::ContinentalGenerator,
    heightmap: heightmap::HeightmapGenerator,
    pub climate: ClimateMap,
    sea_level: f32,
}

impl TerrainGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        let continental = continental::ContinentalGenerator::new(seed, config);
        let heightmap = heightmap::HeightmapGenerator::new(seed, config);
        let climate = ClimateMap::new(seed, config.planet_size as f32);
        
        Self {
            continental,
            heightmap,
            climate,
            sea_level: config.sea_level,
        }
    }
    
    pub fn get_height(&self, x: f64, z: f64) -> f32 {
        // Get continental shelf value (determines if land or ocean)
        let continental_value = self.continental.get_value(x, z);
        
        // Get detailed height from heightmap
        let detail_height = self.heightmap.get_height(x, z, continental_value);
        
        detail_height
    }
    
    pub fn get_biome(&self, x: f64, z: f64, height: f32) -> Biome {
        let temperature = self.climate.get_temperature(x, z);
        let temperature = self.climate.adjust_temperature_for_altitude(temperature, height, self.sea_level);
        
        // Simple distance to water calculation (will be improved)
        let distance_to_water = if height < self.sea_level { 0.0 } else { (height - self.sea_level).min(50.0) };
        let moisture = self.climate.get_moisture(x, z, distance_to_water);
        
        Biome::from_climate(temperature, moisture, height, self.sea_level)
    }
}