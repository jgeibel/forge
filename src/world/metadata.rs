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

pub struct ParameterRegistry {
    metadata: HashMap<&'static str, ParameterMetadata>,
}

impl ParameterRegistry {
    pub fn new() -> Self {
        let mut metadata = HashMap::new();

        // Core Parameters
        metadata.insert("seed", ParameterMetadata {
            name: "Seed",
            field_name: "seed",
            category: ParameterCategory::Core,
            description: "Random seed for world generation",
            min_value: 0.0,
            max_value: u64::MAX as f64,
            units: None,
            ui_visible: false, // Handled separately in UI
        ui_tab: None,
        });

        metadata.insert("planet_size", ParameterMetadata {
            name: "Planet Size",
            field_name: "planet_size",
            category: ParameterCategory::Core,
            description: "Size of the planet in blocks",
            min_value: 1024.0,
            max_value: 262144.0,
            units: Some("blocks"),
            ui_visible: false, // Handled via planet size dropdown
            ui_tab: None,
        });

        metadata.insert("sea_level", ParameterMetadata {
            name: "Sea Level",
            field_name: "sea_level",
            category: ParameterCategory::Core,
            description: "Height of the sea level",
            min_value: 16.0,
            max_value: 200.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        // Ocean Parameters
        metadata.insert("ocean_depth", ParameterMetadata {
            name: "Ocean Depth",
            field_name: "ocean_depth",
            category: ParameterCategory::Ocean,
            description: "Depth of regular ocean areas",
            min_value: 10.0,
            max_value: 100.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Hydrology"),
        });

        metadata.insert("deep_ocean_depth", ParameterMetadata {
            name: "Deep Ocean Depth",
            field_name: "deep_ocean_depth",
            category: ParameterCategory::Ocean,
            description: "Depth of deep ocean trenches",
            min_value: 20.0,
            max_value: 200.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Hydrology"),
        });

        // Continent Parameters
        metadata.insert("continent_threshold", ParameterMetadata {
            name: "Continent Threshold",
            field_name: "continent_threshold",
            category: ParameterCategory::Continent,
            description: "Threshold for continent formation",
            min_value: 0.05,
            max_value: 0.6,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("continent_count", ParameterMetadata {
            name: "Continent Count",
            field_name: "continent_count",
            category: ParameterCategory::Continent,
            description: "Number of major continents",
            min_value: 1.0,
            max_value: 24.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("continent_frequency", ParameterMetadata {
            name: "Continent Frequency",
            field_name: "continent_frequency",
            category: ParameterCategory::Continent,
            description: "Frequency of continent noise",
            min_value: 0.1,
            max_value: 4.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        // Mountain Parameters
        metadata.insert("mountain_height", ParameterMetadata {
            name: "Mountain Height",
            field_name: "mountain_height",
            category: ParameterCategory::Mountain,
            description: "Maximum height of mountains",
            min_value: 50.0,
            max_value: 500.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("mountain_threshold", ParameterMetadata {
            name: "Mountain Threshold",
            field_name: "mountain_threshold",
            category: ParameterCategory::Mountain,
            description: "Threshold for mountain formation",
            min_value: 0.2,
            max_value: 0.8,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("mountain_range_count", ParameterMetadata {
            name: "Mountain Range Count",
            field_name: "mountain_range_count",
            category: ParameterCategory::Mountain,
            description: "Number of mountain ranges",
            min_value: 0.0,
            max_value: 80.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("mountain_range_width", ParameterMetadata {
            name: "Mountain Range Width",
            field_name: "mountain_range_width",
            category: ParameterCategory::Mountain,
            description: "Width of mountain ranges",
            min_value: 40.0,
            max_value: 800.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("mountain_range_strength", ParameterMetadata {
            name: "Mountain Range Strength",
            field_name: "mountain_range_strength",
            category: ParameterCategory::Mountain,
            description: "Strength of mountain ranges",
            min_value: 0.5,
            max_value: 5.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        // Climate Parameters
        metadata.insert("equator_temp_c", ParameterMetadata {
            name: "Equator Temperature",
            field_name: "equator_temp_c",
            category: ParameterCategory::Climate,
            description: "Temperature at the equator",
            min_value: 15.0,
            max_value: 45.0,
            units: Some("°C"),
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        metadata.insert("pole_temp_c", ParameterMetadata {
            name: "Pole Temperature",
            field_name: "pole_temp_c",
            category: ParameterCategory::Climate,
            description: "Temperature at the poles",
            min_value: -60.0,
            max_value: 10.0,
            units: Some("°C"),
            ui_visible: true,
            ui_tab: Some("Terrain"),
        });

        // Island Parameters
        metadata.insert("island_frequency", ParameterMetadata {
            name: "Island Frequency",
            field_name: "island_frequency",
            category: ParameterCategory::Island,
            description: "Frequency of island chains",
            min_value: 1.0,
            max_value: 20.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Islands"),
        });

        metadata.insert("island_threshold", ParameterMetadata {
            name: "Island Threshold",
            field_name: "island_threshold",
            category: ParameterCategory::Island,
            description: "Threshold for island formation",
            min_value: 0.3,
            max_value: 0.8,
            units: None,
            ui_visible: true,
            ui_tab: Some("Islands"),
        });

        metadata.insert("island_height", ParameterMetadata {
            name: "Island Height",
            field_name: "island_height",
            category: ParameterCategory::Island,
            description: "Maximum height of islands",
            min_value: 20.0,
            max_value: 200.0,
            units: Some("blocks"),
            ui_visible: true,
            ui_tab: Some("Islands"),
        });

        // Hydrology Parameters
        metadata.insert("hydrology_resolution", ParameterMetadata {
            name: "Hydrology Resolution",
            field_name: "hydrology_resolution",
            category: ParameterCategory::Hydrology,
            description: "Resolution of hydrology simulation",
            min_value: 64.0,
            max_value: 8192.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Hydrology"),
        });

        metadata.insert("river_flow_threshold", ParameterMetadata {
            name: "River Flow Threshold",
            field_name: "river_flow_threshold",
            category: ParameterCategory::River,
            description: "Minimum flow for river formation",
            min_value: 10.0,
            max_value: 1000.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Hydrology"),
        });

        metadata.insert("lake_flow_threshold", ParameterMetadata {
            name: "Lake Flow Threshold",
            field_name: "lake_flow_threshold",
            category: ParameterCategory::Lake,
            description: "Minimum flow for lake formation",
            min_value: 10.0,
            max_value: 1000.0,
            units: None,
            ui_visible: true,
            ui_tab: Some("Hydrology"),
        });

        // Add remaining parameters with sensible defaults...
        // (Abbreviated for brevity - would include all parameters)

        Self { metadata }
    }

    pub fn get(&self, field_name: &str) -> Option<&ParameterMetadata> {
        self.metadata.get(field_name)
    }

    pub fn all_ui_visible(&self) -> Vec<&ParameterMetadata> {
        self.metadata
            .values()
            .filter(|m| m.ui_visible)
            .collect()
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