use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::{HashMap, HashSet};

use crate::camera::PlayerCamera;
use crate::chunk::CHUNK_SIZE_F32;
use crate::world::generator::WorldGenerator;

const TILE_CHUNKS: i32 = 4;
const TILE_RADIUS: i32 = 12;
// Keep far tiles from overlapping near chunks. With view distance of ~10 chunks
// the inner edge of the first far tile should align with the last near chunk.
// Each tile spans `TILE_CHUNKS` chunks, so an exclusion of 2 means the first
// tile (distance 3) starts 12 chunks out and overlaps seamlessly.
const INNER_EXCLUSION: i32 = 2;
const GRID_RESOLUTION: usize = 16;

#[derive(Resource, Default)]
pub struct FarTileTracker {
    tiles: HashMap<(i32, i32), Entity>,
}

#[derive(Component)]
pub struct FarTile {
    pub tile_x: i32,
    pub tile_z: i32,
}

pub fn update_far_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tracker: ResMut<FarTileTracker>,
    world_gen: Res<WorldGenerator>,
    player_query: Query<&Transform, With<PlayerCamera>>,
) {
    let Ok(transform) = player_query.get_single() else {
        return;
    };

    let tile_world_size = TILE_CHUNKS as f32 * CHUNK_SIZE_F32;
    let player_tile_x = (transform.translation.x / tile_world_size).floor() as i32;
    let player_tile_z = (transform.translation.z / tile_world_size).floor() as i32;

    let mut needed: HashSet<(i32, i32)> = HashSet::new();

    for dz in -TILE_RADIUS..=TILE_RADIUS {
        for dx in -TILE_RADIUS..=TILE_RADIUS {
            let distance = dx.abs().max(dz.abs());
            if distance > TILE_RADIUS {
                continue;
            }
            if distance <= INNER_EXCLUSION {
                continue;
            }

            let tile_x = player_tile_x + dx;
            let tile_z = player_tile_z + dz;
            needed.insert((tile_x, tile_z));

            if tracker.tiles.contains_key(&(tile_x, tile_z)) {
                continue;
            }

            let (mesh, material) = build_far_tile_mesh(&world_gen, tile_x, tile_z, TILE_CHUNKS);
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(material);

            let origin_x = tile_x as f32 * tile_world_size;
            let origin_z = tile_z as f32 * tile_world_size;

            let entity = commands
                .spawn((
                    PbrBundle {
                        mesh: mesh_handle,
                        material: material_handle,
                        transform: Transform::from_translation(Vec3::new(origin_x, 0.0, origin_z)),
                        ..Default::default()
                    },
                    FarTile { tile_x, tile_z },
                ))
                .id();

            tracker.tiles.insert((tile_x, tile_z), entity);
        }
    }

    let current: Vec<(i32, i32)> = tracker.tiles.keys().cloned().collect();
    for key in current {
        if !needed.contains(&key) {
            if let Some(entity) = tracker.tiles.remove(&key) {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn build_far_tile_mesh(
    generator: &WorldGenerator,
    tile_x: i32,
    tile_z: i32,
    tile_chunks: i32,
) -> (Mesh, StandardMaterial) {
    let tile_size = tile_chunks as f32 * CHUNK_SIZE_F32;
    let step = tile_size / GRID_RESOLUTION as f32;

    let origin_x = tile_x as f32 * tile_size;
    let origin_z = tile_z as f32 * tile_size;

    let vertex_count = (GRID_RESOLUTION + 1) * (GRID_RESOLUTION + 1);
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut colors = Vec::with_capacity(vertex_count);

    for z in 0..=GRID_RESOLUTION {
        for x in 0..=GRID_RESOLUTION {
            let world_x = origin_x + x as f32 * step;
            let world_z = origin_z + z as f32 * step;
            let height = generator.get_height(world_x, world_z);
            let biome = generator.get_biome(world_x, world_z);
            let preview = generator.preview_color(world_x, world_z, biome, height);

            positions.push([x as f32 * step, height, z as f32 * step]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                x as f32 / GRID_RESOLUTION as f32,
                z as f32 / GRID_RESOLUTION as f32,
            ]);
            colors.push([
                preview[0] as f32 / 255.0,
                preview[1] as f32 / 255.0,
                preview[2] as f32 / 255.0,
                1.0,
            ]);
        }
    }

    let mut indices = Vec::with_capacity(GRID_RESOLUTION * GRID_RESOLUTION * 6);
    let stride = GRID_RESOLUTION + 1;

    for z in 0..GRID_RESOLUTION {
        for x in 0..GRID_RESOLUTION {
            let i0 = z * stride + x;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;

            indices.extend_from_slice(&[i0 as u32, i2 as u32, i1 as u32]);
            indices.extend_from_slice(&[i1 as u32, i2 as u32, i3 as u32]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    let material = StandardMaterial {
        base_color: Color::WHITE,
        double_sided: true,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.1,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    };

    (mesh, material)
}
