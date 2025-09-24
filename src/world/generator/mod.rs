use bevy::prelude::*;
use bincode::Options;
use flate2::read::GzDecoder;
use image::{ImageBuffer, Rgba};
use noise::Perlin;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};

use super::chunk_store::{
    ChunkPayloadQueue, ChunkPayloadReady, PayloadDebugPlugin, PlanetChunkStore,
};
use super::config::{CurrentTemperature, WorldGenConfig};
use super::persistence::{ChunkPersistencePlugin, DiskChunkPersistence, PersistenceConfig};
use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::chunk::{ChunkPos, CHUNK_SIZE_F32};
use crate::loading::GameState;
use crate::planet::PlanetConfig;

mod continents;
mod hydrology;
mod lithology;
mod mountains;
mod phases;
mod plates;
mod util;

use continents::{generate_continent_sites, ContinentSite};
use hydrology::HydrologySimulation;
use lithology::{generate_plate_lithology, LithologyLayer, LithologyProfile};
use mountains::MountainRangeMap;
use plates::{PlateMap, PlateSample};

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
    plate_map: PlateMap,
    plate_lithology: Vec<LithologyProfile>,
    hydrology: HydrologySimulation,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    config: WorldGenConfig,
    continent_sites: Vec<ContinentSite>,
    mountain_ranges: MountainRangeMap,
    plate_map: PlateMap,
    hydrology: HydrologySimulation,
    plate_lithology: Vec<LithologyProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldMetadataFormat {
    Binary,
    GzJson,
    Json,
}

impl WorldMetadata {
    fn bincode_options() -> impl Options {
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_little_endian()
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<(Self, WorldMetadataFormat), String> {
        let path = path.as_ref();
        let bytes =
            fs::read(path).map_err(|err| format!("failed to open metadata {:?}: {}", path, err))?;

        if let Ok(metadata) = Self::bincode_options().deserialize(&bytes[..]) {
            return Ok((metadata, WorldMetadataFormat::Binary));
        }

        let mut decoder = GzDecoder::new(Cursor::new(&bytes));
        let mut decoded = Vec::new();
        match decoder.read_to_end(&mut decoded) {
            Ok(_) => match serde_json::from_slice(&decoded) {
                Ok(metadata) => Ok((metadata, WorldMetadataFormat::GzJson)),
                Err(err) => Err(format!(
                    "failed to parse gzipped metadata {:?}: {}",
                    path, err
                )),
            },
            Err(_) => match serde_json::from_slice(&bytes) {
                Ok(metadata) => Ok((metadata, WorldMetadataFormat::Json)),
                Err(err) => Err(format!(
                    "failed to parse metadata {:?} as binary, gzipped JSON, or JSON: {}",
                    path, err
                )),
            },
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to create metadata directory {:?}: {}", parent, err)
            })?;
        }
        let file = fs::File::create(path)
            .map_err(|err| format!("failed to write metadata {:?}: {}", path, err))?;
        let mut writer = BufWriter::new(file);
        Self::bincode_options()
            .serialize_into(&mut writer, self)
            .map_err(|err| format!("failed to serialize metadata {:?}: {}", path, err))?;
        writer
            .flush()
            .map_err(|err| format!("failed to flush metadata {:?}: {}", path, err))?;
        Ok(())
    }
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
            plate_map: PlateMap::empty(),
            plate_lithology: Vec::new(),
            hydrology: HydrologySimulation::empty(),
        };

        progress.on_phase(WorldGenPhase::Continents);
        generator.continent_sites = generate_continent_sites(&generator.config);
        generator.plate_map = PlateMap::generate(&generator.config, &generator.continent_sites);

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

        generator.plate_lithology = generate_plate_lithology(
            &generator.config,
            &generator.plate_map,
            |world_x, world_z| {
                let height = generator.get_height(world_x, world_z);
                let water = generator.get_water_level(world_x, world_z);
                (height, water)
            },
        );

        progress.on_phase(WorldGenPhase::Finalize);
        generator
    }

    #[allow(dead_code)]
    pub fn from_planet_config(planet_config: &PlanetConfig) -> Self {
        Self::new(WorldGenConfig::from_planet_config(planet_config))
    }

    pub fn from_metadata(metadata: WorldMetadata) -> Self {
        let seed = metadata.config.seed as u32;
        let continent_noise = Perlin::new(seed);
        let detail_noise = Perlin::new(seed.wrapping_add(1));
        let mountain_noise = Perlin::new(seed.wrapping_add(2));
        let moisture_noise = Perlin::new(seed.wrapping_add(3));
        let temperature_noise = Perlin::new(seed.wrapping_add(4));
        let island_noise = Perlin::new(seed.wrapping_add(5));
        let hydrology_rain_noise = Perlin::new(seed.wrapping_add(6));

        Self {
            config: metadata.config.clone(),
            continent_noise,
            detail_noise,
            mountain_noise,
            moisture_noise,
            temperature_noise,
            island_noise,
            hydrology_rain_noise,
            continent_sites: metadata.continent_sites,
            mountain_ranges: metadata.mountain_ranges,
            plate_map: metadata.plate_map,
            plate_lithology: metadata.plate_lithology,
            hydrology: metadata.hydrology,
        }
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

    pub(super) fn plate_sample(&self, u: f32, v: f32) -> PlateSample {
        self.plate_map.sample(u, v)
    }

    #[allow(dead_code)]
    pub fn lithology_profile_at(&self, world_x: f32, world_z: f32) -> LithologyProfile {
        let planet_size = self.config.planet_size as f32;
        let u = (world_x / planet_size).rem_euclid(1.0);
        let v = (world_z / planet_size).rem_euclid(1.0);
        let weights = self.plate_map.plate_weights(u, v);

        if weights.len() == 1 {
            return self.plate_lithology[weights[0].0].clone();
        }

        let mut surface_depth = 0.0_f32;
        let mut cave_bias = 0.0_f32;
        let mut ore_bias = 0.0_f32;

        let mut best_surface = (weights[0].0, weights[0].1);
        let mut best_basement = best_surface;

        let max_layers = weights
            .iter()
            .map(|(plate, _)| self.plate_lithology[*plate].strata.len())
            .max()
            .unwrap_or(0);

        let mut layer_thickness = vec![0.0_f32; max_layers];
        let mut layer_block = vec![BlockType::Stone; max_layers];
        let mut layer_weight = vec![0.0_f32; max_layers];

        for (plate, weight) in &weights {
            let profile = &self.plate_lithology[*plate];
            let w = *weight;
            surface_depth += w * profile.surface_depth as f32;
            cave_bias += w * profile.cave_bias;
            ore_bias += w * profile.ore_bias;

            if w > best_surface.1 {
                best_surface = (*plate, w);
            }
            if w > best_basement.1 {
                best_basement = (*plate, w);
            }

            for (i, layer) in profile.strata.iter().enumerate() {
                layer_thickness[i] += w * layer.thickness as f32;
                if w > layer_weight[i] {
                    layer_block[i] = layer.block;
                    layer_weight[i] = w;
                }
            }
        }

        let surface_depth = surface_depth.round().clamp(1.0, 16.0) as u8;
        let mut strata = Vec::new();
        for i in 0..max_layers {
            let thickness = layer_thickness[i].round() as i32;
            if thickness <= 0 {
                continue;
            }
            strata.push(LithologyLayer {
                block: layer_block[i],
                thickness: thickness.clamp(1, 64) as u8,
            });
        }

        let surface_block = self.plate_lithology[best_surface.0].surface_block;
        let basement_block = self.plate_lithology[best_basement.0].basement_block;

        LithologyProfile {
            surface_block,
            surface_depth,
            strata,
            basement_block,
            cave_bias: cave_bias.clamp(0.0, 1.0),
            ore_bias: ore_bias.clamp(0.0, 1.0),
        }
    }

    pub fn metadata(&self) -> WorldMetadata {
        WorldMetadata {
            config: self.config.clone(),
            continent_sites: self.continent_sites.clone(),
            mountain_ranges: self.mountain_ranges.clone(),
            plate_map: self.plate_map.clone(),
            hydrology: self.hydrology.clone(),
            plate_lithology: self.plate_lithology.clone(),
        }
    }

    pub fn surface_chunk_y(&self, chunk_x: i32, chunk_z: i32) -> i32 {
        let origin = ChunkPos::new(chunk_x, 0, chunk_z).to_world_pos();
        let center_x = origin.x + CHUNK_SIZE_F32 * 0.5;
        let center_z = origin.z + CHUNK_SIZE_F32 * 0.5;
        let height = self.get_height(center_x, center_z);
        (height / CHUNK_SIZE_F32).floor() as i32
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
            .init_resource::<PlanetChunkStore>()
            .init_resource::<ChunkPayloadQueue>()
            .add_event::<ChunkPayloadReady>()
            .add_plugins(PayloadDebugPlugin)
            .add_plugins(ChunkPersistencePlugin::new(
                DiskChunkPersistence::new(
                    std::env::var("FORGE_PERSISTENCE_DIR")
                        .ok()
                        .filter(|dir| !dir.trim().is_empty())
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("target/chunk_payload_persistence")),
                ),
                PersistenceConfig {
                    enabled: std::env::var("FORGE_PERSISTENCE_ENABLED")
                        .map(|value| !matches!(value.trim(), "0" | "false" | "False" | "FALSE"))
                        .unwrap_or(true),
                },
            ))
            .add_systems(Startup, setup_world_generator)
            .add_systems(
                Update,
                update_temperature.run_if(in_state(GameState::Playing)),
            );
    }
}

fn setup_world_generator(mut commands: Commands, planet_config: Res<PlanetConfig>) {
    let desired_config = WorldGenConfig::from_planet_config(&planet_config);

    let metadata_env = std::env::var("FORGE_WORLD_METADATA_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let metadata_path = metadata_env
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/world_metadata.bin"));
    let save_metadata = std::env::var("FORGE_WORLD_METADATA_SAVE")
        .map(|value| !matches!(value.trim(), "0" | "false" | "False" | "FALSE"))
        .unwrap_or(true);

    let candidate_paths: Vec<PathBuf> = if metadata_env.is_some() {
        vec![metadata_path.clone()]
    } else {
        let mut paths = vec![metadata_path.clone()];
        for extension in ["json", "json.gz"] {
            let candidate = metadata_path.with_extension(extension);
            if candidate != metadata_path && !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }
        paths
    };

    let metadata_load_path = candidate_paths
        .iter()
        .find(|candidate| candidate.exists())
        .cloned();

    match &metadata_load_path {
        Some(path) if path == &metadata_path => {
            info!("Attempting to load world metadata from {:?}", path);
            println!("[world] attempting to load metadata from {:?}", path);
        }
        Some(path) => {
            info!(
                "Attempting to load world metadata from legacy path {:?} (target path {:?})",
                path, metadata_path
            );
            println!(
                "[world] attempting to load metadata from legacy {:?} (target {:?})",
                path, metadata_path
            );
        }
        None => {
            info!(
                "No cached world metadata found at {:?}, generating new world",
                metadata_path
            );
            println!(
                "[world] no metadata at {:?}, starting fresh generation",
                metadata_path
            );
        }
    };

    let mut metadata_loaded = false;
    let mut metadata_format = WorldMetadataFormat::Binary;
    let mut metadata_source_path: Option<PathBuf> = None;

    let world_gen = if let Some(path) = metadata_load_path.clone() {
        match WorldMetadata::load_from_file(&path) {
            Ok((metadata, format)) => {
                if metadata.config == desired_config {
                    metadata_loaded = true;
                    metadata_format = format;
                    metadata_source_path = Some(path.clone());
                    info!(
                        "Loaded cached world metadata from {:?} using {:?} format",
                        path, format
                    );
                    println!(
                        "[world] loaded metadata from {:?} (format: {:?})",
                        path, format
                    );
                    WorldGenerator::from_metadata(metadata)
                } else {
                    warn!(
                        "Metadata {:?} config mismatch (cached planet_size={} seed={} vs desired planet_size={} seed={}); regenerating world.",
                        path,
                        metadata.config.planet_size,
                        metadata.config.seed,
                        desired_config.planet_size,
                        desired_config.seed
                    );
                    info!("World generation (cached mismatch) starting...");
                    println!("[world] regenerate due to metadata mismatch");
                    let generator =
                        WorldGenerator::with_progress(desired_config.clone(), NoopProgress);
                    info!("World generation complete (cached mismatch).");
                    println!("[world] regeneration complete (mismatch)");
                    generator
                }
            }
            Err(error) => {
                warn!(
                    "Failed to load metadata from {:?}: {}. Regenerating world.",
                    path, error
                );
                info!("World generation (metadata load failed) starting...");
                println!("[world] regenerate due to metadata load failure: {}", error);
                let generator = WorldGenerator::with_progress(desired_config.clone(), NoopProgress);
                info!("World generation complete (metadata load failed).");
                println!("[world] regeneration complete (load failure)");
                generator
            }
        }
    } else {
        info!("World generation (no metadata) starting...");
        println!("[world] generating new world (no metadata)");
        let generator = WorldGenerator::with_progress(desired_config.clone(), NoopProgress);
        info!("World generation complete (no metadata).");
        println!("[world] generation complete (no metadata)");
        generator
    };

    let source_path_matches_target = metadata_source_path
        .as_ref()
        .map(|path| path == &metadata_path)
        .unwrap_or(false);
    let needs_upgrade = metadata_loaded && metadata_format != WorldMetadataFormat::Binary;

    if metadata_loaded {
        let source_path_for_log: &Path = metadata_source_path
            .as_deref()
            .unwrap_or(metadata_path.as_path());
        if save_metadata && (needs_upgrade || !source_path_matches_target) {
            if let Err(error) = world_gen.metadata().save_to_file(&metadata_path) {
                warn!(
                    "Failed to write upgraded world metadata to {:?}: {}",
                    metadata_path, error
                );
            } else {
                let upgrade_description = match (needs_upgrade, source_path_matches_target) {
                    (true, true) => "format",
                    (true, false) => "format and path",
                    (false, false) => "path",
                    (false, true) => "",
                };
                info!(
                    "Saved world metadata to {:?} (upgraded {} from {:?}, format {:?} -> Binary)",
                    metadata_path, upgrade_description, source_path_for_log, metadata_format
                );
                println!(
                    "[world] saved metadata to {:?} (upgraded {} from {:?}, format {:?} -> Binary)",
                    metadata_path, upgrade_description, source_path_for_log, metadata_format
                );
            }
        } else {
            info!(
                "Using cached world metadata from {:?}; {}",
                source_path_for_log,
                if save_metadata {
                    "already binary and in-place"
                } else {
                    "autosave disabled"
                }
            );
            println!(
                "[world] using cached metadata from {:?}; {}",
                source_path_for_log,
                if save_metadata {
                    "already binary and in-place"
                } else {
                    "autosave disabled"
                }
            );
        }
    } else if save_metadata {
        if let Err(error) = world_gen.metadata().save_to_file(&metadata_path) {
            warn!(
                "Failed to write world metadata to {:?}: {}",
                metadata_path, error
            );
        } else {
            info!("Saved world metadata to {:?}", metadata_path);
            println!("[world] saved metadata to {:?}", metadata_path);
        }
    } else {
        info!(
            "World metadata autosave disabled; skipping write to {:?}",
            metadata_path
        );
        println!(
            "[world] metadata autosave disabled; skipping write to {:?}",
            metadata_path
        );
    }

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
