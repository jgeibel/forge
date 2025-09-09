use crate::world::noise_layers::{NoiseLayer, LayeredNoise};
use crate::world::WorldGenConfig;

#[derive(Clone)]
pub struct ContinentalGenerator {
    noise: LayeredNoise,
    sea_level: f32,
}

impl ContinentalGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        // Create layered noise for continental shelves
        let mut noise = LayeredNoise::new();
        
        // Primary continental layer - very large scale features
        noise = noise.add_layer(
            NoiseLayer::new_fbm(
                seed as u32,
                config.continental_frequency,
                1.0,
                config.continental_octaves,
            )
        );
        
        // Secondary variation - medium scale
        noise = noise.add_layer(
            NoiseLayer::new_simplex(
                (seed + 100) as u32,
                config.continental_frequency * 3.0,
                0.3,
            )
        );
        
        // Small scale detail
        noise = noise.add_layer(
            NoiseLayer::new_perlin(
                (seed + 200) as u32,
                config.continental_frequency * 10.0,
                0.1,
            )
        );
        
        Self {
            noise,
            sea_level: config.sea_level,
        }
    }
    
    /// Returns a value between -1 and 1 indicating continental shelf
    /// Values > 0 are land, values < 0 are ocean
    pub fn get_value(&self, x: f64, z: f64) -> f32 {
        self.noise.sample_2d(x, z) as f32
    }
    
    /// Returns true if the position is on continental shelf (land)
    pub fn is_land(&self, x: f64, z: f64) -> bool {
        self.get_value(x, z) > 0.0
    }
    
    /// Get the ocean depth multiplier (0 = shore, 1 = deep ocean)
    pub fn ocean_depth_factor(&self, x: f64, z: f64) -> f32 {
        let value = self.get_value(x, z);
        if value >= 0.0 {
            return 0.0;
        }
        
        // Map negative values to ocean depth
        (-value).min(1.0)
    }
    
    /// Get the land height multiplier (0 = shore, 1 = inland)
    pub fn land_height_factor(&self, x: f64, z: f64) -> f32 {
        let value = self.get_value(x, z);
        if value <= 0.0 {
            return 0.0;
        }
        
        // Map positive values to land height
        value.min(1.0)
    }
}