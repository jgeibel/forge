use crate::camera::{CameraController, PlayerCamera};
use crate::celestial::time::GameTime;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::loading::GameState;
use crate::physics::PlayerPhysics;
use crate::planet::CelestialData;
use crate::world::CurrentTemperature;
use bevy::prelude::*;

#[derive(Resource)]
pub struct DebugOverlayState {
    pub enabled: bool,
}

impl Default for DebugOverlayState {
    fn default() -> Self {
        Self { enabled: false }
    }
}

pub struct DebugOverlayPlugin;

impl Plugin for DebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlayState>()
            .add_systems(OnEnter(GameState::Playing), setup_debug_overlay)
            .add_systems(
                Update,
                (toggle_debug_overlay, update_debug_text).run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
struct DebugText;

fn setup_debug_overlay(mut commands: Commands) {
    // Create debug text in top-left corner
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "", // Title will be set dynamically
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(1.0, 1.0, 0.0), // Yellow
                    ..default()
                },
            ),
            TextSection::new(
                "",
                TextStyle {
                    font_size: 16.0,
                    color: Color::srgb(1.0, 1.0, 1.0), // White
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        DebugText,
    ));
}

fn toggle_debug_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_state: ResMut<DebugOverlayState>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        debug_state.enabled = !debug_state.enabled;
        info!(
            "Debug overlay {}",
            if debug_state.enabled {
                "ENABLED (F3)"
            } else {
                "DISABLED (F3)"
            }
        );
    }
}

fn update_debug_text(
    mut text_query: Query<&mut Text, With<DebugText>>,
    player_query: Query<(&Transform, &PlayerPhysics, &CameraController), With<PlayerCamera>>,
    temperature: Res<CurrentTemperature>,
    chunk_query: Query<(&Chunk, &ChunkPos)>,
    debug_state: Res<DebugOverlayState>,
    game_time: Res<GameTime>,
    planet: Res<CelestialData>,
) {
    let Ok((transform, physics, controller)) = player_query.get_single() else {
        return;
    };

    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    if debug_state.enabled {
        // Debug mode - show detailed technical info
        text.sections[0].value = "DEBUG INFO (F3 to toggle)\n".to_string();

        // Calculate player's feet position from AABB (more accurate)
        let eye_pos = transform.translation;
        let feet_pos = Vec3::new(
            eye_pos.x,
            physics.aabb.center.y - physics.aabb.half_extents.y, // AABB bottom is the feet
            eye_pos.z,
        );

        // Find distance to ground below
        let mut ground_distance = 999.0;
        let mut ground_y = -999.0;

        // Check straight down from feet
        for check_y in (0..1000).map(|i| feet_pos.y - i as f32 * 0.01) {
            let check_pos = Vec3::new(feet_pos.x, check_y, feet_pos.z);
            if is_solid_at(check_pos, &chunk_query) {
                // Found a solid block - the top of the block is at floor + 1
                ground_y = check_y.floor() + 1.0;
                ground_distance = feet_pos.y - ground_y;
                break;
            }
        }

        // Calculate which block player is standing in/above
        let block_y = feet_pos.y.floor();
        let offset_in_block = feet_pos.y - block_y;

        // Update debug text with more detailed info
        let aabb_bottom = physics.aabb.center.y - physics.aabb.half_extents.y;
        let aabb_top = physics.aabb.center.y + physics.aabb.half_extents.y;

        text.sections[1].value = format!(
            "Position: X={:.2}, Y={:.2}, Z={:.2}\n\
         Eye Y: {:.2}\n\
         Feet Y: {:.2}\n\
         AABB Center Y: {:.2}\n\
         AABB Bottom: {:.2}, Top: {:.2}\n\
         Block Y: {}\n\
         Offset in block: {:.3}\n\
         Ground Y: {:.2}\n\
         Above ground: {:.3} blocks\n\
         Grounded: {}\n\
         Velocity Y: {:.2}\n\
         Fly Mode: {}\n\
         \n\
         Planet: {}\n\
         Distance from sun: {} AU\n\
         Day length: {} hours\n\
         Gravity: {}g\n\
         \n\
         Expected feet on ground: Y = {}.00\n\
         Actual difference: {:.3}",
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            eye_pos.y,
            feet_pos.y,
            physics.aabb.center.y,
            aabb_bottom,
            aabb_top,
            block_y as i32,
            offset_in_block,
            ground_y,
            ground_distance,
            physics.is_grounded,
            physics.velocity.y,
            controller.fly_mode,
            planet.name,
            planet.orbital_radius,
            planet.rotation_period,
            planet.surface_gravity,
            ground_y as i32,
            feet_pos.y - ground_y,
        );
    } else {
        // Normal mode - show simple position and temperature
        text.sections[0].value = "".to_string(); // No title in normal mode

        // Calculate time display
        let hour = game_time.current_hour as u32;
        let minute = ((game_time.current_hour - hour as f32) * 60.0) as u32;
        let day = game_time.current_day;
        let time_string = format!("Day {} - {:02}:{:02}", day + 1, hour, minute);

        // Simple display with position, temperature, time, and planet info
        text.sections[1].value = format!(
            "Position: X={:.0}, Y={:.0}, Z={:.0}\n\
             Temperature: {:.1}°C\n\
             {}\n\
             Planet: {} ({:.1}g gravity)\n\
             {}",
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            temperature.celsius,
            time_string,
            planet.name,
            planet.surface_gravity,
            if controller.fly_mode {
                "🚁 Fly Mode"
            } else {
                ""
            }
        );
    }
}

fn is_solid_at(pos: Vec3, chunk_query: &Query<(&Chunk, &ChunkPos)>) -> bool {
    let block_pos = IVec3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    );

    let chunk_pos = ChunkPos::new(
        (block_pos.x as f32 / CHUNK_SIZE_F32).floor() as i32,
        (block_pos.y as f32 / CHUNK_SIZE_F32).floor() as i32,
        (block_pos.z as f32 / CHUNK_SIZE_F32).floor() as i32,
    );

    for (chunk, pos) in chunk_query.iter() {
        if *pos == chunk_pos {
            let local_x = block_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = block_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = block_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;

            return chunk.get_block(local_x, local_y, local_z).is_solid();
        }
    }

    false
}
