use super::mesh::ChunkMeshJobs;
use crate::camera::PlayerCamera;
use crate::chunk::{Chunk, ChunkPos, ChunkStorage, CHUNK_SIZE};
use crate::loading::{GameState, LoadingProgress};
use crate::planet::altitude_system::{should_render_chunks, AltitudeRenderSystem};
use crate::planet::config::PLANET_SIZE_BLOCKS;
use crate::world::chunk_store::StoreUpdate;
use crate::world::persistence::{ChunkPersistence, DiskChunkPersistence, PersistenceHandler};
use crate::world::{
    ChunkPayloadQueue, ChunkPayloadReady, PlanetChunkStore, QueuedChunkPayload, WorldGenerator,
};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashSet;
use futures_lite::future;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::Instant;

const MAX_CONCURRENT_CHUNK_TASKS: usize = 6;
const MAX_IMMEDIATE_CHUNKS_PER_FRAME: i32 = 4;
const MAX_QUEUED_CHUNKS_PER_FRAME: i32 = 48;
const PREFETCH_MARGIN: i32 = 2;
const VERTICAL_DISTANCE_WEIGHT: i32 = 4;
const BASE_DYNAMIC_ENQUEUE_BUDGET_MS: f32 = 8.0;
const VIEW_CULL_THRESHOLD: f32 = -0.35;
const VIEW_ALIGNMENT_SCALE: f32 = 240.0;

fn burst_multiplier(backlog: usize) -> i32 {
    if backlog >= 768 {
        6
    } else if backlog >= 384 {
        5
    } else if backlog >= 192 {
        4
    } else if backlog >= 96 {
        3
    } else if backlog >= 32 {
        2
    } else {
        1
    }
}

#[derive(Clone, Copy, Debug)]
struct QueuedChunk {
    priority: i32,
    position: ChunkPos,
}

impl PartialEq for QueuedChunk {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.position == other.position
    }
}

impl Eq for QueuedChunk {}

impl PartialOrd for QueuedChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.position.x.cmp(&other.position.x))
            .then_with(|| self.position.y.cmp(&other.position.y))
            .then_with(|| self.position.z.cmp(&other.position.z))
    }
}

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<ChunkPos>,
    pub initial_spawn_complete: bool,
    candidate_state: CandidateState,
}

/// Resource for tracking chunk generation tasks
#[derive(Resource, Default)]
pub struct ChunkGenerationQueue {
    pending_chunks: BinaryHeap<QueuedChunk>,
    pub tasks: Vec<(ChunkPos, Task<ChunkStorage>)>,
    pub initial_generation_complete: bool,
    scheduled: HashSet<ChunkPos>,
    in_flight_start: HashMap<ChunkPos, f32>,
    recent_durations_ms: VecDeque<f32>,
}

#[derive(Default)]
struct CandidateState {
    last_player_chunk: Option<ChunkPos>,
    last_render_distance: i32,
    last_forward: Vec3,
    cached_candidates: Vec<(ChunkPos, i32)>,
    cursor: usize,
}

impl CandidateState {
    fn remaining_count(&self) -> usize {
        self.cached_candidates.len().saturating_sub(self.cursor)
    }

    fn next(&mut self) -> Option<(ChunkPos, i32)> {
        if self.cursor >= self.cached_candidates.len() {
            return None;
        }
        let result = self.cached_candidates[self.cursor];
        self.cursor += 1;
        Some(result)
    }

    fn should_rebuild(&self, player_chunk: ChunkPos, view_distance: i32, forward: Vec3) -> bool {
        if self
            .last_player_chunk
            .map_or(true, |last| last != player_chunk)
        {
            return true;
        }

        if self.last_render_distance != view_distance {
            return true;
        }

        let stored_forward = self.last_forward;
        let forward_len = forward.length_squared();
        if forward_len > f32::EPSILON {
            let stored_len = stored_forward.length_squared();
            if stored_len > f32::EPSILON {
                let dot = stored_forward.normalize().dot(forward.normalize());
                if dot < 0.9 {
                    return true;
                }
            }
        }

        false
    }

    fn ensure_candidates(
        &mut self,
        player_chunk: ChunkPos,
        view_distance: i32,
        forward: Vec3,
        player_pos: Vec3,
        world_gen: &WorldGenerator,
    ) {
        let normalized_forward = if forward.length_squared() > f32::EPSILON {
            forward.normalize()
        } else {
            Vec3::Z
        };

        if self.should_rebuild(player_chunk, view_distance, normalized_forward) {
            self.rebuild(
                player_chunk,
                view_distance,
                normalized_forward,
                player_pos,
                world_gen,
            );
        }
    }

    fn rebuild(
        &mut self,
        player_chunk: ChunkPos,
        view_distance: i32,
        forward: Vec3,
        player_pos: Vec3,
        world_gen: &WorldGenerator,
    ) {
        let prefetch_distance = view_distance + PREFETCH_MARGIN;
        let max_horizontal_distance_sq = prefetch_distance * prefetch_distance;

        let mut candidates: Vec<(ChunkPos, i32)> = Vec::new();

        for dx in -prefetch_distance..=prefetch_distance {
            for dz in -prefetch_distance..=prefetch_distance {
                let horizontal_distance_sq = dx * dx + dz * dz;
                if horizontal_distance_sq > max_horizontal_distance_sq {
                    continue;
                }

                let chunk_x = player_chunk.x + dx;
                let chunk_z = player_chunk.z + dz;
                let surface_y = world_gen.surface_chunk_y(chunk_x, chunk_z);

                let mut y_targets = Vec::with_capacity(4);
                y_targets.push(surface_y);

                if !y_targets.contains(&player_chunk.y) {
                    y_targets.push(player_chunk.y);
                }

                let upper = surface_y + 1;
                if !y_targets.contains(&upper) {
                    y_targets.push(upper);
                }

                let lower = surface_y - 1;
                if !y_targets.contains(&lower) {
                    y_targets.push(lower);
                }

                for &chunk_y in &y_targets {
                    let chunk_pos = ChunkPos::new(chunk_x, chunk_y, chunk_z);

                    let vertical_distance = (chunk_y - player_chunk.y).abs();
                    let vertical_score = vertical_distance * VERTICAL_DISTANCE_WEIGHT;
                    let mut score = horizontal_distance_sq + vertical_score;

                    let chunk_center =
                        chunk_pos.to_world_pos() + Vec3::splat(CHUNK_SIZE as f32 * 0.5);
                    let to_chunk = chunk_center - player_pos;
                    let distance_sq = to_chunk.length_squared();

                    if distance_sq > f32::EPSILON {
                        let alignment = forward.dot(to_chunk / distance_sq.sqrt());
                        if alignment < VIEW_CULL_THRESHOLD && horizontal_distance_sq > 4 {
                            continue;
                        }
                        let penalty =
                            ((1.0 - alignment).max(0.0_f32) * VIEW_ALIGNMENT_SCALE).round() as i32;
                        score += penalty.max(0);
                    }

                    candidates.push((chunk_pos, score));
                }
            }
        }

        candidates.sort_by_key(|(_, score)| *score);

        self.cached_candidates = candidates;
        self.cursor = 0;
        self.last_player_chunk = Some(player_chunk);
        self.last_render_distance = view_distance;
        self.last_forward = forward;
    }
}

impl ChunkGenerationQueue {
    pub fn contains(&self, position: &ChunkPos) -> bool {
        self.scheduled.contains(position)
    }

    pub fn enqueue_with_priority(&mut self, position: ChunkPos, priority: i32) -> bool {
        if self.contains(&position) {
            return false;
        }

        self.pending_chunks.push(QueuedChunk { priority, position });
        self.scheduled.insert(position);
        true
    }

    fn requeue(&mut self, position: ChunkPos, priority: i32) {
        self.pending_chunks.push(QueuedChunk { priority, position });
    }

    fn pop_pending(&mut self) -> Option<(ChunkPos, i32)> {
        self.pending_chunks
            .pop()
            .map(|QueuedChunk { priority, position }| (position, priority))
    }

    fn drain_pending(&mut self) -> Vec<(ChunkPos, i32)> {
        let mut drained = Vec::with_capacity(self.pending_chunks.len());
        while let Some(QueuedChunk { priority, position }) = self.pending_chunks.pop() {
            drained.push((position, priority));
        }
        drained
    }

    pub fn pending_len(&self) -> usize {
        self.pending_chunks.len()
    }

    fn mark_completed(&mut self, position: &ChunkPos) {
        self.scheduled.remove(position);
    }

    fn record_start(&mut self, position: ChunkPos, start_time: f32) {
        self.in_flight_start.insert(position, start_time);
    }

    fn record_duration(&mut self, position: &ChunkPos, end_time: f32) {
        if let Some(start) = self.in_flight_start.remove(position) {
            let duration_ms = (end_time - start) * 1000.0;
            self.recent_durations_ms.push_back(duration_ms);
            if self.recent_durations_ms.len() > 240 {
                self.recent_durations_ms.pop_front();
            }
        }
    }

    fn average_duration_ms(&self) -> Option<f32> {
        if self.recent_durations_ms.is_empty() {
            None
        } else {
            let sum: f32 = self.recent_durations_ms.iter().copied().sum();
            Some(sum / self.recent_durations_ms.len() as f32)
        }
    }

    fn inflight_count(&self) -> usize {
        self.in_flight_start.len()
    }
}

pub fn load_persisted_pending_chunks(
    mut commands: Commands,
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut loading_progress: ResMut<LoadingProgress>,
    mut chunk_store: ResMut<PlanetChunkStore>,
    persistence: Option<Res<PersistenceHandler<DiskChunkPersistence>>>,
    mut chunk_events: EventWriter<ChunkPayloadReady>,
) {
    let handler = match persistence {
        Some(handler) => handler,
        None => return,
    };

    if chunk_queue.pending_len() == 0 {
        return;
    }

    let pending = chunk_queue.drain_pending();
    let mut remaining = Vec::with_capacity(pending.len());

    for (chunk_pos, priority) in pending {
        let mut spawn_storage: Option<ChunkStorage> = None;
        let mut revision = 1;

        if let Some((arc, rev)) = chunk_store.get_with_revision(&chunk_pos) {
            if !chunk_manager.loaded_chunks.contains(&chunk_pos) {
                spawn_storage = Some(arc.as_ref().clone());
                revision = rev;
            }
        } else if let Ok(Some((persist_revision, bytes))) = handler.handler().load(chunk_pos) {
            match ChunkStorage::from_bytes(&bytes) {
                Ok(storage) => {
                    let arc = chunk_store.insert_with_revision(
                        chunk_pos,
                        storage.clone(),
                        persist_revision,
                    );
                    chunk_events.send(ChunkPayloadReady {
                        position: chunk_pos,
                        revision: persist_revision,
                        storage: arc,
                    });
                    spawn_storage = Some(storage);
                    revision = persist_revision;
                }
                Err(error) => {
                    warn!(
                        "Failed to decode persisted chunk {:?}: {:?}. Falling back to regeneration.",
                        chunk_pos, error
                    );
                }
            }
        }

        if let Some(storage) = spawn_storage {
            let world_pos = chunk_pos.to_world_pos();
            commands.spawn((
                Chunk::from_storage(chunk_pos, storage),
                chunk_pos,
                TransformBundle::from_transform(Transform::from_translation(world_pos)),
                VisibilityBundle::default(),
            ));

            chunk_manager.loaded_chunks.insert(chunk_pos);
            chunk_queue.mark_completed(&chunk_pos);
            loading_progress.chunks_generated += 1;
            debug!(
                "Loaded persisted chunk {:?} (revision {})",
                chunk_pos, revision
            );
        } else {
            remaining.push((chunk_pos, priority));
        }
    }

    for (chunk_pos, priority) in remaining {
        chunk_queue.requeue(chunk_pos, priority);
    }
}

/// System to generate initial spawn area chunks
pub fn generate_initial_chunks(
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    mut loading_progress: ResMut<LoadingProgress>,
) {
    // Only queue chunks once at the beginning
    if chunk_queue.pending_len() == 0
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
                    let _ = chunk_queue.enqueue_with_priority(
                        ChunkPos::new(
                            spawn_chunk_x + x,
                            2 + y, // Start at chunk Y=2 (blocks 64-95)
                            spawn_chunk_z + z,
                        ),
                        0,
                    );
                }
            }
        }
        loading_progress.total_chunks = chunk_queue.pending_len() as u32;
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
    time: Res<Time>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    let initial_tasks = chunk_queue.tasks.len();
    let backlog = chunk_queue.pending_len() + chunk_queue.tasks.len();
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let cpu_budget = cpu_cores.saturating_sub(1).max(MAX_CONCURRENT_CHUNK_TASKS);
    let concurrency_limit = if backlog >= 240 {
        cpu_budget + 2
    } else if backlog >= 160 {
        cpu_budget + 1
    } else if backlog >= 96 {
        cpu_budget
    } else if backlog >= 48 {
        cpu_budget.max(MAX_CONCURRENT_CHUNK_TASKS + 2)
    } else if backlog >= 24 {
        cpu_budget.max(MAX_CONCURRENT_CHUNK_TASKS + 1)
    } else {
        MAX_CONCURRENT_CHUNK_TASKS
    };

    while chunk_queue.tasks.len() < concurrency_limit {
        let Some((chunk_pos, _priority)) = chunk_queue.pop_pending() else {
            break;
        };

        let world_gen = world_gen.clone();
        let task = task_pool.spawn(async move {
            // Bake chunk data in background thread
            world_gen.bake_chunk(chunk_pos)
        });
        chunk_queue.tasks.push((chunk_pos, task));
        chunk_queue.record_start(chunk_pos, time.elapsed_seconds());
    }

    let spawned = chunk_queue.tasks.len() - initial_tasks;
    if spawned > 0 {
        info!(
            "chunk-gen spawn: spawned={} active={} pending={} queued_scheduled={}",
            spawned,
            chunk_queue.tasks.len(),
            chunk_queue.pending_len(),
            chunk_queue.scheduled.len()
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
    mut chunk_store: ResMut<PlanetChunkStore>,
    mut chunk_events: EventWriter<ChunkPayloadReady>,
    time: Res<Time>,
) {
    let mut completed_indices = Vec::new();
    let mut completed_positions = Vec::new();
    let mut completion_records = Vec::new();

    // Check for completed tasks
    for (index, (chunk_pos, task)) in chunk_queue.tasks.iter_mut().enumerate() {
        if let Some(storage) = future::block_on(future::poll_once(task)) {
            let world_pos = chunk_pos.to_world_pos();

            if let StoreUpdate::Updated {
                storage: storage_arc,
                revision,
            } = chunk_store.upsert_storage(*chunk_pos, &storage)
            {
                chunk_events.send(ChunkPayloadReady {
                    position: *chunk_pos,
                    revision,
                    storage: storage_arc,
                });
            }

            let chunk = Chunk::from_storage(*chunk_pos, storage);

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
            completed_positions.push(*chunk_pos);
            completion_records.push((*chunk_pos, time.elapsed_seconds()));

            debug!(
                "Completed chunk {:?}. Progress: {}/{}",
                chunk_pos, loading_progress.chunks_generated, loading_progress.total_chunks
            );
        }
    }

    // Remove scheduled markers for completed chunks
    for position in &completed_positions {
        chunk_queue.mark_completed(position);
    }

    for (position, end_time) in completion_records {
        chunk_queue.record_duration(&position, end_time);
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
            let (_pos, task) = chunk_queue.tasks.swap_remove(*index);
            task.detach();
        }
    }

    // Check if initial generation is complete
    if chunk_queue.pending_len() == 0
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
    mut chunk_queue: ResMut<ChunkGenerationQueue>,
    player_query: Query<&Transform, With<PlayerCamera>>,
    altitude_system: Res<AltitudeRenderSystem>,
    time: Res<Time>,
    world_gen: Res<WorldGenerator>,
    mut chunk_store: ResMut<PlanetChunkStore>,
    mut chunk_events: EventWriter<ChunkPayloadReady>,
    persistence: Option<Res<PersistenceHandler<DiskChunkPersistence>>>,
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

    let frame_start = Instant::now();

    let player_chunk = ChunkPos::from_world_pos(player_transform.translation);
    let view_distance = altitude_system.render_distance as i32;
    let forward: Vec3 = player_transform.forward().into();
    let player_pos = player_transform.translation;

    chunk_manager.candidate_state.ensure_candidates(
        player_chunk,
        view_distance,
        forward,
        player_pos,
        &world_gen,
    );

    let backlog = chunk_queue.pending_len() + chunk_queue.tasks.len();
    let remaining_candidates = chunk_manager.candidate_state.remaining_count();
    let backlog_estimate = backlog + remaining_candidates;
    let burst_multiplier = burst_multiplier(backlog_estimate);
    let budget_ms = BASE_DYNAMIC_ENQUEUE_BUDGET_MS * burst_multiplier as f32;
    let max_immediate = MAX_IMMEDIATE_CHUNKS_PER_FRAME * burst_multiplier;
    let max_queued = MAX_QUEUED_CHUNKS_PER_FRAME * burst_multiplier;

    let mut immediate_spawned = 0;
    let mut queued = 0;
    let mut budget_exhausted = false;
    let mut processed = 0;
    let max_iterations = (burst_multiplier.max(1) * 64) as usize;

    while let Some((chunk_pos, score)) = chunk_manager.candidate_state.next() {
        if processed >= max_iterations {
            break;
        }

        processed += 1;

        if frame_start.elapsed().as_secs_f32() * 1000.0 >= budget_ms {
            budget_exhausted = true;
            break;
        }

        if chunk_manager.loaded_chunks.contains(&chunk_pos) {
            continue;
        }

        if chunk_queue.contains(&chunk_pos) {
            continue;
        }

        if let Some((storage_arc, _revision)) = chunk_store.get_with_revision(&chunk_pos) {
            let storage = storage_arc.as_ref().clone();
            let world_pos = chunk_pos.to_world_pos();
            commands.spawn((
                Chunk::from_storage(chunk_pos, storage),
                chunk_pos,
                TransformBundle::from_transform(Transform::from_translation(world_pos)),
                VisibilityBundle::default(),
            ));
            chunk_manager.loaded_chunks.insert(chunk_pos);
            chunk_queue.mark_completed(&chunk_pos);
            immediate_spawned += 1;

            if immediate_spawned >= max_immediate && queued >= max_queued {
                break;
            }
            continue;
        }

        let mut spawned_from_persistence = false;
        if let Some(handler) = persistence.as_ref() {
            match handler.handler().load(chunk_pos) {
                Ok(Some((revision, bytes))) => match ChunkStorage::from_bytes(&bytes) {
                    Ok(storage) => {
                        let arc =
                            chunk_store.insert_with_revision(chunk_pos, storage.clone(), revision);
                        chunk_events.send(ChunkPayloadReady {
                            position: chunk_pos,
                            revision,
                            storage: arc,
                        });

                        let world_pos = chunk_pos.to_world_pos();
                        commands.spawn((
                            Chunk::from_storage(chunk_pos, storage),
                            chunk_pos,
                            TransformBundle::from_transform(Transform::from_translation(world_pos)),
                            VisibilityBundle::default(),
                        ));
                        chunk_manager.loaded_chunks.insert(chunk_pos);
                        chunk_queue.mark_completed(&chunk_pos);
                        immediate_spawned += 1;
                        spawned_from_persistence = true;
                    }
                    Err(error) => {
                        warn!(
                            "Failed to decode persisted chunk {:?}: {:?}. Queuing regeneration.",
                            chunk_pos, error
                        );
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    warn!(
                        "Failed to load persisted chunk {:?}: {}. Queuing regeneration.",
                        chunk_pos, error
                    );
                }
            }
        }

        if spawned_from_persistence {
            if immediate_spawned >= max_immediate && queued >= max_queued {
                break;
            }
            continue;
        }

        if chunk_queue.enqueue_with_priority(chunk_pos, score) {
            queued += 1;
            if immediate_spawned >= max_immediate && queued >= max_queued {
                break;
            }
        }
    }

    if budget_exhausted {
        info!(
            "chunk-load throttled: budget_ms={:.0} immediate={} queued={} pending={} backlog={} burst={}",
            budget_ms,
            immediate_spawned,
            queued,
            chunk_queue.pending_len(),
            backlog_estimate,
            burst_multiplier
        );
        return;
    }

    if immediate_spawned > 0 || queued > 0 {
        let elapsed_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        info!(
            "chunk-load enqueue: immediate={} queued={} pending={} tasks={} elapsed_ms={:.2} burst={}",
            immediate_spawned,
            queued,
            chunk_queue.pending_len(),
            chunk_queue.tasks.len(),
            elapsed_ms,
            burst_multiplier
        );
    }
}

pub fn sync_dirty_chunks_to_store(
    chunk_query: Query<(&Chunk, &ChunkPos)>,
    mut chunk_store: ResMut<PlanetChunkStore>,
    mut chunk_events: EventWriter<ChunkPayloadReady>,
) {
    for (chunk, chunk_pos) in chunk_query.iter() {
        if !chunk.dirty {
            continue;
        }

        if let StoreUpdate::Updated { storage, revision } =
            chunk_store.upsert_storage(*chunk_pos, &chunk.storage)
        {
            chunk_events.send(ChunkPayloadReady {
                position: *chunk_pos,
                revision,
                storage,
            });
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

pub fn collect_chunk_payloads(
    mut events: EventReader<ChunkPayloadReady>,
    mut queue: ResMut<ChunkPayloadQueue>,
) {
    let mut total_duration_ms = 0.0_f32;
    let mut total_bytes = 0_usize;
    let mut payload_count = 0_usize;

    for event in events.read() {
        let start = Instant::now();
        let bytes = event.storage.encode_bytes();
        let duration_ms = start.elapsed().as_secs_f32() * 1000.0;

        total_duration_ms += duration_ms;
        total_bytes += bytes.len();
        payload_count += 1;

        queue.enqueue(QueuedChunkPayload {
            position: event.position,
            revision: event.revision,
            bytes,
        });
    }

    if payload_count > 0 {
        let average_ms = total_duration_ms / payload_count as f32;
        info!(
            "chunk-payload encode: count={} total_ms={:.2} avg_ms={:.2} total_bytes={}",
            payload_count, total_duration_ms, average_ms, total_bytes
        );
    }
}

pub fn log_chunk_streaming_metrics(
    time: Res<Time>,
    chunk_queue: Res<ChunkGenerationQueue>,
    mesh_jobs: Res<ChunkMeshJobs>,
    mut accumulator: Local<f32>,
) {
    *accumulator += time.delta_seconds();
    if *accumulator < 1.0 {
        return;
    }

    *accumulator = 0.0;

    let pending = chunk_queue.pending_len();
    let active_tasks = chunk_queue.tasks.len();
    let scheduled = chunk_queue.scheduled.len();
    let mesh_tasks = mesh_jobs.task_count();
    let mesh_scheduled = mesh_jobs.scheduled_count();
    let avg_mesh_ms = mesh_jobs.average_duration_ms().unwrap_or(0.0);
    let avg_gen_ms = chunk_queue.average_duration_ms().unwrap_or(0.0);
    let inflight = chunk_queue.inflight_count();

    if pending > 0 || active_tasks > 0 || mesh_tasks > 0 {
        info!(
            "chunk-stream: pending={} active_tasks={} inflight={} scheduled={} gen_avg_ms={:.2} mesh_tasks={} mesh_scheduled={} mesh_avg_ms={:.2}",
            pending,
            active_tasks,
            inflight,
            scheduled,
            avg_gen_ms,
            mesh_tasks,
            mesh_scheduled,
            avg_mesh_ms,
        );
    }
}
