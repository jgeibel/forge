use bevy::prelude::*;

pub mod lighting;
pub mod sky;
pub mod sun;
pub mod time;

use lighting::LightingPlugin;
use sky::SkyPlugin;
use sun::SunPlugin;
use time::TimePlugin;

pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((TimePlugin, SunPlugin, LightingPlugin, SkyPlugin))
            .add_systems(Startup, setup_celestial_system);
    }
}

fn setup_celestial_system() {
    info!("Initializing celestial system with day/night cycle");

    // The sun entity will be created by the SunPlugin
    // The sky will be managed by the SkyPlugin
    // Time tracking is handled by TimePlugin
}
