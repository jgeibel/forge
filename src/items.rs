use crate::block::BlockType;
use crate::camera::PlayerCamera;
use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use crate::inventory::Hotbar;
use bevy::prelude::*;

const ITEM_SIZE: f32 = 0.25; // Smaller size, about 1/4 of a full block
const COLLECTION_RADIUS: f32 = 2.0;
const ITEM_GRAVITY: f32 = -9.8;
const ITEM_BOUNCE_DAMPING: f32 = 0.6;
const MAX_FALL_DISTANCE: f32 = 100.0; // Safety limit
const ITEM_COLLISION_RADIUS: f32 = 0.5; // Distance at which items push each other
const ITEM_COLLISION_STRENGTH: f32 = 2.0; // How strongly items repel each other

#[derive(Component)]
pub struct DroppedItem {
    pub block_type: BlockType,
    pub velocity: Vec3,
    pub spawn_time: f32, // Track when item was created for pulsing effect
}

impl DroppedItem {
    pub fn new(block_type: BlockType, position: Vec3, current_time: f32) -> (Self, Transform) {
        // Add slight random velocity for natural scatter
        let random_x = (position.x * 12.345).sin() * 2.0;
        let random_z = (position.z * 54.321).cos() * 2.0;

        let item = Self {
            block_type,
            velocity: Vec3::new(random_x, 3.0, random_z),
            spawn_time: current_time,
        };

        let transform = Transform::from_translation(position + Vec3::Y * 0.5)
            .with_scale(Vec3::splat(ITEM_SIZE));

        (item, transform)
    }
}

pub fn spawn_dropped_item(
    commands: &mut Commands,
    block_type: BlockType,
    position: Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    texture_atlas: Option<&crate::texture::BlockTextureAtlas>,
    time: &Time,
) {
    let (item, transform) = DroppedItem::new(block_type, position, time.elapsed_seconds());

    // Create a cube mesh with proper UVs for texturing
    let mesh = create_textured_cube_mesh(block_type, texture_atlas);
    let mesh_handle = meshes.add(mesh);

    // Create material with texture atlas if available, add subtle glow
    let material = if let Some(atlas) = texture_atlas {
        materials.add(StandardMaterial {
            base_color_texture: Some(atlas.texture.clone()),
            base_color: Color::WHITE,
            // Add subtle emissive glow based on block type
            emissive: get_emissive_color(block_type),
            emissive_exposure_weight: 0.5, // Increased for more glow
            perceptual_roughness: 0.9,
            metallic: 0.0,
            reflectance: 0.1,
            double_sided: true,
            cull_mode: None,
            alpha_mode: bevy::prelude::AlphaMode::Opaque,
            ..default()
        })
    } else {
        // Fallback to solid color if no texture atlas
        let base_color = Color::srgba(
            block_type.get_color()[0],
            block_type.get_color()[1],
            block_type.get_color()[2],
            1.0,
        );
        materials.add(StandardMaterial {
            base_color,
            emissive: get_emissive_color(block_type),
            emissive_exposure_weight: 0.5, // Increased for more glow
            ..default()
        })
    };

    commands.spawn((
        PbrBundle {
            mesh: mesh_handle,
            material,
            transform,
            ..default()
        },
        item,
    ));
}

// Helper function to create a textured cube mesh for dropped items
fn create_textured_cube_mesh(
    block_type: BlockType,
    texture_atlas: Option<&crate::texture::BlockTextureAtlas>,
) -> Mesh {
    use crate::texture::{BlockFace, BlockState};
    use bevy::render::mesh::{Indices, PrimitiveTopology};

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Get block type name for texture lookup
    let block_name = format!("{:?}", block_type).to_lowercase();

    // Define the 6 faces of a cube
    let faces = [
        // Top face (Y+)
        (
            [
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [0.0, 1.0, 0.0],
            BlockFace::Top,
        ),
        // Bottom face (Y-)
        (
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            [0.0, -1.0, 0.0],
            BlockFace::Bottom,
        ),
        // Front face (Z+)
        (
            [
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, -0.5, 0.5],
            ],
            [0.0, 0.0, 1.0],
            BlockFace::Front,
        ),
        // Back face (Z-)
        (
            [
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            [0.0, 0.0, -1.0],
            BlockFace::Back,
        ),
        // Right face (X+)
        (
            [
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
            ],
            [1.0, 0.0, 0.0],
            BlockFace::Right,
        ),
        // Left face (X-)
        (
            [
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            [-1.0, 0.0, 0.0],
            BlockFace::Left,
        ),
    ];

    // Build each face
    for (face_positions, normal, face_type) in faces.iter() {
        let base_index = positions.len() as u32;

        // Add vertices for this face
        for pos in face_positions.iter() {
            positions.push([pos[0], pos[1], pos[2]]);
            normals.push([normal[0], normal[1], normal[2]]);
        }

        // Get UVs for this face from the texture atlas
        if let Some(atlas) = texture_atlas {
            let (uv_min, uv_max) = atlas.get_uv(&block_name, *face_type, BlockState::Normal);

            // Add UVs based on face orientation
            match face_type {
                BlockFace::Top | BlockFace::Bottom => {
                    uvs.push([uv_min.x, uv_min.y]);
                    uvs.push([uv_max.x, uv_min.y]);
                    uvs.push([uv_max.x, uv_max.y]);
                    uvs.push([uv_min.x, uv_max.y]);
                }
                _ => {
                    uvs.push([uv_min.x, uv_max.y]);
                    uvs.push([uv_min.x, uv_min.y]);
                    uvs.push([uv_max.x, uv_min.y]);
                    uvs.push([uv_max.x, uv_max.y]);
                }
            }
        } else {
            // Default UVs if no atlas
            uvs.push([0.0, 0.0]);
            uvs.push([1.0, 0.0]);
            uvs.push([1.0, 1.0]);
            uvs.push([0.0, 1.0]);
        }

        // Add indices for two triangles
        indices.push(base_index);
        indices.push(base_index + 1);
        indices.push(base_index + 2);
        indices.push(base_index);
        indices.push(base_index + 2);
        indices.push(base_index + 3);
    }

    // Create the mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

// Helper function to get emissive color for a block type
fn get_emissive_color(block_type: BlockType) -> LinearRgba {
    // Return a more noticeable glow color based on block type
    let base_color = block_type.get_color();
    // Make the glow brighter and slightly tinted
    LinearRgba::from(Color::srgba(
        (base_color[0] * 0.6 + 0.2).min(1.0), // Add base brightness
        (base_color[1] * 0.6 + 0.2).min(1.0),
        (base_color[2] * 0.6 + 0.2).min(1.0),
        1.0,
    ))
}

// Helper function to check if there's a solid block at a world position
fn is_solid_block_at(pos: Vec3, chunk_query: &Query<(&Chunk, &ChunkPos)>) -> bool {
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

    for (chunk, c_pos) in chunk_query.iter() {
        if *c_pos == chunk_pos {
            let local_x = block_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = block_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = block_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;

            let block = chunk.get_block(local_x, local_y, local_z);
            return block.is_solid();
        }
    }

    false
}

pub fn update_dropped_items(
    mut items: Query<(
        Entity,
        &mut DroppedItem,
        &mut Transform,
        &Handle<StandardMaterial>,
    )>,
    chunk_query: Query<(&Chunk, &ChunkPos)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (_entity, mut item, mut transform, material_handle) in items.iter_mut() {
        let dt = time.delta_seconds();
        let start_pos = transform.translation;

        // Apply gravity
        item.velocity.y += ITEM_GRAVITY * dt;

        // Calculate next position
        let next_pos = transform.translation + item.velocity * dt;

        // Check for collision with blocks below
        let check_pos = Vec3::new(
            next_pos.x,
            next_pos.y - ITEM_SIZE / 2.0 - 0.01, // Check slightly below item bottom
            next_pos.z,
        );

        if is_solid_block_at(check_pos, &chunk_query) {
            // Hit ground - stop at current position
            if item.velocity.y < 0.0 {
                item.velocity.y *= -ITEM_BOUNCE_DAMPING;

                // Stop bouncing if velocity is too small
                if item.velocity.y.abs() < 0.1 {
                    item.velocity.y = 0.0;
                }
            }

            // Apply friction when on ground
            item.velocity.x *= 0.95;
            item.velocity.z *= 0.95;

            // Keep item on top of the block
            let block_top = check_pos.y.floor() + 1.0;
            transform.translation.y = block_top + ITEM_SIZE / 2.0;
        } else {
            // No collision, update position
            transform.translation = next_pos;

            // Safety check: reset if fallen too far
            if start_pos.y - transform.translation.y > MAX_FALL_DISTANCE {
                // Reset to spawn position to prevent losing items
                transform.translation.y = start_pos.y;
                item.velocity = Vec3::ZERO;
            }
        }

        // Update material with pulsing glow effect
        if let Some(material) = materials.get_mut(material_handle) {
            let elapsed = time.elapsed_seconds() - item.spawn_time;
            let pulse = (elapsed * 3.0).sin() * 0.25 + 0.75; // Faster, more noticeable pulse between 0.5 and 1.0

            // Update emissive intensity for pulsing effect
            let base_emissive = get_emissive_color(item.block_type);
            material.emissive = LinearRgba::from(Color::srgba(
                (base_emissive.red * pulse).min(1.0),
                (base_emissive.green * pulse).min(1.0),
                (base_emissive.blue * pulse).min(1.0),
                1.0,
            ));
        }
    }
}

pub fn apply_item_collisions(
    mut items: Query<(Entity, &mut DroppedItem, &mut Transform)>,
    time: Res<Time>,
) {
    // Collect positions and entities for collision checking
    let mut item_data: Vec<(Entity, Vec3, f32)> = items
        .iter()
        .map(|(entity, _item, transform)| (entity, transform.translation, ITEM_SIZE / 2.0))
        .collect();

    // Check each pair of items for collision
    let dt = time.delta_seconds();
    for i in 0..item_data.len() {
        for j in (i + 1)..item_data.len() {
            let (_entity_a, pos_a, radius_a) = item_data[i];
            let (_entity_b, pos_b, radius_b) = item_data[j];

            // Calculate distance between items
            let diff = pos_a - pos_b;
            let distance = diff.length();
            let min_distance = radius_a + radius_b + 0.05; // Small buffer

            // If items are overlapping, push them apart
            if distance < min_distance && distance > 0.001 {
                let push_direction = diff.normalize();
                let overlap = min_distance - distance;
                let push_force = overlap * ITEM_COLLISION_STRENGTH * dt;

                // Apply push to both items (we'll update them in the next loop)
                item_data[i].1 += push_direction * push_force * 0.5;
                item_data[j].1 -= push_direction * push_force * 0.5;
            }
        }
    }

    // Apply the calculated positions and velocities back to the items
    for (entity, new_pos, _) in item_data {
        if let Ok((_, mut item, mut transform)) = items.get_mut(entity) {
            // Calculate velocity change from position adjustment
            let velocity_change = Vec3::new(
                (new_pos.x - transform.translation.x) * 5.0,
                0.0,
                (new_pos.z - transform.translation.z) * 5.0,
            );

            // Only apply horizontal push, maintain Y from physics
            transform.translation.x = new_pos.x;
            transform.translation.z = new_pos.z;

            // Update velocity with collision response
            item.velocity += velocity_change;

            // Apply some damping to prevent items from sliding forever
            item.velocity.x *= 0.98;
            item.velocity.z *= 0.98;
        }
    }
}

pub fn collect_items(
    mut commands: Commands,
    items: Query<(Entity, &DroppedItem, &Transform)>,
    player: Query<&Transform, With<PlayerCamera>>,
    mut hotbar: ResMut<Hotbar>,
) {
    let Ok(player_transform) = player.get_single() else {
        return;
    };

    for (entity, item, item_transform) in items.iter() {
        let distance = player_transform
            .translation
            .distance(item_transform.translation);

        if distance < COLLECTION_RADIUS {
            // Try to add to hotbar using the new stacking system
            let remaining = hotbar.add_item(item.block_type, 1);

            if remaining == 0 {
                // Successfully added to inventory
                commands.entity(entity).despawn();
                info!("Collected {:?}", item.block_type);
            } else {
                // Inventory full
                info!("Inventory full! Cannot collect {:?}", item.block_type);
            }
        }
    }
}
