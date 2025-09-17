use crate::camera::PlayerCamera;
use crate::chunk::{Chunk, ChunkPos};
use crate::loading::{GameState, LoadingProgress};
use crate::planet::altitude_system::{should_render_chunks, AltitudeRenderSystem};
use crate::planet::config::PLANET_SIZE_BLOCKS;
use crate::world::WorldGenerator;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashSet;
use futures_lite::future;
use std::sync::Arc;

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<ChunkPos>,
    pub initial_spawn_complete: bool,
}

/// Resource for tracking chunk generation tasks
#[derive(Resource, Default)]
pub struct ChunkGenerationQueue {
    pub pending_chunks: Vec<ChunkPos>,
    pub tasks: Vec<(ChunkPos, Task<Chunk>)>,
    pub initial_generation_complete: bool,
}

/// System to generate initial spawn area chunks
pub fn generate_initial_chunks(
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    mut loading_progress: ResMut<LoadingProgress>,
) {
    // Only queue chunks once at the beginning
    if chunk_queue.pending_chunks.is_empty()
        && chunk_queue.tasks.is_empty()
        && !chunk_queue.initial_generation_complete
        && loading_progress.total_chunks == 0
    {
        // Use spawn position from loading progress, or fall back to center
        let (spawn_chunk_x, spawn_chunk_z) =
            if let Some(spawn_pos) = loading_progress.spawn_position {
                let chunk_x = (spawn_pos.x / 32.0).floor() as i32;
                let chunk_z = (spawn_pos.z / 32.0).floor() as i32;
                info!(
                    "Generating chunks around spawn position: ({}, {})",
                    chunk_x, chunk_z
                );
                (chunk_x, chunk_z)
            } else {
                warn!("No spawn position available, using planet center");
                let spawn_chunk_x = (PLANET_SIZE_BLOCKS / 2 / 32) as i32;
                let spawn_chunk_z = (PLANET_SIZE_BLOCKS / 2 / 32) as i32;
                (spawn_chunk_x, spawn_chunk_z)
            };

        // Generate spawn area: 5x5 chunks horizontally, 3 chunks vertically around spawn point
        for x in -2..=2 {
            for y in -1..=1 {
                for z in -2..=2 {
                    chunk_queue.pending_chunks.push(ChunkPos::new(
                        spawn_chunk_x + x,
                        2 + y, // Start at chunk Y=2 (blocks 64-95)
                        spawn_chunk_z + z,
                    ));
                }
            }
        }
        loading_progress.total_chunks = chunk_queue.pending_chunks.len() as u32;
        info!(
            "Queued {} chunks for initial generation",
            loading_progress.total_chunks
        );
    }
}

/// System to spawn chunk generation tasks
pub fn spawn_chunk_tasks(
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    world_gen: Res<WorldGenerator>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    let initial_tasks = chunk_queue.tasks.len();
    let initial_pending = chunk_queue.pending_chunks.len();

    // Spawn up to 4 tasks at a time
    while chunk_queue.tasks.len() < 4 && !chunk_queue.pending_chunks.is_empty() {
        if let Some(chunk_pos) = chunk_queue.pending_chunks.pop() {
            let world_gen = Arc::new(world_gen.clone());
            let task = task_pool.spawn(async move {
                // Generate chunk in background thread
                Chunk::generate_with_world_gen(chunk_pos, &world_gen)
            });
            chunk_queue.tasks.push((chunk_pos, task));
        }
    }

    let spawned = chunk_queue.tasks.len() - initial_tasks;
    if spawned > 0 {
        debug!(
            "Spawned {} new chunk generation tasks. Active: {}, Pending: {}",
            spawned,
            chunk_queue.tasks.len(),
            chunk_queue.pending_chunks.len()
        );
    }
}

/// System to poll chunk generation tasks and spawn completed chunks
pub fn poll_chunk_tasks(
    mut commands: Commands,
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut loading_progress: ResMut<LoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut completed_indices = Vec::new();

    // Check for completed tasks
    for (index, (chunk_pos, task)) in chunk_queue.tasks.iter_mut().enumerate() {
        if let Some(chunk) = future::block_on(future::poll_once(task)) {
            let world_pos = chunk_pos.to_world_pos();

            // Spawn the chunk entity
            commands.spawn((
                chunk,
                *chunk_pos,
                TransformBundle::from_transform(Transform::from_translation(world_pos)),
                VisibilityBundle::default(),
            ));

            chunk_manager.loaded_chunks.insert(*chunk_pos);
            loading_progress.chunks_generated += 1;
            completed_indices.push(index);

            debug!(
                "Completed chunk {:?}. Progress: {}/{}",
                chunk_pos, loading_progress.chunks_generated, loading_progress.total_chunks
            );
        }
    }

    // Remove completed tasks
    if !completed_indices.is_empty() {
        info!(
            "Completed {} chunks. Total progress: {}/{} ({}%)",
            completed_indices.len(),
            loading_progress.chunks_generated,
            loading_progress.total_chunks,
            loading_progress.progress_percentage()
        );

        for index in completed_indices.iter().rev() {
            chunk_queue.tasks.remove(*index);
        }
    }

    // Check if initial generation is complete
    if chunk_queue.pending_chunks.is_empty()
        && chunk_queue.tasks.is_empty()
        && !chunk_queue.initial_generation_complete
    {
        chunk_queue.initial_generation_complete = true;
        next_state.set(GameState::Playing);
        info!("All chunks generated! Starting gameplay.");
    }
}

pub fn spawn_chunks_around_player(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    player_query: Query<&Transform, With<PlayerCamera>>,
    altitude_system: Res<AltitudeRenderSystem>,
    world_gen: Res<WorldGenerator>,
    time: Res<Time>,
) {
    // Wait a bit after initial spawn before generating more chunks
    if !chunk_manager.initial_spawn_complete {
        if time.elapsed_seconds() > 2.0 {
            chunk_manager.initial_spawn_complete = true;
            info!("Beginning dynamic chunk loading around player");
        } else {
            return; // Skip chunk generation for the first 2 seconds
        }
    }

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    // Don't spawn chunks if we're in space
    if !should_render_chunks(player_transform.translation.y) {
        return;
    }

    let player_chunk = ChunkPos::from_world_pos(player_transform.translation);
    let view_distance = altitude_system.render_distance as i32;

    // Limit chunks generated per frame to avoid freezing
    const MAX_CHUNKS_PER_FRAME: i32 = 4;
    let mut chunks_generated = 0;

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
                    let chunk = Chunk::generate_with_world_gen(chunk_pos, &world_gen);
                    let world_pos = chunk_pos.to_world_pos();

                    commands.spawn((
                        chunk,
                        chunk_pos,
                        TransformBundle::from_transform(Transform::from_translation(world_pos)),
                        VisibilityBundle::default(),
                    ));

                    chunk_manager.loaded_chunks.insert(chunk_pos);
                    chunks_generated += 1;

                    // Stop generating chunks this frame if we've hit the limit
                    if chunks_generated >= MAX_CHUNKS_PER_FRAME {
                        return;
                    }
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
            (chunk_pos.y - player_chunk.y)
                .abs()
                .max((chunk_pos.z - player_chunk.z).abs()),
        );

        if distance > despawn_distance || !should_render_chunks(player_transform.translation.y) {
            commands.entity(entity).despawn_recursive();
            chunk_manager.loaded_chunks.remove(chunk_pos);
        }
    }
}
