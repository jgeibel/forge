use noise::{NoiseFn, Perlin, Seedable};
use crate::world::WorldGenConfig;

#[derive(Clone)]
pub struct CaveGenerator {
    cave_noise: Perlin,
    worm_noise: Perlin,
    density: f32,
    threshold: f64,
}

impl CaveGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        let cave_noise = Perlin::new((seed + 3000) as u32);
        let worm_noise = Perlin::new((seed + 3100) as u32);
        
        Self {
            cave_noise,
            worm_noise,
            density: config.cave_density,
            threshold: config.cave_threshold,
        }
    }
    
    pub fn is_cave(&self, x: f64, y: f64, z: f64) -> bool {
        // Don't generate caves in bedrock
        if y < 4.0 {
            return false;
        }
        
        // Different cave generation at different depths
        if y < 40.0 {
            // Deep caves - larger and more interconnected
            self.is_deep_cave(x, y, z)
        } else {
            // Surface caves - smaller and more isolated
            self.is_surface_cave(x, y, z)
        }
    }
    
    fn is_surface_cave(&self, x: f64, y: f64, z: f64) -> bool {
        // 3D noise for cave density
        let scale = 0.05;
        let noise_value = self.cave_noise.get([
            x * scale,
            y * scale * 2.0, // Stretch caves vertically
            z * scale,
        ]);
        
        // Add worm-like caves
        let worm_scale = 0.03;
        let worm_value = self.worm_noise.get([
            x * worm_scale,
            y * worm_scale,
            z * worm_scale,
        ]);
        
        // Combine noise values
        let combined = noise_value * 0.7 + worm_value.abs() * 0.3;
        
        // Higher threshold for surface caves (smaller caves)
        combined > self.threshold + 0.1
    }
    
    fn is_deep_cave(&self, x: f64, y: f64, z: f64) -> bool {
        // Larger scale for bigger caves
        let scale = 0.03;
        let noise_value = self.cave_noise.get([
            x * scale,
            y * scale * 1.5,
            z * scale,
        ]);
        
        // Add large caverns using different frequency
        let cavern_scale = 0.015;
        let cavern_value = self.cave_noise.get([
            x * cavern_scale + 1000.0,
            y * cavern_scale * 0.5, // Flatten caverns
            z * cavern_scale + 1000.0,
        ]);
        
        // Worm caves for connections
        let worm_scale = 0.025;
        let worm_value = self.worm_noise.get([
            x * worm_scale,
            y * worm_scale * 2.0,
            z * worm_scale,
        ]);
        
        // Combine different cave types
        let combined = noise_value * 0.5 + cavern_value * 0.3 + worm_value.abs() * 0.2;
        
        // Lower threshold for deep caves (larger caves)
        // Also increase cave frequency at lower depths
        let depth_factor = (1.0 - (y / 40.0)).max(0.0);
        let adjusted_threshold = self.threshold - (depth_factor * 0.1);
        
        combined > adjusted_threshold
    }
    
    /// Check if a position should have a lava pool (very deep only)
    pub fn is_lava_pool(&self, x: f64, y: f64, z: f64) -> bool {
        if y > 10.0 {
            return false;
        }
        
        // Use different noise for lava pools
        let scale = 0.02;
        let noise_value = self.cave_noise.get([
            x * scale + 5000.0,
            0.0,
            z * scale + 5000.0,
        ]);
        
        // Very rare lava pools
        noise_value > 0.8
    }
}