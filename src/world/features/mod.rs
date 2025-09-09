pub mod caves;
pub mod ores;

use crate::world::WorldGenConfig;
use crate::block::BlockType;
use caves::CaveGenerator;
use ores::OreGenerator;

#[derive(Clone)]
pub struct FeatureGenerator {
    cave_gen: CaveGenerator,
    ore_gen: OreGenerator,
}

impl FeatureGenerator {
    pub fn new(seed: u64, config: &WorldGenConfig) -> Self {
        let cave_gen = CaveGenerator::new(seed, config);
        let ore_gen = OreGenerator::new(seed, config);
        
        Self {
            cave_gen,
            ore_gen,
        }
    }
    
    pub fn is_cave(&self, x: f64, y: f64, z: f64) -> bool {
        self.cave_gen.is_cave(x, y, z)
    }
    
    pub fn get_ore(&self, x: f64, y: f64, z: f64) -> Option<BlockType> {
        self.ore_gen.get_ore(x, y, z)
    }
}