use bevy::prelude::*;

pub mod time;
pub mod sun;
pub mod lighting;
pub mod sky;

use time::{GameTime, TimePlugin};
use sun::{SunPlugin, SunPosition};
use lighting::LightingPlugin;
use sky::SkyPlugin;

pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                TimePlugin,
                SunPlugin,
                LightingPlugin,
                SkyPlugin,
            ))
            .add_systems(Startup, setup_celestial_system);
    }
}

fn setup_celestial_system(
    mut commands: Commands,
) {
    info!("Initializing celestial system with day/night cycle");
    
    // The sun entity will be created by the SunPlugin
    // The sky will be managed by the SkyPlugin
    // Time tracking is handled by TimePlugin
}