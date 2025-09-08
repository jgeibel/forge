#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlockType {
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
    Wood = 4,
    Leaves = 5,
    Sand = 6,
    Water = 7,
    Cobblestone = 8,
    Planks = 9,
    Bedrock = 10,  // Unbreakable planet core
}

impl BlockType {
    pub fn is_solid(&self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water)
    }
    
    pub fn is_breakable(&self) -> bool {
        !matches!(self, BlockType::Bedrock)
    }
    
    pub fn is_transparent(&self) -> bool {
        matches!(self, BlockType::Air | BlockType::Water | BlockType::Leaves)
    }
    
    pub fn is_visible(&self) -> bool {
        !matches!(self, BlockType::Air)
    }
    
    pub fn get_texture_name(&self) -> &str {
        match self {
            BlockType::Air => "air",
            BlockType::Stone => "stone",
            BlockType::Dirt => "dirt",
            BlockType::Grass => "grass",
            BlockType::Wood => "wood",
            BlockType::Leaves => "leaves",
            BlockType::Sand => "sand",
            BlockType::Water => "water",
            BlockType::Cobblestone => "cobblestone",
            BlockType::Planks => "planks",
            BlockType::Bedrock => "bedrock",
        }
    }
    
    pub fn get_texture_indices(&self) -> [u32; 6] {
        match self {
            BlockType::Air => [0; 6],
            BlockType::Stone => [1, 1, 1, 1, 1, 1],
            BlockType::Dirt => [2, 2, 2, 2, 2, 2],
            BlockType::Grass => [0, 2, 3, 3, 3, 3], 
            BlockType::Wood => [5, 5, 4, 4, 4, 4],
            BlockType::Leaves => [6, 6, 6, 6, 6, 6],
            BlockType::Sand => [7, 7, 7, 7, 7, 7],
            BlockType::Water => [8, 8, 8, 8, 8, 8],
            BlockType::Cobblestone => [9, 9, 9, 9, 9, 9],
            BlockType::Planks => [10, 10, 10, 10, 10, 10],
            BlockType::Bedrock => [11, 11, 11, 11, 11, 11],
        }
    }
    
    pub fn get_color(&self) -> [f32; 4] {
        match self {
            BlockType::Grass => [0.5, 0.8, 0.3, 1.0],
            BlockType::Dirt => [0.5, 0.35, 0.2, 1.0],
            BlockType::Stone => [0.5, 0.5, 0.5, 1.0],
            BlockType::Wood => [0.6, 0.4, 0.2, 1.0],
            BlockType::Leaves => [0.2, 0.6, 0.2, 1.0],
            BlockType::Sand => [0.9, 0.8, 0.6, 1.0],
            BlockType::Water => [0.2, 0.4, 0.8, 0.8],
            BlockType::Cobblestone => [0.4, 0.4, 0.4, 1.0],
            BlockType::Planks => [0.7, 0.5, 0.3, 1.0],
            BlockType::Bedrock => [0.1, 0.1, 0.1, 1.0],
            _ => [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl Default for BlockType {
    fn default() -> Self {
        BlockType::Air
    }
}