use std::error::Error;
use std::path::PathBuf;

use forge::block::BlockType;
use forge::chunk::{ChunkPos, ChunkStorage};
use forge::world::chunk_store::{
    flush_queue_to_disk, ChunkPayloadQueue, PlanetChunkStore, QueuedChunkPayload, StoreUpdate,
};
use forge::world::persistence::{ChunkPersistence, DiskChunkPersistence};
use forge::world::{WorldGenConfig, WorldGenerator};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/chunk_payload_debug"));

    let generator = WorldGenerator::new(WorldGenConfig::default());
    let mut store = PlanetChunkStore::default();
    let mut queue = ChunkPayloadQueue::default();

    let chunk_pos = ChunkPos::new(0, 0, 0);
    let storage = generator.bake_chunk(chunk_pos);
    enqueue_if_updated(&mut store, &mut queue, chunk_pos, &storage);

    let mut edited = storage.clone();
    edited.set(0, 0, 0, BlockType::Bedrock);
    edited.set(1, 1, 1, BlockType::Water);
    enqueue_if_updated(&mut store, &mut queue, chunk_pos, &edited);

    let mut queue_for_debug = ChunkPayloadQueue::default();
    let mut queue_for_persist = ChunkPayloadQueue::default();

    for payload in queue.take_all() {
        queue_for_debug.enqueue(payload.clone());
        queue_for_persist.enqueue(payload);
    }

    let written = flush_queue_to_disk(&mut queue_for_debug, &output_dir)?;

    let persist_dir = std::env::var("FORGE_PERSISTENCE_DIR")
        .ok()
        .filter(|dir| !dir.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/chunk_payload_persistence"));

    let mut disk_persistence = DiskChunkPersistence::new(&persist_dir);
    for payload in queue_for_persist.take_all() {
        disk_persistence.persist(&payload)?;
    }

    println!(
        "Wrote {} chunk payload(s) to {}",
        written.len(),
        output_dir.display()
    );
    for path in written {
        println!(" - {}", path.display());
    }

    println!("Persisted chunk payload(s) to {}", persist_dir.display());

    Ok(())
}

fn enqueue_if_updated(
    store: &mut PlanetChunkStore,
    queue: &mut ChunkPayloadQueue,
    position: ChunkPos,
    storage: &ChunkStorage,
) {
    if let StoreUpdate::Updated { storage, revision } = store.upsert_storage(position, storage) {
        queue.enqueue(QueuedChunkPayload {
            position,
            revision,
            bytes: storage.encode_bytes(),
        });
    }
}
