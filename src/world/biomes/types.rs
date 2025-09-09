use crate::block::BlockType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Biome {
    Ocean,
    DeepOcean,
    Beach,
    Plains,
    Forest,
    Desert,
    Mountains,
    SnowyMountains,
    Tundra,
    Jungle,
    Swamp,
    Savanna,
    Mesa,
    IceSpikes,
    FrozenOcean,
    IcePlains,
    Taiga,
}

pub struct BiomeProperties {
    pub surface_block: BlockType,
    pub subsurface_block: BlockType,
    pub underwater_block: BlockType,
    pub tree_density: f32,
    pub grass_density: f32,
    pub temperature: f32,
    pub moisture: f32,
}

impl Biome {
    pub fn surface_block(&self) -> BlockType {
        match self {
            Biome::Ocean | Biome::DeepOcean => BlockType::Sand,
            Biome::FrozenOcean => BlockType::Ice,
            Biome::Beach => BlockType::Sand,
            Biome::Desert | Biome::Mesa => BlockType::Sand,
            Biome::Mountains => BlockType::Stone,
            Biome::SnowyMountains => BlockType::Snow,
            Biome::Tundra | Biome::IcePlains => BlockType::Snow,
            Biome::IceSpikes => BlockType::PackedIce,
            Biome::Taiga => BlockType::Grass,
            _ => BlockType::Grass,
        }
    }
    
    pub fn subsurface_block(&self) -> BlockType {
        match self {
            Biome::Ocean | Biome::DeepOcean | Biome::Beach => BlockType::Sand,
            Biome::FrozenOcean => BlockType::PackedIce,
            Biome::Desert => BlockType::Sand,
            Biome::Mesa => BlockType::Stone, // Will be terracotta in future
            Biome::Mountains | Biome::SnowyMountains => BlockType::Stone,
            Biome::IcePlains | Biome::IceSpikes => BlockType::PackedIce,
            Biome::Tundra => BlockType::Stone,
            _ => BlockType::Dirt,
        }
    }
    
    pub fn from_climate(temperature: f32, moisture: f32, height: f32, sea_level: f32) -> Self {
        // Ocean biomes - check temperature for frozen oceans
        if height < sea_level {
            if temperature < 0.1 {
                return Biome::FrozenOcean;
            }
            if height < sea_level - 20.0 {
                return Biome::DeepOcean;
            }
            return Biome::Ocean;
        }
        
        // Beach - no beaches in frozen regions
        if height < sea_level + 3.0 {
            if temperature < 0.15 {
                return Biome::IcePlains;
            }
            return Biome::Beach;
        }
        
        // Mountain biomes (height-based)
        if height > sea_level + 80.0 {
            if temperature < 0.3 {
                return Biome::SnowyMountains;
            }
            return Biome::Mountains;
        }
        
        // Temperature and moisture based biomes
        match (temperature, moisture) {
            // Polar biomes (very cold)
            (t, m) if t < 0.15 => {
                match m {
                    m if m < 0.3 => Biome::IcePlains,
                    _ => Biome::IceSpikes,
                }
            },
            
            // Sub-polar/Arctic biomes
            (t, m) if t < 0.35 => {
                match m {
                    m if m < 0.3 => Biome::Tundra,
                    m if m < 0.6 => Biome::Taiga,
                    _ => Biome::IcePlains,
                }
            },
            
            // Temperate biomes
            (t, m) if t < 0.65 => {
                match m {
                    m if m < 0.3 => Biome::Plains,
                    m if m < 0.7 => Biome::Forest,
                    _ => Biome::Swamp,
                }
            },
            
            // Warm/Hot biomes
            _ => {
                match moisture {
                    m if m < 0.2 => Biome::Desert,
                    m if m < 0.4 => Biome::Savanna,
                    m if m < 0.6 => Biome::Plains,
                    m if m < 0.8 => Biome::Forest,
                    _ => Biome::Jungle,
                }
            }
        }
    }
    
    pub fn properties(&self) -> BiomeProperties {
        match self {
            Biome::Ocean => BiomeProperties {
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Sand,
                underwater_block: BlockType::Sand,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.5,
                moisture: 1.0,
            },
            Biome::DeepOcean => BiomeProperties {
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Sand,
                underwater_block: BlockType::Stone,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.4,
                moisture: 1.0,
            },
            Biome::Beach => BiomeProperties {
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Sand,
                underwater_block: BlockType::Sand,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.7,
                moisture: 0.4,
            },
            Biome::Plains => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.01,
                grass_density: 0.3,
                temperature: 0.6,
                moisture: 0.4,
            },
            Biome::Forest => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.15,
                grass_density: 0.2,
                temperature: 0.5,
                moisture: 0.6,
            },
            Biome::Desert => BiomeProperties {
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Sand,
                underwater_block: BlockType::Sand,
                tree_density: 0.001,
                grass_density: 0.01,
                temperature: 0.9,
                moisture: 0.1,
            },
            Biome::Mountains => BiomeProperties {
                surface_block: BlockType::Stone,
                subsurface_block: BlockType::Stone,
                underwater_block: BlockType::Stone,
                tree_density: 0.02,
                grass_density: 0.05,
                temperature: 0.3,
                moisture: 0.3,
            },
            Biome::SnowyMountains => BiomeProperties {
                surface_block: BlockType::Stone,
                subsurface_block: BlockType::Stone,
                underwater_block: BlockType::Stone,
                tree_density: 0.01,
                grass_density: 0.0,
                temperature: 0.0,
                moisture: 0.4,
            },
            Biome::Tundra => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.0,
                grass_density: 0.1,
                temperature: 0.1,
                moisture: 0.3,
            },
            Biome::Jungle => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.3,
                grass_density: 0.4,
                temperature: 0.85,
                moisture: 0.9,
            },
            Biome::Swamp => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.08,
                grass_density: 0.15,
                temperature: 0.6,
                moisture: 0.9,
            },
            Biome::Savanna => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.02,
                grass_density: 0.25,
                temperature: 0.8,
                moisture: 0.3,
            },
            Biome::Mesa => BiomeProperties {
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Stone,
                underwater_block: BlockType::Stone,
                tree_density: 0.0,
                grass_density: 0.02,
                temperature: 0.9,
                moisture: 0.2,
            },
            Biome::IceSpikes => BiomeProperties {
                surface_block: BlockType::PackedIce,
                subsurface_block: BlockType::PackedIce,
                underwater_block: BlockType::PackedIce,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.0,
                moisture: 0.5,
            },
            Biome::FrozenOcean => BiomeProperties {
                surface_block: BlockType::Ice,
                subsurface_block: BlockType::PackedIce,
                underwater_block: BlockType::PackedIce,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.0,
                moisture: 1.0,
            },
            Biome::IcePlains => BiomeProperties {
                surface_block: BlockType::Snow,
                subsurface_block: BlockType::PackedIce,
                underwater_block: BlockType::PackedIce,
                tree_density: 0.0,
                grass_density: 0.0,
                temperature: 0.0,
                moisture: 0.3,
            },
            Biome::Taiga => BiomeProperties {
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                underwater_block: BlockType::Dirt,
                tree_density: 0.2,
                grass_density: 0.1,
                temperature: 0.25,
                moisture: 0.5,
            },
        }
    }
}