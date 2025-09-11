use crate::block::BlockType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Hand,
    Pickaxe,
    Shovel,
    Axe,
}

impl Tool {
    /// Get the efficiency multiplier for extracting a specific block type
    pub fn efficiency_for(&self, block: BlockType) -> f32 {
        match (self, block) {
            // Pickaxe is best for stone-like blocks
            (Tool::Pickaxe, BlockType::Stone | BlockType::Cobblestone | BlockType::Ice | BlockType::PackedIce) => 3.0,
            
            // Shovel is best for soft blocks
            (Tool::Shovel, BlockType::Dirt | BlockType::Sand | BlockType::Grass | BlockType::Snow) => 3.0,
            
            // Axe is best for wood (when we have more wood types)
            (Tool::Axe, BlockType::Wood | BlockType::Planks) => 3.0,
            
            // Hand is baseline for everything
            (Tool::Hand, _) => 1.0,
            
            // Other tools on wrong materials are slightly better than hand
            _ => 1.2,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            Tool::Hand => "Hand",
            Tool::Pickaxe => "Pickaxe",
            Tool::Shovel => "Shovel",
            Tool::Axe => "Axe",
        }
    }
}