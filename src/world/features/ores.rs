use noise::{NoiseFn, Perlin, Seedable};
use crate::block::BlockType;
use crate::world::WorldGenConfig;

#[derive(Clone)]
pub struct OreGenerator {
    coal_noise: Perlin,
    iron_noise: Perlin,
    gold_noise: Perlin,
    diamond_noise: Perlin,
    richness: f32,
}

impl OreGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        let coal_noise = Perlin::new((seed + 4000) as u32);
        let iron_noise = Perlin::new((seed + 4100) as u32);
        let gold_noise = Perlin::new((seed + 4200) as u32);
        let diamond_noise = Perlin::new((seed + 4300) as u32);
        
        Self {
            coal_noise,
            iron_noise,
            gold_noise,
            diamond_noise,
            richness: config.ore_richness,
        }
    }
    
    pub fn get_ore(&self, x: f64, y: f64, z: f64) -> Option<BlockType> {
        // Check each ore type based on depth
        
        // Diamond: Very rare, only below y=16
        if y <= 16.0 {
            if self.is_diamond_ore(x, y, z) {
                return Some(BlockType::Stone); // Will be Diamond ore when added
            }
        }
        
        // Gold: Rare, only below y=32
        if y <= 32.0 {
            if self.is_gold_ore(x, y, z) {
                return Some(BlockType::Sand); // Will be Gold ore when added
            }
        }
        
        // Iron: Common, below y=64
        if y <= 64.0 {
            if self.is_iron_ore(x, y, z) {
                return Some(BlockType::Dirt); // Will be Iron ore when added
            }
        }
        
        // Coal: Very common, below y=128
        if y <= 128.0 {
            if self.is_coal_ore(x, y, z) {
                return Some(BlockType::Stone); // Will be Coal ore when added
            }
        }
        
        None
    }
    
    fn is_coal_ore(&self, x: f64, y: f64, z: f64) -> bool {
        let scale = 0.1;
        let threshold = 0.85 - (self.richness * 0.1); // Lower threshold = more ore
        
        // Vein-like generation
        let vein_value = self.coal_noise.get([
            x * scale,
            y * scale * 0.5,
            z * scale,
        ]);
        
        // Small clusters
        let cluster_scale = 0.3;
        let cluster_value = self.coal_noise.get([
            x * cluster_scale + 1000.0,
            y * cluster_scale,
            z * cluster_scale + 1000.0,
        ]);
        
        (vein_value > threshold as f64) || (cluster_value > 0.9)
    }
    
    fn is_iron_ore(&self, x: f64, y: f64, z: f64) -> bool {
        let scale = 0.08;
        let threshold = 0.88 - (self.richness * 0.08);
        
        // More common at lower depths
        let depth_bonus = ((64.0 - y) / 64.0 * 0.1).max(0.0) as f32;
        let adjusted_threshold = threshold - depth_bonus;
        
        let vein_value = self.iron_noise.get([
            x * scale,
            y * scale * 0.5,
            z * scale,
        ]);
        
        let cluster_scale = 0.25;
        let cluster_value = self.iron_noise.get([
            x * cluster_scale + 2000.0,
            y * cluster_scale,
            z * cluster_scale + 2000.0,
        ]);
        
        (vein_value > adjusted_threshold as f64) || (cluster_value > 0.92)
    }
    
    fn is_gold_ore(&self, x: f64, y: f64, z: f64) -> bool {
        let scale = 0.06;
        let threshold = 0.92 - (self.richness * 0.05);
        
        // More common at lower depths
        let depth_bonus = ((32.0 - y) / 32.0 * 0.08).max(0.0) as f32;
        let adjusted_threshold = threshold - depth_bonus;
        
        let vein_value = self.gold_noise.get([
            x * scale,
            y * scale * 0.5,
            z * scale,
        ]);
        
        // Gold appears in smaller veins
        vein_value > adjusted_threshold as f64
    }
    
    fn is_diamond_ore(&self, x: f64, y: f64, z: f64) -> bool {
        let scale = 0.05;
        let threshold = 0.95 - (self.richness * 0.03);
        
        // More common at lower depths
        let depth_bonus = ((16.0 - y) / 16.0 * 0.05).max(0.0) as f32;
        let adjusted_threshold = threshold - depth_bonus;
        
        // Diamonds appear in small clusters
        let cluster_value = self.diamond_noise.get([
            x * scale,
            y * scale,
            z * scale,
        ]);
        
        // Very rare single blocks
        let single_scale = 0.15;
        let single_value = self.diamond_noise.get([
            x * single_scale + 3000.0,
            y * single_scale + 3000.0,
            z * single_scale + 3000.0,
        ]);
        
        (cluster_value > adjusted_threshold as f64) || (single_value > 0.98)
    }
}