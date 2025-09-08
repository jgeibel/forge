use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::{FogSettings, FogFalloff};

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct CameraController {
    pub move_speed: f32,
    pub look_sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            move_speed: 15.0,
            look_sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_camera)
            .add_systems(Update, (
                camera_movement,
                camera_look,
            ));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(16.0, 20.0, 16.0)
                .looking_at(Vec3::new(16.0, 10.0, 0.0), Vec3::Y),
            ..default()
        },
        PlayerCamera,
        CameraController::default(),
        FogSettings {
            color: Color::srgba(0.7, 0.8, 0.9, 1.0),
            falloff: FogFalloff::Linear {
                start: 200.0,
                end: 500.0,
            },
            ..default()
        },
    ));
}

fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &CameraController), With<PlayerCamera>>,
) {
    let Ok((mut transform, controller)) = query.get_single_mut() else {
        return;
    };
    
    let mut velocity = Vec3::ZERO;
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();
    
    if keyboard.pressed(KeyCode::KeyW) {
        velocity += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        velocity -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        velocity -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        velocity += right;
    }
    if keyboard.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        velocity -= Vec3::Y;
    }
    
    if velocity.length_squared() > 0.0 {
        velocity = velocity.normalize();
        transform.translation += velocity * controller.move_speed * time.delta_seconds();
        
        // Clamp altitude to maximum
        transform.translation.y = transform.translation.y.min(crate::planet::config::MAX_ALTITUDE);
    }
}

fn camera_look(
    mut motion_events: EventReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut CameraController), With<PlayerCamera>>,
) {
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
        
        transform.rotation = Quat::from_rotation_y(controller.yaw) * Quat::from_rotation_x(controller.pitch);
    }
}