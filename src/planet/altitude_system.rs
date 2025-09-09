use bevy::prelude::*;
use crate::camera::PlayerCamera;

// Simple render distance constants
const RENDER_DISTANCE_GROUND: f32 = 8.0;   // 8 chunks at ground level
const RENDER_DISTANCE_HIGH: f32 = 8.0;     // 8 chunks when high up

#[derive(Resource)]
pub struct AltitudeRenderSystem {
    pub render_distance: f32,
}

impl Default for AltitudeRenderSystem {
    fn default() -> Self {
        Self {
            render_distance: RENDER_DISTANCE_GROUND,
        }
    }
}

pub fn update_render_distance(
    mut altitude_system: ResMut<AltitudeRenderSystem>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    
    let altitude = camera_transform.translation.y;
    
    // Simple render distance based on altitude
    altitude_system.render_distance = if altitude > 128.0 {
        RENDER_DISTANCE_HIGH
    } else {
        RENDER_DISTANCE_GROUND
    };
}

/// Check if chunks should be rendered (always true now)
pub fn should_render_chunks(_altitude: f32) -> bool {
    true
}