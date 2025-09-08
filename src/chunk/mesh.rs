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

pub fn update_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_query: Query<(Entity, &mut Chunk, Option<&Handle<Mesh>>)>,
    texture_atlas: Option<Res<BlockTextureAtlas>>,
) {
    for (entity, mut chunk, mesh_handle) in chunk_query.iter_mut() {
        if !chunk.dirty {
            continue;
        }
        
        let mesh = generate_chunk_mesh(&chunk, texture_atlas.as_deref());
        chunk.dirty = false;
        
        if mesh.count_vertices() == 0 {
            if mesh_handle.is_some() {
                commands.entity(entity).remove::<Handle<Mesh>>();
                commands.entity(entity).remove::<Handle<StandardMaterial>>();
            }
            continue;
        }
        
        let mesh_handle = meshes.add(mesh);
        let material_handle = if let Some(atlas) = &texture_atlas {
            materials.add(StandardMaterial {
                base_color_texture: Some(atlas.texture.clone()),
                base_color: Color::WHITE,
                perceptual_roughness: 0.9,
                metallic: 0.0,
                reflectance: 0.1,
                double_sided: true,
                cull_mode: None,
                alpha_mode: AlphaMode::Mask(0.5),
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
                ..default()
            })
        };
        
        commands.entity(entity).insert((
            mesh_handle,
            material_handle,
        ));
    }
}

pub fn generate_chunk_mesh(chunk: &Chunk, texture_atlas: Option<&BlockTextureAtlas>) -> Mesh {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    
    let mut block_count = 0;
    let mut face_count = 0;
    
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                if !block.is_visible() {
                    continue;
                }
                block_count += 1;
                
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                let color = if texture_atlas.is_some() {
                    [1.0, 1.0, 1.0, 1.0]  // White when using textures
                } else {
                    block.get_color()  // Use block colors as fallback
                };
                
                // Check left face
                if x == 0 {
                    add_face(&mut vertices, &mut indices, pos, Face::Left, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x - 1, y, z).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Left, color, block, texture_atlas);
                    face_count += 1;
                }
                
                // Check right face
                if x == CHUNK_SIZE - 1 {
                    add_face(&mut vertices, &mut indices, pos, Face::Right, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x + 1, y, z).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Right, color, block, texture_atlas);
                    face_count += 1;
                }
                
                // Check bottom face
                if y == 0 {
                    add_face(&mut vertices, &mut indices, pos, Face::Bottom, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x, y - 1, z).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Bottom, color, block, texture_atlas);
                    face_count += 1;
                }
                
                // Check top face
                if y == CHUNK_SIZE - 1 {
                    add_face(&mut vertices, &mut indices, pos, Face::Top, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x, y + 1, z).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Top, color, block, texture_atlas);
                    face_count += 1;
                }
                
                // Check front face
                if z == 0 {
                    add_face(&mut vertices, &mut indices, pos, Face::Front, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x, y, z - 1).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Front, color, block, texture_atlas);
                    face_count += 1;
                }
                
                // Check back face
                if z == CHUNK_SIZE - 1 {
                    add_face(&mut vertices, &mut indices, pos, Face::Back, color, block, texture_atlas);
                    face_count += 1;
                } else if !chunk.get_block(x, y, z + 1).is_solid() {
                    add_face(&mut vertices, &mut indices, pos, Face::Back, color, block, texture_atlas);
                    face_count += 1;
                }
            }
        }
    }
    
    build_mesh(vertices, indices)
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