use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::{FogSettings, FogFalloff};
use bevy::render::camera::PerspectiveProjection;
use crate::loading::GameState;
use crate::planet::config::PlanetConfig;
use crate::world::WorldGenerator;

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
        app
            .add_systems(OnEnter(GameState::Playing), setup_camera)
            .add_systems(Update, 
                camera_look.run_if(in_state(GameState::Playing))
            );
    }
}

/// Find a guaranteed land spawn position with temperate climate
pub fn find_guaranteed_land_spawn(world_gen: &WorldGenerator, planet_size: f32) -> Vec3 {
    println!("=== SPAWN FINDER: Starting search for land spawn, planet size: {}", planet_size);
    // Sample the world in a grid pattern to find land
    // Start from equator and work outward for better climate chances
    let equator = planet_size / 2.0;
    let sample_spacing = 512.0; // Sample every 512 blocks (16 chunks)
    let max_samples = (planet_size / sample_spacing) as i32;
    println!("=== SPAWN FINDER: Will sample {} x {} grid", max_samples, max_samples);
    
    // Try different distances from equator
    for distance_multiplier in 0..max_samples/2 {
        let z_offset = distance_multiplier as f32 * sample_spacing;
        
        // Try both north and south of equator
        for z_direction in &[1.0, -1.0] {
            let z = equator + (z_offset * z_direction);
            
            // Sample along X axis at this latitude
            for x_sample in 0..max_samples {
                let x = x_sample as f32 * sample_spacing;
                
                // Check if this is land using the terrain generator directly
                // We need to access the continental generator through the terrain generator
                let wrapped_x = x.rem_euclid(planet_size);
                let wrapped_z = z.rem_euclid(planet_size);
                
                // Get the height to determine if it's above sea level
                let height = world_gen.get_height(wrapped_x, wrapped_z);
                
                // Check if it's land (above sea level)
                if height > 64.0 {
                    // Check temperature (we want temperate climate, not too hot or cold)
                    // Using the climate system directly through world generator
                    let temp = world_gen.get_air_temperature(wrapped_x, 80.0, wrapped_z);
                    
                    // Convert to 0-1 range for checking (32-86°F is comfortable)
                    if temp > 32.0 && temp < 86.0 {
                        // Found a good spot!
                        // Find the actual TOP surface by scanning from sky downward
                        info!("Searching for surface at ({}, {}), terrain height: {}", wrapped_x, wrapped_z, height);
                        
                        // Start from sky level
                        let mut surface_y = 200.0;
                        let mut consecutive_air = 0;
                        
                        while surface_y > 0.0 {
                            let block = world_gen.get_block(wrapped_x, surface_y, wrapped_z);
                            
                            // Debug log every 10 blocks
                            if surface_y as i32 % 20 == 0 {
                                info!("  Y={}: block={:?}, consecutive_air={}", surface_y, block, consecutive_air);
                            }
                            
                            if block == crate::block::BlockType::Air {
                                consecutive_air += 1;
                            } else {
                                // We found a solid block after having at least 5 blocks of air above
                                // This ensures we're at the actual surface, not in a cave
                                if consecutive_air >= 5 {
                                    // Check that we're above sea level to avoid underwater spawns
                                    if surface_y > 64.0 {
                                        info!("Found TOP surface at Y={} (had {} air blocks above), block is {:?}", 
                                              surface_y, consecutive_air, block);
                                        // Spawn on top of this block (+1) plus safety margin (+2)
                                        return Vec3::new(wrapped_x, surface_y + 3.0, wrapped_z);
                                    }
                                }
                                consecutive_air = 0;
                            }
                            surface_y -= 1.0;
                        }
                        // If we couldn't find surface properly, use height + safety margin
                        warn!("Could not find proper surface, using height estimate + 5");
                        return Vec3::new(wrapped_x, height + 5.0, wrapped_z);
                    }
                }
            }
        }
    }
    
    // Fallback: just find ANY land, regardless of temperature
    warn!("No temperate land found, searching for any land...");
    for x_sample in 0..max_samples {
        for z_sample in 0..max_samples {
            let x = x_sample as f32 * sample_spacing;
            let z = z_sample as f32 * sample_spacing;
            
            let height = world_gen.get_height(x, z);
            if height > 64.0 {
                info!("Fallback: Searching for surface at ({}, {}), terrain height: {}", x, z, height);
                
                // Find actual TOP surface by scanning from sky
                let mut surface_y = 200.0;
                let mut consecutive_air = 0;
                
                while surface_y > 0.0 {
                    let block = world_gen.get_block(x, surface_y, z);
                    
                    if block == crate::block::BlockType::Air {
                        consecutive_air += 1;
                    } else {
                        // Found solid after air - this is the surface
                        if consecutive_air >= 5 && surface_y > 64.0 {
                            info!("Fallback: Found TOP surface at Y={} with {} air blocks above", surface_y, consecutive_air);
                            return Vec3::new(x, surface_y + 3.0, z);
                        }
                        consecutive_air = 0;
                    }
                    surface_y -= 1.0;
                }
                warn!("Fallback: Could not find proper surface, using height estimate + 5");
                return Vec3::new(x, height + 5.0, z);
            }
        }
    }
    
    // Ultimate fallback: spawn at center anyway
    warn!("No land found! Using default spawn position");
    Vec3::new(equator, 80.0, equator)
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
            transform: Transform::from_xyz(spawn_x, spawn_y, spawn_z)
                .looking_at(Vec3::new(spawn_x + 10.0, spawn_y - 5.0, spawn_z + 10.0), Vec3::Y),
            projection: PerspectiveProjection {
                near: 0.1,  // Standard near plane
                far: 400.0, // Reduced far plane for better precision
                fov: 70.0_f32.to_radians(), // Minecraft uses 70 degrees FOV
                ..default()
            }.into(),
            ..default()
        },
        PlayerCamera,
        CameraController::default(),
        FogSettings {
            color: Color::srgba(0.7, 0.8, 0.9, 1.0),
            falloff: FogFalloff::Linear {
                start: 80.0,   // Start fog earlier (2.5 chunks)
                end: 256.0,    // End at 8 chunks (8 * 32 = 256 blocks)
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