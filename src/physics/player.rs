use bevy::prelude::*;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::camera::{PlayerCamera, CameraController};
use crate::loading::GameState;
use super::collision::{AABB, check_collision_with_world};

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_EYE_HEIGHT: f32 = 1.62;
const STEP_HEIGHT: f32 = 0.6;
const GROUND_MARGIN: f32 = 0.01; // Small margin above ground to prevent getting stuck

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub aabb: AABB,
    pub spawn_protection_timer: f32,  // Prevents falling through world at spawn
}

impl Default for PlayerPhysics {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: false,
            aabb: AABB::new(Vec3::ZERO, Vec3::new(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH)),
            spawn_protection_timer: 2.0,  // 2 seconds of spawn protection
        }
    }
}

pub struct PlayerPhysicsPlugin;

impl Plugin for PlayerPhysicsPlugin {
    fn build(&self, app: &mut App) {
        info!("PlayerPhysicsPlugin initializing");
        app
            .add_systems(Update, (
                ensure_player_has_physics,
                // simple_test_movement,  // Disabled - using full physics now
                update_player_physics,
                apply_player_movement,
            ).chain().run_if(in_state(GameState::Playing)));
    }
}

fn ensure_player_has_physics(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<PlayerCamera>, Without<PlayerPhysics>)>,
) {
    for (entity, transform) in query.iter() {
        let mut physics = PlayerPhysics::default();
        // Initialize AABB at the correct position
        physics.aabb.center = transform.translation - Vec3::new(0.0, PLAYER_EYE_HEIGHT - PLAYER_HEIGHT / 2.0, 0.0);
        let aabb_center = physics.aabb.center;
        
        commands.entity(entity).insert(physics);
        info!("Added PlayerPhysics to camera at position: {:?} (AABB center: {:?})", 
              transform.translation, aabb_center);
    }
}

fn simple_test_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok(mut transform) = query.get_single_mut() else {
        static mut LOGGED: bool = false;
        unsafe {
            if !LOGGED {
                warn!("No PlayerCamera found for simple movement!");
                LOGGED = true;
            }
        }
        return;
    };
    
    // Use camera-relative movement
    let speed = 15.0;
    let mut velocity = Vec3::ZERO;
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();
    
    // Remove Y component for horizontal movement
    let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
    
    if keyboard.pressed(KeyCode::KeyW) {
        velocity += forward_flat;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        velocity -= forward_flat;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        velocity -= right_flat;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        velocity += right_flat;
    }
    if keyboard.pressed(KeyCode::Space) {
        velocity.y += 1.0;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        velocity.y -= 1.0;
    }
    
    if velocity.length_squared() > 0.0 {
        velocity = velocity.normalize() * speed * time.delta_seconds();
        transform.translation += velocity;
        
        // Log occasionally
        static mut LAST_LOG: f64 = 0.0;
        unsafe {
            let current = time.elapsed_seconds_f64();
            if current - LAST_LOG > 1.0 {
                info!("Simple movement working! Position: {:?}", transform.translation);
                LAST_LOG = current;
            }
        }
    }
}

pub fn update_player_physics(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController, &mut PlayerPhysics), With<PlayerCamera>>,
    chunk_query: Query<(&Chunk, &ChunkPos)>,
    command_prompt: Option<Res<crate::ui::CommandPromptState>>,
) {
    // Don't process input if command prompt is open
    if let Some(prompt) = command_prompt {
        if prompt.is_open {
            return;
        }
    }
    let Ok((mut transform, mut controller, mut physics)) = query.get_single_mut() else {
        // Don't warn every frame
        return;
    };
    
    // Log first frame to debug spawn issues
    static mut FIRST_FRAME: bool = true;
    unsafe {
        if FIRST_FRAME {
            info!("PHYSICS FIRST FRAME: Position: {:?}, chunks available: {}", 
                  transform.translation, chunk_query.iter().count());
            FIRST_FRAME = false;
        }
    }
    
    // Update AABB position (centered at player feet + half height)
    physics.aabb.center = transform.translation - Vec3::new(0.0, PLAYER_EYE_HEIGHT - PLAYER_HEIGHT / 2.0, 0.0);
    
    // Check if grounded (check just below the player's feet)
    // Only update grounded status if we're moving down or stationary
    if physics.velocity.y <= 0.1 {
        let ground_check_pos = physics.aabb.center - Vec3::new(0.0, PLAYER_HEIGHT / 2.0 + 0.1, 0.0);
        physics.is_grounded = check_ground(ground_check_pos, &physics.aabb, &chunk_query);
    } else {
        // Moving up (jumping) - not grounded
        physics.is_grounded = false;
    }
    
    // Removed periodic debug logging
    
    // F key toggles fly mode (easier than double-tap)
    if keyboard.just_pressed(KeyCode::KeyF) {
        controller.fly_mode = !controller.fly_mode;
        if controller.fly_mode {
            physics.velocity.y = 0.0;
            info!("🚁 FLY MODE ENABLED (F key) - Use Space/Shift to go up/down");
        } else {
            info!("🚶 GRAVITY MODE ENABLED (F key) - Press Space to jump");
        }
    }
    
    // Handle horizontal movement input
    let mut input_velocity = Vec3::ZERO;
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();
    
    // Remove Y component for horizontal movement in gravity mode
    let (forward_move, right_move) = if controller.fly_mode {
        (forward, right)
    } else {
        (Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero(),
         Vec3::new(right.x, 0.0, right.z).normalize_or_zero())
    };
    
    if keyboard.pressed(KeyCode::KeyW) {
        input_velocity += forward_move;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input_velocity -= forward_move;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input_velocity -= right_move;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input_velocity += right_move;
    }
    
    // Normalize and apply movement speed
    if input_velocity.length_squared() > 0.0 {
        input_velocity = input_velocity.normalize() * controller.move_speed;
    }
    
    // Handle vertical movement based on mode
    if controller.fly_mode {
        // Fly mode - direct control
        physics.velocity = input_velocity;
        if keyboard.pressed(KeyCode::Space) {
            physics.velocity.y = controller.move_speed;
        } else if keyboard.pressed(KeyCode::ShiftLeft) {
            physics.velocity.y = -controller.move_speed;
        } else {
            physics.velocity.y = 0.0;
        }
    } else {
        // Gravity mode
        physics.velocity.x = input_velocity.x;
        physics.velocity.z = input_velocity.z;
        
        // Apply gravity
        if !physics.is_grounded {
            physics.velocity.y += crate::camera::GRAVITY * time.delta_seconds();
            physics.velocity.y = physics.velocity.y.max(-50.0); // Terminal velocity
        }
        
        // Handle jumping and fly mode toggle
        if keyboard.just_pressed(KeyCode::Space) {
            let current_time = time.elapsed_seconds_f64();
            let time_since_last = current_time - controller.last_space_press;
            
            if time_since_last < crate::camera::DOUBLE_TAP_TIME {
                // Double tap detected - toggle fly mode
                controller.fly_mode = !controller.fly_mode;
                if controller.fly_mode {
                    physics.velocity.y = 0.0;
                    info!("🚁 FLY MODE ENABLED - Use Space/Shift to go up/down");
                } else {
                    info!("🚶 GRAVITY MODE ENABLED - Press Space to jump");
                }
            } else if physics.is_grounded && !controller.fly_mode {
                // Single tap while grounded in gravity mode - jump
                physics.velocity.y = crate::camera::JUMP_VELOCITY;
                info!("Jump! (Double-tap Space quickly to toggle fly mode)");
            }
            
            controller.last_space_press = current_time;
        }
        
        // Reset vertical velocity if grounded and not jumping
        if physics.is_grounded && physics.velocity.y < 0.0 {
            physics.velocity.y = 0.0;
        }
    }
}

fn apply_player_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut PlayerPhysics, &CameraController), With<PlayerCamera>>,
    chunk_query: Query<(&Chunk, &ChunkPos)>,
) {
    let Ok((mut transform, mut physics, controller)) = query.get_single_mut() else {
        // Don't warn every frame
        return;
    };
    
    let delta_time = time.delta_seconds();
    let desired_movement = physics.velocity * delta_time;
    
    // Debug logging removed for performance
    
    // Skip collision in fly mode
    if controller.fly_mode {
        transform.translation += desired_movement;
        transform.translation.y = transform.translation.y.min(crate::planet::config::MAX_ALTITUDE);
        return;
    }
    
    // Apply movement with collision detection (separate axes for sliding)
    let mut new_position = transform.translation - Vec3::new(0.0, PLAYER_EYE_HEIGHT - PLAYER_HEIGHT / 2.0, 0.0);
    
    // Try X movement
    let x_movement = Vec3::new(desired_movement.x, 0.0, 0.0);
    let test_aabb = AABB::new(new_position + x_movement, physics.aabb.half_extents);
    if !check_collision_with_world(&test_aabb, &chunk_query) {
        new_position.x += desired_movement.x;
    }
    
    // Try Z movement
    let z_movement = Vec3::new(0.0, 0.0, desired_movement.z);
    let test_aabb = AABB::new(new_position + z_movement, physics.aabb.half_extents);
    if !check_collision_with_world(&test_aabb, &chunk_query) {
        new_position.z += desired_movement.z;
    }
    
    // Try Y movement
    if desired_movement.y <= 0.0 {
        // Moving down or stationary
        // Only apply downward movement if we're not already on the ground
        if !physics.is_grounded || desired_movement.y < -0.1 {
            let y_movement = Vec3::new(0.0, desired_movement.y, 0.0);
            let test_aabb = AABB::new(new_position + y_movement, physics.aabb.half_extents);
            if !check_collision_with_world(&test_aabb, &chunk_query) {
                new_position.y += desired_movement.y;
            } else {
                // Hit ground - find exact ground position and snap to it
                // Binary search for the exact ground position where AABB bottom touches ground
                let mut low = 0.0;
                let mut high = -desired_movement.y.min(-0.01);  // Ensure we have something to search
                
                // Find the exact position where we just touch the ground
                for _ in 0..15 {  // 15 iterations for better precision
                    let mid = (low + high) / 2.0;
                    let test_pos = new_position - Vec3::new(0.0, mid, 0.0);
                    let test_aabb = AABB::new(test_pos, physics.aabb.half_extents);
                    if check_collision_with_world(&test_aabb, &chunk_query) {
                        high = mid;
                    } else {
                        low = mid;
                    }
                }
                
                // Position just above collision (with tiny margin to prevent getting stuck)
                new_position.y -= low - 0.001;
                physics.velocity.y = 0.0;
                physics.is_grounded = true;
            }
        }
    } else {
        // Moving up (jumping)
        let y_movement = Vec3::new(0.0, desired_movement.y, 0.0);
        let test_aabb = AABB::new(new_position + y_movement, physics.aabb.half_extents);
        if !check_collision_with_world(&test_aabb, &chunk_query) {
            new_position.y += desired_movement.y;
        } else {
            // Hit ceiling
            physics.velocity.y = 0.0;
        }
    }
    
    // Auto step-up for small obstacles
    if physics.is_grounded && (desired_movement.x != 0.0 || desired_movement.z != 0.0) {
        let horizontal_blocked = {
            let test_aabb = AABB::new(
                new_position + Vec3::new(desired_movement.x, 0.0, desired_movement.z),
                physics.aabb.half_extents
            );
            check_collision_with_world(&test_aabb, &chunk_query)
        };
        
        if horizontal_blocked {
            // Try stepping up
            for step_height in [0.1, 0.25, 0.5, STEP_HEIGHT] {
                let step_pos = new_position + Vec3::new(desired_movement.x, step_height, desired_movement.z);
                let test_aabb = AABB::new(step_pos, physics.aabb.half_extents);
                if !check_collision_with_world(&test_aabb, &chunk_query) {
                    // Check if there's ground below at this new position
                    let ground_check = AABB::new(
                        step_pos - Vec3::new(0.0, 0.01, 0.0),
                        physics.aabb.half_extents
                    );
                    if check_collision_with_world(&ground_check, &chunk_query) {
                        // We're on solid ground at this height
                        new_position = step_pos;
                    } else {
                        // Try to find ground below
                        let mut found_ground = false;
                        for down_check in 1..=5 {
                            let check_y = step_height - (down_check as f32 * 0.1);
                            if check_y < 0.0 { break; }
                            let check_pos = new_position + Vec3::new(desired_movement.x, check_y, desired_movement.z);
                            let test_aabb = AABB::new(check_pos, physics.aabb.half_extents);
                            let ground_test = AABB::new(
                                check_pos - Vec3::new(0.0, 0.01, 0.0),
                                physics.aabb.half_extents
                            );
                            if !check_collision_with_world(&test_aabb, &chunk_query) && 
                               check_collision_with_world(&ground_test, &chunk_query) {
                                new_position = check_pos;
                                found_ground = true;
                                break;
                            }
                        }
                        if found_ground {
                            break;
                        }
                    }
                    break;
                }
            }
        }
    }
    
    // Update transform (add eye height offset back)
    transform.translation = new_position + Vec3::new(0.0, PLAYER_EYE_HEIGHT - PLAYER_HEIGHT / 2.0, 0.0);
    transform.translation.y = transform.translation.y.min(crate::planet::config::MAX_ALTITUDE);
    
    // Update physics AABB
    physics.aabb.center = new_position;
}

fn check_ground(position: Vec3, aabb: &AABB, chunk_query: &Query<(&Chunk, &ChunkPos)>) -> bool {
    // Check multiple points under the player for better ground detection
    let check_points = [
        position,
        position + Vec3::new(aabb.half_extents.x * 0.9, 0.0, 0.0),
        position - Vec3::new(aabb.half_extents.x * 0.9, 0.0, 0.0),
        position + Vec3::new(0.0, 0.0, aabb.half_extents.z * 0.9),
        position - Vec3::new(0.0, 0.0, aabb.half_extents.z * 0.9),
    ];
    
    for point in &check_points {
        if is_solid_block_at(*point, chunk_query) {
            return true;
        }
    }
    
    false
}

fn is_solid_block_at(position: Vec3, chunk_query: &Query<(&Chunk, &ChunkPos)>) -> bool {
    let block_pos = IVec3::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
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
            
            let block = chunk.get_block(local_x, local_y, local_z);
            return block.is_solid();
        }
    }
    
    false
}