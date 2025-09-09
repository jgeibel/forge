use crate::world::noise_layers::{NoiseLayer, LayeredNoise};
use crate::world::WorldGenConfig;

#[derive(Clone)]
pub struct HeightmapGenerator {
    terrain_noise: LayeredNoise,
    mountain_noise: LayeredNoise,
    sea_level: f32,
    mountain_scale: f32,
}

impl HeightmapGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        // Create layered noise for general terrain
        let mut terrain_noise = LayeredNoise::new();
        
        // Base terrain - rolling hills
        terrain_noise = terrain_noise.add_layer(
            NoiseLayer::new_fbm(
                (seed + 300) as u32,
                config.mountain_frequency * 0.5,
                20.0,
                4,
            )
        );
        
        // Medium detail
        terrain_noise = terrain_noise.add_layer(
            NoiseLayer::new_simplex(
                (seed + 400) as u32,
                config.mountain_frequency,
                10.0,
            )
        );
        
        // Fine detail
        terrain_noise = terrain_noise.add_layer(
            NoiseLayer::new_perlin(
                (seed + 500) as u32,
                config.mountain_frequency * 4.0,
                3.0,
            )
        );
        
        // Mountain ridges - using ridged noise for mountain ranges
        let mut mountain_noise = LayeredNoise::new();
        
        mountain_noise = mountain_noise.add_layer(
            NoiseLayer::new_ridged(
                (seed + 600) as u32,
                config.mountain_frequency * 0.3,
                1.0,
                4,
            )
        );
        
        // Add some variation to mountains
        mountain_noise = mountain_noise.add_layer(
            NoiseLayer::new_billow(
                (seed + 700) as u32,
                config.mountain_frequency * 0.8,
                0.3,
                2,
            )
        );
        
        Self {
            terrain_noise,
            mountain_noise,
            sea_level: config.sea_level,
            mountain_scale: config.mountain_scale,
        }
    }
    
    pub fn get_height(&self, x: f64, z: f64, continental_value: f32) -> f32 {
        if continental_value < -0.1 {
            // Deep ocean
            self.get_ocean_floor(x, z, continental_value)
        } else if continental_value < 0.0 {
            // Shallow ocean / continental shelf
            self.get_shallow_ocean(x, z, continental_value)
        } else if continental_value < 0.2 {
            // Coastal areas
            self.get_coastal_height(x, z, continental_value)
        } else {
            // Inland areas
            self.get_land_height(x, z, continental_value)
        }
    }
    
    fn get_ocean_floor(&self, x: f64, z: f64, continental_value: f32) -> f32 {
        // Deep ocean floor with some variation
        let base_depth = self.sea_level - 32.0;
        let variation = self.terrain_noise.sample_2d(x * 0.5, z * 0.5) * 0.3; // Less variation in ocean
        
        // Deeper oceans for more negative continental values
        let depth_factor = (-continental_value - 0.1).min(1.0);
        base_depth - (depth_factor * 16.0) + variation as f32
    }
    
    fn get_shallow_ocean(&self, x: f64, z: f64, continental_value: f32) -> f32 {
        // Continental shelf - gradual slope to shore
        let ocean_depth = self.sea_level - 16.0;
        let shore_height = self.sea_level - 2.0;
        
        // Interpolate between ocean and shore
        let t = (continental_value + 0.1) / 0.1; // Map -0.1 to 0 => 0 to 1
        let base = ocean_depth + (shore_height - ocean_depth) * t;
        
        // Add some variation
        let variation = self.terrain_noise.sample_2d(x * 0.7, z * 0.7) * 0.2;
        base + variation as f32
    }
    
    fn get_coastal_height(&self, x: f64, z: f64, continental_value: f32) -> f32 {
        // Beaches and coastal plains
        let beach_height = self.sea_level + 2.0;
        let plains_height = self.sea_level + 8.0;
        
        // Interpolate from beach to plains
        let t = continental_value / 0.2;
        let base = beach_height + (plains_height - beach_height) * t;
        
        // Minimal variation for beaches
        let variation = self.terrain_noise.sample_2d(x, z) * 0.15;
        base + variation as f32
    }
    
    fn get_land_height(&self, x: f64, z: f64, continental_value: f32) -> f32 {
        // Base terrain height
        let base_height = self.sea_level + 16.0;
        
        // Rolling hills from terrain noise
        let terrain = self.terrain_noise.sample_2d(x, z) as f32;
        
        // Mountain ranges
        let mountain_strength = ((continental_value - 0.2) * 2.0).min(1.0);
        let mountains = self.mountain_noise.sample_2d(x, z) as f32;
        
        // Only create mountains where the noise is positive
        let mountain_height = if mountains > 0.0 {
            mountains * self.mountain_scale * mountain_strength
        } else {
            0.0
        };
        
        // Combine base, terrain, and mountains
        base_height + terrain + mountain_height
    }
}