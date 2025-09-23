use bevy::prelude::*;
use image::{ImageBuffer, Rgba};
use noise::Perlin;
use std::path::{Path, PathBuf};

use super::config::{CurrentTemperature, WorldGenConfig};
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::PlanetConfig;

mod continents;
mod hydrology;
mod mountains;
mod phases;
mod util;

use continents::{generate_continent_sites, ContinentSite};
use hydrology::HydrologySimulation;
use mountains::MountainRangeMap;

/// Logical phases in the world generation pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldGenPhase {
    Core,
    Continents,
    Terrain,
    Mountains,
    Climate,
    Islands,
    Hydrology,
    Finalize,
}

pub trait WorldGenProgress {
    fn on_phase(&mut self, phase: WorldGenPhase);
}

struct NoopProgress;

impl WorldGenProgress for NoopProgress {
    fn on_phase(&mut self, _phase: WorldGenPhase) {}
}

impl<F> WorldGenProgress for F
where
    F: FnMut(WorldGenPhase),
{
    fn on_phase(&mut self, phase: WorldGenPhase) {
        self(phase);
    }
}

#[derive(Resource, Clone)]
pub struct WorldGenerator {
    config: WorldGenConfig,
    continent_noise: Perlin,
    detail_noise: Perlin,
    mountain_noise: Perlin,
    moisture_noise: Perlin,
    temperature_noise: Perlin,
    island_noise: Perlin,
    hydrology_rain_noise: Perlin,
    continent_sites: Vec<ContinentSite>,
    mountain_ranges: MountainRangeMap,
    hydrology: HydrologySimulation,
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self::new(WorldGenConfig::default())
    }
}

impl WorldGenerator {
    pub fn new(config: WorldGenConfig) -> Self {
        Self::with_progress(config, NoopProgress)
    }

    pub fn with_progress<P>(config: WorldGenConfig, mut progress: P) -> Self
    where
        P: WorldGenProgress,
    {
        progress.on_phase(WorldGenPhase::Core);

        let seed = config.seed as u32;
        let continent_noise = Perlin::new(seed);
        let detail_noise = Perlin::new(seed.wrapping_add(1));
        let mountain_noise = Perlin::new(seed.wrapping_add(2));
        let moisture_noise = Perlin::new(seed.wrapping_add(3));
        let temperature_noise = Perlin::new(seed.wrapping_add(4));
        let island_noise = Perlin::new(seed.wrapping_add(5));
        let hydrology_rain_noise = Perlin::new(seed.wrapping_add(6));

        let mut generator = Self {
            config,
            continent_noise,
            detail_noise,
            mountain_noise,
            moisture_noise,
            temperature_noise,
            island_noise,
            hydrology_rain_noise,
            continent_sites: Vec::new(),
            mountain_ranges: MountainRangeMap::empty(),
            hydrology: HydrologySimulation::empty(),
        };

        progress.on_phase(WorldGenPhase::Continents);
        generator.continent_sites = generate_continent_sites(&generator.config);

        progress.on_phase(WorldGenPhase::Terrain);
        generator.initialize_terrain_phase();

        progress.on_phase(WorldGenPhase::Mountains);
        generator.mountain_ranges =
            MountainRangeMap::generate(&generator.config, &generator.continent_sites, &|u, v| {
                generator.plate_sample(u, v)
            });

        progress.on_phase(WorldGenPhase::Climate);
        generator.initialize_climate_phase();

        progress.on_phase(WorldGenPhase::Islands);
        generator.initialize_island_phase();

        progress.on_phase(WorldGenPhase::Hydrology);
        generator.hydrology = HydrologySimulation::generate(&generator);

        progress.on_phase(WorldGenPhase::Finalize);
        generator
    }

    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        Self::new(WorldGenConfig::from_planet_config(planet_config))
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &WorldGenConfig {
        &self.config
    }

    #[allow(dead_code)]
    pub fn planet_size(&self) -> u32 {
        self.config.planet_size
    }

    fn initialize_terrain_phase(&mut self) {
        // Placeholder for terrain preprocessing; retained for future expansion.
    }

    fn initialize_climate_phase(&mut self) {
        // Currently climate is computed procedurally at sample time.
        // This hook exists to support future cached climate data.
    }

    fn initialize_island_phase(&mut self) {
        // Islands rely on procedural noise at sample time, so no precomputation yet.
    }

    pub fn export_planet_preview<P: AsRef<Path>>(
        &self,
        width: u32,
        height: u32,
        output_path: P,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("preview dimensions must be greater than zero".into());
        }

        let size = self.config.planet_size as f32;
        let image = ImageBuffer::from_fn(width, height, |x, y| {
            let u = x as f32 / width as f32;
            let v = 1.0 - y as f32 / height as f32;
            let world_x = u * size;
            let world_z = v * size;

            let biome = self.get_biome(world_x, world_z);
            let elevation = self.get_height(world_x, world_z);
            let color = self.preview_color(world_x, world_z, biome, elevation);
            Rgba(color)
        });

        image
            .save(output_path)
            .map_err(|err| format!("failed to write preview image: {err}"))
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

    if let Ok(env_value) = std::env::var("FORGE_EXPORT_WORLD_MAP") {
        let output_path: PathBuf = if env_value.trim().is_empty() {
            PathBuf::from("target/world_preview.png")
        } else {
            PathBuf::from(env_value.trim())
        };

        let map_size = std::env::var("FORGE_WORLD_MAP_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|size| *size > 0)
            .unwrap_or(1024);
        let map_height = (map_size / 2).max(1);

        match world_gen.export_planet_preview(map_size, map_height, &output_path) {
            Ok(()) => info!(
                "Exported world preview image to {:?} ({}x{})",
                output_path, map_size, map_height
            ),
            Err(error) => warn!(
                "Failed to export world preview image to {:?}: {}",
                output_path, error
            ),
        }
    }

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
