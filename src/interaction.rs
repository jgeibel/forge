use bevy::prelude::*;
use crate::block::BlockType;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::camera::PlayerCamera;
use crate::inventory::Hotbar;

const REACH_DISTANCE: f32 = 8.0;

#[derive(Resource)]
pub struct SelectedBlock {
    pub position: Option<IVec3>,
    pub normal: Option<Vec3>,
}

impl Default for SelectedBlock {
    fn default() -> Self {
        Self {
            position: None,
            normal: None,
        }
    }
}

pub fn block_interaction_system(
    mut selected_block: ResMut<SelectedBlock>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut chunk_query: Query<(&mut Chunk, &ChunkPos)>,
    hotbar: Res<Hotbar>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    
    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward().as_vec3();
    
    if let Some((hit_pos, hit_normal)) = raycast_voxel(
        ray_origin,
        ray_direction,
        REACH_DISTANCE,
        &chunk_query,
    ) {
        selected_block.position = Some(hit_pos);
        selected_block.normal = Some(hit_normal);
        
        if mouse_button.just_pressed(MouseButton::Left) {
            remove_block(hit_pos, &mut chunk_query);
        }
        
        if mouse_button.just_pressed(MouseButton::Right) {
            if let Some(block_type) = hotbar.get_selected_block() {
                // Don't place if it's Air (used for erasing)
                if block_type != BlockType::Air {
                    let place_pos = hit_pos + hit_normal.as_ivec3();
                    place_block(place_pos, block_type, &mut chunk_query);
                }
            }
        }
    } else {
        selected_block.position = None;
        selected_block.normal = None;
    }
}

pub fn draw_selection_box(
    mut gizmos: Gizmos,
    selected_block: Res<SelectedBlock>,
    time: Res<Time>,
) {
    if let (Some(position), Some(normal)) = (selected_block.position, selected_block.normal) {
        let block_pos = position.as_vec3();
        
        // Draw wireframe cube around selected block
        let size = 1.002; // Slightly larger to avoid z-fighting
        let half_size = size / 2.0;
        let center = block_pos + Vec3::splat(0.5);
        
        // Pulsing effect for better visibility
        let pulse = (time.elapsed_seconds() * 3.0).sin() * 0.1 + 0.9;
        let color = Color::srgba(1.0, 1.0, 1.0, pulse);
        
        // Draw cube edges
        gizmos.cuboid(
            Transform::from_translation(center)
                .with_scale(Vec3::splat(size)),
            color,
        );
        
        // Highlight the targeted face
        let (right, up) = if normal.abs().y > 0.5 {
            // Top or bottom face
            (Vec3::X, Vec3::Z)
        } else if normal.abs().x > 0.5 {
            // Left or right face
            (Vec3::Z, Vec3::Y)
        } else {
            // Front or back face
            (Vec3::X, Vec3::Y)
        };
        
        // Draw highlighted face
        let face_center = center + normal * (half_size + 0.001);
        let face_size = 0.502;
        let corners = [
            face_center - right * face_size - up * face_size,
            face_center + right * face_size - up * face_size,
            face_center + right * face_size + up * face_size,
            face_center - right * face_size + up * face_size,
        ];
        
        // Face outline with stronger color
        gizmos.linestrip(
            [corners[0], corners[1], corners[2], corners[3], corners[0]],
            Color::srgba(1.0, 1.0, 0.0, pulse), // Yellow highlight
        );
        
        // Cross pattern on face for better visibility
        gizmos.line(corners[0], corners[2], Color::srgba(1.0, 1.0, 0.0, pulse * 0.5));
        gizmos.line(corners[1], corners[3], Color::srgba(1.0, 1.0, 0.0, pulse * 0.5));
    }
}

fn raycast_voxel(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_query: &Query<(&mut Chunk, &ChunkPos)>,
) -> Option<(IVec3, Vec3)> {
    // Use DDA algorithm for efficient voxel traversal
    let direction = direction.normalize();
    
    // Starting voxel position
    let mut current_voxel = world_to_block_pos(origin);
    
    // Calculate step direction for each axis
    let step_x = if direction.x > 0.0 { 1 } else { -1 };
    let step_y = if direction.y > 0.0 { 1 } else { -1 };
    let step_z = if direction.z > 0.0 { 1 } else { -1 };
    
    // Calculate the distance to the next voxel boundary for each axis
    let next_voxel_boundary_x = if direction.x > 0.0 {
        (current_voxel.x + 1) as f32
    } else {
        current_voxel.x as f32
    };
    
    let next_voxel_boundary_y = if direction.y > 0.0 {
        (current_voxel.y + 1) as f32
    } else {
        current_voxel.y as f32
    };
    
    let next_voxel_boundary_z = if direction.z > 0.0 {
        (current_voxel.z + 1) as f32
    } else {
        current_voxel.z as f32
    };
    
    // Calculate t values for each axis (distance along ray to reach next voxel boundary)
    let mut t_max_x = if direction.x.abs() > 0.0001 {
        (next_voxel_boundary_x - origin.x) / direction.x
    } else {
        f32::MAX
    };
    
    let mut t_max_y = if direction.y.abs() > 0.0001 {
        (next_voxel_boundary_y - origin.y) / direction.y
    } else {
        f32::MAX
    };
    
    let mut t_max_z = if direction.z.abs() > 0.0001 {
        (next_voxel_boundary_z - origin.z) / direction.z
    } else {
        f32::MAX
    };
    
    // Calculate how much to increase t for each step in each direction
    let t_delta_x = if direction.x.abs() > 0.0001 {
        1.0 / direction.x.abs()
    } else {
        f32::MAX
    };
    
    let t_delta_y = if direction.y.abs() > 0.0001 {
        1.0 / direction.y.abs()
    } else {
        f32::MAX
    };
    
    let t_delta_z = if direction.z.abs() > 0.0001 {
        1.0 / direction.z.abs()
    } else {
        f32::MAX
    };
    
    let mut distance_traveled = 0.0;
    let mut previous_voxel = current_voxel;
    
    // Maximum iterations to prevent infinite loops
    let max_steps = (max_distance * 3.0) as i32;
    let mut steps = 0;
    
    while distance_traveled < max_distance && steps < max_steps {
        // Check current voxel for solid block
        if let Some(block) = get_block_at_world_pos(current_voxel, chunk_query) {
            if block.is_solid() {
                let normal = calculate_hit_normal(previous_voxel, current_voxel);
                return Some((current_voxel, normal));
            }
        }
        
        previous_voxel = current_voxel;
        
        // Step to next voxel boundary
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                // Step in X direction
                current_voxel.x += step_x;
                distance_traveled = t_max_x;
                t_max_x += t_delta_x;
            } else {
                // Step in Z direction
                current_voxel.z += step_z;
                distance_traveled = t_max_z;
                t_max_z += t_delta_z;
            }
        } else {
            if t_max_y < t_max_z {
                // Step in Y direction
                current_voxel.y += step_y;
                distance_traveled = t_max_y;
                t_max_y += t_delta_y;
            } else {
                // Step in Z direction
                current_voxel.z += step_z;
                distance_traveled = t_max_z;
                t_max_z += t_delta_z;
            }
        }
        
        steps += 1;
    }
    
    None
}

fn calculate_hit_normal(from_block: IVec3, to_block: IVec3) -> Vec3 {
    let diff = to_block - from_block;
    
    if diff.x != 0 {
        Vec3::new(-diff.x as f32, 0.0, 0.0)
    } else if diff.y != 0 {
        Vec3::new(0.0, -diff.y as f32, 0.0)
    } else if diff.z != 0 {
        Vec3::new(0.0, 0.0, -diff.z as f32)
    } else {
        Vec3::Y
    }
}

fn world_to_block_pos(world_pos: Vec3) -> IVec3 {
    IVec3::new(
        world_pos.x.floor() as i32,
        world_pos.y.floor() as i32,
        world_pos.z.floor() as i32,
    )
}

fn get_block_at_world_pos(
    world_pos: IVec3,
    chunk_query: &Query<(&mut Chunk, &ChunkPos)>,
) -> Option<BlockType> {
    let chunk_pos = ChunkPos::new(
        (world_pos.x as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.y as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.z as f32 / CHUNK_SIZE_F32).floor() as i32,
    );
    
    for (chunk, pos) in chunk_query.iter() {
        if *pos == chunk_pos {
            let local_x = world_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = world_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = world_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
            
            return Some(chunk.get_block(local_x, local_y, local_z));
        }
    }
    
    None
}

fn remove_block(
    world_pos: IVec3,
    chunk_query: &mut Query<(&mut Chunk, &ChunkPos)>,
) {
    let chunk_pos = ChunkPos::new(
        (world_pos.x as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.y as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.z as f32 / CHUNK_SIZE_F32).floor() as i32,
    );
    
    for (mut chunk, pos) in chunk_query.iter_mut() {
        if *pos == chunk_pos {
            let local_x = world_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = world_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = world_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
            
            // Check if block is breakable (not bedrock)
            let block = chunk.get_block(local_x, local_y, local_z);
            if block.is_breakable() {
                chunk.set_block(local_x, local_y, local_z, BlockType::Air);
                chunk.dirty = true; // Mark chunk for mesh regeneration
            }
            return;
        }
    }
}

fn place_block(
    world_pos: IVec3,
    block_type: BlockType,
    chunk_query: &mut Query<(&mut Chunk, &ChunkPos)>,
) {
    let chunk_pos = ChunkPos::new(
        (world_pos.x as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.y as f32 / CHUNK_SIZE_F32).floor() as i32,
        (world_pos.z as f32 / CHUNK_SIZE_F32).floor() as i32,
    );
    
    for (mut chunk, pos) in chunk_query.iter_mut() {
        if *pos == chunk_pos {
            let local_x = world_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = world_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = world_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;
            
            // Only place if current block is air
            if chunk.get_block(local_x, local_y, local_z) == BlockType::Air {
                chunk.set_block(local_x, local_y, local_z, block_type);
                chunk.dirty = true; // Mark chunk for mesh regeneration
            }
            return;
        }
    }
}