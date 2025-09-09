use crate::world::WorldGenConfig;

pub struct OceanGenerator {
    sea_level: f32,
}

impl OceanGenerator {
    pub fn new(_seed: u64, config: &WorldGenConfig) -> Self {
        Self {
            sea_level: config.sea_level,
        }
    }
    
    pub fn is_underwater(&self, height: f32) -> bool {
        height < self.sea_level
    }
    
    pub fn water_depth(&self, height: f32) -> f32 {
        if height >= self.sea_level {
            return 0.0;
        }
        self.sea_level - height
    }
}