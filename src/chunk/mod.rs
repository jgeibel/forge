use crate::loading::GameState;
use bevy::prelude::*;

pub mod data;
pub mod manager;
pub mod mesh;

pub use data::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
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
                    manager::spawn_chunk_tasks,
                    manager::poll_chunk_tasks,
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
                    mesh::update_chunk_meshes,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
