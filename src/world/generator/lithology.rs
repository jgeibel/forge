use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::block::BlockType;
use crate::world::config::WorldGenConfig;

use super::plates::{PlateInfo, PlateMap};

#[derive(Clone, Debug)]
pub struct LithologyLayer {
    pub block: BlockType,
    pub thickness: u8,
}

#[derive(Clone, Debug)]
pub struct LithologyProfile {
    pub surface_block: BlockType,
    pub surface_depth: u8,
    pub strata: Vec<LithologyLayer>,
    pub basement_block: BlockType,
    pub cave_bias: f32,
    pub ore_bias: f32,
}

pub fn generate_plate_lithology<F>(
    config: &WorldGenConfig,
    plate_map: &PlateMap,
    mut sample_height: F,
) -> Vec<LithologyProfile>
where
    F: FnMut(f32, f32) -> (f32, f32),
{
    let mut rng = StdRng::seed_from_u64(config.seed.wrapping_add(0x9E3779B97F4A7C15));
    plate_map
        .plates
        .iter()
        .map(|plate| build_profile(config, plate, &mut rng, &mut sample_height))
        .collect()
}

fn build_profile<F>(
    config: &WorldGenConfig,
    plate: &PlateInfo,
    rng: &mut StdRng,
    sample_height: &mut F,
) -> LithologyProfile
where
    F: FnMut(f32, f32) -> (f32, f32),
{
    let planet_size = config.planet_size as f32;
    let u = plate.centroid.x.rem_euclid(1.0);
    let v = plate.centroid.y.rem_euclid(1.0);
    let world_x = u * planet_size;
    let world_z = v * planet_size;

    let (height, water_level) = sample_height(world_x, world_z);
    let is_land = height >= water_level - 1.0;

    if is_land {
        continental_profile(rng)
    } else {
        oceanic_profile(rng)
    }
}

fn continental_profile(rng: &mut StdRng) -> LithologyProfile {
    let surface_depth = rng.gen_range(2..=5);
    let regolith_thickness = rng.gen_range(4..=9);
    let sedimentary_thickness = rng.gen_range(24..=40);
    let metamorphic_thickness = rng.gen_range(16..=28);

    LithologyProfile {
        surface_block: BlockType::Grass,
        surface_depth: surface_depth as u8,
        strata: vec![
            LithologyLayer {
                block: BlockType::Dirt,
                thickness: regolith_thickness as u8,
            },
            LithologyLayer {
                block: BlockType::Stone,
                thickness: sedimentary_thickness as u8,
            },
            LithologyLayer {
                block: BlockType::Cobblestone,
                thickness: metamorphic_thickness as u8,
            },
        ],
        basement_block: BlockType::Stone,
        cave_bias: rng.gen_range(0.45..0.75),
        ore_bias: rng.gen_range(0.6..0.95),
    }
}

fn oceanic_profile(rng: &mut StdRng) -> LithologyProfile {
    let surface_depth = rng.gen_range(1..=3);
    let sediment_thickness = rng.gen_range(3..=8);
    let basalt_thickness = rng.gen_range(24..=36);

    LithologyProfile {
        surface_block: BlockType::Sand,
        surface_depth: surface_depth as u8,
        strata: vec![
            LithologyLayer {
                block: BlockType::Sand,
                thickness: sediment_thickness as u8,
            },
            LithologyLayer {
                block: BlockType::Stone,
                thickness: basalt_thickness as u8,
            },
        ],
        basement_block: BlockType::Bedrock,
        cave_bias: rng.gen_range(0.2..0.5),
        ore_bias: rng.gen_range(0.4..0.7),
    }
}
