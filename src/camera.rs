use crate::loading::GameState;
use crate::planet::config::PlanetConfig;
use crate::world::WorldGenerator;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::{FogFalloff, FogSettings};
use bevy::prelude::*;
use bevy::render::camera::PerspectiveProjection;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct CameraController {
    pub move_speed: f32,
    pub look_sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub fly_mode: bool,
    pub last_space_press: f64,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            move_speed: 15.0,
            look_sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
            fly_mode: false,
            last_space_press: 0.0,
        }
    }
}

pub const GRAVITY: f32 = -20.0;
pub const JUMP_VELOCITY: f32 = 8.0;
pub const DOUBLE_TAP_TIME: f64 = 0.3;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_camera)
            .add_systems(Update, camera_look.run_if(in_state(GameState::Playing)));
    }
}

/// Choose a central spawn position anchored a few blocks above the terrain height.
pub fn find_guaranteed_land_spawn(world_gen: &WorldGenerator, planet_size: f32) -> Vec3 {
    let center = (planet_size.max(1.0)) * 0.5;
    let height = world_gen.get_height(center, center);
    // Spawn a few blocks above ground so the player settles naturally.
    Vec3::new(center, height + 4.0, center)
}

fn setup_camera(
    mut commands: Commands,
    loading_progress: Res<crate::loading::LoadingProgress>,
    planet_config: Res<PlanetConfig>,
) {
    // Use the spawn position determined during loading
    let (spawn_x, spawn_y, spawn_z) = if let Some(spawn_pos) = loading_progress.spawn_position {
        info!("Using pre-determined spawn position: {:?}", spawn_pos);
        (spawn_pos.x, spawn_pos.y, spawn_pos.z)
    } else {
        warn!("No spawn position found in loading progress, using default");
        let center = planet_config.size_chunks as f32 * 16.0;
        (center, 80.0, center)
    };

    info!("Spawning camera at ({}, {}, {})", spawn_x, spawn_y, spawn_z);

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(spawn_x, spawn_y, spawn_z).looking_at(
                Vec3::new(spawn_x + 10.0, spawn_y - 5.0, spawn_z + 10.0),
                Vec3::Y,
            ),
            projection: PerspectiveProjection {
                near: 0.1,                  // Standard near plane
                far: 400.0,                 // Reduced far plane for better precision
                fov: 70.0_f32.to_radians(), // Minecraft uses 70 degrees FOV
                ..default()
            }
            .into(),
            ..default()
        },
        PlayerCamera,
        CameraController::default(),
        FogSettings {
            color: Color::srgba(0.7, 0.8, 0.9, 1.0),
            falloff: FogFalloff::Linear {
                start: 80.0, // Start fog earlier (2.5 chunks)
                end: 256.0,  // End at 8 chunks (8 * 32 = 256 blocks)
            },
            ..default()
        },
    ));
}

// Movement is now handled by physics::player module

fn camera_look(
    mut motion_events: EventReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut CameraController), With<PlayerCamera>>,
) {
    // Console now handles its own input blocking
    let Ok((mut transform, mut controller)) = query.get_single_mut() else {
        return;
    };

    let mut delta = Vec2::ZERO;
    for event in motion_events.read() {
        delta += event.delta;
    }

    if delta.length_squared() > 0.0 {
        controller.yaw -= delta.x * controller.look_sensitivity;
        controller.pitch -= delta.y * controller.look_sensitivity;
        controller.pitch = controller.pitch.clamp(-1.5, 1.5);

        transform.rotation =
            Quat::from_rotation_y(controller.yaw) * Quat::from_rotation_x(controller.pitch);
    }
}
