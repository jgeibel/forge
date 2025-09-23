use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::render::view::RenderLayers;
use bevy::ui::{Display, TargetCamera};
use bevy::window::{PresentMode, PrimaryWindow, WindowRef, WindowResolution};

use forge::planet::PlanetSize;
use forge::world::{Biome, WorldGenConfig, WorldGenPhase, WorldGenerator};
use std::collections::HashMap;

mod source_updater;

const MAP_WIDTH: u32 = 512; // Lower initial resolution for faster rendering
const MAP_HEIGHT: u32 = 256; // Lower initial resolution for faster rendering

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "World Builder - Map".into(),
                resolution: WindowResolution::new(1024.0, 512.0), // 2x the map resolution for comfortable viewing
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<ButtonMaterials>()
        .init_resource::<DetailWindow>()
        .add_event::<RegenerateRequested>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // Button handlers (max 10 systems per tuple in Bevy)
                handle_parameter_buttons,
                handle_parameter_reset_buttons,
                handle_world_size_buttons,
                handle_tab_buttons,
                handle_visualization_buttons,
                handle_regenerate_button,
                handle_save_to_source_button,
            ),
        )
        .add_systems(
            Update,
            (
                // UI updates
                sync_visualization_highlights,
                sync_tab_highlights,
                update_tab_sections,
                update_phase_status_text,
                update_value_text,
                update_selection_text,
                update_location_popup,
            ),
        )
        .add_systems(
            Update,
            (
                // Map interactions
                handle_map_zoom,
                handle_map_pan,
                handle_map_click,
                handle_scroll_events,
                apply_selection_marker,
                redraw_map_when_needed,
                update_detail_view,
            ),
        )
        .run();
}

#[derive(Resource)]
struct ButtonMaterials {
    normal: BackgroundColor,
    hovered: BackgroundColor,
    pressed: BackgroundColor,
    active: BackgroundColor,
    tab_normal: BackgroundColor,
    tab_active: BackgroundColor,
}

impl Default for ButtonMaterials {
    fn default() -> Self {
        Self {
            normal: BackgroundColor(Color::srgba(0.15, 0.17, 0.22, 0.95)),
            hovered: BackgroundColor(Color::srgba(0.22, 0.25, 0.32, 0.95)),
            pressed: BackgroundColor(Color::srgba(0.30, 0.35, 0.45, 0.95)),
            active: BackgroundColor(Color::srgba(0.25, 0.35, 0.55, 0.95)),
            tab_normal: BackgroundColor(Color::srgba(0.12, 0.14, 0.18, 0.95)),
            tab_active: BackgroundColor(Color::srgba(0.18, 0.25, 0.38, 0.95)),
        }
    }
}

#[derive(Resource)]
struct WorldBuilderState {
    working: WorldGenConfig,
    active: WorldGenConfig,
    defaults: WorldGenConfig, // Store the original defaults for comparison
    generator: WorldGenerator,
    planet_sizes: Vec<PlanetSize>,
    planet_size_index: usize,
    visualization: MapVisualization,
    active_tab: ParameterTab,
    repaint_requested: bool,
    selection: Option<SelectionDetail>,
    changed_parameters: HashMap<String, bool>, // Track which parameters have changed
    // Camera controls for the map
    camera_zoom: f32,
    camera_translation: Vec2,
    is_panning: bool,
    last_mouse_position: Option<Vec2>,
    // Popup state
    show_popup: bool,
    popup_world_pos: Option<(f32, f32)>,
    // Detail inspection
    detail_center: Option<Vec2>, // Center of the detail view in world coordinates
    phase_history: Vec<WorldGenPhase>,
    phase_history_dirty: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct SelectionDetail {
    world_x: f32,
    world_z: f32,
    height: f32,
    biome: Biome,
    temperature_c: f32,
    moisture: f32,
    rainfall: f32,
    water_level: f32,
    river_intensity: f32,
    major_river: f32,
}

#[derive(Resource)]
struct MapTextures {
    map: Handle<Image>,
}

#[derive(Event, Default)]
struct RegenerateRequested;

#[derive(Component)]
struct DetailImage;

#[derive(Component)]
struct ScrollContent;

#[derive(Component)]
struct SelectionMarker;

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum MapVisualization {
    Biomes,
    Elevation,
    Moisture,
    Temperature,
    Hydrology,
    MajorRivers,
}

impl MapVisualization {
    const ALL: [Self; 6] = [
        MapVisualization::Biomes,
        MapVisualization::Elevation,
        MapVisualization::Moisture,
        MapVisualization::Temperature,
        MapVisualization::Hydrology,
        MapVisualization::MajorRivers,
    ];

    fn label(&self) -> &'static str {
        match self {
            MapVisualization::Biomes => "Biomes",
            MapVisualization::Elevation => "Elevation",
            MapVisualization::Moisture => "Moisture",
            MapVisualization::Temperature => "Temperature",
            MapVisualization::Hydrology => "Hydrology",
            MapVisualization::MajorRivers => "Major Rivers",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParameterTab {
    Core,
    Continents,
    Terrain,
    Mountains,
    Climate,
    Islands,
    Hydrology,
}

impl ParameterTab {
    const ALL: [Self; 7] = [
        ParameterTab::Core,
        ParameterTab::Continents,
        ParameterTab::Terrain,
        ParameterTab::Mountains,
        ParameterTab::Climate,
        ParameterTab::Islands,
        ParameterTab::Hydrology,
    ];

    fn label(&self) -> &'static str {
        match self {
            ParameterTab::Core => "Core",
            ParameterTab::Continents => "Continents",
            ParameterTab::Terrain => "Terrain",
            ParameterTab::Mountains => "Mountains",
            ParameterTab::Climate => "Climate",
            ParameterTab::Islands => "Islands",
            ParameterTab::Hydrology => "Hydrology",
        }
    }
}

#[derive(Component, Clone, Copy)]
struct ParameterButton {
    field: ParameterField,
    delta: f32,
}

#[derive(Component, Clone, Copy)]
struct ParameterResetButton {
    field: ParameterField,
}

#[derive(Component)]
struct RegenerateButton;

#[derive(Component)]
struct SaveToSourceButton;

#[derive(Component)]
struct PhaseStatusText;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct VisualizationButton {
    mode: MapVisualization,
}

#[derive(Component)]
struct WorldSizeLabel;

#[derive(Component)]
struct SelectionSummaryText;

#[derive(Component)]
struct LocationPopup;

#[derive(Component)]
struct LocationPopupText;

#[derive(Component)]
struct ParameterValueText {
    field: ParameterField,
}

#[derive(Component)]
struct PlanetSizeButton {
    delta: i32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct TabButton {
    tab: ParameterTab,
}

#[derive(Component)]
struct TabSection {
    tab: ParameterTab,
}

#[derive(Component)]
struct MapSprite;

#[derive(Component)]
struct DetailWindowCamera;

#[derive(Component)]
struct MapCamera;

#[derive(Component)]
struct InspectionMarker;

#[derive(Resource, Default)]
struct DetailWindow {
    entity: Option<Entity>,
    camera: Option<Entity>,
    last_center: Option<Vec2>,
    marker_entity: Option<Entity>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParameterField {
    SeaLevel,
    OceanDepth,
    DeepOceanDepth,
    ContinentCount,
    ContinentFrequency,
    ContinentThreshold,
    ContinentPower,
    ContinentBias,
    ContinentRadius,
    ContinentEdgePower,
    ContinentBeltWidth,
    ContinentRepulsionStrength,
    ContinentDriftGain,
    ContinentDriftBeltGain,
    DetailFrequency,
    DetailAmplitude,
    MountainFrequency,
    MountainHeight,
    MountainThreshold,
    MountainRangeCount,
    MountainRangeWidth,
    MountainRangeStrength,
    MountainRangeSpurChance,
    MountainRangeSpurStrength,
    MountainRangeRoughness,
    MountainErosionIterations,
    MountainConvergenceBoost,
    MountainDivergencePenalty,
    MountainShearBoost,
    MountainArcThreshold,
    MountainArcStrength,
    MountainArcWidthFactor,
    MoistureFrequency,
    EquatorTemperature,
    PoleTemperature,
    LapseRate,
    TemperatureVariation,
    HighlandBonus,
    IslandFrequency,
    IslandThreshold,
    IslandHeight,
    IslandFalloff,
    HydrologyResolution,
    HydrologyRainfall,
    HydrologyRainfallVariance,
    HydrologyRainfallFrequency,
    HydrologyIterations,
    HydrologyTimeStep,
    HydrologyInfiltrationRate,
    HydrologyBaseflow,
    HydrologyErosionRate,
    HydrologyDepositionRate,
    HydrologySedimentCapacity,
    HydrologyBankfullDepth,
    HydrologyFloodplainSoftening,
    HydrologyMinimumSlope,
    HydrologyShorelineRadius,
    HydrologyShorelineMaxHeight,
    HydrologyShorelineSmoothing,
}

impl ParameterField {
    fn label(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "Sea Level",
            ParameterField::OceanDepth => "Ocean Depth",
            ParameterField::DeepOceanDepth => "Deep Ocean Depth",
            ParameterField::ContinentCount => "Continent Count",
            ParameterField::ContinentFrequency => "Continent Frequency",
            ParameterField::ContinentThreshold => "Continent Threshold",
            ParameterField::ContinentPower => "Continent Power",
            ParameterField::ContinentBias => "Continent Bias",
            ParameterField::ContinentRadius => "Continent Radius",
            ParameterField::ContinentEdgePower => "Edge Power",
            ParameterField::ContinentBeltWidth => "Belt Width",
            ParameterField::ContinentRepulsionStrength => "Site Repulsion",
            ParameterField::ContinentDriftGain => "Drift Gain",
            ParameterField::ContinentDriftBeltGain => "Drift Belt Gain",
            ParameterField::DetailFrequency => "Detail Frequency",
            ParameterField::DetailAmplitude => "Detail Amplitude",
            ParameterField::MountainFrequency => "Mountain Frequency",
            ParameterField::MountainHeight => "Mountain Height",
            ParameterField::MountainThreshold => "Mountain Threshold",
            ParameterField::MountainRangeCount => "Range Count",
            ParameterField::MountainRangeWidth => "Range Width",
            ParameterField::MountainRangeStrength => "Range Strength",
            ParameterField::MountainRangeSpurChance => "Spur Chance",
            ParameterField::MountainRangeSpurStrength => "Spur Strength",
            ParameterField::MountainRangeRoughness => "Roughness",
            ParameterField::MountainErosionIterations => "Erosion Passes",
            ParameterField::MountainConvergenceBoost => "Convergence Boost",
            ParameterField::MountainDivergencePenalty => "Divergence Penalty",
            ParameterField::MountainShearBoost => "Shear Boost",
            ParameterField::MountainArcThreshold => "Arc Threshold",
            ParameterField::MountainArcStrength => "Arc Strength",
            ParameterField::MountainArcWidthFactor => "Arc Width",
            ParameterField::MoistureFrequency => "Moisture Frequency",
            ParameterField::EquatorTemperature => "Equator Temp (°C)",
            ParameterField::PoleTemperature => "Pole Temp (°C)",
            ParameterField::LapseRate => "Lapse Rate (°C/block)",
            ParameterField::TemperatureVariation => "Temperature Variation",
            ParameterField::HighlandBonus => "Highland Bonus",
            ParameterField::IslandFrequency => "Island Frequency",
            ParameterField::IslandThreshold => "Island Threshold",
            ParameterField::IslandHeight => "Island Height",
            ParameterField::IslandFalloff => "Island Falloff",
            ParameterField::HydrologyResolution => "Hydrology Resolution",
            ParameterField::HydrologyRainfall => "Rainfall",
            ParameterField::HydrologyRainfallVariance => "Rainfall Variance",
            ParameterField::HydrologyRainfallFrequency => "Rainfall Frequency",
            ParameterField::HydrologyIterations => "Iterations",
            ParameterField::HydrologyTimeStep => "Time Step",
            ParameterField::HydrologyInfiltrationRate => "Infiltration",
            ParameterField::HydrologyBaseflow => "Baseflow",
            ParameterField::HydrologyErosionRate => "Erosion Rate",
            ParameterField::HydrologyDepositionRate => "Deposition Rate",
            ParameterField::HydrologySedimentCapacity => "Sediment Capacity",
            ParameterField::HydrologyBankfullDepth => "Bankfull Depth",
            ParameterField::HydrologyFloodplainSoftening => "Floodplain Softening",
            ParameterField::HydrologyMinimumSlope => "Minimum Slope",
            ParameterField::HydrologyShorelineRadius => "Shoreline Radius",
            ParameterField::HydrologyShorelineMaxHeight => "Shoreline Max Height",
            ParameterField::HydrologyShorelineSmoothing => "Shoreline Smoothing",
        }
    }

    fn adjust(&self, config: &mut WorldGenConfig, delta: f32) {
        match self {
            ParameterField::SeaLevel => {
                config.sea_level = (config.sea_level + delta).clamp(16.0, 200.0);
            }
            ParameterField::OceanDepth => {
                config.ocean_depth = (config.ocean_depth + delta).clamp(10.0, 100.0);
            }
            ParameterField::DeepOceanDepth => {
                config.deep_ocean_depth = (config.deep_ocean_depth + delta).clamp(20.0, 200.0);
            }
            ParameterField::ContinentCount => {
                let count = (config.continent_count as i32 + delta as i32).clamp(1, 24);
                config.continent_count = count as u32;
            }
            ParameterField::ContinentFrequency => {
                let freq = (config.continent_frequency + delta as f64).clamp(0.1, 4.0);
                config.continent_frequency = freq;
            }
            ParameterField::ContinentThreshold => {
                let threshold = (config.continent_threshold + delta).clamp(0.05, 0.6);
                config.continent_threshold = threshold;
            }
            ParameterField::ContinentPower => {
                config.continent_power = (config.continent_power + delta).clamp(0.2, 5.0);
            }
            ParameterField::ContinentBias => {
                config.continent_bias = (config.continent_bias + delta).clamp(0.0, 0.6);
            }
            ParameterField::ContinentRadius => {
                config.continent_radius = (config.continent_radius + delta).clamp(0.05, 0.6);
            }
            ParameterField::ContinentEdgePower => {
                config.continent_edge_power = (config.continent_edge_power + delta).clamp(0.2, 4.0);
            }
            ParameterField::ContinentBeltWidth => {
                config.continent_belt_width =
                    (config.continent_belt_width + delta).clamp(0.05, 0.45);
            }
            ParameterField::ContinentRepulsionStrength => {
                config.continent_repulsion_strength =
                    (config.continent_repulsion_strength + delta).clamp(0.0, 0.3);
            }
            ParameterField::ContinentDriftGain => {
                config.continent_drift_gain = (config.continent_drift_gain + delta).clamp(0.0, 0.4);
            }
            ParameterField::ContinentDriftBeltGain => {
                config.continent_drift_belt_gain =
                    (config.continent_drift_belt_gain + delta).clamp(0.0, 1.2);
            }
            ParameterField::DetailFrequency => {
                let freq = (config.detail_frequency + delta as f64).clamp(1.0, 15.0);
                config.detail_frequency = freq;
            }
            ParameterField::DetailAmplitude => {
                config.detail_amplitude = (config.detail_amplitude + delta).clamp(1.0, 30.0);
            }
            ParameterField::MountainFrequency => {
                let freq = (config.mountain_frequency + delta as f64).clamp(0.2, 8.0);
                config.mountain_frequency = freq;
            }
            ParameterField::MountainHeight => {
                config.mountain_height = (config.mountain_height + delta).clamp(50.0, 500.0);
            }
            ParameterField::MountainThreshold => {
                config.mountain_threshold = (config.mountain_threshold + delta).clamp(0.1, 0.9);
            }
            ParameterField::MountainRangeCount => {
                let updated =
                    (config.mountain_range_count as i32 + delta.round() as i32).clamp(0, 80);
                config.mountain_range_count = updated as u32;
            }
            ParameterField::MountainRangeWidth => {
                config.mountain_range_width =
                    (config.mountain_range_width + delta).clamp(40.0, 800.0);
            }
            ParameterField::MountainRangeStrength => {
                config.mountain_range_strength =
                    (config.mountain_range_strength + delta).clamp(0.0, 5.0);
            }
            ParameterField::MountainRangeSpurChance => {
                config.mountain_range_spur_chance =
                    (config.mountain_range_spur_chance + delta).clamp(0.0, 1.0);
            }
            ParameterField::MountainRangeSpurStrength => {
                config.mountain_range_spur_strength =
                    (config.mountain_range_spur_strength + delta).clamp(0.0, 2.0);
            }
            ParameterField::MountainRangeRoughness => {
                config.mountain_range_roughness =
                    (config.mountain_range_roughness + delta).clamp(0.0, 2.5);
            }
            ParameterField::MountainErosionIterations => {
                let updated =
                    (config.mountain_erosion_iterations as i32 + delta.round() as i32).clamp(0, 12);
                config.mountain_erosion_iterations = updated as u32;
            }
            ParameterField::MountainConvergenceBoost => {
                config.mountain_convergence_boost =
                    (config.mountain_convergence_boost + delta).clamp(0.0, 1.5);
            }
            ParameterField::MountainDivergencePenalty => {
                config.mountain_divergence_penalty =
                    (config.mountain_divergence_penalty + delta).clamp(0.0, 1.0);
            }
            ParameterField::MountainShearBoost => {
                config.mountain_shear_boost = (config.mountain_shear_boost + delta).clamp(0.0, 0.6);
            }
            ParameterField::MountainArcThreshold => {
                config.mountain_arc_threshold =
                    (config.mountain_arc_threshold + delta).clamp(0.0, 1.0);
            }
            ParameterField::MountainArcStrength => {
                config.mountain_arc_strength =
                    (config.mountain_arc_strength + delta).clamp(0.0, 1.5);
            }
            ParameterField::MountainArcWidthFactor => {
                config.mountain_arc_width_factor =
                    (config.mountain_arc_width_factor + delta).clamp(0.05, 1.0);
            }
            ParameterField::MoistureFrequency => {
                let freq = (config.moisture_frequency + delta as f64).clamp(0.1, 6.0);
                config.moisture_frequency = freq;
            }
            ParameterField::EquatorTemperature => {
                config.equator_temp_c = (config.equator_temp_c + delta).clamp(10.0, 45.0);
            }
            ParameterField::PoleTemperature => {
                config.pole_temp_c = (config.pole_temp_c + delta).clamp(-60.0, 10.0);
            }
            ParameterField::LapseRate => {
                config.lapse_rate_c_per_block =
                    (config.lapse_rate_c_per_block + delta * 0.001).clamp(0.001, 0.02);
            }
            ParameterField::TemperatureVariation => {
                config.temperature_variation =
                    (config.temperature_variation + delta).clamp(0.0, 20.0);
            }
            ParameterField::HighlandBonus => {
                config.highland_bonus = (config.highland_bonus + delta).clamp(0.0, 50.0);
            }
            ParameterField::IslandFrequency => {
                let freq = (config.island_frequency + delta as f64).clamp(0.1, 8.0);
                config.island_frequency = freq;
            }
            ParameterField::IslandThreshold => {
                config.island_threshold = (config.island_threshold + delta).clamp(0.0, 0.99);
            }
            ParameterField::IslandHeight => {
                config.island_height = (config.island_height + delta).clamp(0.0, 50.0);
            }
            ParameterField::IslandFalloff => {
                config.island_falloff = (config.island_falloff + delta).clamp(0.1, 6.0);
            }
            ParameterField::HydrologyResolution => {
                let base = config.hydrology_resolution as i32;
                let change = ((delta / 32.0).round() as i32) * 32;
                let updated = (base + change).clamp(128, 4096);
                config.hydrology_resolution = updated as u32;
            }
            ParameterField::HydrologyRainfall => {
                config.hydrology_rainfall = (config.hydrology_rainfall + delta).clamp(0.1, 10.0);
            }
            ParameterField::HydrologyRainfallVariance => {
                config.hydrology_rainfall_variance =
                    (config.hydrology_rainfall_variance + delta).clamp(0.0, 2.0);
            }
            ParameterField::HydrologyRainfallFrequency => {
                let freq = (config.hydrology_rainfall_frequency + delta as f64).clamp(0.1, 6.0);
                config.hydrology_rainfall_frequency = freq;
            }
            ParameterField::HydrologyIterations => {
                let step = (delta.round() as i32).clamp(-40, 40);
                let updated = (config.hydrology_iterations as i32 + step).clamp(1, 512);
                config.hydrology_iterations = updated as u32;
            }
            ParameterField::HydrologyTimeStep => {
                config.hydrology_time_step =
                    (config.hydrology_time_step + delta * 0.05).clamp(0.01, 5.0);
            }
            ParameterField::HydrologyInfiltrationRate => {
                config.hydrology_infiltration_rate =
                    (config.hydrology_infiltration_rate + delta * 0.02).clamp(0.0, 0.9);
            }
            ParameterField::HydrologyBaseflow => {
                config.hydrology_baseflow =
                    (config.hydrology_baseflow + delta * 0.01).clamp(0.0, 0.5);
            }
            ParameterField::HydrologyErosionRate => {
                config.hydrology_erosion_rate =
                    (config.hydrology_erosion_rate + delta * 0.02).clamp(0.01, 2.0);
            }
            ParameterField::HydrologyDepositionRate => {
                config.hydrology_deposition_rate =
                    (config.hydrology_deposition_rate + delta * 0.02).clamp(0.01, 2.0);
            }
            ParameterField::HydrologySedimentCapacity => {
                config.hydrology_sediment_capacity =
                    (config.hydrology_sediment_capacity + delta * 0.02).clamp(0.05, 2.0);
            }
            ParameterField::HydrologyBankfullDepth => {
                config.hydrology_bankfull_depth =
                    (config.hydrology_bankfull_depth + delta).clamp(2.0, 80.0);
            }
            ParameterField::HydrologyFloodplainSoftening => {
                config.hydrology_floodplain_softening =
                    (config.hydrology_floodplain_softening + delta).clamp(0.0, 40.0);
            }
            ParameterField::HydrologyMinimumSlope => {
                config.hydrology_minimum_slope =
                    (config.hydrology_minimum_slope + delta * 0.0001).clamp(0.0001, 0.05);
            }
            ParameterField::HydrologyShorelineRadius => {
                config.hydrology_shoreline_radius =
                    (config.hydrology_shoreline_radius + delta * 8.0).clamp(16.0, 512.0);
            }
            ParameterField::HydrologyShorelineMaxHeight => {
                config.hydrology_shoreline_max_height =
                    (config.hydrology_shoreline_max_height + delta).clamp(0.0, 80.0);
            }
            ParameterField::HydrologyShorelineSmoothing => {
                let updated = (config.hydrology_shoreline_smoothing as i32 + delta.round() as i32)
                    .clamp(0, 8);
                config.hydrology_shoreline_smoothing = updated as u32;
            }
        }
    }

    fn working_value(&self, config: &WorldGenConfig) -> f64 {
        match self {
            ParameterField::SeaLevel => config.sea_level as f64,
            ParameterField::OceanDepth => config.ocean_depth as f64,
            ParameterField::DeepOceanDepth => config.deep_ocean_depth as f64,
            ParameterField::ContinentCount => config.continent_count as f64,
            ParameterField::ContinentFrequency => config.continent_frequency,
            ParameterField::ContinentThreshold => config.continent_threshold as f64,
            ParameterField::ContinentPower => config.continent_power as f64,
            ParameterField::ContinentBias => config.continent_bias as f64,
            ParameterField::ContinentRadius => config.continent_radius as f64,
            ParameterField::ContinentEdgePower => config.continent_edge_power as f64,
            ParameterField::ContinentBeltWidth => config.continent_belt_width as f64,
            ParameterField::ContinentRepulsionStrength => {
                config.continent_repulsion_strength as f64
            }
            ParameterField::ContinentDriftGain => config.continent_drift_gain as f64,
            ParameterField::ContinentDriftBeltGain => config.continent_drift_belt_gain as f64,
            ParameterField::DetailFrequency => config.detail_frequency,
            ParameterField::DetailAmplitude => config.detail_amplitude as f64,
            ParameterField::MountainFrequency => config.mountain_frequency,
            ParameterField::MountainHeight => config.mountain_height as f64,
            ParameterField::MountainThreshold => config.mountain_threshold as f64,
            ParameterField::MountainRangeCount => config.mountain_range_count as f64,
            ParameterField::MountainRangeWidth => config.mountain_range_width as f64,
            ParameterField::MountainRangeStrength => config.mountain_range_strength as f64,
            ParameterField::MountainRangeSpurChance => config.mountain_range_spur_chance as f64,
            ParameterField::MountainRangeSpurStrength => config.mountain_range_spur_strength as f64,
            ParameterField::MountainRangeRoughness => config.mountain_range_roughness as f64,
            ParameterField::MountainErosionIterations => config.mountain_erosion_iterations as f64,
            ParameterField::MountainConvergenceBoost => config.mountain_convergence_boost as f64,
            ParameterField::MountainDivergencePenalty => config.mountain_divergence_penalty as f64,
            ParameterField::MountainShearBoost => config.mountain_shear_boost as f64,
            ParameterField::MountainArcThreshold => config.mountain_arc_threshold as f64,
            ParameterField::MountainArcStrength => config.mountain_arc_strength as f64,
            ParameterField::MountainArcWidthFactor => config.mountain_arc_width_factor as f64,
            ParameterField::MoistureFrequency => config.moisture_frequency,
            ParameterField::EquatorTemperature => config.equator_temp_c as f64,
            ParameterField::PoleTemperature => config.pole_temp_c as f64,
            ParameterField::LapseRate => config.lapse_rate_c_per_block as f64,
            ParameterField::TemperatureVariation => config.temperature_variation as f64,
            ParameterField::HighlandBonus => config.highland_bonus as f64,
            ParameterField::IslandFrequency => config.island_frequency,
            ParameterField::IslandThreshold => config.island_threshold as f64,
            ParameterField::IslandHeight => config.island_height as f64,
            ParameterField::IslandFalloff => config.island_falloff as f64,
            ParameterField::HydrologyResolution => config.hydrology_resolution as f64,
            ParameterField::HydrologyRainfall => config.hydrology_rainfall as f64,
            ParameterField::HydrologyRainfallVariance => config.hydrology_rainfall_variance as f64,
            ParameterField::HydrologyRainfallFrequency => config.hydrology_rainfall_frequency,
            ParameterField::HydrologyIterations => config.hydrology_iterations as f64,
            ParameterField::HydrologyTimeStep => config.hydrology_time_step as f64,
            ParameterField::HydrologyInfiltrationRate => config.hydrology_infiltration_rate as f64,
            ParameterField::HydrologyBaseflow => config.hydrology_baseflow as f64,
            ParameterField::HydrologyErosionRate => config.hydrology_erosion_rate as f64,
            ParameterField::HydrologyDepositionRate => config.hydrology_deposition_rate as f64,
            ParameterField::HydrologySedimentCapacity => config.hydrology_sediment_capacity as f64,
            ParameterField::HydrologyBankfullDepth => config.hydrology_bankfull_depth as f64,
            ParameterField::HydrologyFloodplainSoftening => {
                config.hydrology_floodplain_softening as f64
            }
            ParameterField::HydrologyMinimumSlope => config.hydrology_minimum_slope as f64,
            ParameterField::HydrologyShorelineRadius => config.hydrology_shoreline_radius as f64,
            ParameterField::HydrologyShorelineMaxHeight => {
                config.hydrology_shoreline_max_height as f64
            }
            ParameterField::HydrologyShorelineSmoothing => {
                config.hydrology_shoreline_smoothing as f64
            }
        }
    }

    fn format_value(&self, config: &WorldGenConfig) -> String {
        match self {
            ParameterField::SeaLevel => format!("{:.1}", config.sea_level),
            ParameterField::OceanDepth => format!("{:.1}", config.ocean_depth),
            ParameterField::DeepOceanDepth => format!("{:.1}", config.deep_ocean_depth),
            ParameterField::ContinentCount => format!("{}", config.continent_count),
            ParameterField::ContinentFrequency => format!("{:.2}", config.continent_frequency),
            ParameterField::ContinentThreshold => format!("{:.2}", config.continent_threshold),
            ParameterField::ContinentPower => format!("{:.2}", config.continent_power),
            ParameterField::ContinentBias => format!("{:.2}", config.continent_bias),
            ParameterField::ContinentRadius => format!("{:.2}", config.continent_radius),
            ParameterField::ContinentEdgePower => format!("{:.2}", config.continent_edge_power),
            ParameterField::ContinentBeltWidth => format!("{:.2}", config.continent_belt_width),
            ParameterField::ContinentRepulsionStrength => {
                format!("{:.3}", config.continent_repulsion_strength)
            }
            ParameterField::ContinentDriftGain => format!("{:.3}", config.continent_drift_gain),
            ParameterField::ContinentDriftBeltGain => {
                format!("{:.2}", config.continent_drift_belt_gain)
            }
            ParameterField::DetailFrequency => format!("{:.2}", config.detail_frequency),
            ParameterField::DetailAmplitude => format!("{:.1}", config.detail_amplitude),
            ParameterField::MountainFrequency => format!("{:.2}", config.mountain_frequency),
            ParameterField::MountainHeight => format!("{:.1}", config.mountain_height),
            ParameterField::MountainThreshold => format!("{:.2}", config.mountain_threshold),
            ParameterField::MountainRangeCount => format!("{}", config.mountain_range_count),
            ParameterField::MountainRangeWidth => {
                format!("{:.0}", config.mountain_range_width)
            }
            ParameterField::MountainRangeStrength => {
                format!("{:.2}", config.mountain_range_strength)
            }
            ParameterField::MountainRangeSpurChance => {
                format!("{:.2}", config.mountain_range_spur_chance)
            }
            ParameterField::MountainRangeSpurStrength => {
                format!("{:.2}", config.mountain_range_spur_strength)
            }
            ParameterField::MountainRangeRoughness => {
                format!("{:.2}", config.mountain_range_roughness)
            }
            ParameterField::MountainErosionIterations => {
                format!("{}", config.mountain_erosion_iterations)
            }
            ParameterField::MountainConvergenceBoost => {
                format!("{:.2}", config.mountain_convergence_boost)
            }
            ParameterField::MountainDivergencePenalty => {
                format!("{:.2}", config.mountain_divergence_penalty)
            }
            ParameterField::MountainShearBoost => {
                format!("{:.2}", config.mountain_shear_boost)
            }
            ParameterField::MountainArcThreshold => {
                format!("{:.2}", config.mountain_arc_threshold)
            }
            ParameterField::MountainArcStrength => {
                format!("{:.2}", config.mountain_arc_strength)
            }
            ParameterField::MountainArcWidthFactor => {
                format!("{:.2}", config.mountain_arc_width_factor)
            }
            ParameterField::MoistureFrequency => format!("{:.2}", config.moisture_frequency),
            ParameterField::EquatorTemperature => format!("{:.1}", config.equator_temp_c),
            ParameterField::PoleTemperature => format!("{:.1}", config.pole_temp_c),
            ParameterField::LapseRate => format!("{:.3}", config.lapse_rate_c_per_block),
            ParameterField::TemperatureVariation => format!("{:.1}", config.temperature_variation),
            ParameterField::HighlandBonus => format!("{:.1}", config.highland_bonus),
            ParameterField::IslandFrequency => format!("{:.2}", config.island_frequency),
            ParameterField::IslandThreshold => format!("{:.2}", config.island_threshold),
            ParameterField::IslandHeight => format!("{:.1}", config.island_height),
            ParameterField::IslandFalloff => format!("{:.2}", config.island_falloff),
            ParameterField::HydrologyResolution => format!("{}", config.hydrology_resolution),
            ParameterField::HydrologyRainfall => format!("{:.2}", config.hydrology_rainfall),
            ParameterField::HydrologyRainfallVariance => {
                format!("{:.2}", config.hydrology_rainfall_variance)
            }
            ParameterField::HydrologyRainfallFrequency => {
                format!("{:.2}", config.hydrology_rainfall_frequency)
            }
            ParameterField::HydrologyIterations => {
                format!("{}", config.hydrology_iterations)
            }
            ParameterField::HydrologyTimeStep => {
                format!("{:.2}", config.hydrology_time_step)
            }
            ParameterField::HydrologyInfiltrationRate => {
                format!("{:.3}", config.hydrology_infiltration_rate)
            }
            ParameterField::HydrologyBaseflow => {
                format!("{:.3}", config.hydrology_baseflow)
            }
            ParameterField::HydrologyErosionRate => {
                format!("{:.3}", config.hydrology_erosion_rate)
            }
            ParameterField::HydrologyDepositionRate => {
                format!("{:.3}", config.hydrology_deposition_rate)
            }
            ParameterField::HydrologySedimentCapacity => {
                format!("{:.3}", config.hydrology_sediment_capacity)
            }
            ParameterField::HydrologyBankfullDepth => {
                format!("{:.1}", config.hydrology_bankfull_depth)
            }
            ParameterField::HydrologyFloodplainSoftening => {
                format!("{:.1}", config.hydrology_floodplain_softening)
            }
            ParameterField::HydrologyMinimumSlope => {
                format!("{:.4}", config.hydrology_minimum_slope)
            }
            ParameterField::HydrologyShorelineRadius => {
                format!("{:.0}", config.hydrology_shoreline_radius)
            }
            ParameterField::HydrologyShorelineMaxHeight => {
                format!("{:.1}", config.hydrology_shoreline_max_height)
            }
            ParameterField::HydrologyShorelineSmoothing => {
                format!("{}", config.hydrology_shoreline_smoothing)
            }
        }
    }

    fn differs(&self, working: &WorldGenConfig, active: &WorldGenConfig) -> bool {
        let a = self.working_value(working);
        let b = self.working_value(active);
        (a - b).abs() > self.epsilon()
    }

    fn epsilon(&self) -> f64 {
        match self {
            ParameterField::SeaLevel => 0.01,
            ParameterField::OceanDepth => 0.01,
            ParameterField::DeepOceanDepth => 0.01,
            ParameterField::ContinentCount => 0.5,
            ParameterField::ContinentFrequency => 0.001,
            ParameterField::IslandThreshold => 0.01,
            ParameterField::IslandFalloff => 0.01,
            ParameterField::ContinentThreshold => 0.001,
            ParameterField::ContinentPower => 0.001,
            ParameterField::ContinentBias => 0.001,
            ParameterField::ContinentRadius => 0.001,
            ParameterField::ContinentEdgePower => 0.001,
            ParameterField::ContinentBeltWidth => 0.001,
            ParameterField::ContinentRepulsionStrength => 0.0005,
            ParameterField::ContinentDriftGain => 0.0005,
            ParameterField::ContinentDriftBeltGain => 0.001,
            ParameterField::DetailFrequency => 0.001,
            ParameterField::DetailAmplitude => 0.01,
            ParameterField::MountainFrequency => 0.001,
            ParameterField::MountainRangeCount => 0.5,
            ParameterField::MountainRangeWidth => 0.1,
            ParameterField::MountainRangeStrength => 0.001,
            ParameterField::MountainRangeSpurChance => 0.001,
            ParameterField::MountainRangeSpurStrength => 0.001,
            ParameterField::MountainRangeRoughness => 0.001,
            ParameterField::MountainErosionIterations => 1.0,
            ParameterField::MountainConvergenceBoost => 0.001,
            ParameterField::MountainDivergencePenalty => 0.001,
            ParameterField::MountainShearBoost => 0.0005,
            ParameterField::MountainArcThreshold => 0.001,
            ParameterField::MountainArcStrength => 0.001,
            ParameterField::MountainArcWidthFactor => 0.001,
            ParameterField::MountainThreshold => 0.001,
            ParameterField::HydrologyResolution => 1.0,
            ParameterField::HydrologyRainfall => 0.001,
            ParameterField::HydrologyRainfallVariance => 0.001,
            ParameterField::HydrologyRainfallFrequency => 0.001,
            ParameterField::HydrologyIterations => 1.0,
            ParameterField::HydrologyTimeStep => 0.001,
            ParameterField::HydrologyInfiltrationRate => 0.0005,
            ParameterField::HydrologyBaseflow => 0.0005,
            ParameterField::HydrologyErosionRate => 0.0005,
            ParameterField::HydrologyDepositionRate => 0.0005,
            ParameterField::HydrologySedimentCapacity => 0.0005,
            ParameterField::HydrologyBankfullDepth => 0.05,
            ParameterField::HydrologyFloodplainSoftening => 0.05,
            ParameterField::HydrologyMinimumSlope => 0.00001,
            ParameterField::HydrologyShorelineRadius => 1.0,
            ParameterField::HydrologyShorelineMaxHeight => 0.05,
            ParameterField::HydrologyShorelineSmoothing => 1.0,
            ParameterField::MoistureFrequency => 0.001,
            ParameterField::EquatorTemperature => 0.05,
            ParameterField::PoleTemperature => 0.05,
            ParameterField::LapseRate => 0.0001,
            ParameterField::TemperatureVariation => 0.01,
            ParameterField::HighlandBonus => 0.05,
            _ => 0.005,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "Sea level height in blocks (meters); ocean surface elevation.",
            ParameterField::OceanDepth => "Depth of continental shelf regions below sea level in blocks (meters).",
            ParameterField::DeepOceanDepth => "Depth of abyssal ocean trenches in blocks (meters).",
            ParameterField::ContinentCount => "Target number of large landmasses; higher values split the noise into more continents.",
            ParameterField::ContinentFrequency => "Low-frequency noise controlling continent placement; higher values create more variation per unit area.",
            ParameterField::ContinentThreshold => "Cutoff for land vs ocean; lower thresholds produce more land and wider continents.",
            ParameterField::ContinentPower => "Exponent applied to continent noise; higher values emphasize the interiors of continents.",
            ParameterField::ContinentBias => "Offset added before thresholding; raises this value to favor land creation.",
            ParameterField::ContinentRadius => "Radius of Poisson disk sites influencing continent interiors in normalized map space.",
            ParameterField::ContinentEdgePower => "Controls how sharply continent influence fades toward coastlines.",
            ParameterField::ContinentBeltWidth => "Width of the latitude belt that favors spawning large continent sites.",
            ParameterField::ContinentRepulsionStrength => "Strength of the relaxation push that keeps continent seeds from clumping.",
            ParameterField::ContinentDriftGain => "Base magnitude for simulated plate drift vectors; feeds mountain placement and arcs.",
            ParameterField::ContinentDriftBeltGain => "Additional drift multiplier applied to seeds inside the preferred belt direction.",
            ParameterField::DetailFrequency => "Frequency of mid-scale terrain detail noise; higher values create smaller hills and ridges.",
            ParameterField::DetailAmplitude => "Amplitude of detail noise in blocks (meters); increases contrast in rolling terrain.",
            ParameterField::MountainFrequency => "Base frequency of mountain noise; adjust to cluster mountains closer together or spread them out.",
            ParameterField::MountainHeight => "Peak height above terrain in blocks (meters); realistic mountain elevation.",
            ParameterField::MountainThreshold => "Noise threshold for promoting terrain into mountains; raise to reduce mountain coverage.",
            ParameterField::MountainRangeCount => "Number of long mountain belts seeded across the world; larger planets can support more distinct ranges.",
            ParameterField::MountainRangeWidth => "Average width of a mountain belt in blocks (meters); controls how broad each range appears on the map.",
            ParameterField::MountainRangeStrength => "Extra elevation multiplier applied along the belt centerline; higher values exaggerate relief inside a range.",
            ParameterField::MountainRangeSpurChance => "Probability that a ridge segment sprouts secondary arms; raising it increases branching and cross-range structure.",
            ParameterField::MountainRangeSpurStrength => "Relative elevation boost applied to spur ridges compared to the main belt.",
            ParameterField::MountainRangeRoughness => "Noise amplitude used along the belt to create bulges, gaps, and braided crests.",
            ParameterField::MountainErosionIterations => "Number of smoothing iterations applied to the cached mountain field before hydrology.",
            ParameterField::MountainConvergenceBoost => "Additional strength multiplier for mountains forming on convergent plate boundaries.",
            ParameterField::MountainDivergencePenalty => "Penalty applied to mountain height where plates move apart or stretch.",
            ParameterField::MountainShearBoost => "Strength multiplier contributed by shear motion along transform boundaries.",
            ParameterField::MountainArcThreshold => "Minimum convergence value required to spawn offshore volcanic arcs.",
            ParameterField::MountainArcStrength => "Height multiplier applied to volcanic arcs generated along subduction zones.",
            ParameterField::MountainArcWidthFactor => "Relative width of volcanic arcs compared to their parent mountain range crest.",
            ParameterField::MoistureFrequency => "Frequency of the moisture noise used for biomes; higher values add more variation.",
            ParameterField::EquatorTemperature => "Baseline near-sea-level temperature at the equator in °C.",
            ParameterField::PoleTemperature => "Baseline near-sea-level temperature at the poles in °C.",
            ParameterField::LapseRate => "Temperature drop per block (meter) of elevation gain.",
            ParameterField::TemperatureVariation => "Amplitude of the temperature noise layered over the latitude gradient.",
            ParameterField::HighlandBonus => "Plateau elevation in blocks (meters); raises continental interiors.",
            ParameterField::IslandFrequency => "Noise frequency used for standalone islands; higher values create more island opportunities.",
            ParameterField::IslandThreshold => "Mask threshold islands must exceed to appear; lower values yield more islands.",
            ParameterField::IslandHeight => "Island peak height in blocks (meters) above ocean floor.",
            ParameterField::IslandFalloff => "Exponent controlling how quickly island influence fades away from land; higher values confine islands to deep ocean.",
            ParameterField::HydrologyResolution => "Grid resolution for the coupled water + erosion simulation; higher values capture finer drainage details at the cost of generation time.",
            ParameterField::HydrologyRainfall => "Amount of water injected per hydrology cell; higher values strengthen flow everywhere.",
            ParameterField::HydrologyRainfallVariance => "Scales how strongly rainfall fluctuates across the planet; 0 keeps things uniform, higher values create distinct wet and dry regions.",
            ParameterField::HydrologyRainfallFrequency => "Spatial frequency of rainfall variation; lower values give broad climate belts, higher values produce smaller storm cells.",
            ParameterField::HydrologyIterations => "How many erosion / deposition passes run; more passes allow the river network to equilibrate.",
            ParameterField::HydrologyTimeStep => "Years simulated per iteration. Larger steps accelerate change but can destabilize steep slopes.",
            ParameterField::HydrologyInfiltrationRate => "Fraction of rainfall that soaks into soil before running off, tempering flashy floods.",
            ParameterField::HydrologyBaseflow => "Background groundwater discharge that keeps rivers alive through dry seasons.",
            ParameterField::HydrologyErosionRate => "Strength multiplier for fluvial incision; higher values carve deeper, narrower valleys.",
            ParameterField::HydrologyDepositionRate => "Controls how quickly suspended sediment settles once the flow slackens.",
            ParameterField::HydrologySedimentCapacity => "Baseline carrying capacity of water. Raising it lets rivers haul more material before depositing.",
            ParameterField::HydrologyBankfullDepth => "Target channel depth at bankfull discharge; deeper banks hold more water before flooding.",
            ParameterField::HydrologyFloodplainSoftening => "Vertical smoothing at water edges that shapes wide, gentle floodplains instead of cliffs.",
            ParameterField::HydrologyMinimumSlope => "Numerical slope floor ensuring wetlands and deltas still drain without oscillation.",
            ParameterField::HydrologyShorelineRadius => "Horizontal distance (in blocks) inland from the ocean to treat as shoreline for smoothing and beach generation.",
            ParameterField::HydrologyShorelineMaxHeight => "Maximum elevation above sea level (in blocks) that participates in shoreline smoothing.",
            ParameterField::HydrologyShorelineSmoothing => "Number of blur passes applied to the shoreline mask for smooth beach outlines.",
        }
    }

    fn range_hint(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "16 - 200 blocks (meters)",
            ParameterField::OceanDepth => "10 - 100 blocks (meters)",
            ParameterField::DeepOceanDepth => "20 - 200 blocks (meters)",
            ParameterField::ContinentCount => "1 - 24",
            ParameterField::ContinentFrequency => "0.1 - 4.0",
            ParameterField::ContinentThreshold => "0.05 - 0.60",
            ParameterField::ContinentPower => "0.2 - 5.0",
            ParameterField::ContinentBias => "0.00 - 0.60",
            ParameterField::ContinentRadius => "0.05 - 0.60",
            ParameterField::ContinentEdgePower => "0.2 - 4.0",
            ParameterField::ContinentBeltWidth => "0.05 - 0.45",
            ParameterField::ContinentRepulsionStrength => "0.00 - 0.30",
            ParameterField::ContinentDriftGain => "0.00 - 0.40",
            ParameterField::ContinentDriftBeltGain => "0.0 - 1.2",
            ParameterField::DetailFrequency => "1.0 - 15.0",
            ParameterField::DetailAmplitude => "1 - 30 blocks (meters)",
            ParameterField::MountainFrequency => "0.2 - 8.0",
            ParameterField::MountainHeight => "50 - 500 blocks (meters)",
            ParameterField::MountainThreshold => "0.1 - 0.9",
            ParameterField::MountainRangeCount => "0 - 60 ranges",
            ParameterField::MountainRangeWidth => "40 - 800 blocks (meters)",
            ParameterField::MountainRangeStrength => "0.0 - 3.0",
            ParameterField::MountainRangeSpurChance => "0.0 - 1.0",
            ParameterField::MountainRangeSpurStrength => "0.0 - 1.5",
            ParameterField::MountainRangeRoughness => "0.0 - 2.0",
            ParameterField::MountainErosionIterations => "0 - 8 passes",
            ParameterField::MountainConvergenceBoost => "0.0 - 1.5",
            ParameterField::MountainDivergencePenalty => "0.0 - 1.0",
            ParameterField::MountainShearBoost => "0.0 - 0.6",
            ParameterField::MountainArcThreshold => "0.0 - 1.0",
            ParameterField::MountainArcStrength => "0.0 - 1.5",
            ParameterField::MountainArcWidthFactor => "0.05 - 1.0",
            ParameterField::MoistureFrequency => "0.1 - 6.0",
            ParameterField::EquatorTemperature => "10 - 45 °C",
            ParameterField::PoleTemperature => "-60 - 10 °C",
            ParameterField::LapseRate => "0.001 - 0.020 °C/block",
            ParameterField::TemperatureVariation => "0 - 20",
            ParameterField::HighlandBonus => "0 - 50 blocks (meters)",
            ParameterField::IslandFrequency => "0.1 - 8.0",
            ParameterField::IslandThreshold => "0.00 - 0.99",
            ParameterField::IslandHeight => "0 - 50 blocks (meters)",
            ParameterField::IslandFalloff => "0.1 - 6.0",
            ParameterField::HydrologyResolution => "128 - 4096 cells",
            ParameterField::HydrologyRainfall => "0.1 - 10.0",
            ParameterField::HydrologyRainfallVariance => "0.0 - 2.0",
            ParameterField::HydrologyRainfallFrequency => "0.1 - 6.0",
            ParameterField::HydrologyIterations => "1 - 512 passes",
            ParameterField::HydrologyTimeStep => "0.01 - 5.0 years",
            ParameterField::HydrologyInfiltrationRate => "0.00 - 0.90",
            ParameterField::HydrologyBaseflow => "0.00 - 0.50",
            ParameterField::HydrologyErosionRate => "0.01 - 2.00",
            ParameterField::HydrologyDepositionRate => "0.01 - 2.00",
            ParameterField::HydrologySedimentCapacity => "0.05 - 2.00",
            ParameterField::HydrologyBankfullDepth => "2 - 80 blocks (meters)",
            ParameterField::HydrologyFloodplainSoftening => "0 - 40 blocks (meters)",
            ParameterField::HydrologyMinimumSlope => "0.0001 - 0.05",
            ParameterField::HydrologyShorelineRadius => "16 - 512 blocks (meters)",
            ParameterField::HydrologyShorelineMaxHeight => "0 - 80 blocks (meters)",
            ParameterField::HydrologyShorelineSmoothing => "0 - 8 passes",
        }
    }

    fn get_field_name(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "sea_level",
            ParameterField::OceanDepth => "ocean_depth",
            ParameterField::DeepOceanDepth => "deep_ocean_depth",
            ParameterField::ContinentCount => "continent_count",
            ParameterField::ContinentFrequency => "continent_frequency",
            ParameterField::ContinentThreshold => "continent_threshold",
            ParameterField::ContinentPower => "continent_power",
            ParameterField::ContinentBias => "continent_bias",
            ParameterField::ContinentRadius => "continent_radius",
            ParameterField::ContinentEdgePower => "continent_edge_power",
            ParameterField::ContinentBeltWidth => "continent_belt_width",
            ParameterField::ContinentRepulsionStrength => "continent_repulsion_strength",
            ParameterField::ContinentDriftGain => "continent_drift_gain",
            ParameterField::ContinentDriftBeltGain => "continent_drift_belt_gain",
            ParameterField::DetailFrequency => "detail_frequency",
            ParameterField::DetailAmplitude => "detail_amplitude",
            ParameterField::MountainFrequency => "mountain_frequency",
            ParameterField::MountainHeight => "mountain_height",
            ParameterField::MountainThreshold => "mountain_threshold",
            ParameterField::MountainRangeCount => "mountain_range_count",
            ParameterField::MountainRangeWidth => "mountain_range_width",
            ParameterField::MountainRangeStrength => "mountain_range_strength",
            ParameterField::MountainRangeSpurChance => "mountain_range_spur_chance",
            ParameterField::MountainRangeSpurStrength => "mountain_range_spur_strength",
            ParameterField::MountainRangeRoughness => "mountain_range_roughness",
            ParameterField::MountainErosionIterations => "mountain_erosion_iterations",
            ParameterField::MountainConvergenceBoost => "mountain_convergence_boost",
            ParameterField::MountainDivergencePenalty => "mountain_divergence_penalty",
            ParameterField::MountainShearBoost => "mountain_shear_boost",
            ParameterField::MountainArcThreshold => "mountain_arc_threshold",
            ParameterField::MountainArcStrength => "mountain_arc_strength",
            ParameterField::MountainArcWidthFactor => "mountain_arc_width_factor",
            ParameterField::MoistureFrequency => "moisture_frequency",
            ParameterField::EquatorTemperature => "equator_temp_c",
            ParameterField::PoleTemperature => "pole_temp_c",
            ParameterField::LapseRate => "lapse_rate_c_per_block",
            ParameterField::TemperatureVariation => "temperature_variation",
            ParameterField::HighlandBonus => "highland_bonus",
            ParameterField::IslandFrequency => "island_frequency",
            ParameterField::IslandThreshold => "island_threshold",
            ParameterField::IslandHeight => "island_height",
            ParameterField::IslandFalloff => "island_falloff",
            ParameterField::HydrologyResolution => "hydrology_resolution",
            ParameterField::HydrologyRainfall => "hydrology_rainfall",
            ParameterField::HydrologyRainfallVariance => "hydrology_rainfall_variance",
            ParameterField::HydrologyRainfallFrequency => "hydrology_rainfall_frequency",
            ParameterField::HydrologyIterations => "hydrology_iterations",
            ParameterField::HydrologyTimeStep => "hydrology_time_step",
            ParameterField::HydrologyInfiltrationRate => "hydrology_infiltration_rate",
            ParameterField::HydrologyBaseflow => "hydrology_baseflow",
            ParameterField::HydrologyErosionRate => "hydrology_erosion_rate",
            ParameterField::HydrologyDepositionRate => "hydrology_deposition_rate",
            ParameterField::HydrologySedimentCapacity => "hydrology_sediment_capacity",
            ParameterField::HydrologyBankfullDepth => "hydrology_bankfull_depth",
            ParameterField::HydrologyFloodplainSoftening => "hydrology_floodplain_softening",
            ParameterField::HydrologyMinimumSlope => "hydrology_minimum_slope",
            ParameterField::HydrologyShorelineRadius => "hydrology_shoreline_radius",
            ParameterField::HydrologyShorelineMaxHeight => "hydrology_shoreline_max_height",
            ParameterField::HydrologyShorelineSmoothing => "hydrology_shoreline_smoothing",
        }
    }

    fn is_changed(&self, working: &WorldGenConfig, defaults: &WorldGenConfig) -> bool {
        let working_val = self.working_value(working);
        let default_val = self.working_value(defaults);
        (working_val - default_val).abs() > self.epsilon()
    }
}

const CORE_FIELDS: &[ParameterField] = &[
    ParameterField::SeaLevel,
    ParameterField::OceanDepth,
    ParameterField::DeepOceanDepth,
];

const CONTINENT_FIELDS: &[ParameterField] = &[
    ParameterField::ContinentCount,
    ParameterField::ContinentFrequency,
    ParameterField::ContinentThreshold,
    ParameterField::ContinentPower,
    ParameterField::ContinentBias,
    ParameterField::ContinentRadius,
    ParameterField::ContinentEdgePower,
    ParameterField::ContinentBeltWidth,
    ParameterField::ContinentRepulsionStrength,
    ParameterField::ContinentDriftGain,
    ParameterField::ContinentDriftBeltGain,
];

const TERRAIN_FIELDS: &[ParameterField] = &[
    ParameterField::DetailFrequency,
    ParameterField::DetailAmplitude,
    ParameterField::HighlandBonus,
];

const MOUNTAIN_FIELDS: &[ParameterField] = &[
    ParameterField::MountainFrequency,
    ParameterField::MountainThreshold,
    ParameterField::MountainHeight,
    ParameterField::MountainRangeCount,
    ParameterField::MountainRangeWidth,
    ParameterField::MountainRangeStrength,
    ParameterField::MountainRangeSpurChance,
    ParameterField::MountainRangeSpurStrength,
    ParameterField::MountainRangeRoughness,
    ParameterField::MountainErosionIterations,
    ParameterField::MountainConvergenceBoost,
    ParameterField::MountainDivergencePenalty,
    ParameterField::MountainShearBoost,
    ParameterField::MountainArcThreshold,
    ParameterField::MountainArcStrength,
    ParameterField::MountainArcWidthFactor,
];

const CLIMATE_FIELDS: &[ParameterField] = &[
    ParameterField::MoistureFrequency,
    ParameterField::EquatorTemperature,
    ParameterField::PoleTemperature,
    ParameterField::LapseRate,
    ParameterField::TemperatureVariation,
];

const ISLAND_FIELDS: &[ParameterField] = &[
    ParameterField::IslandFrequency,
    ParameterField::IslandThreshold,
    ParameterField::IslandHeight,
    ParameterField::IslandFalloff,
];

const HYDROLOGY_FIELDS: &[ParameterField] = &[
    ParameterField::HydrologyResolution,
    ParameterField::HydrologyRainfall,
    ParameterField::HydrologyRainfallVariance,
    ParameterField::HydrologyRainfallFrequency,
    ParameterField::HydrologyIterations,
    ParameterField::HydrologyTimeStep,
    ParameterField::HydrologyInfiltrationRate,
    ParameterField::HydrologyBaseflow,
    ParameterField::HydrologyErosionRate,
    ParameterField::HydrologyDepositionRate,
    ParameterField::HydrologySedimentCapacity,
    ParameterField::HydrologyBankfullDepth,
    ParameterField::HydrologyFloodplainSoftening,
    ParameterField::HydrologyMinimumSlope,
    ParameterField::HydrologyShorelineRadius,
    ParameterField::HydrologyShorelineMaxHeight,
    ParameterField::HydrologyShorelineSmoothing,
];

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    materials: Res<ButtonMaterials>,
    _asset_server: Res<AssetServer>,
) {
    let planet_sizes = vec![
        PlanetSize::Tiny,
        PlanetSize::Small,
        PlanetSize::Medium,
        PlanetSize::Default,
        PlanetSize::Large,
        PlanetSize::Huge,
    ];

    let working = WorldGenConfig::default();
    let visualization = MapVisualization::Biomes;

    let active = working.clone();
    let mut initial_phases = Vec::new();
    let generator = WorldGenerator::with_progress(active.clone(), |phase| {
        info!("Initial world generation phase: {:?}", phase);
        initial_phases.push(phase);
    });

    let planet_size_index = find_closest_size_index(&planet_sizes, working.planet_size as i32);

    let defaults = WorldGenConfig::default();
    let changed_parameters = HashMap::new();

    commands.insert_resource(WorldBuilderState {
        working,
        active,
        defaults,
        generator,
        planet_sizes,
        planet_size_index,
        visualization,
        active_tab: ParameterTab::Core,
        repaint_requested: true,
        selection: None,
        changed_parameters,
        camera_zoom: 2.0, // Start zoomed in to fill the window
        camera_translation: Vec2::ZERO,
        is_panning: false,
        last_mouse_position: None,
        show_popup: false,
        popup_world_pos: None,
        detail_center: None,
        phase_history: initial_phases,
        phase_history_dirty: true,
    });

    // Map texture and sprite
    let mut map_image = Image::new_fill(
        Extent3d {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8, 0u8, 0u8, 255u8],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    map_image.sampler = ImageSampler::nearest();
    let map_handle = images.add(map_image);

    commands.insert_resource(MapTextures {
        map: map_handle.clone(),
    });

    // Camera for the map (world space) - only spawn if targeting main window
    let mut camera_bundle = Camera2dBundle {
        camera: Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.03)),
            order: 0, // Main camera
            ..default()
        },
        ..default()
    };
    camera_bundle.projection.scale = 0.5; // Start zoomed in to fill window

    commands.spawn((
        camera_bundle,
        MapCamera,
        RenderLayers::default(), // Main camera sees default layer 0
    ));

    // Map sprite in world space
    commands.spawn((
        SpriteBundle {
            texture: map_handle.clone(),
            ..default()
        },
        MapSprite,
    ));

    // Selection marker in world space
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(12.0, 12.0)),
                color: Color::srgb(1.0, 0.3, 0.2),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 10.0), // Above the map
            visibility: Visibility::Hidden,
            ..default()
        },
        SelectionMarker,
    ));

    // Location popup (UI element that follows world space)
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                visibility: Visibility::Hidden,
                z_index: ZIndex::Global(1000), // Above everything
                ..default()
            },
            LocationPopup,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default() // Use Bevy's default font
                    },
                ),
                LocationPopupText,
            ));
        });

    // Visualization buttons panel at top of map
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.08, 0.09, 0.12, 0.85)),
            z_index: ZIndex::Global(100),
            ..default()
        })
        .with_children(|parent| {
            for mode in MapVisualization::ALL {
                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            width: Val::Px(90.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: materials.normal,
                        border_color: BorderColor(Color::srgba(0.25, 0.28, 0.35, 0.6)),
                        ..default()
                    })
                    .insert(VisualizationButton { mode })
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section(
                            mode.label(),
                            TextStyle {
                                font_size: 12.0,
                                color: Color::srgb(0.9, 0.93, 1.0),
                                ..default()
                            },
                        ));
                    });
            }
        });

    // Status text for detail inspection
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        text: Text::from_section(
            "Left-click to inspect blocks | Middle-click to pan | Right-click for info",
            TextStyle {
                font_size: 14.0,
                color: Color::srgba(0.9, 0.9, 0.9, 0.8),
                ..default()
            },
        ),
        ..default()
    });

    let control_window = commands
        .spawn(Window {
            title: "World Builder - Controls".into(),
            resolution: WindowResolution::new(520.0, 720.0),
            present_mode: PresentMode::AutoVsync,
            resizable: true,
            ..default()
        })
        .id();

    let control_camera = commands
        .spawn((Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(control_window)),
                clear_color: ClearColorConfig::Custom(Color::srgb(0.06, 0.07, 0.09)),
                order: 1,
                ..default()
            },
            ..default()
        },))
        .id();

    build_control_panel(&mut commands, control_camera, &materials);
}

fn build_control_panel(
    commands: &mut Commands,
    control_camera: Entity,
    materials: &ButtonMaterials,
) {
    let root = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(Val::Px(20.0)),
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.04, 0.045, 0.06)),
                ..default()
            },
            TargetCamera(control_camera),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        // Header with title and action buttons
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::bottom(Val::Px(12.0)),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                border_color: BorderColor(Color::srgba(0.3, 0.35, 0.45, 0.5)),
                ..default()
            })
            .with_children(|header| {
                header.spawn(TextBundle::from_section(
                    "WORLD BUILDER",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::srgb(0.9, 0.92, 0.98),
                        ..default()
                    },
                ));

                // Action buttons in header
                header
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|buttons| {
                        buttons
                            .spawn(ButtonBundle {
                                style: Style {
                                    width: Val::Px(150.0),
                                    height: Val::Px(38.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.15, 0.25, 0.4, 0.95,
                                )),
                                border_color: BorderColor(Color::srgba(0.35, 0.45, 0.6, 0.8)),
                                ..default()
                            })
                            .insert(RegenerateButton)
                            .with_children(|b| {
                                b.spawn(TextBundle::from_section(
                                    "GENERATE WORLD",
                                    TextStyle {
                                        font_size: 14.0,
                                        color: Color::srgb(0.95, 0.97, 1.0),
                                        ..default()
                                    },
                                ));
                            });

                        // Save to Source button
                        buttons
                            .spawn(ButtonBundle {
                                style: Style {
                                    width: Val::Px(120.0),
                                    height: Val::Px(28.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.18, 0.12, 0.25, 0.95,
                                )),
                                border_color: BorderColor(Color::srgba(0.45, 0.25, 0.45, 0.8)),
                                ..default()
                            })
                            .insert(SaveToSourceButton)
                            .with_children(|b| {
                                b.spawn(TextBundle::from_section(
                                    "SAVE TO CODE",
                                    TextStyle {
                                        font_size: 14.0,
                                        color: Color::srgb(0.98, 0.9, 0.93),
                                        ..default()
                                    },
                                ));
                            });
                    });
            });

        parent.spawn((
            TextBundle::from_section(
                "Last generation phases: (pending)",
                TextStyle {
                    font_size: 14.0,
                    color: Color::srgb(0.74, 0.78, 0.9),
                    ..default()
                },
            )
            .with_style(Style {
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            }),
            PhaseStatusText,
        ));

        // Main content area - World Parameters
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|main_content| {
                // World size section
                main_content
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.08, 0.09, 0.12, 0.5)),
                        border_color: BorderColor(Color::srgba(0.2, 0.22, 0.28, 0.5)),
                        ..default()
                    })
                    .with_children(|size_section| {
                        size_section.spawn(TextBundle::from_section(
                            "WORLD SIZE",
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(0.65, 0.7, 0.8, 0.9),
                                ..default()
                            },
                        ));

                        size_section
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn(button_bundle(materials, Vec2::new(32.0, 28.0)))
                                    .insert(PlanetSizeButton { delta: -1 })
                                    .with_children(|button| {
                                        button.spawn(TextBundle::from_section(
                                            "<",
                                            TextStyle {
                                                font_size: 18.0,
                                                color: Color::srgb(0.9, 0.93, 1.0),
                                                ..default()
                                            },
                                        ));
                                    });
                                let mut bundle = TextBundle::from_section(
                                    "",
                                    TextStyle {
                                        font_size: 15.0,
                                        color: Color::srgb(0.9, 0.92, 1.0),
                                        ..default()
                                    },
                                );
                                bundle.text.justify = JustifyText::Center;
                                bundle.style = Style {
                                    min_width: Val::Px(280.0),
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                };
                                row.spawn(bundle).insert(WorldSizeLabel);

                                row.spawn(button_bundle(materials, Vec2::new(32.0, 28.0)))
                                    .insert(PlanetSizeButton { delta: 1 })
                                    .with_children(|button| {
                                        button.spawn(TextBundle::from_section(
                                            ">",
                                            TextStyle {
                                                font_size: 18.0,
                                                color: Color::srgb(0.9, 0.93, 1.0),
                                                ..default()
                                            },
                                        ));
                                    });
                            });
                    });

                // Parameter tabs
                main_content
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            padding: UiRect::vertical(Val::Px(8.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|row| {
                        for tab in ParameterTab::ALL {
                            row.spawn(ButtonBundle {
                                style: Style {
                                    width: Val::Px(120.0),
                                    height: Val::Px(36.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: materials.tab_normal,
                                border_color: BorderColor(Color::srgba(0.25, 0.28, 0.35, 0.6)),
                                ..default()
                            })
                            .insert(TabButton { tab })
                            .with_children(|button| {
                                button.spawn(TextBundle::from_section(
                                    tab.label().to_uppercase(),
                                    TextStyle {
                                        font_size: 13.0,
                                        color: Color::srgb(0.85, 0.88, 0.95),
                                        ..default()
                                    },
                                ));
                            });
                        }
                    });

                // Parameter panels with scrollable container
                main_content
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            border: UiRect::all(Val::Px(1.0)),
                            min_height: Val::Px(400.0),
                            max_height: Val::Px(500.0),
                            overflow: Overflow::clip_y(),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.08, 0.09, 0.12, 0.5)),
                        border_color: BorderColor(Color::srgba(0.2, 0.22, 0.28, 0.5)),
                        ..default()
                    })
                    .with_children(|container| {
                        // Scrollable content area
                        container
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    position_type: PositionType::Relative,
                                    top: Val::Px(0.0),
                                    ..default()
                                },
                                ..default()
                            })
                            .insert(ScrollContent)
                            .with_children(|sections| {
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Core,
                                    CORE_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Continents,
                                    CONTINENT_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Terrain,
                                    TERRAIN_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Mountains,
                                    MOUNTAIN_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Climate,
                                    CLIMATE_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Islands,
                                    ISLAND_FIELDS,
                                    ParameterTab::Core,
                                );
                                spawn_tab_section(
                                    sections,
                                    materials,
                                    ParameterTab::Hydrology,
                                    HYDROLOGY_FIELDS,
                                    ParameterTab::Core,
                                );
                            });
                    });
            });
    });
}

fn button_bundle(materials: &ButtonMaterials, size: Vec2) -> ButtonBundle {
    ButtonBundle {
        style: Style {
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        background_color: materials.normal,
        border_color: BorderColor(Color::srgba(0.25, 0.28, 0.35, 0.6)),
        ..default()
    }
}

fn field_step(field: ParameterField) -> f32 {
    match field {
        ParameterField::SeaLevel => 2.0,
        ParameterField::OceanDepth => 2.0,
        ParameterField::DeepOceanDepth => 2.0,
        ParameterField::ContinentCount => 1.0,
        ParameterField::ContinentFrequency => 0.05,
        ParameterField::ContinentThreshold => 0.02,
        ParameterField::ContinentPower => 0.05,
        ParameterField::ContinentBias => 0.01,
        ParameterField::ContinentRadius => 0.01,
        ParameterField::ContinentEdgePower => 0.05,
        ParameterField::ContinentBeltWidth => 0.01,
        ParameterField::ContinentRepulsionStrength => 0.005,
        ParameterField::ContinentDriftGain => 0.005,
        ParameterField::ContinentDriftBeltGain => 0.02,
        ParameterField::DetailFrequency => 0.1,
        ParameterField::DetailAmplitude => 1.0,
        ParameterField::MountainFrequency => 0.1,
        ParameterField::MountainHeight => 4.0,
        ParameterField::MountainThreshold => 0.02,
        ParameterField::MountainRangeCount => 1.0,
        ParameterField::MountainRangeWidth => 10.0,
        ParameterField::MountainRangeStrength => 0.1,
        ParameterField::MountainRangeSpurChance => 0.05,
        ParameterField::MountainRangeSpurStrength => 0.05,
        ParameterField::MountainRangeRoughness => 0.05,
        ParameterField::MountainErosionIterations => 1.0,
        ParameterField::MountainConvergenceBoost => 0.05,
        ParameterField::MountainDivergencePenalty => 0.05,
        ParameterField::MountainShearBoost => 0.02,
        ParameterField::MountainArcThreshold => 0.05,
        ParameterField::MountainArcStrength => 0.05,
        ParameterField::MountainArcWidthFactor => 0.05,
        ParameterField::MoistureFrequency => 0.05,
        ParameterField::EquatorTemperature => 1.0,
        ParameterField::PoleTemperature => 1.0,
        ParameterField::LapseRate => 1.0,
        ParameterField::TemperatureVariation => 0.5,
        ParameterField::HighlandBonus => 2.0,
        ParameterField::IslandFrequency => 0.1,
        ParameterField::IslandThreshold => 0.02,
        ParameterField::IslandHeight => 2.0,
        ParameterField::IslandFalloff => 0.1,
        ParameterField::HydrologyResolution => 128.0,
        ParameterField::HydrologyRainfall => 0.1,
        ParameterField::HydrologyRainfallVariance => 0.05,
        ParameterField::HydrologyRainfallFrequency => 0.05,
        ParameterField::HydrologyIterations => 10.0,
        ParameterField::HydrologyTimeStep => 0.1,
        ParameterField::HydrologyInfiltrationRate => 0.02,
        ParameterField::HydrologyBaseflow => 0.02,
        ParameterField::HydrologyErosionRate => 0.02,
        ParameterField::HydrologyDepositionRate => 0.02,
        ParameterField::HydrologySedimentCapacity => 0.02,
        ParameterField::HydrologyBankfullDepth => 1.0,
        ParameterField::HydrologyFloodplainSoftening => 1.0,
        ParameterField::HydrologyMinimumSlope => 0.0005,
        ParameterField::HydrologyShorelineRadius => 16.0,
        ParameterField::HydrologyShorelineMaxHeight => 1.0,
        ParameterField::HydrologyShorelineSmoothing => 1.0,
    }
}

fn spawn_tab_section(
    parent: &mut ChildBuilder,
    materials: &ButtonMaterials,
    tab: ParameterTab,
    fields: &[ParameterField],
    active_tab: ParameterTab,
) {
    let display = if tab == active_tab {
        Display::Flex
    } else {
        Display::None
    };

    parent
        .spawn((
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    display,
                    ..default()
                },
                ..default()
            },
            TabSection { tab },
        ))
        .with_children(|section| {
            for &field in fields {
                spawn_parameter_row(section, materials, field);
            }
        });
}

fn spawn_parameter_row(
    parent: &mut ChildBuilder,
    materials: &ButtonMaterials,
    field: ParameterField,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            ..default()
        })
        .with_children(|column| {
            column
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(
                        TextBundle::from_section(
                            field.label(),
                            TextStyle {
                                font_size: 15.0,
                                color: Color::srgb(0.78, 0.82, 0.94),
                                ..default()
                            },
                        )
                        .with_style(Style {
                            min_width: Val::Px(170.0),
                            ..default()
                        }),
                    );

                    row.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(6.0),
                            padding: UiRect::all(Val::Px(4.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.06, 0.07, 0.1, 0.85)),
                        border_color: BorderColor(Color::srgba(0.18, 0.2, 0.25, 0.4)),
                        ..default()
                    })
                    .with_children(|controls| {
                        controls
                            .spawn(button_bundle(materials, Vec2::new(26.0, 24.0)))
                            .insert(ParameterButton {
                                field,
                                delta: -field_step(field),
                            })
                            .with_children(|button| {
                                button.spawn(TextBundle::from_section(
                                    "-",
                                    TextStyle {
                                        font_size: 15.0,
                                        color: Color::srgb(0.9, 0.93, 1.0),
                                        ..default()
                                    },
                                ));
                            });

                        let mut value_bundle = TextBundle::from_section(
                            "",
                            TextStyle {
                                font_size: 15.0,
                                color: Color::srgb(0.95, 0.97, 1.0),
                                ..default()
                            },
                        );
                        value_bundle.style = Style {
                            min_width: Val::Px(92.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        };

                        controls
                            .spawn(value_bundle)
                            .insert(ParameterValueText { field });

                        controls
                            .spawn(button_bundle(materials, Vec2::new(26.0, 24.0)))
                            .insert(ParameterButton {
                                field,
                                delta: field_step(field),
                            })
                            .with_children(|button| {
                                button.spawn(TextBundle::from_section(
                                    "+",
                                    TextStyle {
                                        font_size: 15.0,
                                        color: Color::srgb(0.9, 0.93, 1.0),
                                        ..default()
                                    },
                                ));
                            });

                        controls
                            .spawn(button_bundle(materials, Vec2::new(56.0, 24.0)))
                            .insert(ParameterResetButton { field })
                            .with_children(|button| {
                                button.spawn(TextBundle::from_section(
                                    "Reset",
                                    TextStyle {
                                        font_size: 13.0,
                                        color: Color::srgb(0.92, 0.95, 1.0),
                                        ..default()
                                    },
                                ));
                            });
                    });
                });

            let desc = format!(
                "{}
Range: {}",
                field.description(),
                field.range_hint()
            );

            column.spawn(
                TextBundle::from_section(
                    desc,
                    TextStyle {
                        font_size: 13.0,
                        color: Color::srgb(0.6, 0.64, 0.74),
                        ..default()
                    },
                )
                .with_style(Style {
                    max_width: Val::Px(420.0),
                    ..default()
                }),
            );
        });
}

fn handle_parameter_buttons(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &ParameterButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<WorldBuilderState>,
) {
    for (interaction, mut color, button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                button.field.adjust(&mut state.working, button.delta);

                // Check if this parameter has changed from defaults
                let field_name = button.field.get_field_name();
                let is_changed = button.field.is_changed(&state.working, &state.defaults);
                state
                    .changed_parameters
                    .insert(field_name.to_string(), is_changed);
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn handle_parameter_reset_buttons(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &ParameterResetButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<WorldBuilderState>,
) {
    for (interaction, mut color, button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                reset_parameter(button.field, &mut state);
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn handle_world_size_buttons(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &PlanetSizeButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<WorldBuilderState>,
) {
    let len = state.planet_sizes.len() as i32;
    for (interaction, mut color, button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                if len == 0 {
                    continue;
                }
                let current = state.planet_size_index as i32;
                let next = (current + button.delta).clamp(0, len - 1);
                state.planet_size_index = next as usize;
                apply_planet_size(&mut state);
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn handle_visualization_buttons(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &VisualizationButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<WorldBuilderState>,
) {
    for (interaction, mut color, button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                if state.visualization != button.mode {
                    state.visualization = button.mode;
                    state.repaint_requested = true;
                }
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn sync_visualization_highlights(
    state: Res<WorldBuilderState>,
    materials: Res<ButtonMaterials>,
    mut query: Query<(&VisualizationButton, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    for (button, interaction, mut color) in query.iter_mut() {
        let idle = if state.visualization == button.mode {
            materials.active
        } else {
            materials.normal
        };

        *color = match *interaction {
            Interaction::Pressed => materials.pressed,
            Interaction::Hovered => materials.hovered,
            Interaction::None => idle,
        };
    }
}

fn handle_tab_buttons(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &TabButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<WorldBuilderState>,
) {
    for (interaction, mut color, button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                if state.active_tab != button.tab {
                    state.active_tab = button.tab;
                }
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn sync_tab_highlights(
    state: Res<WorldBuilderState>,
    materials: Res<ButtonMaterials>,
    mut query: Query<(&TabButton, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    for (button, interaction, mut color) in query.iter_mut() {
        let idle = if state.active_tab == button.tab {
            materials.tab_active
        } else {
            materials.tab_normal
        };

        *color = match *interaction {
            Interaction::Pressed => materials.pressed,
            Interaction::Hovered => materials.hovered,
            Interaction::None => idle,
        };
    }
}

fn update_tab_sections(state: Res<WorldBuilderState>, mut query: Query<(&TabSection, &mut Style)>) {
    if !state.is_changed() {
        return;
    }
    for (section, mut style) in query.iter_mut() {
        style.display = if section.tab == state.active_tab {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn update_phase_status_text(
    mut state: ResMut<WorldBuilderState>,
    mut query: Query<&mut Text, With<PhaseStatusText>>,
) {
    if !state.phase_history_dirty {
        return;
    }

    state.phase_history_dirty = false;

    let Ok(mut text) = query.get_single_mut() else {
        return;
    };

    let content = if state.phase_history.is_empty() {
        "Last generation phases: (pending)".to_string()
    } else {
        let phases: Vec<&'static str> = state
            .phase_history
            .iter()
            .map(|phase| phase_display_name(*phase))
            .collect();
        format!("Last generation phases: {}", phases.join(" → "))
    };

    text.sections[0].value = content;
}

fn handle_regenerate_button(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RegenerateButton>),
    >,
    mut regenerate: EventWriter<RegenerateRequested>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                regenerate.send_default();
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn handle_map_zoom(
    mut wheel_events: EventReader<MouseWheel>,
    mut camera_query: Query<(&mut OrthographicProjection, &mut Transform), With<Camera2d>>,
    mut state: ResMut<WorldBuilderState>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    // Only process events if the main window has focus
    let Ok(window) = windows.get_single() else {
        return;
    };

    if !window.focused {
        return;
    }

    for event in wheel_events.read() {
        // Use the mouse wheel Y delta for zooming (reduced sensitivity)
        let zoom_delta = -event.y * 0.01; // Smooth zoom
        state.camera_zoom = (state.camera_zoom * (1.0 + zoom_delta)).clamp(0.1, 50.0);

        // Update the first camera we find (should be the map camera)
        // In Bevy: projection.scale < 1.0 = zoomed IN, > 1.0 = zoomed OUT
        // But our camera_zoom is opposite: > 1.0 = zoomed IN, < 1.0 = zoomed OUT
        // So we need to invert it for the projection
        for (mut projection, _transform) in camera_query.iter_mut() {
            projection.scale = 1.0 / state.camera_zoom;
            break; // Only update the first camera
        }
    }
}

fn handle_map_pan(
    mut state: ResMut<WorldBuilderState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = window_query.get_single() else {
        return;
    };

    if !window.focused {
        return;
    }

    // Check if we should start or stop panning (use middle mouse button)
    if mouse_button.just_pressed(MouseButton::Middle) {
        if let Some(cursor_position) = window.cursor_position() {
            state.is_panning = true;
            state.last_mouse_position = Some(cursor_position);
        }
    }

    if mouse_button.just_released(MouseButton::Middle) {
        state.is_panning = false;
        state.last_mouse_position = None;
    }

    // Handle panning motion
    if state.is_panning {
        let mut delta = Vec2::ZERO;
        for event in motion_events.read() {
            delta += event.delta;
        }

        if delta.length() > 0.01 {
            // Adjust for zoom level and invert Y (screen coords are inverted)
            delta *= state.camera_zoom;
            delta.y = -delta.y;

            state.camera_translation -= delta;

            // Update the first camera we find
            for mut camera in camera_query.iter_mut() {
                camera.translation.x = state.camera_translation.x;
                camera.translation.y = state.camera_translation.y;
                break; // Only update first camera
            }
        }
    }
}

fn update_location_popup(
    state: Res<WorldBuilderState>,
    mut popup_query: Query<
        (&mut Visibility, &mut Transform),
        (With<LocationPopup>, Without<Camera2d>),
    >,
    mut text_query: Query<&mut Text, With<LocationPopupText>>,
) {
    // Only show popup if we have a selection and popup is enabled
    if let Some(selection) = state.selection {
        if state.show_popup {
            // Show the popup
            for (mut visibility, mut transform) in popup_query.iter_mut() {
                *visibility = Visibility::Visible;

                // Position popup near the selected location (in world space)
                if let Some((world_x, world_z)) = state.popup_world_pos {
                    transform.translation.x = world_x;
                    transform.translation.y = world_z + 30.0; // Offset above the selection
                    transform.translation.z = 100.0; // Above everything else
                }
            }

            // Update popup text
            for mut text in text_query.iter_mut() {
                text.sections[0].value = format!(
                    "Pos: ({:.0}, {:.0})\nHeight: {:.1}m\nBiome: {}\nTemp: {:.1}°C\nMoisture: {:.1}%",
                    selection.world_x,
                    selection.world_z,
                    selection.height,
                    format!("{:?}", selection.biome),
                    selection.temperature_c,
                    selection.moisture * 100.0
                );
            }
        } else {
            // Hide the popup
            for (mut visibility, _) in popup_query.iter_mut() {
                *visibility = Visibility::Hidden;
            }
        }
    } else {
        // No selection, hide popup
        for (mut visibility, _) in popup_query.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_detail_view(
    mut commands: Commands,
    state: Res<WorldBuilderState>,
    mut images: ResMut<Assets<Image>>,
    mut detail_window: ResMut<DetailWindow>,
    mut detail_query: Query<&mut Handle<Image>, With<DetailImage>>,
) {
    // Check if we need to create or update the detail window
    if let Some(center) = state.detail_center {
        // Create detail window if it doesn't exist
        if detail_window.entity.is_none() {
            create_detail_window(
                &mut commands,
                &mut detail_window,
                &mut images,
                &state.generator,
                center,
                state.visualization,
            );
            detail_window.last_center = Some(center);
            return; // Window creation happens this frame
        }

        // Check if the center has changed (new click location)
        let should_render = detail_window.last_center != Some(center);

        if should_render {
            // Render the 512x512 block-level view
            if let Ok(mut image_handle) = detail_query.get_single_mut() {
                let mut detail_image = Image::new_fill(
                    Extent3d {
                        width: 512,
                        height: 512,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[0, 0, 0, 255],
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                detail_image.sampler = ImageSampler::nearest();

                // Render 512x512 blocks centered at the click position
                render_block_detail(
                    &mut detail_image,
                    &state.generator,
                    center,
                    state.visualization,
                );

                // Update the image handle
                let new_handle = images.add(detail_image);
                *image_handle = new_handle;

                detail_window.last_center = Some(center);
                info!(
                    "Rendered detail view at world position ({:.0}, {:.0})",
                    center.x, center.y
                );
            }
        }
    }
}

fn create_detail_window(
    commands: &mut Commands,
    detail_window: &mut DetailWindow,
    images: &mut Assets<Image>,
    generator: &WorldGenerator,
    center: Vec2,
    visualization: MapVisualization,
) {
    // Create the detail window
    let window_entity = commands
        .spawn(Window {
            title: "World Builder - Block Detail (512x512)".into(),
            resolution: WindowResolution::new(512.0, 512.0),
            present_mode: PresentMode::AutoVsync,
            resizable: false,
            ..default()
        })
        .id();

    // Create camera for detail window (on render layer 1)
    let camera_entity = commands
        .spawn((
            Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
                ..default()
            },
            DetailWindowCamera,
            RenderLayers::layer(1), // Detail camera only sees layer 1
        ))
        .id();

    // Create initial image with the actual detail content
    let mut detail_image = Image::new_fill(
        Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    detail_image.sampler = ImageSampler::nearest();

    // Render the initial detail view
    render_block_detail(&mut detail_image, generator, center, visualization);
    let image_handle = images.add(detail_image);

    // Create sprite to display the detail image (on render layer 1)
    commands.spawn((
        SpriteBundle {
            texture: image_handle,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        DetailImage,
        RenderLayers::layer(1), // Detail sprite only renders on layer 1
    ));

    // Store references
    detail_window.entity = Some(window_entity);
    detail_window.camera = Some(camera_entity);
}

fn render_block_detail(
    image: &mut Image,
    generator: &WorldGenerator,
    center: Vec2,
    visualization: MapVisualization,
) {
    let data = &mut image.data;

    // Each pixel represents exactly 1 block
    // Center the 512x512 area around the clicked position
    let start_x = center.x - 256.0;
    let start_z = center.y - 256.0;

    info!(
        "Rendering detail view from ({:.0}, {:.0}) to ({:.0}, {:.0})",
        start_x,
        start_z,
        start_x + 512.0,
        start_z + 512.0
    );

    // Sample the center point to see what we're looking at
    let sample_height = generator.get_height(center.x, center.y);
    info!(
        "Center point at ({:.0}, {:.0}): height={:.1}",
        center.x, center.y, sample_height
    );

    // Render the detail view with the same coordinate system as the main map
    for y in 0..512 {
        for x in 0..512 {
            let world_x = start_x + x as f32;
            // Direct mapping - no flip needed since we're handling it in the click handler
            let world_z = start_z + y as f32;

            // Get the color for this exact block
            let color = color_for_mode(generator, world_x, world_z, visualization);

            let index = ((y * 512 + x) * 4) as usize;
            data[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn handle_save_to_source_button(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SaveToSourceButton>),
    >,
    state: Res<WorldBuilderState>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;

                // Detect changes from defaults
                let changes = source_updater::detect_changes(&state.working, &state.defaults);

                if changes.is_empty() {
                    info!("No changes detected from defaults");
                } else {
                    info!("Detected {} changed parameters:", changes.len());
                    for change in &changes {
                        info!("  {} = {}", change.const_name, change.new_value);
                    }

                    // Update the source file
                    if let Err(err) = source_updater::update_source_file(&changes) {
                        warn!("Failed to update source file: {err}");
                    } else {
                        info!(
                            "Successfully updated src/world/defaults.rs with {} changes",
                            changes.len()
                        );
                    }
                }
            }
            Interaction::Hovered => *color = materials.hovered,
            Interaction::None => *color = materials.normal,
        }
    }
}

fn update_value_text(
    state: Res<WorldBuilderState>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<WorldSizeLabel>>,
        Query<(&ParameterValueText, &mut Text)>,
    )>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = text_queries.p0().get_single_mut() {
        let working_size = state.working.planet_size;
        let active_size = state.active.planet_size;
        let size_label = state
            .planet_sizes
            .get(state.planet_size_index)
            .map(|size| planet_size_label(*size))
            .unwrap_or("Custom");
        let mut display = format!(
            "{} • {} blocks • {:.1} km",
            size_label,
            working_size,
            working_size as f64 / 1000.0
        );
        if working_size != active_size {
            display.push_str(" *");
        }
        text.sections[0].value = display;
    }

    for (component, mut text) in text_queries.p1().iter_mut() {
        let mut value = component.field.format_value(&state.working);
        if component.field.differs(&state.working, &state.active) {
            value.push_str(" *");
        }
        text.sections[0].value = value;
    }
}

fn update_selection_text(
    state: Res<WorldBuilderState>,
    mut query: Query<&mut Text, With<SelectionSummaryText>>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut text) = query.get_single_mut() {
        if let Some(selection) = state.selection {
            text.sections[0].value = format!(
                "Position: ({:.0}, {:.0})\n\nTerrain:\n  • Height: {:.1}\n  • Biome: {:?}\n\nClimate:\n  • Temp: {:.1}°C\n  • Moisture: {:.2}\n  • Rainfall: {:.2}\n\nWater:\n  • Level: {:.1}\n  • River: {:.2}\n  • Major River: {:.2}",
                selection.world_x,
                selection.world_z,
                selection.height,
                selection.biome,
                selection.temperature_c,
                selection.moisture,
                selection.rainfall,
                selection.water_level,
                selection.river_intensity,
                selection.major_river,
            );
            text.sections[0].style.font_size = 12.0;
        } else {
            text.sections[0].value = "Click on the map to inspect a location.".to_string();
            text.sections[0].style.font_size = 13.0;
        }
    }
}

fn apply_selection_marker(
    state: Res<WorldBuilderState>,
    mut marker_query: Query<(&mut Transform, &mut Visibility), With<SelectionMarker>>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok((mut transform, mut visibility)) = marker_query.get_single_mut() {
        if let Some(selection) = state.selection {
            // Convert world terrain coordinates to map sprite position
            let map_size = state.generator.planet_size() as f32;
            let u = selection.world_x / map_size;
            let v = selection.world_z / map_size;

            // Convert to world space position on the map sprite
            let x = (u - 0.5) * MAP_WIDTH as f32;
            let y = (0.5 - v) * MAP_HEIGHT as f32; // Invert Y axis

            transform.translation.x = x;
            transform.translation.y = y;
            transform.translation.z = 10.0; // Above the map
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn redraw_map_when_needed(
    mut state: ResMut<WorldBuilderState>,
    mut regenerate: EventReader<RegenerateRequested>,
    mut images: ResMut<Assets<Image>>,
    textures: Res<MapTextures>,
    mut sprite_query: Query<&mut Handle<Image>, With<MapSprite>>,
) {
    let mut rebuild_generator = false;
    for _ in regenerate.read() {
        rebuild_generator = true;
    }

    if rebuild_generator {
        state.active = state.working.clone();
        let active_config = state.active.clone();
        let mut phases = Vec::new();
        state.generator = WorldGenerator::with_progress(active_config, |phase| {
            info!("World generation phase: {:?}", phase);
            phases.push(phase);
        });
        state.phase_history = phases;
        state.phase_history_dirty = true;
        if let Some(selection) = state.selection {
            state.selection = Some(refresh_selection(
                &state.generator,
                selection.world_x,
                selection.world_z,
            ));
        }
        state.repaint_requested = true;
    }

    // Only re-render when explicitly requested
    if !state.repaint_requested {
        return;
    }

    // Always use base resolution for the overview map
    let texture_width = MAP_WIDTH;
    let texture_height = MAP_HEIGHT;

    info!(
        "Rendering overview map at {}x{}",
        texture_width, texture_height
    );

    // Create a new image with base resolution
    let mut new_image = Image::new_fill(
        Extent3d {
            width: texture_width,
            height: texture_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    // Paint the entire map (full planet view)
    paint_map(&mut new_image, &state.generator, state.visualization);

    // Create a new handle for the updated image
    let new_handle = images.add(new_image);

    // Update the sprite to use the new texture
    if let Ok(mut sprite_handle) = sprite_query.get_single_mut() {
        *sprite_handle = new_handle;
        info!("Updated map texture");
    } else {
        warn!("Could not find MapSprite to update");
    }

    // Remove the old image to free memory
    images.remove(&textures.map);

    state.repaint_requested = false;
}

fn handle_map_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
    mut state: ResMut<WorldBuilderState>,
    mut detail_window: ResMut<DetailWindow>,
    _marker_query: Query<Entity, With<InspectionMarker>>,
) {
    // Left click for detail inspection
    if buttons.just_pressed(MouseButton::Left) {
        info!("Left click detected!");

        let Ok(window) = windows.get_single() else {
            warn!("Could not find primary window");
            return;
        };

        let Some(cursor) = window.cursor_position() else {
            warn!("No cursor position available");
            return;
        };

        info!("Cursor position: {:?}", cursor);

        let Ok((camera, camera_transform)) = camera_query.get_single() else {
            warn!("Could not find map camera");
            return;
        };

        // Convert screen position to world position
        let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor) else {
            return;
        };

        // Map world position to terrain coordinates
        // The map sprite is centered at (0,0) with size MAP_WIDTH x MAP_HEIGHT
        let map_x = world_pos.x + (MAP_WIDTH as f32 / 2.0);
        let map_y = world_pos.y + (MAP_HEIGHT as f32 / 2.0);

        // Convert to world terrain coordinates
        // Note: We need to flip the Y axis because:
        // - In Bevy screen space, Y increases upward
        // - In texture space, Y=0 is at the top and increases downward
        // - The paint_map function maps y/height directly to world_z
        let map_size = state.generator.planet_size() as f32;
        let world_x = (map_x / MAP_WIDTH as f32) * map_size;
        let world_z = ((MAP_HEIGHT as f32 - map_y) / MAP_HEIGHT as f32) * map_size;

        // Set detail center for block-level inspection (note: using X and Z for terrain)
        state.detail_center = Some(Vec2::new(world_x, world_z));
        info!(
            "Inspecting blocks at world position ({:.0}, {:.0})",
            world_x, world_z
        );
        info!(
            "Map click at ({:.1}, {:.1}) -> world ({:.0}, {:.0})",
            map_x, map_y, world_x, world_z
        );

        // Remove old marker if it exists
        if let Some(old_marker) = detail_window.marker_entity {
            commands.entity(old_marker).despawn_recursive();
        }

        // Create a red square marker on the map showing the 512x512 area
        let marker_size = 512.0 * (MAP_WIDTH as f32 / map_size);
        let marker_entity = commands
            .spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                        custom_size: Some(Vec2::new(marker_size, marker_size)),
                        ..default()
                    },
                    transform: Transform::from_xyz(world_pos.x, world_pos.y, 10.0),
                    ..default()
                },
                InspectionMarker,
                RenderLayers::default(), // On the main map layer
            ))
            .id();

        detail_window.marker_entity = Some(marker_entity);
    }

    // Right click for selection info
    if buttons.just_pressed(MouseButton::Right) {
        let Ok(window) = windows.get_single() else {
            return;
        };

        let Some(cursor) = window.cursor_position() else {
            return;
        };

        let Ok((camera, camera_transform)) = camera_query.get_single() else {
            return;
        };

        // Convert screen position to world position
        let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor) else {
            return;
        };

        // Map world position to terrain coordinates
        // The map sprite is centered at (0,0) with size MAP_WIDTH x MAP_HEIGHT
        let map_x = world_pos.x + (MAP_WIDTH as f32 / 2.0);
        let map_z = world_pos.y + (MAP_HEIGHT as f32 / 2.0);

        // Convert to world terrain coordinates
        let map_size = state.generator.planet_size() as f32;
        let world_x = (map_x / MAP_WIDTH as f32) * map_size;
        let world_z = (map_z / MAP_HEIGHT as f32) * map_size;

        // Store the selection and popup position
        state.selection = Some(refresh_selection(&state.generator, world_x, world_z));
        state.popup_world_pos = Some((world_pos.x, world_pos.y));
        state.show_popup = true;
    }
}

fn handle_scroll_events(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut scroll_query: Query<&mut Style, With<ScrollContent>>,
) {
    let mut scroll_delta = 0.0;
    for event in scroll_events.read() {
        // Reduced from 30.0 to 10.0 for more precise control
        // Also account for discrete vs pixel scrolling
        let multiplier = match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => 10.0, // Line-based scrolling (mouse wheel notches)
            bevy::input::mouse::MouseScrollUnit::Pixel => 0.5, // Pixel-based scrolling (trackpad)
        };
        scroll_delta += event.y * multiplier;
    }

    if scroll_delta.abs() > 0.01 {
        if let Ok(mut style) = scroll_query.get_single_mut() {
            if let Val::Px(current_top) = style.top {
                let new_top = (current_top + scroll_delta).min(0.0);
                style.top = Val::Px(new_top);
            } else {
                style.top = Val::Px(scroll_delta.min(0.0));
            }
        }
    }
}

fn refresh_selection(generator: &WorldGenerator, world_x: f32, world_z: f32) -> SelectionDetail {
    let world_x = world_x.rem_euclid(generator.planet_size() as f32);
    let world_z = world_z.rem_euclid(generator.planet_size() as f32);
    let height = generator.get_height(world_x, world_z);
    let biome = generator.get_biome(world_x, world_z);
    let temperature_c = generator.get_temperature_c(world_x, world_z);
    let moisture = generator.get_moisture(world_x, world_z);
    let rainfall = generator.rainfall_intensity(world_x, world_z);
    let water_level = generator.get_water_level(world_x, world_z);
    let river_intensity = generator.river_intensity(world_x, world_z);
    let major_river = generator.major_river_factor(world_x, world_z);

    SelectionDetail {
        world_x,
        world_z,
        height,
        biome,
        temperature_c,
        moisture,
        rainfall,
        water_level,
        river_intensity,
        major_river,
    }
}

fn paint_map(image: &mut Image, generator: &WorldGenerator, visualization: MapVisualization) {
    let width = image.texture_descriptor.size.width;
    let height = image.texture_descriptor.size.height;
    let planet_size = generator.planet_size() as f32;

    let data = &mut image.data;
    data.resize((width * height * 4) as usize, 0);

    // No super-sampling for overview map
    let sample_rate = 1;

    for y in 0..height {
        for x in 0..width {
            // Map pixel to world coordinates (entire planet)
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let world_x = u * planet_size;
            let world_z = v * planet_size;

            let color = if sample_rate > 1 {
                // Super-sampling for smoother appearance when zoomed in
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let step = planet_size / (width as f32 * sample_rate as f32);

                for sy in 0..sample_rate {
                    for sx in 0..sample_rate {
                        let sample_x = world_x + sx as f32 * step;
                        let sample_z = world_z + sy as f32 * step;
                        let sample_color =
                            color_for_mode(generator, sample_x, sample_z, visualization);
                        r += sample_color[0] as u32;
                        g += sample_color[1] as u32;
                        b += sample_color[2] as u32;
                    }
                }

                let samples = (sample_rate * sample_rate) as u32;
                [
                    (r / samples) as u8,
                    (g / samples) as u8,
                    (b / samples) as u8,
                    255,
                ]
            } else {
                color_for_mode(generator, world_x, world_z, visualization)
            };

            let index = ((y * width + x) * 4) as usize;
            data[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn color_for_mode(
    generator: &WorldGenerator,
    world_x: f32,
    world_z: f32,
    visualization: MapVisualization,
) -> [u8; 4] {
    match visualization {
        MapVisualization::Biomes => {
            let height = generator.get_height(world_x, world_z);
            let biome = generator.get_biome(world_x, world_z);
            let base_color = generator.preview_color(world_x, world_z, biome, height);

            // Add height-based shading to show terrain variation
            apply_height_shading(base_color, height, generator.config())
        }
        MapVisualization::Elevation => {
            let height = generator.get_height(world_x, world_z);
            elevation_color(height, generator.config())
        }
        MapVisualization::Moisture => {
            let moisture = generator.get_moisture(world_x, world_z);
            moisture_color(moisture)
        }
        MapVisualization::Temperature => {
            let temp = generator.get_temperature_c(world_x, world_z);
            temperature_color(temp)
        }
        MapVisualization::Hydrology => hydrology_color(generator, world_x, world_z),
        MapVisualization::MajorRivers => major_river_color(generator, world_x, world_z),
    }
}

fn apply_height_shading(base_color: [u8; 4], height: f32, config: &WorldGenConfig) -> [u8; 4] {
    let sea_level = config.sea_level;

    // Only apply shading to land
    if height <= sea_level {
        return base_color;
    }

    // Calculate elevation above sea level
    let elevation = height - sea_level;
    let max_elevation = config.mountain_height + config.highland_bonus;

    // VERY DRAMATIC shading for clear visibility
    // Each meter of elevation creates visible change
    let shade_factor = if elevation < 2.0 {
        // Very flat - dark green
        0.5 + (elevation / 2.0) * 0.1
    } else if elevation < 5.0 {
        // Slight rise - medium dark
        0.6 + (elevation - 2.0) / 3.0 * 0.15
    } else if elevation < 10.0 {
        // Low hills - normal brightness
        0.75 + (elevation - 5.0) / 5.0 * 0.25
    } else if elevation < 20.0 {
        // Rolling hills - noticeably brighter
        1.0 + (elevation - 10.0) / 10.0 * 0.3
    } else if elevation < 40.0 {
        // Higher hills - much brighter
        1.3 + (elevation - 20.0) / 20.0 * 0.3
    } else if elevation < 80.0 {
        // Highlands - very bright
        1.6 + (elevation - 40.0) / 40.0 * 0.2
    } else {
        // Mountains - almost white at peaks
        let mountain_factor = ((elevation - 80.0) / (max_elevation - 80.0)).clamp(0.0, 1.0);
        1.8 + mountain_factor * 0.5
    };

    // Apply the shading with MUCH stronger effect
    let r = (base_color[0] as f32 * shade_factor).min(255.0) as u8;
    let g = (base_color[1] as f32 * shade_factor).min(255.0) as u8;
    let b = (base_color[2] as f32 * shade_factor).min(255.0) as u8;

    [r, g, b, base_color[3]]
}

fn elevation_color(height: f32, config: &WorldGenConfig) -> [u8; 4] {
    let sea_level = config.sea_level;
    if height < sea_level {
        let depth = (sea_level - height) / config.deep_ocean_depth;
        let depth = depth.clamp(0.0, 1.0);
        let shallow = [48, 108, 192];
        let deep = [4, 24, 64];
        let color = lerp_rgb(shallow, deep, depth);
        [color[0], color[1], color[2], 255]
    } else {
        let max_height = sea_level + config.mountain_height + config.highland_bonus;
        let t = ((height - sea_level) / (max_height - sea_level)).clamp(0.0, 1.0);
        let low = [60, 120, 60];
        let high = [240, 240, 240];
        let color = lerp_rgb(low, high, t);
        [color[0], color[1], color[2], 255]
    }
}

fn moisture_color(value: f32) -> [u8; 4] {
    let t = value.clamp(0.0, 1.0);
    let dry = [200, 160, 80];
    let mid = [90, 170, 90];
    let wet = [60, 120, 200];
    let color = if t < 0.5 {
        let blend = t * 2.0;
        lerp_rgb(dry, mid, blend)
    } else {
        let blend = (t - 0.5) * 2.0;
        lerp_rgb(mid, wet, blend)
    };
    [color[0], color[1], color[2], 255]
}

fn temperature_color(temp_c: f32) -> [u8; 4] {
    let min_c = -40.0;
    let max_c = 45.0;
    let t = ((temp_c - min_c) / (max_c - min_c)).clamp(0.0, 1.0);
    let cold = [30, 80, 200];
    let temperate = [90, 170, 120];
    let hot = [220, 90, 40];
    let color = if t < 0.5 {
        let blend = t * 2.0;
        lerp_rgb(cold, temperate, blend)
    } else {
        let blend = (t - 0.5) * 2.0;
        lerp_rgb(temperate, hot, blend)
    };
    [color[0], color[1], color[2], 255]
}

fn hydrology_color(generator: &WorldGenerator, world_x: f32, world_z: f32) -> [u8; 4] {
    let height = generator.get_height(world_x, world_z);
    let sea_level = generator.config().sea_level;
    let water_level = generator.get_water_level(world_x, world_z);
    let river_intensity = generator.river_intensity(world_x, world_z).clamp(0.0, 1.0);
    let major_factor = generator
        .major_river_factor(world_x, world_z)
        .clamp(0.0, 1.0);

    if river_intensity > 0.02 {
        let deep = [18, 92, 210];
        let shallow = [96, 180, 230];
        let blend = (river_intensity + major_factor * 0.5).clamp(0.0, 1.0);
        let color = lerp_rgb(shallow, deep, blend);
        return [color[0], color[1], color[2], 255];
    }

    if water_level > sea_level + 0.5 {
        let color = [70, 140, 210];
        return [color[0], color[1], color[2], 255];
    }

    if height < sea_level {
        let depth = ((sea_level - height) / generator.config().deep_ocean_depth).clamp(0.0, 1.0);
        let color = lerp_rgb([40, 90, 160], [8, 30, 80], depth);
        return [color[0], color[1], color[2], 255];
    }

    let rainfall = generator.rainfall_intensity(world_x, world_z);
    let base = generator.config().hydrology_rainfall.max(0.001);
    let variance = generator.config().hydrology_rainfall_variance.max(0.0);
    let expected_max = base * (1.0 + variance.max(0.1));
    let wet_factor = (rainfall / expected_max).clamp(0.0, 1.0);
    let dryness = (1.0 - wet_factor * (1.0 + major_factor * 0.5)).clamp(0.0, 1.0);
    let moist = [120, 160, 120];
    let dry = [180, 140, 90];
    let mut color = lerp_rgb(moist, dry, dryness);
    if major_factor > 0.05 {
        let highlight = [40, 120, 220];
        let blend = major_factor;
        color = lerp_rgb(color, highlight, blend);
    }
    [color[0], color[1], color[2], 255]
}

fn major_river_color(generator: &WorldGenerator, world_x: f32, world_z: f32) -> [u8; 4] {
    let height = generator.get_height(world_x, world_z);
    let sea_level = generator.config().sea_level;
    if height <= sea_level {
        return [12, 32, 96, 255];
    }

    let river_intensity = generator.river_intensity(world_x, world_z).clamp(0.0, 1.0);
    let major_factor = generator
        .major_river_factor(world_x, world_z)
        .clamp(0.0, 1.0);
    let rainfall = generator.rainfall_intensity(world_x, world_z);
    let base = generator.config().hydrology_rainfall.max(0.001);
    let variance = generator.config().hydrology_rainfall_variance.max(0.0);
    let expected_max = base * (1.0 + variance.max(0.1));
    let wet_factor = (rainfall / expected_max).clamp(0.0, 1.0);

    let background = lerp_rgb([68, 80, 88], [60, 160, 120], wet_factor);
    let mut r = background[0] as f32;
    let mut g = background[1] as f32;
    let mut b = background[2] as f32;

    if river_intensity > 0.01 {
        let river_highlight = lerp_rgb([100, 140, 200], [30, 90, 220], river_intensity);
        let blend = river_intensity.max(0.2);
        r = (1.0 - blend) * r + blend * river_highlight[0] as f32;
        g = (1.0 - blend) * g + blend * river_highlight[1] as f32;
        b = (1.0 - blend) * b + blend * river_highlight[2] as f32;
    }

    if major_factor > 0.0 {
        let major_highlight = [220, 150, 40];
        let blend = major_factor.clamp(0.0, 1.0);
        r = (1.0 - blend) * r + blend * major_highlight[0] as f32;
        g = (1.0 - blend) * g + blend * major_highlight[1] as f32;
        b = (1.0 - blend) * b + blend * major_highlight[2] as f32;
    }

    [
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
        255,
    ]
}

fn phase_display_name(phase: WorldGenPhase) -> &'static str {
    match phase {
        WorldGenPhase::Core => "Core",
        WorldGenPhase::Continents => "Continents",
        WorldGenPhase::Terrain => "Terrain",
        WorldGenPhase::Mountains => "Mountains",
        WorldGenPhase::Climate => "Climate",
        WorldGenPhase::Islands => "Islands",
        WorldGenPhase::Hydrology => "Hydrology",
        WorldGenPhase::Finalize => "Finalize",
    }
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        ((1.0 - t) * a[0] as f32 + t * b[0] as f32) as u8,
        ((1.0 - t) * a[1] as f32 + t * b[1] as f32) as u8,
        ((1.0 - t) * a[2] as f32 + t * b[2] as f32) as u8,
    ]
}

fn find_closest_size_index(options: &[PlanetSize], target_blocks: i32) -> usize {
    let mut best_index = 0;
    let mut best_distance = i64::MAX;
    for (index, option) in options.iter().enumerate() {
        let distance = ((option.blocks() - target_blocks) as i64).abs();
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }
    best_index
}

fn planet_size_label(size: PlanetSize) -> &'static str {
    match size {
        PlanetSize::Tiny => "Tiny",
        PlanetSize::Small => "Small",
        PlanetSize::Medium => "Medium",
        PlanetSize::Default => "Default",
        PlanetSize::Large => "Large",
        PlanetSize::Huge => "Huge",
        PlanetSize::Realistic => "Realistic",
        PlanetSize::Continental => "Continental",
    }
}

fn apply_planet_size(state: &mut WorldBuilderState) {
    if let Some(size) = state.planet_sizes.get(state.planet_size_index).copied() {
        let new_size = (size.chunks().max(1) as u32).saturating_mul(32);
        if new_size != 0 {
            state.working.planet_size = new_size;
        }
    }
}

fn reset_parameter(field: ParameterField, state: &mut WorldBuilderState) {
    // Use the actual defaults, not the scaled version
    let defaults = WorldGenConfig::default();
    match field {
        ParameterField::SeaLevel => state.working.sea_level = defaults.sea_level,
        ParameterField::OceanDepth => state.working.ocean_depth = defaults.ocean_depth,
        ParameterField::DeepOceanDepth => {
            state.working.deep_ocean_depth = defaults.deep_ocean_depth
        }
        ParameterField::ContinentCount => state.working.continent_count = defaults.continent_count,
        ParameterField::ContinentFrequency => {
            state.working.continent_frequency = defaults.continent_frequency
        }
        ParameterField::ContinentThreshold => {
            state.working.continent_threshold = defaults.continent_threshold
        }
        ParameterField::ContinentPower => state.working.continent_power = defaults.continent_power,
        ParameterField::ContinentBias => state.working.continent_bias = defaults.continent_bias,
        ParameterField::ContinentRadius => {
            state.working.continent_radius = defaults.continent_radius
        }
        ParameterField::ContinentEdgePower => {
            state.working.continent_edge_power = defaults.continent_edge_power
        }
        ParameterField::ContinentBeltWidth => {
            state.working.continent_belt_width = defaults.continent_belt_width
        }
        ParameterField::ContinentRepulsionStrength => {
            state.working.continent_repulsion_strength = defaults.continent_repulsion_strength
        }
        ParameterField::ContinentDriftGain => {
            state.working.continent_drift_gain = defaults.continent_drift_gain
        }
        ParameterField::ContinentDriftBeltGain => {
            state.working.continent_drift_belt_gain = defaults.continent_drift_belt_gain
        }
        ParameterField::DetailFrequency => {
            state.working.detail_frequency = defaults.detail_frequency
        }
        ParameterField::DetailAmplitude => {
            state.working.detail_amplitude = defaults.detail_amplitude
        }
        ParameterField::MountainFrequency => {
            state.working.mountain_frequency = defaults.mountain_frequency
        }
        ParameterField::MountainHeight => state.working.mountain_height = defaults.mountain_height,
        ParameterField::MountainThreshold => {
            state.working.mountain_threshold = defaults.mountain_threshold
        }
        ParameterField::MountainRangeCount => {
            state.working.mountain_range_count = defaults.mountain_range_count
        }
        ParameterField::MountainRangeWidth => {
            state.working.mountain_range_width = defaults.mountain_range_width
        }
        ParameterField::MountainRangeStrength => {
            state.working.mountain_range_strength = defaults.mountain_range_strength
        }
        ParameterField::MountainRangeSpurChance => {
            state.working.mountain_range_spur_chance = defaults.mountain_range_spur_chance
        }
        ParameterField::MountainRangeSpurStrength => {
            state.working.mountain_range_spur_strength = defaults.mountain_range_spur_strength
        }
        ParameterField::MountainRangeRoughness => {
            state.working.mountain_range_roughness = defaults.mountain_range_roughness
        }
        ParameterField::MountainErosionIterations => {
            state.working.mountain_erosion_iterations = defaults.mountain_erosion_iterations
        }
        ParameterField::MountainConvergenceBoost => {
            state.working.mountain_convergence_boost = defaults.mountain_convergence_boost
        }
        ParameterField::MountainDivergencePenalty => {
            state.working.mountain_divergence_penalty = defaults.mountain_divergence_penalty
        }
        ParameterField::MountainShearBoost => {
            state.working.mountain_shear_boost = defaults.mountain_shear_boost
        }
        ParameterField::MountainArcThreshold => {
            state.working.mountain_arc_threshold = defaults.mountain_arc_threshold
        }
        ParameterField::MountainArcStrength => {
            state.working.mountain_arc_strength = defaults.mountain_arc_strength
        }
        ParameterField::MountainArcWidthFactor => {
            state.working.mountain_arc_width_factor = defaults.mountain_arc_width_factor
        }
        ParameterField::MoistureFrequency => {
            state.working.moisture_frequency = defaults.moisture_frequency
        }
        ParameterField::EquatorTemperature => {
            state.working.equator_temp_c = defaults.equator_temp_c
        }
        ParameterField::PoleTemperature => state.working.pole_temp_c = defaults.pole_temp_c,
        ParameterField::LapseRate => {
            state.working.lapse_rate_c_per_block = defaults.lapse_rate_c_per_block
        }
        ParameterField::TemperatureVariation => {
            state.working.temperature_variation = defaults.temperature_variation
        }
        ParameterField::HighlandBonus => state.working.highland_bonus = defaults.highland_bonus,
        ParameterField::IslandFrequency => {
            state.working.island_frequency = defaults.island_frequency
        }
        ParameterField::IslandThreshold => {
            state.working.island_threshold = defaults.island_threshold
        }
        ParameterField::IslandHeight => state.working.island_height = defaults.island_height,
        ParameterField::IslandFalloff => state.working.island_falloff = defaults.island_falloff,
        ParameterField::HydrologyResolution => {
            state.working.hydrology_resolution = defaults.hydrology_resolution
        }
        ParameterField::HydrologyRainfall => {
            state.working.hydrology_rainfall = defaults.hydrology_rainfall
        }
        ParameterField::HydrologyRainfallVariance => {
            state.working.hydrology_rainfall_variance = defaults.hydrology_rainfall_variance
        }
        ParameterField::HydrologyRainfallFrequency => {
            state.working.hydrology_rainfall_frequency = defaults.hydrology_rainfall_frequency
        }
        ParameterField::HydrologyIterations => {
            state.working.hydrology_iterations = defaults.hydrology_iterations
        }
        ParameterField::HydrologyTimeStep => {
            state.working.hydrology_time_step = defaults.hydrology_time_step
        }
        ParameterField::HydrologyInfiltrationRate => {
            state.working.hydrology_infiltration_rate = defaults.hydrology_infiltration_rate
        }
        ParameterField::HydrologyBaseflow => {
            state.working.hydrology_baseflow = defaults.hydrology_baseflow
        }
        ParameterField::HydrologyErosionRate => {
            state.working.hydrology_erosion_rate = defaults.hydrology_erosion_rate
        }
        ParameterField::HydrologyDepositionRate => {
            state.working.hydrology_deposition_rate = defaults.hydrology_deposition_rate
        }
        ParameterField::HydrologySedimentCapacity => {
            state.working.hydrology_sediment_capacity = defaults.hydrology_sediment_capacity
        }
        ParameterField::HydrologyBankfullDepth => {
            state.working.hydrology_bankfull_depth = defaults.hydrology_bankfull_depth
        }
        ParameterField::HydrologyFloodplainSoftening => {
            state.working.hydrology_floodplain_softening = defaults.hydrology_floodplain_softening
        }
        ParameterField::HydrologyMinimumSlope => {
            state.working.hydrology_minimum_slope = defaults.hydrology_minimum_slope
        }
        ParameterField::HydrologyShorelineRadius => {
            state.working.hydrology_shoreline_radius = defaults.hydrology_shoreline_radius
        }
        ParameterField::HydrologyShorelineMaxHeight => {
            state.working.hydrology_shoreline_max_height = defaults.hydrology_shoreline_max_height
        }
        ParameterField::HydrologyShorelineSmoothing => {
            state.working.hydrology_shoreline_smoothing = defaults.hydrology_shoreline_smoothing
        }
    }
}
