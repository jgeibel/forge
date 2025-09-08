pub mod config;
pub mod coordinates;
pub mod altitude_system;

use bevy::prelude::*;

pub use config::*;
pub use altitude_system::*;

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlanetConfig>()
            .init_resource::<AltitudeRenderSystem>()
            .add_systems(Update, altitude_system::update_render_distance);
    }
}