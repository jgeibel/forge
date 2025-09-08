use bevy::prelude::*;
use crate::block::BlockType;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::camera::PlayerCamera;

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
            let place_pos = hit_pos + hit_normal.as_ivec3();
            place_block(place_pos, BlockType::Stone, &mut chunk_query);
        }
    } else {
        selected_block.position = None;
        selected_block.normal = None;
    }
}

pub fn draw_selection_box(
    mut gizmos: Gizmos,
    selected_block: Res<SelectedBlock>,
) {
    if let (Some(position), Some(normal)) = (selected_block.position, selected_block.normal) {
        let block_center = position.as_vec3() + Vec3::splat(0.5);
        
        // Calculate face quad vertices based on normal
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
        
        // Offset the face slightly outside the block to avoid z-fighting
        let face_center = block_center + normal * 0.502;
        
        // Draw the face as a filled rectangle
        let half_size = 0.501;
        let corners = [
            face_center - right * half_size - up * half_size,
            face_center + right * half_size - up * half_size,
            face_center + right * half_size + up * half_size,
            face_center - right * half_size + up * half_size,
        ];
        
        // Draw face outline
        gizmos.linestrip(
            [corners[0], corners[1], corners[2], corners[3], corners[0]],
            Color::srgba(1.0, 1.0, 1.0, 0.8),
        );
        
        // Draw semi-transparent face fill using two triangles
        let face_color = Color::srgba(1.0, 1.0, 1.0, 0.15);
        
        // Triangle 1
        gizmos.line(corners[0], corners[2], face_color);
        gizmos.line(corners[1], corners[3], face_color);
        
        // Add grid lines for better visibility
        for i in 1..4 {
            let t = i as f32 / 4.0;
            gizmos.line(
                corners[0].lerp(corners[1], t),
                corners[3].lerp(corners[2], t),
                Color::srgba(1.0, 1.0, 1.0, 0.1),
            );
            gizmos.line(
                corners[0].lerp(corners[3], t),
                corners[1].lerp(corners[2], t),
                Color::srgba(1.0, 1.0, 1.0, 0.1),
            );
        }
    }
}

fn raycast_voxel(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_query: &Query<(&mut Chunk, &ChunkPos)>,
) -> Option<(IVec3, Vec3)> {
    let direction = direction.normalize();
    let mut current = origin;
    let step = 0.01;
    let mut previous_block_pos = world_to_block_pos(origin);
    
    while current.distance(origin) < max_distance {
        let block_pos = world_to_block_pos(current);
        
        if let Some(block) = get_block_at_world_pos(block_pos, chunk_query) {
            if block.is_solid() {
                let normal = calculate_hit_normal(previous_block_pos, block_pos);
                return Some((block_pos, normal));
            }
        }
        
        if block_pos != previous_block_pos {
            previous_block_pos = block_pos;
        }
        current += direction * step;
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
            
            chunk.set_block(local_x, local_y, local_z, block_type);
            return;
        }
    }
}