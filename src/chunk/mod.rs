use crate::loading::GameState;
use bevy::prelude::*;

pub mod data;
pub mod manager;
pub mod mesh;

#[allow(unused_imports)]
pub use data::{
    Chunk, ChunkPayload, ChunkPayloadError, ChunkPos, ChunkStorage, VoxelRun,
    CHUNK_PAYLOAD_VERSION, CHUNK_SIZE, CHUNK_SIZE_F32,
};
pub use manager::{ChunkGenerationQueue, ChunkManager};

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            .init_resource::<ChunkGenerationQueue>()
            // World generation systems during loading
            .add_systems(
                Update,
                (
                    manager::generate_initial_chunks,
                    manager::load_persisted_pending_chunks,
                    manager::spawn_chunk_tasks,
                    manager::poll_chunk_tasks,
                    manager::sync_dirty_chunks_to_store,
                    manager::collect_chunk_payloads,
                    mesh::update_chunk_meshes, // Also generate meshes during world generation
                )
                    .chain()
                    .run_if(in_state(GameState::GeneratingWorld)),
            )
            // Regular chunk management during gameplay
            .add_systems(
                Update,
                (
                    manager::spawn_chunks_around_player,
                    manager::despawn_far_chunks,
                    manager::sync_dirty_chunks_to_store,
                    manager::collect_chunk_payloads,
                    mesh::update_chunk_meshes,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
