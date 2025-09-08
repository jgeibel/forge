use bevy::prelude::*;
use bevy::pbr::{FogSettings, FogFalloff};
use crate::camera::PlayerCamera;
use crate::planet::altitude_system::AltitudeRenderSystem;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_fog)
            .add_systems(Update, update_fog_by_altitude);
    }
}

fn setup_fog() {
    // Fog will be added to camera in camera module
}

fn update_fog_by_altitude(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut fog_query: Query<&mut FogSettings>,
    _altitude_system: Res<AltitudeRenderSystem>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    
    let Ok(mut fog) = fog_query.get_single_mut() else {
        return;
    };
    
    let altitude = camera_transform.translation.y;
    
    // Simple fog that gets slightly denser at higher altitudes
    let t = (altitude / 256.0).min(1.0);
    fog.falloff = FogFalloff::Linear {
        start: 200.0 - 50.0 * t,  // 200 at ground, 150 at max altitude
        end: 500.0 - 100.0 * t,    // 500 at ground, 400 at max altitude
    };
    fog.color = Color::srgba(0.7, 0.8, 0.9, 1.0);
}