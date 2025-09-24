use bevy::math::Vec2;

use super::super::{
    util::{lerp_color, lerp_f32},
    WorldGenerator,
};
use crate::block::BlockType;
use crate::chunk::{ChunkPos, ChunkStorage};
use crate::world::biome::Biome;

impl WorldGenerator {
    pub fn get_biome(&self, world_x: f32, world_z: f32) -> Biome {
        let height = self.get_height(world_x, world_z);
        let temperature_c = self.temperature_at_height(world_x, world_z, height);
        let moisture = self.get_moisture(world_x, world_z);

        self.classify_biome_at_position(world_x, world_z, height, temperature_c, moisture)
    }

    pub fn get_block(&self, world_x: f32, world_y: f32, world_z: f32) -> BlockType {
        if world_y < 2.0 {
            return BlockType::Bedrock;
        }

        let height = self.get_height(world_x, world_z);
        let biome = self.get_biome(world_x, world_z);
        let water_surface = self.get_water_level(world_x, world_z);

        if world_y as f32 > height {
            if (world_y as f32) <= water_surface {
                return match biome {
                    Biome::FrozenOcean | Biome::IceCap => BlockType::Ice,
                    _ => BlockType::Water,
                };
            }
            return BlockType::Air;
        }

        if world_y >= height - 1.0 {
            return biome.surface_block();
        }

        if world_y >= height - 4.0 {
            return biome.subsurface_block();
        }

        BlockType::Stone
    }

    pub fn bake_chunk(&self, chunk_pos: ChunkPos) -> ChunkStorage {
        let world_origin = chunk_pos.to_world_pos();
        ChunkStorage::from_fn(|x, y, z| {
            let world_x = world_origin.x + x as f32;
            let world_y = world_origin.y + y as f32;
            let world_z = world_origin.z + z as f32;
            self.get_block(world_x, world_y, world_z)
        })
    }

    pub fn preview_color(&self, world_x: f32, world_z: f32, biome: Biome, height: f32) -> [u8; 4] {
        let sea_level = self.config.sea_level;
        let water_depth = (sea_level - height).max(0.0);

        let base = match biome {
            Biome::DeepOcean => {
                let t = (water_depth / self.config.deep_ocean_depth).clamp(0.0, 1.0);
                lerp_color([12, 36, 92], [2, 9, 28], t)
            }
            Biome::Ocean => {
                let t = (water_depth / self.config.ocean_depth).clamp(0.0, 1.0);
                lerp_color([30, 90, 180], [8, 48, 128], t)
            }
            Biome::FrozenOcean | Biome::IceCap => [210, 230, 240],
            Biome::Beach => [216, 200, 160],
            Biome::Desert => [236, 212, 120],
            Biome::Savanna => [198, 182, 96],
            Biome::TropicalRainforest => [44, 118, 56],
            Biome::TemperateGrassland => [100, 176, 80],
            Biome::TemperateForest => [70, 140, 72],
            Biome::BorealForest => [60, 120, 104],
            Biome::Tundra => [150, 160, 150],
            Biome::Snow => [240, 240, 245],
            Biome::Mountain => [130, 130, 130],
            Biome::SnowyMountain => [232, 236, 242],
        };

        let min_height = sea_level - self.config.deep_ocean_depth;
        let max_height = sea_level + self.config.mountain_height + 64.0;
        let normalized = ((height - min_height) / (max_height - min_height)).clamp(0.0, 1.0);
        let shade = 0.6 + normalized * 0.4;

        let [r, g, b] = base;
        let mut color = [
            ((r as f32) * shade).min(255.0) as u8,
            ((g as f32) * shade).min(255.0) as u8,
            ((b as f32) * shade).min(255.0) as u8,
            255,
        ];

        let river_intensity = self.river_intensity(world_x, world_z);
        let major_river = self.major_river_factor(world_x, world_z);

        if major_river > 0.1 {
            color[0] = 5;
            color[1] = 30;
            color[2] = 100;
        } else if river_intensity > 0.02 {
            let river_color = [20.0, 90.0, 210.0];
            let blend = river_intensity.clamp(0.0, 1.0);
            color[0] = lerp_f32(color[0] as f32, river_color[0], blend) as u8;
            color[1] = lerp_f32(color[1] as f32, river_color[1], blend) as u8;
            color[2] = lerp_f32(color[2] as f32, river_color[2], blend) as u8;
        }

        color
    }

    fn classify_beach_biome(
        &self,
        world_x: f32,
        world_z: f32,
        height: f32,
        temp_c: f32,
    ) -> Option<Biome> {
        let sea_level = self.config.sea_level;
        let elevation_above_sea = height - sea_level;

        if elevation_above_sea < -2.0 || elevation_above_sea > 12.0 {
            return None;
        }

        let components = self.terrain_components(world_x, world_z);
        let hydro = self.sample_hydrology(world_x, world_z, components.base_height);

        let coastal_factor = hydro.coastal_factor;

        if coastal_factor < 0.15 {
            return None;
        }

        if hydro.river_intensity > 0.12 || hydro.lake_intensity > 0.12 {
            return None;
        }

        if hydro.water_level - components.base_height > 6.0 {
            return None;
        }

        let (distance_to_water, avg_slope) =
            self.calculate_coastal_properties(world_x, world_z, height);

        if distance_to_water > 120.0 {
            return None;
        }

        let slope_factor = (1.0 - avg_slope.min(0.8) / 0.8).max(0.0);
        let elevation_factor = if elevation_above_sea < 1.0 {
            1.0
        } else if elevation_above_sea < 6.0 {
            (6.0 - elevation_above_sea) / 5.0
        } else {
            0.0
        };
        let base_probability = slope_factor * elevation_factor * (0.4 + 0.6 * coastal_factor);

        if base_probability <= 0.02 {
            return None;
        }

        let beach_probability = self.calculate_beach_probability(world_x, world_z, slope_factor)
            * (0.4 + 0.6 * coastal_factor);
        if beach_probability < 0.08 {
            return None;
        }

        let temp_adjustment = if temp_c < -5.0 {
            Biome::FrozenOcean
        } else if temp_c < 2.0 {
            Biome::Snow
        } else if temp_c < 8.0 {
            Biome::TemperateGrassland
        } else if temp_c < 16.0 {
            Biome::TemperateForest
        } else {
            Biome::Beach
        };

        Some(temp_adjustment)
    }

    fn classify_biome_at_position(
        &self,
        world_x: f32,
        world_z: f32,
        height: f32,
        temp_c: f32,
        moisture: f32,
    ) -> Biome {
        let sea_level = self.config.sea_level;
        let deep_ocean_cutoff = sea_level - self.config.deep_ocean_depth;
        let shallow_ocean_cutoff = sea_level - 1.5;

        if height < deep_ocean_cutoff {
            return if temp_c <= -2.0 {
                Biome::FrozenOcean
            } else {
                Biome::DeepOcean
            };
        }

        if height < shallow_ocean_cutoff {
            return if temp_c <= -2.0 {
                Biome::FrozenOcean
            } else {
                Biome::Ocean
            };
        }

        if let Some(beach_biome) = self.classify_beach_biome(world_x, world_z, height, temp_c) {
            return beach_biome;
        }

        let elevation = height - sea_level;
        let mountain_limit = self.config.highland_bonus * 0.6 + self.config.mountain_height * 0.35;

        if elevation > mountain_limit {
            return if temp_c < -5.0 {
                Biome::SnowyMountain
            } else {
                Biome::Mountain
            };
        }

        if temp_c < -15.0 {
            return Biome::IceCap;
        }
        if temp_c < -5.0 {
            return Biome::Snow;
        }
        if temp_c < 0.0 {
            return Biome::Tundra;
        }

        if temp_c < 8.0 {
            return if moisture < 0.35 {
                Biome::BorealForest
            } else {
                Biome::TemperateForest
            };
        }

        if temp_c < 18.0 {
            if moisture < 0.25 {
                return Biome::TemperateGrassland;
            } else if moisture < 0.6 {
                return Biome::TemperateForest;
            } else {
                return Biome::TropicalRainforest;
            }
        }

        if temp_c < 26.0 {
            if moisture < 0.2 {
                return Biome::Desert;
            } else if moisture < 0.45 {
                return Biome::Savanna;
            } else {
                return Biome::TropicalRainforest;
            }
        }

        if moisture < 0.15 {
            Biome::Desert
        } else if moisture < 0.45 {
            Biome::Savanna
        } else {
            Biome::TropicalRainforest
        }
    }

    fn calculate_coastal_properties(&self, world_x: f32, world_z: f32, height: f32) -> (f32, f32) {
        let sea_level = self.config.sea_level;
        let mut min_distance = f32::MAX;
        let mut height_samples = Vec::new();

        for radius in [10.0, 20.0, 40.0, 80.0, 120.0, 160.0] {
            let sample_count = 16;
            for i in 0..sample_count {
                let angle = (i as f32) * std::f32::consts::TAU / (sample_count as f32);
                let offset = Vec2::new(angle.cos(), angle.sin()) * radius;
                let sample_height = self.get_height(world_x + offset.x, world_z + offset.y);
                height_samples.push(sample_height);

                if sample_height < sea_level {
                    min_distance = min_distance.min(radius);
                    break;
                }
            }
        }

        if min_distance == f32::MAX {
            min_distance = 200.0;
        }

        let mut total_slope = 0.0;
        let mut sample_count = 0;
        for sample_height in height_samples {
            let height_diff = (sample_height - height).abs();
            let distance = 20.0;
            if distance > 0.0 {
                total_slope += height_diff / distance;
                sample_count += 1;
            }
        }

        let avg_slope = if sample_count > 0 {
            total_slope / sample_count as f32
        } else {
            0.0
        };

        (min_distance, avg_slope)
    }

    fn calculate_beach_probability(&self, world_x: f32, world_z: f32, slope_factor: f32) -> f32 {
        let (u, v) = self.normalized_uv(world_x, world_z);
        let beach_noise = self.fractal_periodic(
            &self.detail_noise,
            u,
            v,
            self.config.detail_frequency * 0.5,
            2,
            2.0,
            0.5,
        ) as f32;

        let base_probability = slope_factor * 0.85;
        let noise_influence = (beach_noise + 1.0) * 0.5 * 0.3;

        (base_probability + noise_influence).clamp(0.0, 1.0)
    }
}
