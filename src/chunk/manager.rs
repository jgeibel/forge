use bevy::prelude::*;
use bevy::utils::HashSet;
use crate::chunk::{Chunk, ChunkPos};
use crate::camera::PlayerCamera;
use crate::planet::altitude_system::{AltitudeRenderSystem, should_render_chunks};

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<ChunkPos>,
}

pub fn spawn_chunks_around_player(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    player_query: Query<&Transform, With<PlayerCamera>>,
    altitude_system: Res<AltitudeRenderSystem>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    
    // Don't spawn chunks if we're in space
    if !should_render_chunks(player_transform.translation.y) {
        return;
    }
    
    let player_chunk = ChunkPos::from_world_pos(player_transform.translation);
    let view_distance = altitude_system.render_distance as i32;
    
    for dx in -view_distance..=view_distance {
        for dy in -2..=2 {
            for dz in -view_distance..=view_distance {
                // Use circular loading to avoid square edges
                let horizontal_distance = ((dx * dx + dz * dz) as f32).sqrt();
                if horizontal_distance > view_distance as f32 {
                    continue; // Skip chunks outside the circle
                }
                
                let chunk_pos = ChunkPos::new(
                    player_chunk.x + dx,
                    player_chunk.y + dy,
                    player_chunk.z + dz,
                );
                
                if !chunk_manager.loaded_chunks.contains(&chunk_pos) {
                    let chunk = Chunk::generate_terrain(chunk_pos);
                    let world_pos = chunk_pos.to_world_pos();
                    
                    commands.spawn((
                        chunk,
                        chunk_pos,
                        TransformBundle::from_transform(
                            Transform::from_translation(world_pos)
                        ),
                        VisibilityBundle::default(),
                    ));
                    
                    chunk_manager.loaded_chunks.insert(chunk_pos);
                }
            }
        }
    }
}

pub fn despawn_far_chunks(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    player_query: Query<&Transform, With<PlayerCamera>>,
    chunk_query: Query<(Entity, &ChunkPos)>,
    altitude_system: Res<AltitudeRenderSystem>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    
    let player_chunk = ChunkPos::from_world_pos(player_transform.translation);
    let despawn_distance = (altitude_system.render_distance as i32) + 2;
    
    for (entity, chunk_pos) in chunk_query.iter() {
        let distance = (chunk_pos.x - player_chunk.x).abs().max(
            (chunk_pos.y - player_chunk.y).abs().max(
                (chunk_pos.z - player_chunk.z).abs()
            )
        );
        
        if distance > despawn_distance || !should_render_chunks(player_transform.translation.y) {
            commands.entity(entity).despawn_recursive();
            chunk_manager.loaded_chunks.remove(chunk_pos);
        }
    }
}