pub mod config;
pub mod coordinates;
pub mod altitude_system;
pub mod celestial_data;

use bevy::prelude::*;

pub use config::*;
pub use altitude_system::*;
pub use celestial_data::{CelestialData, RotationDirection, AtmosphericComposition};

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlanetConfig>()
            .init_resource::<CelestialData>()
            .init_resource::<AltitudeRenderSystem>()
            .add_systems(Startup, setup_planet)
            .add_systems(Update, altitude_system::update_render_distance);
    }
}

fn setup_planet(
    mut commands: Commands,
    planet_config: Res<PlanetConfig>,
) {
    // Create celestial data based on the planet config
    let celestial = CelestialData::earth_like(planet_config.name.clone());

    info!("Initialized planet: {}", celestial.name);
    info!("  Size: {} chunks ({:.1} km circumference)",
        planet_config.size_chunks,
        (planet_config.size_chunks * 32) as f32 / 1000.0
    );
    info!("  Distance from sun: {} AU", celestial.orbital_radius);
    info!("  Day length: {} hours", celestial.rotation_period);
    info!("  Year length: {} days", celestial.orbital_period);
    info!("  Surface gravity: {}g", celestial.surface_gravity);
    info!("  Average temperature: {:.1}°C", celestial.base_temperature - 273.15);
    info!("  Solar constant: {} W/m²", celestial.solar_constant);

    commands.insert_resource(celestial);
}