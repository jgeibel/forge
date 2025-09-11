use bevy::prelude::*;
use std::f32::consts::PI;
use crate::chunk::ChunkPos;
use crate::celestial::time::GameTime;
use crate::celestial::sun::calculate_local_sun_angle;

#[derive(Component, Debug, Clone)]
pub struct ChunkLighting {
    pub chunk_pos: ChunkPos,
    pub sun_angle: f32,  // Local sun elevation angle
    pub light_level: f32,  // 0.0 = full dark, 1.0 = full bright
    pub is_day: bool,
    pub sky_color: Color,
    pub fog_color: Color,
    pub last_update: f64,
}

impl ChunkLighting {
    pub fn new(chunk_pos: ChunkPos) -> Self {
        Self {
            chunk_pos,
            sun_angle: 0.0,
            light_level: 1.0,
            is_day: true,
            sky_color: Color::srgb(0.5, 0.7, 1.0),
            fog_color: Color::srgb(0.7, 0.8, 0.9),
            last_update: 0.0,
        }
    }
    
    pub fn update(&mut self, game_time: &GameTime) {
        // Calculate chunk center position
        let chunk_world_pos = self.chunk_pos.to_world_pos();
        let chunk_center_x = chunk_world_pos.x + 16.0;  // Center of 32-block chunk
        let chunk_center_z = chunk_world_pos.z + 16.0;
        
        // Calculate local sun angle for this chunk
        self.sun_angle = calculate_local_sun_angle(
            chunk_center_x,
            chunk_center_z,
            game_time,
        );
        
        // Determine if it's day or night for this chunk
        self.is_day = self.sun_angle > 0.0;
        
        // Calculate light level and colors based on sun angle
        if self.sun_angle <= -0.2 {
            // Deep night
            self.light_level = 0.1;  // Moonlight
            self.sky_color = Color::srgb(0.05, 0.05, 0.15);
            self.fog_color = Color::srgb(0.02, 0.02, 0.05);
        } else if self.sun_angle <= 0.0 {
            // Twilight (dusk/dawn)
            let t = (self.sun_angle + 0.2) / 0.2;  // -0.2 to 0.0 -> 0.0 to 1.0
            self.light_level = 0.1 + 0.4 * t;
            
            // Gradient from night to twilight colors
            self.sky_color = Color::srgb(
                0.05 + 0.45 * t,
                0.05 + 0.25 * t,
                0.15 + 0.35 * t,
            );
            self.fog_color = Color::srgb(
                0.02 + 0.48 * t,
                0.02 + 0.28 * t,
                0.05 + 0.35 * t,
            );
        } else if self.sun_angle < 0.1 {
            // Golden hour (sunrise/sunset)
            let t = self.sun_angle / 0.1;  // 0.0 to 0.1 -> 0.0 to 1.0
            self.light_level = 0.5 + 0.5 * t;
            
            // Orange/pink to blue gradient
            self.sky_color = Color::srgb(
                0.5 + 0.2 * (1.0 - t),  // More red at horizon
                0.3 + 0.4 * t,
                0.5 + 0.5 * t,
            );
            self.fog_color = Color::srgb(
                0.5 + 0.2 * (1.0 - t),
                0.3 + 0.5 * t,
                0.4 + 0.5 * t,
            );
        } else {
            // Full daylight
            let intensity = (self.sun_angle / (PI / 2.0)).min(1.0);
            self.light_level = 1.0;
            
            // Bright blue sky, brighter at noon
            self.sky_color = Color::srgb(
                0.4 + 0.1 * intensity,
                0.6 + 0.1 * intensity,
                0.9 + 0.1 * intensity,
            );
            self.fog_color = Color::srgb(
                0.7 + 0.1 * intensity,
                0.8 + 0.1 * intensity,
                0.9 + 0.05 * intensity,
            );
        }
        
        self.last_update = game_time.total_seconds;
    }
    
    // Interpolate between two chunk lighting states for smooth boundaries
    pub fn interpolate(&self, other: &ChunkLighting, t: f32) -> ChunkLighting {
        ChunkLighting {
            chunk_pos: self.chunk_pos,
            sun_angle: self.sun_angle + (other.sun_angle - self.sun_angle) * t,
            light_level: self.light_level + (other.light_level - self.light_level) * t,
            is_day: if t < 0.5 { self.is_day } else { other.is_day },
            // Simple color interpolation without component access
            sky_color: if t < 0.5 { self.sky_color } else { other.sky_color },
            fog_color: if t < 0.5 { self.fog_color } else { other.fog_color },
            last_update: self.last_update.max(other.last_update),
        }
    }
}

#[derive(Resource)]
pub struct LightingUpdateTimer {
    timer: Timer,
}

impl Default for LightingUpdateTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<LightingUpdateTimer>()
            .add_systems(Update, (
                update_chunk_lighting,
                apply_player_lighting,
            ).chain());
    }
}

fn update_chunk_lighting(
    time: Res<Time>,
    game_time: Res<GameTime>,
    mut timer: ResMut<LightingUpdateTimer>,
    mut chunk_query: Query<&mut ChunkLighting>,
) {
    timer.timer.tick(time.delta());
    
    if timer.timer.just_finished() {
        // Update lighting for all loaded chunks
        for mut lighting in chunk_query.iter_mut() {
            // Only update if enough time has passed (optimization)
            if game_time.total_seconds - lighting.last_update > 0.5 {
                lighting.update(&game_time);
            }
        }
    }
}

fn apply_player_lighting(
    _player_query: Query<&Transform, With<crate::camera::CameraController>>,
    _chunk_query: Query<&ChunkLighting>,
) {
    // TODO: Re-enable fog integration once we figure out the correct FogSettings API
    // For now, the sky color changes and sun movement will demonstrate the day/night cycle
}