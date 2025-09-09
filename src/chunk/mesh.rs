use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::chunk::{Chunk, CHUNK_SIZE};
use crate::texture::{BlockFace, BlockState, BlockTextureAtlas};

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

pub fn update_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_query: Query<(Entity, &mut Chunk, Option<&Handle<Mesh>>)>,
    texture_atlas: Option<Res<BlockTextureAtlas>>,
    time: Res<Time>,
) {
    let mut meshes_generated = 0;
    let total_dirty = chunk_query.iter().filter(|(_, chunk, _)| chunk.dirty).count();
    let start_time = time.elapsed_seconds();
    
    for (entity, mut chunk, mesh_handle) in chunk_query.iter_mut() {
        if !chunk.dirty {
            continue;
        }
        
        // Generate separate meshes for opaque and water blocks
        let (opaque_mesh, water_mesh) = generate_chunk_meshes(&chunk, texture_atlas.as_deref());
        chunk.dirty = false;
        meshes_generated += 1;
        
        // Remove old water mesh children if they exist
        commands.entity(entity).despawn_descendants();
        
        // Handle opaque mesh
        if opaque_mesh.count_vertices() == 0 && water_mesh.count_vertices() == 0 {
            if mesh_handle.is_some() {
                commands.entity(entity).remove::<Handle<Mesh>>();
                commands.entity(entity).remove::<Handle<StandardMaterial>>();
            }
            continue;
        }
        
        // Add opaque mesh to the main entity if it has vertices
        if opaque_mesh.count_vertices() > 0 {
            let opaque_mesh_handle = meshes.add(opaque_mesh);
            let opaque_material = if let Some(atlas) = &texture_atlas {
                materials.add(StandardMaterial {
                    base_color_texture: Some(atlas.texture.clone()),
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.9,
                    metallic: 0.0,
                    reflectance: 0.1,
                    // Note: We use double_sided rendering to ensure blocks are visible from all angles
                    // This is less performant but more robust given our mesh generation
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Opaque, // Changed from Mask to prevent texture artifacting
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
            
            commands.entity(entity).insert((
                opaque_mesh_handle,
                opaque_material,
            ));
        } else if water_mesh.count_vertices() > 0 {
            // If there's no opaque mesh but there is water, we still need a placeholder mesh
            // Otherwise the chunk entity won't render properly
            commands.entity(entity).remove::<Handle<Mesh>>();
            commands.entity(entity).remove::<Handle<StandardMaterial>>();
        }
        
        // Add water mesh as a child entity if it has vertices
        if water_mesh.count_vertices() > 0 {
            let water_mesh_handle = meshes.add(water_mesh);
            let water_material = if let Some(atlas) = &texture_atlas {
                materials.add(StandardMaterial {
                    base_color_texture: Some(atlas.texture.clone()),
                    base_color: Color::srgba(0.2, 0.6, 1.2, 0.85), // Semi-transparent bright blue
                    perceptual_roughness: 0.05,
                    metallic: 0.1,
                    reflectance: 0.6,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Premultiplied,
                    emissive: Color::srgba(0.0, 0.1, 0.3, 1.0).into(), // Blue emissive glow
                    ..default()
                })
            } else {
                materials.add(StandardMaterial {
                    base_color: Color::srgba(0.1, 0.4, 0.9, 0.8), // Deeper blue, more opaque
                    perceptual_roughness: 0.1,
                    metallic: 0.0,
                    reflectance: 0.4,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: AlphaMode::Premultiplied,
                    ..default()
                })
            };
            
            // Spawn water as a child entity
            let water_entity = commands.spawn((
                water_mesh_handle,
                water_material,
                WaterMesh,
                TransformBundle::default(),
                VisibilityBundle::default(),
            )).id();
            
            commands.entity(entity).add_child(water_entity);
        }
    }
    
    if meshes_generated > 0 {
        let elapsed = time.elapsed_seconds() - start_time;
        info!("Generated {} chunk meshes in {:.2}s (of {} dirty chunks)", 
            meshes_generated, elapsed, total_dirty);
    } else if total_dirty > 0 {
        debug!("No meshes generated but {} chunks are still dirty", total_dirty);
    }
}

pub fn generate_chunk_meshes(chunk: &Chunk, texture_atlas: Option<&BlockTextureAtlas>) -> (Mesh, Mesh) {
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
                    [1.0, 1.0, 1.0, 1.0]  // White when using textures
                } else {
                    block.get_color()  // Use block colors as fallback
                };
                
                // Check each face for visibility
                // Left face
                if x == 0 {
                    // At chunk boundary - render solid blocks always, skip water (likely continues in next chunk)
                    if !is_water {
                        add_face(vertices, indices, pos, Face::Left, color, block, texture_atlas);
                    }
                } else {
                    let adjacent = chunk.get_block(x - 1, y, z);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Left, color, block, texture_atlas);
                    }
                }
                
                // Right face
                if x == CHUNK_SIZE - 1 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(vertices, indices, pos, Face::Right, color, block, texture_atlas);
                    }
                } else {
                    let adjacent = chunk.get_block(x + 1, y, z);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Right, color, block, texture_atlas);
                    }
                }
                
                // Bottom face
                if y == 0 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(vertices, indices, pos, Face::Bottom, color, block, texture_atlas);
                    }
                } else {
                    let adjacent = chunk.get_block(x, y - 1, z);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Bottom, color, block, texture_atlas);
                    }
                }
                
                // Top face
                if y == CHUNK_SIZE - 1 {
                    // At chunk boundary - always render (water surface needs to be visible)
                    add_face(vertices, indices, pos, Face::Top, color, block, texture_atlas);
                } else {
                    let adjacent = chunk.get_block(x, y + 1, z);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Top, color, block, texture_atlas);
                    }
                }
                
                // Front face
                if z == 0 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(vertices, indices, pos, Face::Front, color, block, texture_atlas);
                    }
                } else {
                    let adjacent = chunk.get_block(x, y, z - 1);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Front, color, block, texture_atlas);
                    }
                }
                
                // Back face
                if z == CHUNK_SIZE - 1 {
                    // At chunk boundary - render solid blocks always, skip water
                    if !is_water {
                        add_face(vertices, indices, pos, Face::Back, color, block, texture_atlas);
                    }
                } else {
                    let adjacent = chunk.get_block(x, y, z + 1);
                    if should_render_face(block, adjacent) {
                        add_face(vertices, indices, pos, Face::Back, color, block, texture_atlas);
                    }
                }
            }
        }
    }
    
    (build_mesh(opaque_vertices, opaque_indices), 
     build_mesh(water_vertices, water_indices))
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
    use crate::block::BlockType;
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
        
        let (uv_min, uv_max) = atlas.get_uv(
            block.get_texture_name(),
            block_face,
            BlockState::Normal,
        );
        
        // Flip V coordinates for side faces to correct orientation
        match face {
            Face::Left | Face::Right | Face::Front | Face::Back => [
                [uv_min.x, uv_max.y],  // Bottom-left (was top-left)
                [uv_max.x, uv_max.y],  // Bottom-right (was top-right)
                [uv_max.x, uv_min.y],  // Top-right (was bottom-right)
                [uv_min.x, uv_min.y],  // Top-left (was bottom-left)
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
            Face::Left | Face::Right | Face::Front | Face::Back => [
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [0.0, 0.0],
            ],
            Face::Top | Face::Bottom => [
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ],
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
        start_index, start_index + 1, start_index + 2,
        start_index, start_index + 2, start_index + 3,
    ]);
}

fn build_mesh(vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList, 
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD
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