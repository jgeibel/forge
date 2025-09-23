use crate::block::BlockType;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Biome {
    DeepOcean,
    Ocean,
    FrozenOcean,
    Beach,
    Desert,
    Savanna,
    TropicalRainforest,
    TemperateGrassland,
    TemperateForest,
    BorealForest,
    Tundra,
    Snow,
    Mountain,
    SnowyMountain,
    IceCap,
}

impl Biome {
    pub fn surface_block(&self) -> BlockType {
        match self {
            Biome::DeepOcean | Biome::Ocean => BlockType::Sand,
            Biome::FrozenOcean | Biome::IceCap => BlockType::Ice,
            Biome::Beach | Biome::Desert => BlockType::Sand,
            Biome::Savanna
            | Biome::TropicalRainforest
            | Biome::TemperateGrassland
            | Biome::TemperateForest
            | Biome::BorealForest => BlockType::Grass,
            Biome::Tundra | Biome::Snow | Biome::SnowyMountain => BlockType::Snow,
            Biome::Mountain => BlockType::Stone,
        }
    }

    pub fn subsurface_block(&self) -> BlockType {
        match self {
            Biome::DeepOcean | Biome::Ocean | Biome::Beach => BlockType::Sand,
            Biome::FrozenOcean | Biome::IceCap => BlockType::PackedIce,
            Biome::Desert => BlockType::Sand,
            Biome::Savanna
            | Biome::TropicalRainforest
            | Biome::TemperateGrassland
            | Biome::TemperateForest
            | Biome::BorealForest => BlockType::Dirt,
            Biome::Tundra | Biome::Snow | Biome::SnowyMountain => BlockType::PackedIce,
            Biome::Mountain => BlockType::Stone,
        }
    }
}
