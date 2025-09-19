use std::fs;
use std::path::Path;

use bevy::prelude::*;
use bevy::render::camera::{RenderTarget, ScalingMode};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::ui::{Display, TargetCamera};
use bevy::window::{PresentMode, PrimaryWindow, WindowRef, WindowResolution};

use forge::planet::{PlanetConfig, PlanetSize};
use forge::world::{Biome, WorldGenConfig, WorldGenerator};

const MAP_WIDTH: u32 = 1024;
const MAP_HEIGHT: u32 = 512;
const DETAIL_SIZE: u32 = 192;
const DETAIL_WORLD_SPAN: f32 = 2048.0;
const DEFAULTS_PATH: &str = "docs/world_builder_defaults.json";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "World Builder - Map".into(),
                resolution: WindowResolution::new(MAP_WIDTH as f32, MAP_HEIGHT as f32),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<ButtonMaterials>()
        .add_event::<RegenerateRequested>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_parameter_buttons,
                handle_parameter_reset_buttons,
                handle_world_size_buttons,
                handle_tab_buttons,
                handle_visualization_buttons,
                sync_visualization_highlights,
                sync_tab_highlights,
                update_tab_sections,
                handle_regenerate_button,
                handle_save_defaults_button,
                handle_map_click,
                handle_scroll_events,
                update_value_text,
                update_selection_text,
                apply_selection_marker,
                redraw_map_when_needed,
                update_detail_texture,
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
    generator: WorldGenerator,
    planet_sizes: Vec<PlanetSize>,
    planet_size_index: usize,
    visualization: MapVisualization,
    active_tab: ParameterTab,
    repaint_requested: bool,
    selection: Option<SelectionDetail>,
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
    detail: Handle<Image>,
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
    Terrain,
    Islands,
    Hydrology,
}

impl ParameterTab {
    const ALL: [Self; 3] = [
        ParameterTab::Terrain,
        ParameterTab::Islands,
        ParameterTab::Hydrology,
    ];

    fn label(&self) -> &'static str {
        match self {
            ParameterTab::Terrain => "Terrain",
            ParameterTab::Islands => "Islands",
            ParameterTab::Hydrology => "Hydrology",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredDefaults {
    config: WorldGenConfig,
    visualization: MapVisualization,
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
struct SaveDefaultsButton;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct VisualizationButton {
    mode: MapVisualization,
}

#[derive(Component)]
struct WorldSizeLabel;

#[derive(Component)]
struct SelectionSummaryText;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParameterField {
    SeaLevel,
    ContinentCount,
    ContinentFrequency,
    ContinentThreshold,
    MountainHeight,
    MoistureFrequency,
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
    HydrologyMajorRiverCount,
    HydrologyMajorRiverBoost,
    RiverFlowThreshold,
    RiverDepthScale,
    RiverMaxDepth,
    RiverSurfaceRatio,
    LakeFlowThreshold,
    LakeDepth,
    LakeShoreBlend,
}

impl ParameterField {
    fn label(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "Sea Level",
            ParameterField::ContinentCount => "Continent Count",
            ParameterField::ContinentFrequency => "Continent Frequency",
            ParameterField::ContinentThreshold => "Continent Threshold",
            ParameterField::MountainHeight => "Mountain Height",
            ParameterField::MoistureFrequency => "Moisture Frequency",
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
            ParameterField::HydrologyMajorRiverCount => "Major River Count",
            ParameterField::HydrologyMajorRiverBoost => "Major River Boost",
            ParameterField::RiverFlowThreshold => "River Flow Threshold",
            ParameterField::RiverDepthScale => "River Depth Scale",
            ParameterField::RiverMaxDepth => "River Max Depth",
            ParameterField::RiverSurfaceRatio => "River Surface Ratio",
            ParameterField::LakeFlowThreshold => "Lake Flow Threshold",
            ParameterField::LakeDepth => "Lake Depth",
            ParameterField::LakeShoreBlend => "Lake Shore Blend",
        }
    }

    fn adjust(&self, config: &mut WorldGenConfig, delta: f32) {
        match self {
            ParameterField::SeaLevel => {
                config.sea_level = (config.sea_level + delta).clamp(16.0, 200.0);
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
            ParameterField::MountainHeight => {
                config.mountain_height = (config.mountain_height + delta).clamp(8.0, 256.0);
            }
            ParameterField::MoistureFrequency => {
                let freq = (config.moisture_frequency + delta as f64).clamp(0.1, 6.0);
                config.moisture_frequency = freq;
            }
            ParameterField::TemperatureVariation => {
                config.temperature_variation =
                    (config.temperature_variation + delta).clamp(0.0, 20.0);
            }
            ParameterField::HighlandBonus => {
                config.highland_bonus = (config.highland_bonus + delta).clamp(0.0, 200.0);
            }
            ParameterField::IslandFrequency => {
                let freq = (config.island_frequency + delta as f64).clamp(0.1, 8.0);
                config.island_frequency = freq;
            }
            ParameterField::IslandThreshold => {
                config.island_threshold = (config.island_threshold + delta).clamp(0.0, 0.99);
            }
            ParameterField::IslandHeight => {
                config.island_height = (config.island_height + delta).clamp(0.0, 128.0);
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
            ParameterField::HydrologyMajorRiverCount => {
                let count = (config.hydrology_major_river_count as i32 + delta as i32).clamp(0, 12);
                config.hydrology_major_river_count = count as u32;
            }
            ParameterField::HydrologyMajorRiverBoost => {
                config.hydrology_major_river_boost =
                    (config.hydrology_major_river_boost + delta).clamp(0.0, 10.0);
            }
            ParameterField::RiverFlowThreshold => {
                config.river_flow_threshold =
                    (config.river_flow_threshold + delta).clamp(10.0, 5000.0);
            }
            ParameterField::RiverDepthScale => {
                config.river_depth_scale =
                    (config.river_depth_scale + delta).clamp(0.0, 1.0);
            }
            ParameterField::RiverMaxDepth => {
                config.river_max_depth = (config.river_max_depth + delta).clamp(0.0, 128.0);
            }
            ParameterField::RiverSurfaceRatio => {
                config.river_surface_ratio =
                    (config.river_surface_ratio + delta).clamp(0.1, 1.0);
            }
            ParameterField::LakeFlowThreshold => {
                config.lake_flow_threshold =
                    (config.lake_flow_threshold + delta).clamp(10.0, 5000.0);
            }
            ParameterField::LakeDepth => {
                config.lake_depth = (config.lake_depth + delta).clamp(0.0, 64.0);
            }
            ParameterField::LakeShoreBlend => {
                config.lake_shore_blend = (config.lake_shore_blend + delta).clamp(0.0, 16.0);
            }
        }
    }

    fn working_value(&self, config: &WorldGenConfig) -> f64 {
        match self {
            ParameterField::SeaLevel => config.sea_level as f64,
            ParameterField::ContinentCount => config.continent_count as f64,
            ParameterField::ContinentFrequency => config.continent_frequency,
            ParameterField::ContinentThreshold => config.continent_threshold as f64,
            ParameterField::MountainHeight => config.mountain_height as f64,
            ParameterField::MoistureFrequency => config.moisture_frequency,
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
            ParameterField::HydrologyMajorRiverCount => config.hydrology_major_river_count as f64,
            ParameterField::HydrologyMajorRiverBoost => config.hydrology_major_river_boost as f64,
            ParameterField::RiverFlowThreshold => config.river_flow_threshold as f64,
            ParameterField::RiverDepthScale => config.river_depth_scale as f64,
            ParameterField::RiverMaxDepth => config.river_max_depth as f64,
            ParameterField::RiverSurfaceRatio => config.river_surface_ratio as f64,
            ParameterField::LakeFlowThreshold => config.lake_flow_threshold as f64,
            ParameterField::LakeDepth => config.lake_depth as f64,
            ParameterField::LakeShoreBlend => config.lake_shore_blend as f64,
        }
    }

    fn format_value(&self, config: &WorldGenConfig) -> String {
        match self {
            ParameterField::SeaLevel => format!("{:.1}", config.sea_level),
            ParameterField::ContinentCount => format!("{}", config.continent_count),
            ParameterField::ContinentFrequency => format!("{:.2}", config.continent_frequency),
            ParameterField::ContinentThreshold => format!("{:.2}", config.continent_threshold),
            ParameterField::MountainHeight => format!("{:.1}", config.mountain_height),
            ParameterField::MoistureFrequency => format!("{:.2}", config.moisture_frequency),
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
            ParameterField::HydrologyMajorRiverCount => {
                format!("{}", config.hydrology_major_river_count)
            }
            ParameterField::HydrologyMajorRiverBoost => {
                format!("{:.1}", config.hydrology_major_river_boost)
            }
            ParameterField::RiverFlowThreshold => format!("{:.0}", config.river_flow_threshold),
            ParameterField::RiverDepthScale => format!("{:.2}", config.river_depth_scale),
            ParameterField::RiverMaxDepth => format!("{:.1}", config.river_max_depth),
            ParameterField::RiverSurfaceRatio => format!("{:.2}", config.river_surface_ratio),
            ParameterField::LakeFlowThreshold => format!("{:.0}", config.lake_flow_threshold),
            ParameterField::LakeDepth => format!("{:.1}", config.lake_depth),
            ParameterField::LakeShoreBlend => format!("{:.1}", config.lake_shore_blend),
        }
    }

    fn differs(&self, working: &WorldGenConfig, active: &WorldGenConfig) -> bool {
        let a = self.working_value(working);
        let b = self.working_value(active);
        (a - b).abs() > self.epsilon()
    }

    fn epsilon(&self) -> f64 {
        match self {
            ParameterField::ContinentCount => 0.5,
            ParameterField::IslandThreshold => 0.01,
            ParameterField::IslandFalloff => 0.01,
            ParameterField::HydrologyResolution => 1.0,
            ParameterField::HydrologyRainfall => 0.001,
            ParameterField::HydrologyRainfallVariance => 0.001,
            ParameterField::HydrologyRainfallFrequency => 0.001,
            ParameterField::HydrologyMajorRiverCount => 0.5,
            ParameterField::HydrologyMajorRiverBoost => 0.005,
            ParameterField::RiverFlowThreshold => 1.0,
            ParameterField::RiverDepthScale => 0.0005,
            ParameterField::RiverMaxDepth => 0.05,
            ParameterField::RiverSurfaceRatio => 0.001,
            ParameterField::LakeFlowThreshold => 1.0,
            ParameterField::LakeDepth => 0.05,
            ParameterField::LakeShoreBlend => 0.05,
            _ => 0.005,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "Absolute waterline height; raising it floods low terrain.",
            ParameterField::ContinentCount => "Target number of large landmasses; higher values split the noise into more continents.",
            ParameterField::ContinentFrequency => "Low-frequency noise controlling continent placement; higher values create more variation per unit area.",
            ParameterField::ContinentThreshold => "Cutoff for land vs ocean; lower thresholds produce more land and wider continents.",
            ParameterField::MountainHeight => "Maximum elevation added by the mountain mask for continental interiors.",
            ParameterField::MoistureFrequency => "Frequency of the moisture noise used for biomes; higher values add more variation.",
            ParameterField::TemperatureVariation => "Amplitude of the temperature noise layered over the latitude gradient.",
            ParameterField::HighlandBonus => "Broad plateau boost applied to interior land before mountain peaks.",
            ParameterField::IslandFrequency => "Noise frequency used for standalone islands; higher values create more island opportunities.",
            ParameterField::IslandThreshold => "Mask threshold islands must exceed to appear; lower values yield more islands.",
            ParameterField::IslandHeight => "Maximum extra elevation granted to qualifying islands above the ocean floor.",
            ParameterField::IslandFalloff => "Exponent controlling how quickly island influence fades away from land; higher values confine islands to deep ocean.",
            ParameterField::HydrologyResolution => "Grid resolution for the water flow simulation; higher values capture finer drainage details at the cost of generation time.",
            ParameterField::HydrologyRainfall => "Amount of water injected per hydrology cell; higher values strengthen flow everywhere.",
            ParameterField::HydrologyRainfallVariance => "Scales how strongly rainfall fluctuates across the planet; 0 keeps things uniform, higher values create distinct wet and dry regions.",
            ParameterField::HydrologyRainfallFrequency => "Spatial frequency of rainfall variation; lower values give broad climate belts, higher values produce smaller storm cells.",
            ParameterField::HydrologyMajorRiverCount => "How many major catchments receive extra rainfall to form large rivers; lower values keep them rare.",
            ParameterField::HydrologyMajorRiverBoost => "Additional rainfall injected into major river basins, controlling how dominant the large rivers become.",
            ParameterField::RiverFlowThreshold => "Flow accumulation required before a channel becomes a river.",
            ParameterField::RiverDepthScale => "Depth carved per unit flow; increase to dig deeper channels once rivers form.",
            ParameterField::RiverMaxDepth => "Upper bound on river carving depth to keep valleys from over-eroding.",
            ParameterField::RiverSurfaceRatio => "Fraction of carved depth used to raise water surface above the river bed.",
            ParameterField::LakeFlowThreshold => "Accumulation needed for sinks/springs to become lakes instead of rivers.",
            ParameterField::LakeDepth => "Maximum depth of lake basins carved at sinks.",
            ParameterField::LakeShoreBlend => "Height band used to soften lake shorelines and avoid sheer cliffs.",
        }
    }

    fn range_hint(&self) -> &'static str {
        match self {
            ParameterField::SeaLevel => "16 - 200 blocks",
            ParameterField::ContinentCount => "1 - 24",
            ParameterField::ContinentFrequency => "0.1 - 4.0",
            ParameterField::ContinentThreshold => "0.05 - 0.60",
            ParameterField::MountainHeight => "8 - 256 blocks",
            ParameterField::MoistureFrequency => "0.1 - 6.0",
            ParameterField::TemperatureVariation => "0 - 20",
            ParameterField::HighlandBonus => "0 - 200 blocks",
            ParameterField::IslandFrequency => "0.1 - 8.0",
            ParameterField::IslandThreshold => "0.00 - 0.99",
            ParameterField::IslandHeight => "0 - 128 blocks",
            ParameterField::IslandFalloff => "0.1 - 6.0",
            ParameterField::HydrologyResolution => "128 - 4096 cells",
            ParameterField::HydrologyRainfall => "0.1 - 10.0",
            ParameterField::HydrologyRainfallVariance => "0.0 - 2.0",
            ParameterField::HydrologyRainfallFrequency => "0.1 - 6.0",
            ParameterField::HydrologyMajorRiverCount => "0 - 12 basins",
            ParameterField::HydrologyMajorRiverBoost => "0.0 - 10.0",
            ParameterField::RiverFlowThreshold => "10 - 5000",
            ParameterField::RiverDepthScale => "0.0 - 1.0",
            ParameterField::RiverMaxDepth => "0 - 128 blocks",
            ParameterField::RiverSurfaceRatio => "0.1 - 1.0",
            ParameterField::LakeFlowThreshold => "10 - 5000",
            ParameterField::LakeDepth => "0 - 64 blocks",
            ParameterField::LakeShoreBlend => "0 - 16 blocks",
        }
    }
}

const TERRAIN_FIELDS: &[ParameterField] = &[
    ParameterField::SeaLevel,
    ParameterField::ContinentCount,
    ParameterField::ContinentFrequency,
    ParameterField::ContinentThreshold,
    ParameterField::MountainHeight,
    ParameterField::MoistureFrequency,
    ParameterField::TemperatureVariation,
    ParameterField::HighlandBonus,
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
    ParameterField::HydrologyMajorRiverCount,
    ParameterField::HydrologyMajorRiverBoost,
    ParameterField::RiverFlowThreshold,
    ParameterField::RiverDepthScale,
    ParameterField::RiverMaxDepth,
    ParameterField::RiverSurfaceRatio,
    ParameterField::LakeFlowThreshold,
    ParameterField::LakeDepth,
    ParameterField::LakeShoreBlend,
];

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    materials: Res<ButtonMaterials>,
) {
    let planet_sizes = vec![
        PlanetSize::Tiny,
        PlanetSize::Small,
        PlanetSize::Medium,
        PlanetSize::Default,
        PlanetSize::Large,
        PlanetSize::Huge,
    ];

    let (working, visualization) = load_defaults()
        .map(|defaults| (defaults.config, defaults.visualization))
        .unwrap_or_else(|| {
            let base_planet = PlanetConfig::default();
            let config = WorldGenConfig::from_planet_config(&base_planet);
            (config, MapVisualization::Biomes)
        });

    let active = working.clone();
    let generator = WorldGenerator::new(active.clone());

    let planet_size_index = find_closest_size_index(&planet_sizes, working.planet_size as i32);

    commands.insert_resource(WorldBuilderState {
        working,
        active,
        generator,
        planet_sizes,
        planet_size_index,
        visualization,
        active_tab: ParameterTab::Terrain,
        repaint_requested: true,
        selection: None,
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
        RenderAssetUsages::default(),
    );
    map_image.sampler = ImageSampler::nearest();
    let map_handle = images.add(map_image);

    let mut detail_image = Image::new_fill(
        Extent3d {
            width: DETAIL_SIZE,
            height: DETAIL_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8, 0u8, 0u8, 255u8],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    detail_image.sampler = ImageSampler::nearest();
    let detail_handle = images.add(detail_image);

    commands.insert_resource(MapTextures {
        map: map_handle.clone(),
        detail: detail_handle.clone(),
    });

    // Cameras and map entities
    commands.spawn(Camera2dBundle {
        camera: Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.02, 0.03)),
            ..default()
        },
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: MAP_WIDTH as f32,
                min_height: MAP_HEIGHT as f32,
            },
            ..default()
        },
        ..default()
    });

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(ImageBundle {
                image: UiImage::new(map_handle.clone()),
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            });

            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(1.0, 0.3, 0.2)),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                SelectionMarker,
            ));
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

    build_control_panel(&mut commands, control_camera, &materials, detail_handle);
}

fn build_control_panel(
    commands: &mut Commands,
    control_camera: Entity,
    materials: &ButtonMaterials,
    detail_handle: Handle<Image>,
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
        // Header with improved styling
        parent
            .spawn(NodeBundle {
                style: Style {
                    padding: UiRect::bottom(Val::Px(8.0)),
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
            });

        // Two column layout
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    column_gap: Val::Px(16.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|columns| {
                // LEFT COLUMN - Map & Visualization
                columns
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            flex_basis: Val::Percent(50.0),
                            row_gap: Val::Px(12.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|left_col| {
                        // Map Visualization Controls
                        left_col.spawn(TextBundle::from_section(
                            "MAP VISUALIZATION",
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(0.65, 0.7, 0.8, 0.9),
                                ..default()
                            },
                        ));

                        left_col
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    row_gap: Val::Px(8.0),
                                    column_gap: Val::Px(8.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.09, 0.12, 0.5,
                                )),
                                border_color: BorderColor(Color::srgba(0.2, 0.22, 0.28, 0.5)),
                                ..default()
                            })
                            .with_children(|row| {
                                for mode in MapVisualization::ALL {
                                    row.spawn(ButtonBundle {
                                        style: Style {
                                            width: Val::Px(100.0),
                                            height: Val::Px(32.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        background_color: materials.normal,
                                        border_color: BorderColor(Color::srgba(
                                            0.25, 0.28, 0.35, 0.6,
                                        )),
                                        ..default()
                                    })
                                    .insert(VisualizationButton { mode })
                                    .with_children(|button| {
                                        button.spawn(TextBundle::from_section(
                                            mode.label(),
                                            TextStyle {
                                                font_size: 13.0,
                                                color: Color::srgb(0.9, 0.93, 1.0),
                                                ..default()
                                            },
                                        ));
                                    });
                                }
                            });

                        // Selection Preview
                        left_col.spawn(TextBundle::from_section(
                            "SELECTION PREVIEW",
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(0.65, 0.7, 0.8, 0.9),
                                ..default()
                            },
                        ));

                        left_col
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(16.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.09, 0.12, 0.5,
                                )),
                                border_color: BorderColor(Color::srgba(0.2, 0.22, 0.28, 0.5)),
                                ..default()
                            })
                            .with_children(|row| {
                                // Wrap the image in a container with a border
                                row.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Px(196.0),
                                        height: Val::Px(196.0),
                                        padding: UiRect::all(Val::Px(2.0)),
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::srgba(
                                        0.1, 0.11, 0.14, 0.8,
                                    )),
                                    border_color: BorderColor(Color::srgba(0.3, 0.35, 0.4, 0.8)),
                                    ..default()
                                })
                                .with_children(|container| {
                                    container
                                        .spawn(ImageBundle {
                                            image: UiImage::new(detail_handle.clone()),
                                            style: Style {
                                                width: Val::Px(192.0),
                                                height: Val::Px(192.0),
                                                ..default()
                                            },
                                            ..default()
                                        })
                                        .insert(DetailImage);
                                });

                                row.spawn(
                                    TextBundle::from_section(
                                        "Click on the map to inspect a location.",
                                        TextStyle {
                                            font_size: 13.0,
                                            color: Color::srgb(0.75, 0.8, 0.88),
                                            ..default()
                                        },
                                    )
                                    .with_style(Style {
                                        flex_wrap: FlexWrap::Wrap,
                                        max_width: Val::Px(240.0),
                                        ..default()
                                    }),
                                )
                                .insert(SelectionSummaryText);
                            });

                        // Action buttons
                        left_col
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(12.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.09, 0.12, 0.5,
                                )),
                                border_color: BorderColor(Color::srgba(0.2, 0.22, 0.28, 0.5)),
                                ..default()
                            })
                            .with_children(|row| {
                                row.spawn(ButtonBundle {
                                    style: Style {
                                        width: Val::Px(150.0),
                                        height: Val::Px(42.0),
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

                                row.spawn(ButtonBundle {
                                    style: Style {
                                        width: Val::Px(150.0),
                                        height: Val::Px(42.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::srgba(
                                        0.12, 0.18, 0.25, 0.95,
                                    )),
                                    border_color: BorderColor(Color::srgba(0.25, 0.35, 0.45, 0.8)),
                                    ..default()
                                })
                                .insert(SaveDefaultsButton)
                                .with_children(|b| {
                                    b.spawn(TextBundle::from_section(
                                        "SAVE DEFAULTS",
                                        TextStyle {
                                            font_size: 14.0,
                                            color: Color::srgb(0.9, 0.93, 0.98),
                                            ..default()
                                        },
                                    ));
                                });
                            });
                    });

                // RIGHT COLUMN - World Parameters
                columns
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            flex_basis: Val::Percent(50.0),
                            row_gap: Val::Px(12.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|right_col| {
                        // World size section
                        right_col
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.09, 0.12, 0.5,
                                )),
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
                        right_col
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
                                        border_color: BorderColor(Color::srgba(
                                            0.25, 0.28, 0.35, 0.6,
                                        )),
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
                        right_col
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    border: UiRect::all(Val::Px(1.0)),
                                    min_height: Val::Px(400.0),
                                    max_height: Val::Px(500.0),
                                    overflow: Overflow::clip_y(),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.09, 0.12, 0.5,
                                )),
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
                                            ParameterTab::Terrain,
                                            TERRAIN_FIELDS,
                                            ParameterTab::Terrain,
                                        );
                                        spawn_tab_section(
                                            sections,
                                            materials,
                                            ParameterTab::Islands,
                                            ISLAND_FIELDS,
                                            ParameterTab::Terrain,
                                        );
                                        spawn_tab_section(
                                            sections,
                                            materials,
                                            ParameterTab::Hydrology,
                                            HYDROLOGY_FIELDS,
                                            ParameterTab::Terrain,
                                        );
                                    });
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
        ParameterField::ContinentCount => 1.0,
        ParameterField::ContinentFrequency => 0.05,
        ParameterField::ContinentThreshold => 0.02,
        ParameterField::MountainHeight => 4.0,
        ParameterField::MoistureFrequency => 0.05,
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
        ParameterField::HydrologyMajorRiverCount => 1.0,
        ParameterField::HydrologyMajorRiverBoost => 0.5,
        ParameterField::RiverFlowThreshold => 10.0,
        ParameterField::RiverDepthScale => 0.01,
        ParameterField::RiverMaxDepth => 1.0,
        ParameterField::RiverSurfaceRatio => 0.05,
        ParameterField::LakeFlowThreshold => 10.0,
        ParameterField::LakeDepth => 1.0,
        ParameterField::LakeShoreBlend => 0.5,
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

fn handle_save_defaults_button(
    materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SaveDefaultsButton>),
    >,
    state: Res<WorldBuilderState>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = materials.pressed;
                if let Err(err) = save_defaults(&state) {
                    warn!("Failed to save defaults: {err}");
                } else {
                    info!("Saved world builder defaults to {DEFAULTS_PATH}");
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
    mut marker_query: Query<(&mut Style, &mut Visibility), With<SelectionMarker>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    if !state.is_changed() {
        return;
    }

    let Ok(window) = window_query.get_single() else {
        return;
    };

    let width = window.width();
    let height = window.height();

    if let Ok((mut style, mut visibility)) = marker_query.get_single_mut() {
        if let Some(selection) = state.selection {
            let map_size = state.generator.planet_size() as f32;
            let u = selection.world_x / map_size;
            let v = selection.world_z / map_size;
            let x = (u * width).clamp(0.0, width);
            let y = (v * height).clamp(0.0, height);
            style.left = Val::Px(x - 6.0);
            style.right = Val::Auto;
            style.top = Val::Px(y - 6.0);
            style.bottom = Val::Auto;
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
) {
    let mut rebuild_generator = false;
    for _ in regenerate.read() {
        rebuild_generator = true;
    }

    if rebuild_generator {
        state.active = state.working.clone();
        state.generator = WorldGenerator::new(state.active.clone());
        if let Some(selection) = state.selection {
            state.selection = Some(refresh_selection(
                &state.generator,
                selection.world_x,
                selection.world_z,
            ));
        }
        state.repaint_requested = true;
    }

    if !state.repaint_requested {
        return;
    }

    if let Some(image) = images.get_mut(&textures.map) {
        info!(
            "Painting world map ({}x{})",
            image.texture_descriptor.size.width, image.texture_descriptor.size.height
        );
        paint_map(image, &state.generator, state.visualization);
    } else {
        warn!("World map image asset missing during repaint");
    }

    state.repaint_requested = false;
}

fn update_detail_texture(
    state: Res<WorldBuilderState>,
    mut images: ResMut<Assets<Image>>,
    textures: Res<MapTextures>,
) {
    if !state.is_changed() {
        return;
    }

    let Some(selection) = state.selection else {
        return;
    };

    if let Some(image) = images.get_mut(&textures.detail) {
        paint_detail(image, &state.generator, selection, state.visualization);
    }
}

fn handle_map_click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<WorldBuilderState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };

    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let width = window.width();
    let height = window.height();

    let u = (cursor.x.clamp(0.0, width)) / width;
    let v = (cursor.y.clamp(0.0, height)) / height;

    let map_size = state.generator.planet_size() as f32;
    let world_x = u * map_size;
    let world_z = v * map_size;

    state.selection = Some(refresh_selection(&state.generator, world_x, world_z));
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

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let world_x = u * planet_size;
            let world_z = v * planet_size;
            let color = color_for_mode(generator, world_x, world_z, visualization);
            let index = ((y * width + x) * 4) as usize;
            data[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn paint_detail(
    image: &mut Image,
    generator: &WorldGenerator,
    selection: SelectionDetail,
    visualization: MapVisualization,
) {
    let width = image.texture_descriptor.size.width;
    let height = image.texture_descriptor.size.height;
    let data = &mut image.data;
    data.resize((width * height * 4) as usize, 0);

    let span = DETAIL_WORLD_SPAN;
    let start_x = selection.world_x - span * 0.5;
    let start_z = selection.world_z - span * 0.5;

    let planet_size = generator.planet_size() as f32;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / width as f32;
            let fz = y as f32 / height as f32;
            let world_x = (start_x + fx * span).rem_euclid(planet_size);
            let world_z = (start_z + fz * span).rem_euclid(planet_size);
            let color = color_for_mode(generator, world_x, world_z, visualization);
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
            generator.preview_color(world_x, world_z, biome, height)
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
    let river_boost = generator.config().hydrology_major_river_boost.max(0.0);
    let expected_max = base * (1.0 + variance.max(0.1) + river_boost);
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
    let river_boost = generator.config().hydrology_major_river_boost.max(0.0);
    let expected_max = base * (1.0 + variance.max(0.1) + river_boost);
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

fn load_defaults() -> Option<StoredDefaults> {
    let path = Path::new(DEFAULTS_PATH);
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_defaults(state: &WorldBuilderState) -> Result<(), String> {
    let stored = StoredDefaults {
        config: state.working.clone(),
        visualization: state.visualization,
    };
    let json = serde_json::to_string_pretty(&stored).map_err(|err| err.to_string())?;
    if let Some(parent) = Path::new(DEFAULTS_PATH).parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(DEFAULTS_PATH, json).map_err(|err| err.to_string())
}

fn default_config_for_state(state: &WorldBuilderState) -> WorldGenConfig {
    let mut planet = PlanetConfig::default();
    let size_chunks = (state.working.planet_size / 32).max(1) as i32;
    planet.size_chunks = size_chunks;
    planet.seed = state.working.seed as u64;
    planet.sea_level = state.working.sea_level;
    WorldGenConfig::from_planet_config(&planet)
}

fn reset_parameter(field: ParameterField, state: &mut WorldBuilderState) {
    let defaults = default_config_for_state(state);
    match field {
        ParameterField::SeaLevel => state.working.sea_level = defaults.sea_level,
        ParameterField::ContinentCount => state.working.continent_count = defaults.continent_count,
        ParameterField::ContinentFrequency => {
            state.working.continent_frequency = defaults.continent_frequency
        }
        ParameterField::ContinentThreshold => {
            state.working.continent_threshold = defaults.continent_threshold
        }
        ParameterField::MountainHeight => state.working.mountain_height = defaults.mountain_height,
        ParameterField::MoistureFrequency => {
            state.working.moisture_frequency = defaults.moisture_frequency
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
        ParameterField::HydrologyMajorRiverCount => {
            state.working.hydrology_major_river_count = defaults.hydrology_major_river_count
        }
        ParameterField::HydrologyMajorRiverBoost => {
            state.working.hydrology_major_river_boost = defaults.hydrology_major_river_boost
        }
        ParameterField::RiverFlowThreshold => {
            state.working.river_flow_threshold = defaults.river_flow_threshold
        }
        ParameterField::RiverDepthScale => {
            state.working.river_depth_scale = defaults.river_depth_scale
        }
        ParameterField::RiverMaxDepth => state.working.river_max_depth = defaults.river_max_depth,
        ParameterField::RiverSurfaceRatio => {
            state.working.river_surface_ratio = defaults.river_surface_ratio
        }
        ParameterField::LakeFlowThreshold => {
            state.working.lake_flow_threshold = defaults.lake_flow_threshold
        }
        ParameterField::LakeDepth => state.working.lake_depth = defaults.lake_depth,
        ParameterField::LakeShoreBlend => {
            state.working.lake_shore_blend = defaults.lake_shore_blend
        }
    }
}
