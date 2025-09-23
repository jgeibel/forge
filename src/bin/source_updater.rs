use std::fs;
use syn::{Item, Expr, parse_file};
use forge::world::WorldGenConfig;

/// Represents a changed parameter with its new value
#[derive(Debug, Clone)]
pub struct ParameterChange {
    pub const_name: String,
    pub new_value: String,
}

/// Detects which parameters have changed from defaults
pub fn detect_changes(working: &WorldGenConfig, defaults: &WorldGenConfig) -> Vec<ParameterChange> {
    let mut changes = Vec::new();

    // Helper macro to check each field
    macro_rules! check_field {
        ($field:ident, $const_name:expr) => {
            if working.$field != defaults.$field {
                let value = match stringify!($field) {
                    // u64 fields
                    "seed" => format!("{}", working.$field),
                    // u32 fields
                    "planet_size" | "continent_count" | "mountain_range_count" |
                    "hydrology_resolution" | "hydrology_major_river_count" => format!("{}", working.$field),
                    // f64 fields
                    "continent_frequency" | "detail_frequency" | "mountain_frequency" |
                    "moisture_frequency" | "island_frequency" | "hydrology_rainfall_frequency" => format!("{}", working.$field),
                    // f32 fields (everything else)
                    _ => format!("{}", working.$field),
                };
                changes.push(ParameterChange {
                    const_name: $const_name.to_string(),
                    new_value: value,
                });
            }
        };
    }

    // Check all fields
    check_field!(seed, "SEED");
    check_field!(planet_size, "PLANET_SIZE");
    check_field!(sea_level, "SEA_LEVEL");
    check_field!(ocean_depth, "OCEAN_DEPTH");
    check_field!(deep_ocean_depth, "DEEP_OCEAN_DEPTH");
    check_field!(continent_threshold, "CONTINENT_THRESHOLD");
    check_field!(continent_power, "CONTINENT_POWER");
    check_field!(continent_bias, "CONTINENT_BIAS");
    check_field!(continent_count, "CONTINENT_COUNT");
    check_field!(continent_radius, "CONTINENT_RADIUS");
    check_field!(continent_edge_power, "CONTINENT_EDGE_POWER");
    check_field!(continent_frequency, "CONTINENT_FREQUENCY");
    check_field!(detail_frequency, "DETAIL_FREQUENCY");
    check_field!(detail_amplitude, "DETAIL_AMPLITUDE");
    check_field!(mountain_frequency, "MOUNTAIN_FREQUENCY");
    check_field!(mountain_height, "MOUNTAIN_HEIGHT");
    check_field!(mountain_threshold, "MOUNTAIN_THRESHOLD");
    check_field!(mountain_range_count, "MOUNTAIN_RANGE_COUNT");
    check_field!(mountain_range_width, "MOUNTAIN_RANGE_WIDTH");
    check_field!(mountain_range_strength, "MOUNTAIN_RANGE_STRENGTH");
    check_field!(mountain_range_spur_chance, "MOUNTAIN_RANGE_SPUR_CHANCE");
    check_field!(mountain_range_spur_strength, "MOUNTAIN_RANGE_SPUR_STRENGTH");
    check_field!(mountain_range_roughness, "MOUNTAIN_RANGE_ROUGHNESS");
    check_field!(moisture_frequency, "MOISTURE_FREQUENCY");
    check_field!(equator_temp_c, "EQUATOR_TEMP_C");
    check_field!(pole_temp_c, "POLE_TEMP_C");
    check_field!(lapse_rate_c_per_block, "LAPSE_RATE_C_PER_BLOCK");
    check_field!(temperature_variation, "TEMPERATURE_VARIATION");
    check_field!(highland_bonus, "HIGHLAND_BONUS");
    check_field!(island_frequency, "ISLAND_FREQUENCY");
    check_field!(island_threshold, "ISLAND_THRESHOLD");
    check_field!(island_height, "ISLAND_HEIGHT");
    check_field!(island_falloff, "ISLAND_FALLOFF");
    check_field!(hydrology_resolution, "HYDROLOGY_RESOLUTION");
    check_field!(hydrology_rainfall, "HYDROLOGY_RAINFALL");
    check_field!(hydrology_rainfall_variance, "HYDROLOGY_RAINFALL_VARIANCE");
    check_field!(hydrology_rainfall_frequency, "HYDROLOGY_RAINFALL_FREQUENCY");
    check_field!(hydrology_major_river_count, "HYDROLOGY_MAJOR_RIVER_COUNT");
    check_field!(hydrology_major_river_boost, "HYDROLOGY_MAJOR_RIVER_BOOST");
    check_field!(river_flow_threshold, "RIVER_FLOW_THRESHOLD");
    check_field!(river_depth_scale, "RIVER_DEPTH_SCALE");
    check_field!(river_max_depth, "RIVER_MAX_DEPTH");
    check_field!(river_surface_ratio, "RIVER_SURFACE_RATIO");
    check_field!(lake_flow_threshold, "LAKE_FLOW_THRESHOLD");
    check_field!(lake_depth, "LAKE_DEPTH");
    check_field!(lake_shore_blend, "LAKE_SHORE_BLEND");

    changes
}

/// Updates the defaults.rs source file with the changed parameters
pub fn update_source_file(changes: &[ParameterChange]) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }

    let defaults_path = "src/world/defaults.rs";
    let source = fs::read_to_string(defaults_path)
        .map_err(|e| format!("Failed to read defaults.rs: {}", e))?;

    let mut syntax_tree = parse_file(&source)
        .map_err(|e| format!("Failed to parse defaults.rs: {}", e))?;

    // Update the constants in the AST
    for item in &mut syntax_tree.items {
        if let Item::Const(item_const) = item {
            let const_name = item_const.ident.to_string();

            // Check if this constant needs updating
            if let Some(change) = changes.iter().find(|c| c.const_name == const_name) {
                // Parse the new value based on type
                let new_expr = if const_name == "SEED" {
                    syn::parse_str::<Expr>(&format!("{}_u64", change.new_value))
                } else if const_name == "PLANET_SIZE" || const_name.contains("COUNT") ||
                          const_name == "HYDROLOGY_RESOLUTION" {
                    syn::parse_str::<Expr>(&format!("{}_u32", change.new_value))
                } else if const_name.contains("FREQUENCY") && !const_name.contains("RAINFALL") {
                    syn::parse_str::<Expr>(&format!("{}_f64", change.new_value))
                } else {
                    syn::parse_str::<Expr>(&format!("{}_f32", change.new_value))
                };

                if let Ok(expr) = new_expr {
                    item_const.expr = Box::new(expr);
                }
            }
        }
    }

    // Convert the AST back to source code
    let updated_source = prettyplease::unparse(&syntax_tree);

    // Write the updated source back to the file
    fs::write(defaults_path, updated_source)
        .map_err(|e| format!("Failed to write updated defaults.rs: {}", e))?;

    println!("Updated {} parameters in defaults.rs", changes.len());
    for change in changes {
        println!("  {} = {}", change.const_name, change.new_value);
    }

    Ok(())
}