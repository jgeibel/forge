use crate::camera::PlayerCamera;
use bevy::prelude::*;

// Simple render distance constants (chunk count)
// Keep ground distance aligned with the far-tile inner exclusion so there is
// no visible gap between near meshes and distant tiles.
const RENDER_DISTANCE_GROUND: f32 = 10.0;
const RENDER_DISTANCE_HIGH: f32 = 12.0;

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
    altitude_system.render_distance = if altitude > 160.0 {
        RENDER_DISTANCE_HIGH
    } else {
        RENDER_DISTANCE_GROUND
    };
}

/// Check if chunks should be rendered (always true now)
pub fn should_render_chunks(_altitude: f32) -> bool {
    true
}
