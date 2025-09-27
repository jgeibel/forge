use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterCategory {
    Core,
    Ocean,
    Continent,
    Terrain,
    Mountain,
    Climate,
    Island,
    Hydrology,
    River,
    Lake,
}

#[allow(dead_code)]
impl ParameterCategory {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Core => "Core",
            Self::Ocean => "Ocean",
            Self::Continent => "Continent",
            Self::Terrain => "Terrain",
            Self::Mountain => "Mountain",
            Self::Climate => "Climate",
            Self::Island => "Island",
            Self::Hydrology => "Hydrology",
            Self::River => "River",
            Self::Lake => "Lake",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    pub name: &'static str,
    pub field_name: &'static str,
    pub category: ParameterCategory,
    pub description: &'static str,
    pub min_value: f64,
    pub max_value: f64,
    pub units: Option<&'static str>,
    pub ui_visible: bool,
    pub ui_tab: Option<&'static str>,
}

#[allow(dead_code)]
pub struct ParameterRegistry {
    metadata: HashMap<&'static str, ParameterMetadata>,
}

#[allow(dead_code)]
impl ParameterRegistry {
    pub fn new() -> Self {
        let mut metadata = HashMap::new();

        // Core Parameters
        metadata.insert(
            "seed",
            ParameterMetadata {
                name: "Seed",
                field_name: "seed",
                category: ParameterCategory::Core,
                description: "Random seed for world generation",
                min_value: 0.0,
                max_value: u64::MAX as f64,
                units: None,
                ui_visible: false, // Handled separately in UI
                ui_tab: None,
            },
        );

        metadata.insert(
            "planet_size",
            ParameterMetadata {
                name: "Planet Size",
                field_name: "planet_size",
                category: ParameterCategory::Core,
                description: "Size of the planet in blocks",
                min_value: 1024.0,
                max_value: 262144.0,
                units: Some("blocks"),
                ui_visible: false, // Handled via planet size dropdown
                ui_tab: None,
            },
        );

        metadata.insert(
            "sea_level",
            ParameterMetadata {
                name: "Sea Level",
                field_name: "sea_level",
                category: ParameterCategory::Core,
                description: "Height of the sea level",
                min_value: 16.0,
                max_value: 200.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Core"),
            },
        );

        // Ocean Parameters
        metadata.insert(
            "ocean_depth",
            ParameterMetadata {
                name: "Ocean Depth",
                field_name: "ocean_depth",
                category: ParameterCategory::Ocean,
                description: "Depth of regular ocean areas",
                min_value: 10.0,
                max_value: 100.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Core"),
            },
        );

        metadata.insert(
            "deep_ocean_depth",
            ParameterMetadata {
                name: "Deep Ocean Depth",
                field_name: "deep_ocean_depth",
                category: ParameterCategory::Ocean,
                description: "Depth of deep ocean trenches",
                min_value: 20.0,
                max_value: 200.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Core"),
            },
        );

        // Continent Parameters
        metadata.insert(
            "continent_threshold",
            ParameterMetadata {
                name: "Continent Threshold",
                field_name: "continent_threshold",
                category: ParameterCategory::Continent,
                description: "Threshold for continent formation",
                min_value: 0.05,
                max_value: 0.6,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_count",
            ParameterMetadata {
                name: "Continent Count",
                field_name: "continent_count",
                category: ParameterCategory::Continent,
                description: "Number of major continents",
                min_value: 1.0,
                max_value: 24.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_frequency",
            ParameterMetadata {
                name: "Continent Frequency",
                field_name: "continent_frequency",
                category: ParameterCategory::Continent,
                description: "Frequency of continent noise",
                min_value: 0.1,
                max_value: 4.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_power",
            ParameterMetadata {
                name: "Continent Power",
                field_name: "continent_power",
                category: ParameterCategory::Continent,
                description:
                    "Exponent applied to continent noise; higher values emphasize interiors.",
                min_value: 0.2,
                max_value: 5.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_bias",
            ParameterMetadata {
                name: "Continent Bias",
                field_name: "continent_bias",
                category: ParameterCategory::Continent,
                description: "Adds bias before thresholding; raise to favor land over ocean.",
                min_value: 0.0,
                max_value: 0.6,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_radius",
            ParameterMetadata {
                name: "Continent Radius",
                field_name: "continent_radius",
                category: ParameterCategory::Continent,
                description: "Radius of continent site influence in normalized map space.",
                min_value: 0.05,
                max_value: 0.6,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_edge_power",
            ParameterMetadata {
                name: "Edge Power",
                field_name: "continent_edge_power",
                category: ParameterCategory::Continent,
                description: "Controls how sharply continent influence fades toward coasts.",
                min_value: 0.2,
                max_value: 4.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_belt_width",
            ParameterMetadata {
                name: "Belt Width",
                field_name: "continent_belt_width",
                category: ParameterCategory::Continent,
                description:
                    "Width of the latitude band that favors large continent sites (0-0.5).",
                min_value: 0.05,
                max_value: 0.45,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_repulsion_strength",
            ParameterMetadata {
                name: "Site Repulsion",
                field_name: "continent_repulsion_strength",
                category: ParameterCategory::Continent,
                description:
                    "Strength of the relaxation push that keeps continent seeds separated.",
                min_value: 0.0,
                max_value: 0.3,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_drift_gain",
            ParameterMetadata {
                name: "Drift Gain",
                field_name: "continent_drift_gain",
                category: ParameterCategory::Continent,
                description: "Base magnitude for simulated plate drift vectors (0 = static).",
                min_value: 0.0,
                max_value: 0.4,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        metadata.insert(
            "continent_drift_belt_gain",
            ParameterMetadata {
                name: "Drift Belt Gain",
                field_name: "continent_drift_belt_gain",
                category: ParameterCategory::Continent,
                description:
                    "Additional drift multiplier applied inside the preferred belt orientation.",
                min_value: 0.0,
                max_value: 1.2,
                units: None,
                ui_visible: true,
                ui_tab: Some("Continents"),
            },
        );

        // Terrain Parameters
        metadata.insert(
            "detail_frequency",
            ParameterMetadata {
                name: "Detail Frequency",
                field_name: "detail_frequency",
                category: ParameterCategory::Terrain,
                description: "Frequency of mid-scale height variation.",
                min_value: 1.0,
                max_value: 15.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "detail_amplitude",
            ParameterMetadata {
                name: "Detail Amplitude",
                field_name: "detail_amplitude",
                category: ParameterCategory::Terrain,
                description: "Amplitude of mid-scale height variation in blocks (meters).",
                min_value: 1.0,
                max_value: 30.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "micro_detail_scale",
            ParameterMetadata {
                name: "Micro Detail Scale",
                field_name: "micro_detail_scale",
                category: ParameterCategory::Terrain,
                description: "Approximate size of micro terrain features in blocks.",
                min_value: 4.0,
                max_value: 128.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "micro_detail_amplitude",
            ParameterMetadata {
                name: "Micro Detail Amplitude",
                field_name: "micro_detail_amplitude",
                category: ParameterCategory::Terrain,
                description:
                    "Vertical strength of micro-scale variation added after rolling hills.",
                min_value: 0.0,
                max_value: 20.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "micro_detail_roughness",
            ParameterMetadata {
                name: "Micro Detail Roughness",
                field_name: "micro_detail_roughness",
                category: ParameterCategory::Terrain,
                description: "Persistence between micro-detail octaves; higher values keep more fine structure.",
                min_value: 0.2,
                max_value: 0.95,
                units: None,
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "micro_detail_land_blend",
            ParameterMetadata {
                name: "Micro Detail Land Blend",
                field_name: "micro_detail_land_blend",
                category: ParameterCategory::Terrain,
                description: "Exponent governing how micro detail fades toward coasts.",
                min_value: 0.2,
                max_value: 2.5,
                units: None,
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        metadata.insert(
            "highland_bonus",
            ParameterMetadata {
                name: "Highland Bonus",
                field_name: "highland_bonus",
                category: ParameterCategory::Terrain,
                description:
                    "Additional elevation applied to continental interiors in blocks (meters).",
                min_value: 0.0,
                max_value: 50.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Terrain"),
            },
        );

        // Mountain Parameters
        metadata.insert(
            "mountain_frequency",
            ParameterMetadata {
                name: "Mountain Frequency",
                field_name: "mountain_frequency",
                category: ParameterCategory::Mountain,
                description: "Base frequency of mountain noise controlling cluster spacing.",
                min_value: 0.2,
                max_value: 8.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_height",
            ParameterMetadata {
                name: "Mountain Height",
                field_name: "mountain_height",
                category: ParameterCategory::Mountain,
                description: "Maximum height of mountains",
                min_value: 50.0,
                max_value: 500.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_threshold",
            ParameterMetadata {
                name: "Mountain Threshold",
                field_name: "mountain_threshold",
                category: ParameterCategory::Mountain,
                description: "Threshold for mountain formation",
                min_value: 0.1,
                max_value: 0.9,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_range_count",
            ParameterMetadata {
                name: "Mountain Range Count",
                field_name: "mountain_range_count",
                category: ParameterCategory::Mountain,
                description: "Number of mountain ranges",
                min_value: 0.0,
                max_value: 80.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_range_width",
            ParameterMetadata {
                name: "Mountain Range Width",
                field_name: "mountain_range_width",
                category: ParameterCategory::Mountain,
                description: "Width of mountain ranges",
                min_value: 40.0,
                max_value: 800.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_range_strength",
            ParameterMetadata {
                name: "Mountain Range Strength",
                field_name: "mountain_range_strength",
                category: ParameterCategory::Mountain,
                description: "Strength of mountain ranges",
                min_value: 0.5,
                max_value: 5.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_erosion_iterations",
            ParameterMetadata {
                name: "Erosion Passes",
                field_name: "mountain_erosion_iterations",
                category: ParameterCategory::Mountain,
                description:
                    "Number of smoothing iterations applied to mountain ranges before hydrology.",
                min_value: 0.0,
                max_value: 8.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_convergence_boost",
            ParameterMetadata {
                name: "Convergence Boost",
                field_name: "mountain_convergence_boost",
                category: ParameterCategory::Mountain,
                description:
                    "Additional mountain strength multiplier along convergent plate boundaries.",
                min_value: 0.0,
                max_value: 1.5,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_divergence_penalty",
            ParameterMetadata {
                name: "Divergence Penalty",
                field_name: "mountain_divergence_penalty",
                category: ParameterCategory::Mountain,
                description: "Penalty applied to mountain strength where plates move apart.",
                min_value: 0.0,
                max_value: 1.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_shear_boost",
            ParameterMetadata {
                name: "Shear Boost",
                field_name: "mountain_shear_boost",
                category: ParameterCategory::Mountain,
                description:
                    "Multiplier applied where plates slide past one another (transform faults).",
                min_value: 0.0,
                max_value: 0.6,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_arc_threshold",
            ParameterMetadata {
                name: "Arc Threshold",
                field_name: "mountain_arc_threshold",
                category: ParameterCategory::Mountain,
                description: "Minimum convergence required before volcanic arcs spawn offshore.",
                min_value: 0.0,
                max_value: 1.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_arc_strength",
            ParameterMetadata {
                name: "Arc Strength",
                field_name: "mountain_arc_strength",
                category: ParameterCategory::Mountain,
                description:
                    "Relative height of volcanic island arcs generated along subduction zones.",
                min_value: 0.0,
                max_value: 1.5,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        metadata.insert(
            "mountain_arc_width_factor",
            ParameterMetadata {
                name: "Arc Width",
                field_name: "mountain_arc_width_factor",
                category: ParameterCategory::Mountain,
                description:
                    "Relative width of volcanic arcs compared to their parent range crest.",
                min_value: 0.05,
                max_value: 1.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Mountains"),
            },
        );

        // Climate Parameters
        metadata.insert(
            "moisture_frequency",
            ParameterMetadata {
                name: "Moisture Frequency",
                field_name: "moisture_frequency",
                category: ParameterCategory::Climate,
                description: "Frequency of biome moisture noise",
                min_value: 0.1,
                max_value: 6.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Climate"),
            },
        );

        metadata.insert(
            "temperature_variation",
            ParameterMetadata {
                name: "Temperature Variation",
                field_name: "temperature_variation",
                category: ParameterCategory::Climate,
                description: "Amplitude of temperature noise layered over latitude.",
                min_value: 0.0,
                max_value: 20.0,
                units: Some("°C"),
                ui_visible: true,
                ui_tab: Some("Climate"),
            },
        );

        metadata.insert(
            "equator_temp_c",
            ParameterMetadata {
                name: "Equator Temperature",
                field_name: "equator_temp_c",
                category: ParameterCategory::Climate,
                description: "Temperature at the equator",
                min_value: 10.0,
                max_value: 45.0,
                units: Some("°C"),
                ui_visible: true,
                ui_tab: Some("Climate"),
            },
        );

        metadata.insert(
            "pole_temp_c",
            ParameterMetadata {
                name: "Pole Temperature",
                field_name: "pole_temp_c",
                category: ParameterCategory::Climate,
                description: "Temperature at the poles",
                min_value: -60.0,
                max_value: 10.0,
                units: Some("°C"),
                ui_visible: true,
                ui_tab: Some("Climate"),
            },
        );

        metadata.insert(
            "lapse_rate_c_per_block",
            ParameterMetadata {
                name: "Lapse Rate",
                field_name: "lapse_rate_c_per_block",
                category: ParameterCategory::Climate,
                description: "Temperature drop per meter of elevation.",
                min_value: 0.001,
                max_value: 0.02,
                units: Some("°C/block"),
                ui_visible: true,
                ui_tab: Some("Climate"),
            },
        );

        // Island Parameters
        metadata.insert(
            "island_frequency",
            ParameterMetadata {
                name: "Island Frequency",
                field_name: "island_frequency",
                category: ParameterCategory::Island,
                description: "Frequency of island chains",
                min_value: 1.0,
                max_value: 20.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Islands"),
            },
        );

        metadata.insert(
            "island_threshold",
            ParameterMetadata {
                name: "Island Threshold",
                field_name: "island_threshold",
                category: ParameterCategory::Island,
                description: "Threshold for island formation",
                min_value: 0.3,
                max_value: 0.8,
                units: None,
                ui_visible: true,
                ui_tab: Some("Islands"),
            },
        );

        metadata.insert(
            "island_height",
            ParameterMetadata {
                name: "Island Height",
                field_name: "island_height",
                category: ParameterCategory::Island,
                description: "Maximum height of islands",
                min_value: 20.0,
                max_value: 200.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Islands"),
            },
        );

        // Hydrology Parameters
        metadata.insert(
            "hydrology_resolution",
            ParameterMetadata {
                name: "Hydrology Resolution",
                field_name: "hydrology_resolution",
                category: ParameterCategory::Hydrology,
                description: "Resolution of hydrology simulation",
                min_value: 64.0,
                max_value: 8192.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_rainfall",
            ParameterMetadata {
                name: "Rainfall",
                field_name: "hydrology_rainfall",
                category: ParameterCategory::Hydrology,
                description: "Baseline rainfall multiplier applied planet-wide.",
                min_value: 0.1,
                max_value: 4.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_rainfall_variance",
            ParameterMetadata {
                name: "Rainfall Variance",
                field_name: "hydrology_rainfall_variance",
                category: ParameterCategory::Hydrology,
                description: "Strength of rainfall noise layered on humidity belts.",
                min_value: 0.0,
                max_value: 3.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_rainfall_frequency",
            ParameterMetadata {
                name: "Rainfall Frequency",
                field_name: "hydrology_rainfall_frequency",
                category: ParameterCategory::Hydrology,
                description: "Spatial frequency of rainfall variation; lower values yield broad belts, higher values produce patchy storms.",
                min_value: 0.1,
                max_value: 6.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_rainfall_contrast",
            ParameterMetadata {
                name: "Rainfall Contrast",
                field_name: "hydrology_rainfall_contrast",
                category: ParameterCategory::Hydrology,
                description: "Exponent that sharpens rainfall differences, exaggerating deserts vs. rainforests.",
                min_value: 0.3,
                max_value: 3.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_rainfall_dry_factor",
            ParameterMetadata {
                name: "Rainfall Dry Floor",
                field_name: "hydrology_rainfall_dry_factor",
                category: ParameterCategory::Hydrology,
                description: "Minimum rainfall multiplier retained even in the driest cells (0 allows true deserts).",
                min_value: 0.0,
                max_value: 0.8,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_river_density",
            ParameterMetadata {
                name: "River Density",
                field_name: "hydrology_river_density",
                category: ParameterCategory::Hydrology,
                description: "Fraction of the drainage network promoted to visible rivers (0-1).",
                min_value: 0.01,
                max_value: 0.6,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_river_width_scale",
            ParameterMetadata {
                name: "River Width Scale",
                field_name: "hydrology_river_width_scale",
                category: ParameterCategory::Hydrology,
                description: "Adjusts how broadly channels carve into surrounding terrain.",
                min_value: 0.2,
                max_value: 3.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_river_depth_scale",
            ParameterMetadata {
                name: "River Depth Scale",
                field_name: "hydrology_river_depth_scale",
                category: ParameterCategory::Hydrology,
                description: "Controls maximum channel excavation depth for major rivers (blocks).",
                min_value: 2.0,
                max_value: 40.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_meander_strength",
            ParameterMetadata {
                name: "Meander Strength",
                field_name: "hydrology_meander_strength",
                category: ParameterCategory::Hydrology,
                description: "Increases lateral wandering of rivers for more sinuous paths.",
                min_value: 0.0,
                max_value: 1.5,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_pond_density",
            ParameterMetadata {
                name: "Pond Density",
                field_name: "hydrology_pond_density",
                category: ParameterCategory::Hydrology,
                description: "How readily enclosed basins become ponds (higher = more ponds).",
                min_value: 0.01,
                max_value: 0.8,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_pond_min_radius",
            ParameterMetadata {
                name: "Pond Min Radius",
                field_name: "hydrology_pond_min_radius",
                category: ParameterCategory::Hydrology,
                description: "Minimum representative radius for generated ponds (blocks).",
                min_value: 2.0,
                max_value: 32.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_pond_max_radius",
            ParameterMetadata {
                name: "Pond Max Radius",
                field_name: "hydrology_pond_max_radius",
                category: ParameterCategory::Hydrology,
                description: "Maximum representative radius for ponds before they merge into lakes (blocks).",
                min_value: 4.0,
                max_value: 96.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_estuary_length",
            ParameterMetadata {
                name: "Estuary Length",
                field_name: "hydrology_estuary_length",
                category: ParameterCategory::Hydrology,
                description: "Distance inland (blocks) to blend rivers into coastal water levels.",
                min_value: 32.0,
                max_value: 1024.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_floodplain_radius",
            ParameterMetadata {
                name: "Floodplain Radius",
                field_name: "hydrology_floodplain_radius",
                category: ParameterCategory::Hydrology,
                description:
                    "Softens terrain near rivers and ponds to this approximate distance (blocks).",
                min_value: 0.0,
                max_value: 96.0,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_coastal_blend",
            ParameterMetadata {
                name: "Coastal Blend",
                field_name: "hydrology_coastal_blend",
                category: ParameterCategory::Hydrology,
                description: "Strength of shoreline smoothing applied around coasts (0-1).",
                min_value: 0.0,
                max_value: 1.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_major_river_count",
            ParameterMetadata {
                name: "Major River Count",
                field_name: "hydrology_major_river_count",
                category: ParameterCategory::Hydrology,
                description: "Maximum number of long continental rivers promoted each world.",
                min_value: 0.0,
                max_value: 32.0,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_major_river_min_flow",
            ParameterMetadata {
                name: "Major River Min Flow",
                field_name: "hydrology_major_river_min_flow",
                category: ParameterCategory::Hydrology,
                description: "Fraction of peak discharge required for a channel to remain part of a major river.",
                min_value: 0.0,
                max_value: 0.5,
                units: None,
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        metadata.insert(
            "hydrology_major_river_depth_boost",
            ParameterMetadata {
                name: "Major River Depth Boost",
                field_name: "hydrology_major_river_depth_boost",
                category: ParameterCategory::Hydrology,
                description: "Minimum depth multiplier applied along major river trunks (blocks).",
                min_value: 0.1,
                max_value: 2.5,
                units: Some("blocks"),
                ui_visible: true,
                ui_tab: Some("Hydrology"),
            },
        );

        // Add remaining parameters with sensible defaults...
        // (Abbreviated for brevity - would include all parameters)

        Self { metadata }
    }

    pub fn get(&self, field_name: &str) -> Option<&ParameterMetadata> {
        self.metadata.get(field_name)
    }

    pub fn all_ui_visible(&self) -> Vec<&ParameterMetadata> {
        self.metadata.values().filter(|m| m.ui_visible).collect()
    }

    pub fn by_category(&self, category: ParameterCategory) -> Vec<&ParameterMetadata> {
        self.metadata
            .values()
            .filter(|m| m.category == category)
            .collect()
    }

    pub fn by_tab(&self, tab: &str) -> Vec<&ParameterMetadata> {
        self.metadata
            .values()
            .filter(|m| m.ui_tab == Some(tab))
            .collect()
    }
}

impl Default for ParameterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
