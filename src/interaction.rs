use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::inventory::Hotbar;
use crate::items;
use crate::tools::Tool;
use bevy::prelude::*;

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

#[derive(Resource, Default)]
pub struct BlockExtractionState {
    pub extracting_pos: Option<IVec3>,
    pub progress: f32,
    pub total_time: f32,
    pub current_tool: Tool,
    pub last_particle_spawn: f32,
    pub extracting_block_type: Option<BlockType>,
}

#[derive(Component)]
pub struct ExtractingBlock {
    pub original_transform: Transform,
    pub wobble_seed: f32,
}

pub fn block_interaction_system(
    mut selected_block: ResMut<SelectedBlock>,
    mut extraction_state: ResMut<BlockExtractionState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut chunk_query: Query<(&mut Chunk, &ChunkPos)>,
    mut hotbar: ResMut<Hotbar>,
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    texture_atlas: Option<Res<crate::texture::BlockTextureAtlas>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward().as_vec3();

    if let Some((hit_pos, hit_normal)) =
        raycast_voxel(ray_origin, ray_direction, REACH_DISTANCE, &chunk_query)
    {
        selected_block.position = Some(hit_pos);
        selected_block.normal = Some(hit_normal);

        // Handle extraction (hold left mouse)
        if mouse_button.pressed(MouseButton::Left) {
            // Get the block type
            if let Some(block_type) = get_block_at_world_pos(hit_pos, &chunk_query) {
                if block_type.is_breakable() && block_type != BlockType::Air {
                    // Start or continue extraction
                    if extraction_state.extracting_pos != Some(hit_pos) {
                        // Start new extraction
                        extraction_state.extracting_pos = Some(hit_pos);
                        extraction_state.progress = 0.0;
                        extraction_state.current_tool = Tool::Hand; // TODO: Get actual tool
                        extraction_state.extracting_block_type = Some(block_type);
                        extraction_state.last_particle_spawn = 0.0;

                        let base_time = block_type.extraction_time();
                        let efficiency = extraction_state.current_tool.efficiency_for(block_type);
                        extraction_state.total_time = base_time / efficiency;
                    }

                    // Update progress
                    extraction_state.progress += time.delta_seconds();

                    // Check if extraction complete
                    if extraction_state.progress >= extraction_state.total_time {
                        // Spawn dropped item
                        let world_pos = hit_pos.as_vec3() + Vec3::splat(0.5);
                        items::spawn_dropped_item(
                            &mut commands,
                            block_type,
                            world_pos,
                            &mut meshes,
                            &mut materials,
                            texture_atlas.as_deref(),
                            &time,
                        );

                        // Remove the block
                        remove_block(hit_pos, &mut chunk_query);

                        // Reset extraction state
                        extraction_state.extracting_pos = None;
                        extraction_state.progress = 0.0;
                        extraction_state.extracting_block_type = None;
                        extraction_state.last_particle_spawn = 0.0;
                    }
                }
            }
        } else {
            // Reset extraction if button released
            extraction_state.extracting_pos = None;
            extraction_state.progress = 0.0;
            extraction_state.extracting_block_type = None;
            extraction_state.last_particle_spawn = 0.0;
        }

        if mouse_button.just_pressed(MouseButton::Right) {
            if let Some(block_type) = hotbar.get_selected_block() {
                // Don't place if it's Air (used for erasing)
                if block_type != BlockType::Air {
                    let place_pos = hit_pos + hit_normal.as_ivec3();
                    // Try to place block and use item from inventory
                    if place_block(place_pos, block_type, &mut chunk_query) {
                        hotbar.use_selected_item();
                    }
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
    extraction_state: Res<BlockExtractionState>,
    time: Res<Time>,
) {
    // Don't draw selection box if we're currently extracting a block
    // This makes the extraction animation more visible
    if extraction_state.extracting_pos.is_some() && extraction_state.progress > 0.0 {
        return;
    }

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
            Transform::from_translation(center).with_scale(Vec3::splat(size)),
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
        gizmos.line(
            corners[0],
            corners[2],
            Color::srgba(1.0, 1.0, 0.0, pulse * 0.5),
        );
        gizmos.line(
            corners[1],
            corners[3],
            Color::srgba(1.0, 1.0, 0.0, pulse * 0.5),
        );
    }
}

/// Visual feedback system for block extraction - shows edge separation lines and wobble
pub fn update_extraction_visual(
    mut commands: Commands,
    mut gizmos: Gizmos,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut extraction_state: ResMut<BlockExtractionState>,
    chunk_query: Query<(&Chunk, &ChunkPos)>,
    time: Res<Time>,
) {
    if let Some(pos) = extraction_state.extracting_pos {
        if extraction_state.progress > 0.0 {
            let progress = (extraction_state.progress / extraction_state.total_time).min(1.0);
            let block_center = pos.as_vec3() + Vec3::splat(0.5);

            // Check which faces have neighboring blocks
            let has_neighbor = [
                get_block_at_world_pos_readonly(pos + IVec3::X, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // +X
                get_block_at_world_pos_readonly(pos - IVec3::X, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // -X
                get_block_at_world_pos_readonly(pos + IVec3::Y, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // +Y
                get_block_at_world_pos_readonly(pos - IVec3::Y, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // -Y
                get_block_at_world_pos_readonly(pos + IVec3::Z, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // +Z
                get_block_at_world_pos_readonly(pos - IVec3::Z, &chunk_query)
                    .map_or(false, |b| b.is_solid()), // -Z
            ];

            // Enhanced color progression with pulsing effect
            let pulse = (time.elapsed_seconds() * 8.0).sin() * 0.5 + 0.5;
            let base_color = if progress < 0.33 {
                // White -> Yellow
                Color::srgba(1.0, 1.0, 1.0 - progress * 3.0, 1.0)
            } else if progress < 0.66 {
                // Yellow -> Orange
                Color::srgba(1.0, 1.0 - (progress - 0.33) * 1.5, 0.0, 1.0)
            } else {
                // Orange -> Green (almost complete)
                Color::srgba(
                    1.0 - (progress - 0.66) * 3.0,
                    1.0,
                    (progress - 0.66) * 3.0,
                    1.0,
                )
            };

            // Make the cracks much more visible
            let crack_thickness = 0.52 + progress * 0.03; // Thicker base and more growth
            let crack_length = progress; // Cracks grow along edges

            // Draw progressive cracks on each connected face
            // X faces
            if has_neighbor[0] {
                // +X face
                let face_center = block_center + Vec3::X * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::Y,
                    Vec3::Z,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }
            if has_neighbor[1] {
                // -X face
                let face_center = block_center - Vec3::X * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::Y,
                    Vec3::Z,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }

            // Y faces
            if has_neighbor[2] {
                // +Y face
                let face_center = block_center + Vec3::Y * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::X,
                    Vec3::Z,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }
            if has_neighbor[3] {
                // -Y face
                let face_center = block_center - Vec3::Y * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::X,
                    Vec3::Z,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }

            // Z faces
            if has_neighbor[4] {
                // +Z face
                let face_center = block_center + Vec3::Z * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::X,
                    Vec3::Y,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }
            if has_neighbor[5] {
                // -Z face
                let face_center = block_center - Vec3::Z * 0.5;
                draw_progressive_cracks(
                    &mut gizmos,
                    face_center,
                    Vec3::X,
                    Vec3::Y,
                    crack_thickness,
                    crack_length,
                    base_color,
                    pulse,
                );
            }

            // Spawn particles periodically during extraction
            let particle_interval = 0.1; // Spawn particles every 0.1 seconds
            if extraction_state.progress - extraction_state.last_particle_spawn > particle_interval
            {
                if let Some(block_type) = extraction_state.extracting_block_type {
                    crate::particles::spawn_extraction_particles(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        block_center,
                        block_type,
                        progress,
                    );
                    extraction_state.last_particle_spawn = extraction_state.progress;
                }
            }

            // Add subtle extraction completion flash only
            if progress > 0.95 {
                let flash_intensity = (progress - 0.95) * 20.0;
                // Draw a subtle expanding cube to indicate completion
                gizmos.cuboid(
                    Transform::from_translation(block_center)
                        .with_scale(Vec3::splat(1.0 + flash_intensity * 0.05)),
                    Color::srgba(1.0, 1.0, 1.0, flash_intensity * 0.2),
                );
            }
        }
    }
}

fn draw_progressive_cracks(
    gizmos: &mut Gizmos,
    center: Vec3,
    right: Vec3,
    up: Vec3,
    size: f32,
    progress: f32,
    color: Color,
    pulse: f32,
) {
    let corners = [
        center - right * 0.5 - up * 0.5, // Bottom-left
        center + right * 0.5 - up * 0.5, // Bottom-right
        center + right * 0.5 + up * 0.5, // Top-right
        center - right * 0.5 + up * 0.5, // Top-left
    ];

    // Draw cracks that grow along each edge based on progress
    let crack_segments = 8; // Number of segments per edge for jagged effect

    // Bottom edge (0 -> 1)
    draw_crack_segment(
        gizmos,
        corners[0],
        corners[1],
        progress,
        crack_segments,
        color,
        pulse,
        size,
    );

    // Right edge (1 -> 2)
    draw_crack_segment(
        gizmos,
        corners[1],
        corners[2],
        progress,
        crack_segments,
        color,
        pulse,
        size,
    );

    // Top edge (2 -> 3)
    draw_crack_segment(
        gizmos,
        corners[2],
        corners[3],
        progress,
        crack_segments,
        color,
        pulse,
        size,
    );

    // Left edge (3 -> 0)
    draw_crack_segment(
        gizmos,
        corners[3],
        corners[0],
        progress,
        crack_segments,
        color,
        pulse,
        size,
    );

    // Draw subtle diagonal cracks from corners when progress > 0.7
    if progress > 0.7 {
        let diagonal_progress = (progress - 0.7) * 3.33; // Scale to 0-1 range
        let diagonal_color = color.with_alpha(0.5 * diagonal_progress);

        // Draw simple diagonal lines from corners, not jagged
        for corner in &corners {
            let dir_to_center = (center - *corner).normalize();
            let crack_end = *corner + dir_to_center * diagonal_progress * 0.2;
            gizmos.line(*corner, crack_end, diagonal_color);
        }
    }
}

fn draw_crack_segment(
    gizmos: &mut Gizmos,
    start: Vec3,
    end: Vec3,
    progress: f32,
    segments: usize,
    color: Color,
    pulse: f32,
    thickness: f32,
) {
    let actual_end = start + (end - start) * progress.min(1.0);
    draw_jagged_line(gizmos, start, actual_end, segments, color, pulse, thickness);
}

fn draw_jagged_line(
    gizmos: &mut Gizmos,
    start: Vec3,
    end: Vec3,
    segments: usize,
    color: Color,
    pulse: f32,
    thickness: f32,
) {
    let mut points = Vec::new();
    points.push(start);

    let segment_vec = (end - start) / segments as f32;
    let perpendicular = segment_vec.cross(Vec3::Y).normalize();

    for i in 1..segments {
        let _t = i as f32 / segments as f32;
        let base_point = start + segment_vec * i as f32;

        // Add some random-looking offset for jagged appearance
        let offset = (i as f32 * 7.13 + start.x * 3.7 + start.z * 5.3).sin() * 0.02;
        let jitter = perpendicular * offset * (1.0 + pulse * 0.5);

        points.push(base_point + jitter);
    }

    points.push(end);

    // Draw the jagged line with varying thickness
    for i in 0..points.len() - 1 {
        let segment_pulse = ((i as f32 * 0.5 + pulse * 3.0).sin() + 1.0) * 0.5;
        let segment_color = color.with_alpha(0.7 + segment_pulse * 0.3);

        // Draw multiple lines for thickness effect
        let offsets = [0.0, thickness * 0.25, -thickness * 0.25];
        for offset in offsets {
            let offset_vec = perpendicular * offset;
            gizmos.line(
                points[i] + offset_vec,
                points[i + 1] + offset_vec,
                segment_color,
            );
        }
    }
}

fn draw_face_edges(
    gizmos: &mut Gizmos,
    center: Vec3,
    right: Vec3,
    up: Vec3,
    size: f32,
    color: Color,
) {
    let corners = [
        center - right * size - up * size,
        center + right * size - up * size,
        center + right * size + up * size,
        center - right * size + up * size,
    ];

    gizmos.linestrip(
        [corners[0], corners[1], corners[2], corners[3], corners[0]],
        color,
    );
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

fn get_block_at_world_pos_readonly(
    world_pos: IVec3,
    chunk_query: &Query<(&Chunk, &ChunkPos)>,
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

fn remove_block(world_pos: IVec3, chunk_query: &mut Query<(&mut Chunk, &ChunkPos)>) {
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
) -> bool {
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
                return true;
            }
            return false;
        }
    }
    false
}
