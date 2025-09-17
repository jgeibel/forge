use bevy::prelude::*;

use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::PlanetConfig;

/// Resource tracking the current air temperature at player position.
#[derive(Resource, Default)]
pub struct CurrentTemperature {
    pub fahrenheit: f32,
    pub celsius: f32,
    last_chunk_x: i32,
    last_chunk_z: i32,
}

impl CurrentTemperature {
    pub fn new() -> Self {
        Self {
            fahrenheit: 70.0,
            celsius: 21.0,
            last_chunk_x: i32::MAX,
            last_chunk_z: i32::MAX,
        }
    }

    pub fn update(&mut self, fahrenheit: f32) {
        self.fahrenheit = fahrenheit;
        self.celsius = (fahrenheit - 32.0) * 5.0 / 9.0;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Biome {
    TemperatePlains,
    ShallowWater,
    DeepWater,
}

impl Biome {
    pub fn surface_block(&self) -> BlockType {
        match self {
            Biome::TemperatePlains => BlockType::Grass,
            Biome::ShallowWater => BlockType::Sand,
            Biome::DeepWater => BlockType::Sand,
        }
    }

    pub fn subsurface_block(&self) -> BlockType {
        match self {
            Biome::TemperatePlains => BlockType::Dirt,
            Biome::ShallowWater => BlockType::Sand,
            Biome::DeepWater => BlockType::Sand,
        }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct WorldGenConfig {
    pub seed: u64,
    pub planet_size: u32,
    pub sea_level: f32,
    pub ground_level: f32,
}

impl Default for WorldGenConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            planet_size: 16384,
            sea_level: 64.0,
            ground_level: 72.0,
        }
    }
}

impl WorldGenConfig {
    pub fn from_planet_config(config: &PlanetConfig) -> Self {
        Self {
            seed: config.seed,
            planet_size: config.size_chunks as u32 * 32,
            sea_level: config.sea_level,
            ground_level: (config.sea_level + 8.0).max(8.0),
        }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct WorldGenerator {
    config: WorldGenConfig,
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self {
            config: WorldGenConfig::default(),
        }
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        Self { config }
    }

    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        Self::new(WorldGenConfig::from_planet_config(planet_config))
    }

    pub fn config(&self) -> &WorldGenConfig {
        &self.config
    }

    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        let wrap = self.config.planet_size as f32;
        if wrap <= 0.0 {
            return self.config.ground_level;
        }

        let wrapped_x = world_x.rem_euclid(wrap);
        let wrapped_z = world_z.rem_euclid(wrap);

        // Simple stitched rolling hills using sine waves for now.
        let hill_x = (wrapped_x * 0.01).sin() * 6.0;
        let hill_z = (wrapped_z * 0.01).cos() * 6.0;
        let broad_shape = (wrapped_x * 0.002).sin() * 12.0 + (wrapped_z * 0.002).cos() * 12.0;

        self.config.ground_level + hill_x + hill_z + broad_shape
    }

    pub fn get_biome(&self, world_x: f32, world_z: f32) -> Biome {
        let height = self.get_height(world_x, world_z);
        if height < self.config.sea_level - 8.0 {
            Biome::DeepWater
        } else if height < self.config.sea_level {
            Biome::ShallowWater
        } else {
            Biome::TemperatePlains
        }
    }

    pub fn get_block(&self, world_x: f32, world_y: f32, world_z: f32) -> BlockType {
        if world_y < 0.0 {
            return BlockType::Bedrock;
        }

        let height = self.get_height(world_x, world_z);
        let biome = self.get_biome(world_x, world_z);

        if world_y > height {
            if world_y <= self.config.sea_level {
                return BlockType::Water;
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

    pub fn get_air_temperature(&self, world_x: f32, _y: f32, world_z: f32) -> f32 {
        let base = 68.0;
        let elevation_delta = (self.get_height(world_x, world_z) - self.config.sea_level) * -0.1;
        base + elevation_delta
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTemperature>()
            .add_systems(Startup, setup_world_generator)
            .add_systems(
                Update,
                update_temperature.run_if(in_state(GameState::Playing)),
            );
    }
}

fn setup_world_generator(mut commands: Commands, planet_config: Res<PlanetConfig>) {
    let world_gen = WorldGenerator::from_planet_config(&planet_config);
    commands.insert_resource(world_gen);
}

fn update_temperature(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    world_gen: Res<WorldGenerator>,
    mut temperature: ResMut<CurrentTemperature>,
) {
    let Ok(transform) = camera_query.get_single() else {
        return;
    };

    let pos = transform.translation;
    let chunk_x = (pos.x / 32.0).floor() as i32;
    let chunk_z = (pos.z / 32.0).floor() as i32;

    if chunk_x != temperature.last_chunk_x || chunk_z != temperature.last_chunk_z {
        let temp_f = world_gen.get_air_temperature(pos.x, pos.y, pos.z);
        temperature.update(temp_f);
        temperature.last_chunk_x = chunk_x;
        temperature.last_chunk_z = chunk_z;
    }
}
