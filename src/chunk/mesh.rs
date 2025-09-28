use crate::chunk::{Chunk, CHUNK_SIZE};
use crate::texture::{BlockFace, BlockState, BlockTextureAtlas};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::tasks::{IoTaskPool, Task};
use bevy::utils::HashSet;
use futures_lite::future;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

/// Component to mark water mesh entities
#[derive(Component)]
pub struct WaterMesh;

#[derive(Resource, Default)]
pub struct ChunkMeshJobs {
    tasks: Vec<(Entity, Task<MeshBuildResult>)>,
    scheduled: HashSet<Entity>,
    recent_durations: VecDeque<f32>,
}

impl ChunkMeshJobs {
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn scheduled_count(&self) -> usize {
        self.scheduled.len()
    }

    pub fn average_duration_ms(&self) -> Option<f32> {
        if self.recent_durations.is_empty() {
            None
        } else {
            let sum: f32 = self.recent_durations.iter().copied().sum();
            Some(sum / self.recent_durations.len() as f32)
        }
    }
}

struct MeshBuildResult {
    opaque_mesh: Mesh,
    water_mesh: Mesh,
    duration: f32,
}

const MESH_HISTORY_SAMPLES: usize = 120;

pub fn queue_chunk_mesh_builds(
    mut chunk_query: Query<(Entity, &mut Chunk)>,
    texture_atlas: Option<Res<BlockTextureAtlas>>,
    mut mesh_jobs: ResMut<ChunkMeshJobs>,
) {
    let atlas_snapshot = texture_atlas
        .as_ref()
        .map(|atlas| Arc::new((**atlas).clone()));
    let task_pool = IoTaskPool::get();

    for (entity, mut chunk) in chunk_query.iter_mut() {
        if !chunk.dirty || mesh_jobs.scheduled.contains(&entity) {
            continue;
        }

        let storage = chunk.storage.clone();
        let chunk_pos = chunk.position;
        chunk.dirty = false;
        mesh_jobs.scheduled.insert(entity);

        let atlas_for_task = atlas_snapshot.clone();
        let task = task_pool.spawn(async move {
            let start = Instant::now();
            let mut chunk_copy = Chunk::from_storage(chunk_pos, storage);
            chunk_copy.dirty = false;
            let atlas_ref = atlas_for_task.as_ref().map(|atlas| atlas.as_ref());
            let (opaque_mesh, water_mesh) = generate_chunk_meshes(&chunk_copy, atlas_ref);
            MeshBuildResult {
                opaque_mesh,
                water_mesh,
                duration: start.elapsed().as_secs_f32(),
            }
        });

        mesh_jobs.tasks.push((entity, task));
    }
}

pub fn apply_chunk_mesh_results(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_jobs: ResMut<ChunkMeshJobs>,
    texture_atlas: Option<Res<BlockTextureAtlas>>,
) {
    let atlas_option = texture_atlas.as_deref();

    let mut finished_indices = Vec::new();
    let mut finished_payloads = Vec::new();
    let mut total_duration_ms = 0.0_f32;
    let mut total_vertices = 0_usize;
    let mut processed = 0_usize;

    for (index, (entity, task)) in mesh_jobs.tasks.iter_mut().enumerate() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            finished_indices.push(index);
            finished_payloads.push((*entity, result));
        }
    }

    for (entity, result) in finished_payloads.into_iter() {
        mesh_jobs.scheduled.remove(&entity);
        mesh_jobs
            .recent_durations
            .push_back(result.duration * 1000.0);
        if mesh_jobs.recent_durations.len() > MESH_HISTORY_SAMPLES {
            mesh_jobs.recent_durations.pop_front();
        }

        let Some(mut entity_commands) = commands.get_entity(entity) else {
            continue;
        };

        entity_commands.despawn_descendants();

        let opaque_vertices = result.opaque_mesh.count_vertices();
        let water_vertices = result.water_mesh.count_vertices();

        processed += 1;
        total_duration_ms += result.duration * 1000.0;
        total_vertices += (opaque_vertices + water_vertices) as usize;

        if opaque_vertices == 0 && water_vertices == 0 {
            entity_commands.remove::<Handle<Mesh>>();
            entity_commands.remove::<Handle<StandardMaterial>>();
        } else if opaque_vertices > 0 {
            let mesh_handle = meshes.add(result.opaque_mesh);
            let material = if let Some(atlas) = atlas_option {
                materials.add(StandardMaterial {
                    base_color_texture: Some(atlas.texture.clone()),
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.9,
                    metallic: 0.0,
                    reflectance: 0.1,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            } else {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.9,
                    metallic: 0.0,
                    reflectance: 0.1,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            };

            entity_commands.insert((mesh_handle, material));
        } else {
            entity_commands.remove::<Handle<Mesh>>();
            entity_commands.remove::<Handle<StandardMaterial>>();
        }

        drop(entity_commands);

        if water_vertices > 0 {
            let water_mesh_handle = meshes.add(result.water_mesh);
            let water_material = if let Some(atlas) = atlas_option {
                materials.add(StandardMaterial {
                    base_color_texture: Some(atlas.texture.clone()),
                    base_color: Color::srgba(0.2, 0.6, 1.2, 0.85),
                    perceptual_roughness: 0.05,
                    metallic: 0.1,
                    reflectance: 0.6,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Premultiplied,
                    emissive: Color::srgba(0.0, 0.1, 0.3, 1.0).into(),
                    ..default()
                })
            } else {
                materials.add(StandardMaterial {
                    base_color: Color::srgba(0.1, 0.4, 0.9, 0.8),
                    perceptual_roughness: 0.1,
                    metallic: 0.0,
                    reflectance: 0.4,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Premultiplied,
                    ..default()
                })
            };

            let water_entity = commands
                .spawn((
                    water_mesh_handle,
                    water_material,
                    WaterMesh,
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                ))
                .id();

            commands.entity(entity).add_child(water_entity);
        }
    }

    if processed > 0 {
        let average_ms = total_duration_ms / processed as f32;
        info!(
            "chunk-mesh apply: count={} total_ms={:.2} avg_ms={:.2} total_vertices={}",
            processed, total_duration_ms, average_ms, total_vertices
        );
    }

    if !finished_indices.is_empty() {
        finished_indices.sort_unstable();
        for index in finished_indices.into_iter().rev() {
            let (_entity, task) = mesh_jobs.tasks.swap_remove(index);
            task.detach();
        }
    }
}

pub fn generate_chunk_meshes(
    chunk: &Chunk,
    texture_atlas: Option<&BlockTextureAtlas>,
) -> (Mesh, Mesh) {
    // Separate vertices for opaque and water meshes
    let mut opaque_vertices: Vec<Vertex> = Vec::with_capacity(1024);
    let mut opaque_indices: Vec<u32> = Vec::with_capacity(1536);
    let mut water_vertices: Vec<Vertex> = Vec::with_capacity(256);
    let mut water_indices: Vec<u32> = Vec::with_capacity(384);

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                if !block.is_visible() {
                    continue;
                }

                let pos = Vec3::new(x as f32, y as f32, z as f32);
                let is_water = block.is_liquid();

                // Choose which mesh to add to
                let (vertices, indices) = if is_water {
                    (&mut water_vertices, &mut water_indices)
                } else {
                    (&mut opaque_vertices, &mut opaque_indices)
                };

                let color = if texture_atlas.is_some() {
                    [1.0, 1.0, 1.0, 1.0] // White when using textures
                } else {
                    block.get_color() // Use block colors as fallback
                };

                // Check each face for visibility
                // Left face
                if x == 0 {
                    // At chunk boundary - render solid blocks always, skip water (likely continues in next chunk)
                    if !is_water {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Left,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                } else {
                    let adjacent = chunk.get_block(x - 1, y, z);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Left,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }

                // Right face
                if x == CHUNK_SIZE - 1 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Right,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                } else {
                    let adjacent = chunk.get_block(x + 1, y, z);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Right,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }

                // Bottom face
                if y == 0 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Bottom,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                } else {
                    let adjacent = chunk.get_block(x, y - 1, z);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Bottom,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }

                // Top face
                if y == CHUNK_SIZE - 1 {
                    // At chunk boundary - always render (water surface needs to be visible)
                    add_face(
                        vertices,
                        indices,
                        pos,
                        Face::Top,
                        color,
                        block,
                        texture_atlas,
                    );
                } else {
                    let adjacent = chunk.get_block(x, y + 1, z);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Top,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }

                // Front face
                if z == 0 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Front,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                } else {
                    let adjacent = chunk.get_block(x, y, z - 1);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Front,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }

                // Back face
                if z == CHUNK_SIZE - 1 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Back,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                } else {
                    let adjacent = chunk.get_block(x, y, z + 1);
                    if should_render_face(block, adjacent) {
                        add_face(
                            vertices,
                            indices,
                            pos,
                            Face::Back,
                            color,
                            block,
                            texture_atlas,
                        );
                    }
                }
            }
        }
    }

    (
        build_mesh(opaque_vertices, opaque_indices),
        build_mesh(water_vertices, water_indices),
    )
}

/// Determine if a face should be rendered based on block adjacency
fn should_render_face(block: crate::block::BlockType, adjacent: crate::block::BlockType) -> bool {
    use crate::block::BlockType;

    // Never render faces between identical solid blocks
    if block == adjacent && block.is_solid() {
        return false;
    }

    // Don't render faces between water blocks
    if block.is_liquid() && adjacent.is_liquid() {
        return false;
    }

    // For water blocks, only render against air (not solid blocks to reduce overdraw)
    if block.is_liquid() {
        return adjacent == BlockType::Air;
    }

    // For solid blocks, only render if adjacent is not solid
    // This prevents rendering faces between different solid blocks
    if block.is_solid() && adjacent.is_solid() {
        return false;
    }

    // Otherwise render the face
    true
}

enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}

fn add_face(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    pos: Vec3,
    face: Face,
    color: [f32; 4],
    block: crate::block::BlockType,
    texture_atlas: Option<&BlockTextureAtlas>,
) {
    let start_index = vertices.len() as u32;

    let (positions, normal) = match face {
        Face::Top => (
            [
                [pos.x, pos.y + 1.0, pos.z],
                [pos.x + 1.0, pos.y + 1.0, pos.z],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0],
                [pos.x, pos.y + 1.0, pos.z + 1.0],
            ],
            [0.0, 1.0, 0.0],
        ),
        Face::Bottom => (
            [
                [pos.x, pos.y, pos.z + 1.0],
                [pos.x + 1.0, pos.y, pos.z + 1.0],
                [pos.x + 1.0, pos.y, pos.z],
                [pos.x, pos.y, pos.z],
            ],
            [0.0, -1.0, 0.0],
        ),
        Face::Left => (
            [
                [pos.x, pos.y, pos.z + 1.0],
                [pos.x, pos.y, pos.z],
                [pos.x, pos.y + 1.0, pos.z],
                [pos.x, pos.y + 1.0, pos.z + 1.0],
            ],
            [-1.0, 0.0, 0.0],
        ),
        Face::Right => (
            [
                [pos.x + 1.0, pos.y, pos.z],
                [pos.x + 1.0, pos.y, pos.z + 1.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z],
            ],
            [1.0, 0.0, 0.0],
        ),
        Face::Front => (
            [
                [pos.x, pos.y, pos.z],
                [pos.x + 1.0, pos.y, pos.z],
                [pos.x + 1.0, pos.y + 1.0, pos.z],
                [pos.x, pos.y + 1.0, pos.z],
            ],
            [0.0, 0.0, -1.0],
        ),
        Face::Back => (
            [
                [pos.x + 1.0, pos.y, pos.z + 1.0],
                [pos.x, pos.y, pos.z + 1.0],
                [pos.x, pos.y + 1.0, pos.z + 1.0],
                [pos.x + 1.0, pos.y + 1.0, pos.z + 1.0],
            ],
            [0.0, 0.0, 1.0],
        ),
    };

    // Get UV coordinates from texture atlas
    let uvs = if let Some(atlas) = texture_atlas {
        let block_face = match face {
            Face::Top => BlockFace::Top,
            Face::Bottom => BlockFace::Bottom,
            Face::Front => BlockFace::Front,
            Face::Back => BlockFace::Back,
            Face::Left => BlockFace::Left,
            Face::Right => BlockFace::Right,
        };

        let (uv_min, uv_max) =
            atlas.get_uv(block.get_texture_name(), block_face, BlockState::Normal);

        // Flip V coordinates for side faces to correct orientation
        match face {
            Face::Left | Face::Right | Face::Front | Face::Back => [
                [uv_min.x, uv_max.y], // Bottom-left (was top-left)
                [uv_max.x, uv_max.y], // Bottom-right (was top-right)
                [uv_max.x, uv_min.y], // Top-right (was bottom-right)
                [uv_min.x, uv_min.y], // Top-left (was bottom-left)
            ],
            Face::Top | Face::Bottom => [
                [uv_min.x, uv_min.y],
                [uv_max.x, uv_min.y],
                [uv_max.x, uv_max.y],
                [uv_min.x, uv_max.y],
            ],
        }
    } else {
        // Default UVs when no texture atlas
        match face {
            Face::Left | Face::Right | Face::Front | Face::Back => {
                [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]
            }
            Face::Top | Face::Bottom => [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        }
    };

    for i in 0..4 {
        vertices.push(Vertex {
            position: positions[i],
            normal,
            uv: uvs[i],
            color,
        });
    }

    indices.extend_from_slice(&[
        start_index,
        start_index + 1,
        start_index + 2,
        start_index,
        start_index + 2,
        start_index + 3,
    ]);
}

fn build_mesh(vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let positions: Vec<[f32; 3]> = vertices.iter().map(|v| v.position).collect();
    let normals: Vec<[f32; 3]> = vertices.iter().map(|v| v.normal).collect();
    let uvs: Vec<[f32; 2]> = vertices.iter().map(|v| v.uv).collect();
    let colors: Vec<[f32; 4]> = vertices.iter().map(|v| v.color).collect();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
