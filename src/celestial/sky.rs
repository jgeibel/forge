use crate::celestial::sun::SunPosition;
use crate::celestial::time::GameTime;
use bevy::prelude::*;

#[derive(Component)]
pub struct SkyDome;

#[derive(Resource)]
pub struct SkySettings {
    pub base_color: Color,
    pub horizon_color: Color,
    pub sun_color: Color,
    pub star_visibility: f32, // 0.0 = no stars, 1.0 = full visibility
}

impl Default for SkySettings {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.5, 0.7, 1.0),
            horizon_color: Color::srgb(0.7, 0.8, 0.9),
            sun_color: Color::srgb(1.0, 0.95, 0.8),
            star_visibility: 0.0,
        }
    }
}

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkySettings>()
            .add_systems(Update, update_sky_colors);
    }
}

fn update_sky_colors(
    game_time: Res<GameTime>,
    sun_position: Res<SunPosition>,
    mut sky_settings: ResMut<SkySettings>,
    mut clear_color: ResMut<ClearColor>,
) {
    let sun_angle = sun_position.angle_from_horizon;

    if sun_angle <= -0.2 {
        // Deep night - dark blue/black with stars
        sky_settings.base_color = Color::srgb(0.02, 0.02, 0.08);
        sky_settings.horizon_color = Color::srgb(0.05, 0.05, 0.15);
        sky_settings.sun_color = Color::srgb(0.8, 0.8, 0.9); // Moon color
        sky_settings.star_visibility = 1.0;
    } else if sun_angle <= 0.0 {
        // Twilight - deep blue to purple gradient
        let t = (sun_angle + 0.2) / 0.2;
        sky_settings.base_color = Color::srgb(0.02 + 0.18 * t, 0.02 + 0.18 * t, 0.08 + 0.32 * t);
        sky_settings.horizon_color = Color::srgb(0.05 + 0.35 * t, 0.05 + 0.25 * t, 0.15 + 0.35 * t);
        sky_settings.sun_color = Color::srgb(0.8 + 0.2 * t, 0.8 + 0.15 * t, 0.9 - 0.1 * t);
        sky_settings.star_visibility = 1.0 - t;
    } else if sun_angle < 0.15 {
        // Golden hour - orange/pink gradient
        let t = sun_angle / 0.15;
        sky_settings.base_color = Color::srgb(0.2 + 0.3 * t, 0.2 + 0.5 * t, 0.4 + 0.6 * t);
        sky_settings.horizon_color = Color::srgb(
            0.4 + 0.3 * (1.0 - t) + 0.3 * t, // Orange to light blue
            0.3 + 0.2 * (1.0 - t) + 0.5 * t,
            0.5 + 0.4 * t,
        );
        sky_settings.sun_color = Color::srgb(
            1.0,
            0.95 - 0.15 * (1.0 - t), // More orange at horizon
            0.8 - 0.3 * (1.0 - t),
        );
        sky_settings.star_visibility = 0.0;
    } else {
        // Daytime - blue sky
        let intensity = (sun_angle / (std::f32::consts::PI / 2.0)).min(1.0);
        sky_settings.base_color = Color::srgb(
            0.4 + 0.1 * intensity,
            0.6 + 0.1 * intensity,
            0.95 + 0.05 * intensity,
        );
        sky_settings.horizon_color = Color::srgb(
            0.7 + 0.1 * intensity,
            0.8 + 0.1 * intensity,
            0.9 + 0.05 * intensity,
        );
        sky_settings.sun_color = Color::srgb(
            1.0,
            0.98,
            0.95 - 0.05 * (1.0 - intensity), // Slightly yellower when lower
        );
        sky_settings.star_visibility = 0.0;
    }

    // Update clear color to match sky
    clear_color.0 = sky_settings.base_color;
}

// Helper function to create a gradient between sky colors
pub fn sky_gradient(base: Color, horizon: Color, height_factor: f32) -> Color {
    let t = height_factor.clamp(0.0, 1.0);
    // Simple interpolation without component access
    if t < 0.5 {
        horizon
    } else {
        base
    }
}
