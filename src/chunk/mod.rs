use bevy::prelude::*;

pub mod data;
pub mod manager;
pub mod mesh;

pub use data::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
pub use manager::ChunkManager;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChunkManager>()
            .add_systems(Update, (
                manager::spawn_chunks_around_player,
                manager::despawn_far_chunks,
                mesh::update_chunk_meshes,
            ).chain());
    }
}