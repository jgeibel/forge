use bevy::prelude::*;

pub mod generator;
pub mod noise_layers;
pub mod biomes;
pub mod terrain;
pub mod features;
pub mod hydrology;

pub use generator::{WorldGenerator, WorldGenConfig};
pub use biomes::Biome;

use crate::planet::PlanetConfig;
use crate::camera::PlayerCamera;
use crate::loading::GameState;

/// Resource tracking the current air temperature at player position
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
            fahrenheit: 70.0,  // Default comfortable temperature
            celsius: 21.0,
            last_chunk_x: i32::MAX,  // Force update on first frame
            last_chunk_z: i32::MAX,
        }
    }
    
    pub fn update(&mut self, fahrenheit: f32) {
        self.fahrenheit = fahrenheit;
        self.celsius = (fahrenheit - 32.0) * 5.0 / 9.0;
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CurrentTemperature>()
            .add_systems(Startup, setup_world_generator)
            .add_systems(Update, update_temperature.run_if(in_state(GameState::Playing)));
    }
}

fn setup_world_generator(
    mut commands: Commands,
    planet_config: Res<PlanetConfig>,
) {
    let world_gen = WorldGenerator::from_planet_config(&planet_config);
    commands.insert_resource(world_gen);
}

/// Update temperature when player moves to a new chunk
fn update_temperature(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    world_gen: Res<WorldGenerator>,
    mut temperature: ResMut<CurrentTemperature>,
) {
    let Ok(transform) = camera_query.get_single() else {
        return;
    };
    
    let pos = transform.translation;
    
    // Calculate current chunk position
    let chunk_x = (pos.x / 32.0).floor() as i32;
    let chunk_z = (pos.z / 32.0).floor() as i32;
    
    // Only update if we've moved to a different chunk
    if chunk_x != temperature.last_chunk_x || chunk_z != temperature.last_chunk_z {
        let temp_f = world_gen.get_air_temperature(pos.x, pos.y, pos.z);
        temperature.update(temp_f);
        temperature.last_chunk_x = chunk_x;
        temperature.last_chunk_z = chunk_z;
    }
}