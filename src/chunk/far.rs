use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::{HashMap, HashSet};

use crate::camera::PlayerCamera;
use crate::chunk::CHUNK_SIZE_F32;
use crate::planet::altitude_system::AltitudeRenderSystem;
use crate::world::generator::WorldGenerator;

const TILE_CHUNKS: i32 = 4;
const TILE_RADIUS: i32 = 12;
const GRID_RESOLUTION: usize = 16;
const OVERLAP_MARGIN_CHUNKS: i32 = 3;
const EDGE_FADE_WIDTH: f32 = 3.0;
const COLOR_SAMPLE_OFFSETS: [(f32, f32); 5] = [
    (0.0, 0.0),
    (-0.35, -0.35),
    (-0.35, 0.35),
    (0.35, -0.35),
    (0.35, 0.35),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EdgeFade {
    west: bool,
    east: bool,
    north: bool,
    south: bool,
}

#[derive(Resource, Default)]
pub struct FarTileTracker {
    tiles: HashMap<(i32, i32), Entity>,
}

#[derive(Component)]
pub struct FarTile {
    pub tile_x: i32,
    pub tile_z: i32,
    pub edge_fade: EdgeFade,
}

pub fn update_far_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tracker: ResMut<FarTileTracker>,
    world_gen: Res<WorldGenerator>,
    altitude_system: Res<AltitudeRenderSystem>,
    player_query: Query<&Transform, With<PlayerCamera>>,
    tile_query: Query<&FarTile>,
) {
    let Ok(transform) = player_query.get_single() else {
        return;
    };

    let tile_world_size = TILE_CHUNKS as f32 * CHUNK_SIZE_F32;
    let player_tile_x = (transform.translation.x / tile_world_size).floor() as i32;
    let player_tile_z = (transform.translation.z / tile_world_size).floor() as i32;

    let mut needed: HashSet<(i32, i32)> = HashSet::new();

    let desired_start_chunks =
        (altitude_system.render_distance as i32 - OVERLAP_MARGIN_CHUNKS).max(0);
    let mut inner_exclusion = desired_start_chunks / TILE_CHUNKS;
    if inner_exclusion > 0 {
        inner_exclusion -= 1;
    }
    inner_exclusion = inner_exclusion.clamp(0, TILE_RADIUS - 1);

    for dz in -TILE_RADIUS..=TILE_RADIUS {
        for dx in -TILE_RADIUS..=TILE_RADIUS {
            let distance = dx.abs().max(dz.abs());
            if distance > TILE_RADIUS {
                continue;
            }
            if distance <= inner_exclusion {
                continue;
            }

            let tile_x = player_tile_x + dx;
            let tile_z = player_tile_z + dz;
            needed.insert((tile_x, tile_z));

            let edge_fade = compute_edge_fade(dx, dz);

            let mut spawn_tile = true;
            if let Some(&entity) = tracker.tiles.get(&(tile_x, tile_z)) {
                if let Ok(existing) = tile_query.get(entity) {
                    if existing.edge_fade == edge_fade {
                        spawn_tile = false;
                    } else {
                        commands.entity(entity).despawn_recursive();
                        tracker.tiles.remove(&(tile_x, tile_z));
                    }
                } else {
                    tracker.tiles.remove(&(tile_x, tile_z));
                }
            }

            if !spawn_tile {
                continue;
            }

            let (mesh, material) =
                build_far_tile_mesh(&world_gen, tile_x, tile_z, TILE_CHUNKS, edge_fade);
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
                    FarTile {
                        tile_x,
                        tile_z,
                        edge_fade,
                    },
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
    edge_fade: EdgeFade,
) -> (Mesh, StandardMaterial) {
    let tile_size = tile_chunks as f32 * CHUNK_SIZE_F32;
    let step = tile_size / GRID_RESOLUTION as f32;

    let origin_x = tile_x as f32 * tile_size;
    let origin_z = tile_z as f32 * tile_size;

    let grid_size = GRID_RESOLUTION + 1;
    let vertex_count = grid_size * grid_size;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut raw_colors = vec![[0.0_f32; 3]; vertex_count];
    let mut alphas = vec![0.0_f32; vertex_count];
    let sample_weight = COLOR_SAMPLE_OFFSETS.len() as f32;

    for z in 0..=GRID_RESOLUTION {
        for x in 0..=GRID_RESOLUTION {
            let idx = z * grid_size + x;
            let world_x = origin_x + x as f32 * step;
            let world_z = origin_z + z as f32 * step;

            let mut height_accum = 0.0_f32;
            let mut color_accum = [0.0_f32; 3];

            for &(offset_x, offset_z) in &COLOR_SAMPLE_OFFSETS {
                let sample_world_x = world_x + offset_x * step;
                let sample_world_z = world_z + offset_z * step;
                let sample_height = generator.get_height(sample_world_x, sample_world_z);
                let sample_biome = generator.get_biome(sample_world_x, sample_world_z);
                let sample_color = generator.preview_color(
                    sample_world_x,
                    sample_world_z,
                    sample_biome,
                    sample_height,
                );

                height_accum += sample_height;
                color_accum[0] += srgb_u8_to_linear(sample_color[0]);
                color_accum[1] += srgb_u8_to_linear(sample_color[1]);
                color_accum[2] += srgb_u8_to_linear(sample_color[2]);
            }

            let height = height_accum / sample_weight;

            positions.push([x as f32 * step, height, z as f32 * step]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([
                x as f32 / GRID_RESOLUTION as f32,
                z as f32 / GRID_RESOLUTION as f32,
            ]);

            let mut alpha = 1.0_f32;

            if edge_fade.west {
                let fade = (x as f32 / EDGE_FADE_WIDTH).clamp(0.0, 1.0).powf(1.5);
                alpha = alpha.min(fade);
            }
            if edge_fade.east {
                let distance = (GRID_RESOLUTION - x) as f32;
                let fade = (distance / EDGE_FADE_WIDTH).clamp(0.0, 1.0).powf(1.5);
                alpha = alpha.min(fade);
            }
            if edge_fade.north {
                let fade = (z as f32 / EDGE_FADE_WIDTH).clamp(0.0, 1.0).powf(1.5);
                alpha = alpha.min(fade);
            }
            if edge_fade.south {
                let distance = (GRID_RESOLUTION - z) as f32;
                let fade = (distance / EDGE_FADE_WIDTH).clamp(0.0, 1.0).powf(1.5);
                alpha = alpha.min(fade);
            }

            alphas[idx] = alpha;

            let inv_weight = 1.0 / sample_weight;
            raw_colors[idx] = [
                color_accum[0] * inv_weight,
                color_accum[1] * inv_weight,
                color_accum[2] * inv_weight,
            ];
        }
    }

    let smoothed_colors = smooth_color_grid(&raw_colors, grid_size);

    let mut colors = Vec::with_capacity(vertex_count);
    for idx in 0..vertex_count {
        let mut color = smoothed_colors[idx];
        let alpha = alphas[idx];
        if alpha < 1.0 {
            color[0] *= alpha;
            color[1] *= alpha;
            color[2] *= alpha;
        }
        colors.push([color[0], color[1], color[2], alpha]);
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
        alpha_mode: AlphaMode::Premultiplied,
        ..default()
    };

    (mesh, material)
}

fn srgb_u8_to_linear(value: u8) -> f32 {
    let srgb = value as f32 / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn smooth_color_grid(raw_colors: &[[f32; 3]], grid_size: usize) -> Vec<[f32; 3]> {
    let mut smoothed = vec![[0.0_f32; 3]; raw_colors.len()];

    for z in 0..grid_size {
        for x in 0..grid_size {
            let idx = z * grid_size + x;
            let mut accum = [0.0_f32; 3];
            let mut weight = 0.0_f32;

            for dz in -1..=1 {
                for dx in -1..=1 {
                    let neighbor_x = x as isize + dx;
                    let neighbor_z = z as isize + dz;

                    if neighbor_x < 0
                        || neighbor_z < 0
                        || neighbor_x >= grid_size as isize
                        || neighbor_z >= grid_size as isize
                    {
                        continue;
                    }

                    let kernel_weight = match (dx.abs(), dz.abs()) {
                        (0, 0) => 4.0,
                        (0, 1) | (1, 0) => 2.0,
                        _ => 1.0,
                    };

                    let neighbor_idx = neighbor_z as usize * grid_size + neighbor_x as usize;
                    let sample = raw_colors[neighbor_idx];
                    accum[0] += sample[0] * kernel_weight;
                    accum[1] += sample[1] * kernel_weight;
                    accum[2] += sample[2] * kernel_weight;
                    weight += kernel_weight;
                }
            }

            if weight > 0.0 {
                let inv = 1.0 / weight;
                smoothed[idx] = [accum[0] * inv, accum[1] * inv, accum[2] * inv];
            } else {
                smoothed[idx] = raw_colors[idx];
            }
        }
    }

    smoothed
}

fn compute_edge_fade(dx: i32, dz: i32) -> EdgeFade {
    EdgeFade {
        west: dx == -TILE_RADIUS,
        east: dx == TILE_RADIUS,
        north: dz == -TILE_RADIUS,
        south: dz == TILE_RADIUS,
    }
}
