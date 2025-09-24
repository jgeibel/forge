use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;

use crate::chunk::{ChunkPos, ChunkStorage};

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn compute_storage_hash(storage: &ChunkStorage) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for block in storage.iter() {
        hash ^= block.to_u8() as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Event, Clone)]
pub struct ChunkPayloadReady {
    pub position: ChunkPos,
    pub revision: u32,
    pub storage: Arc<ChunkStorage>,
}

#[derive(Clone)]
struct ChunkRecord {
    storage: Arc<ChunkStorage>,
    hash: u64,
    revision: u32,
}

#[derive(Debug)]
pub enum StoreUpdate {
    Updated {
        storage: Arc<ChunkStorage>,
        revision: u32,
    },
    Unchanged,
}

#[derive(Resource, Default)]
pub struct PlanetChunkStore {
    records: HashMap<ChunkPos, ChunkRecord>,
    dirty: HashSet<ChunkPos>,
}

impl PlanetChunkStore {
    pub fn get(&self, position: &ChunkPos) -> Option<Arc<ChunkStorage>> {
        self.records
            .get(position)
            .map(|record| record.storage.clone())
    }

    pub fn get_with_revision(&self, position: &ChunkPos) -> Option<(Arc<ChunkStorage>, u32)> {
        self.records
            .get(position)
            .map(|record| (record.storage.clone(), record.revision))
    }

    pub fn insert_with_revision(
        &mut self,
        position: ChunkPos,
        storage: ChunkStorage,
        revision: u32,
    ) -> Arc<ChunkStorage> {
        let hash = compute_storage_hash(&storage);
        let storage_arc = Arc::new(storage);
        self.records.insert(
            position,
            ChunkRecord {
                storage: storage_arc.clone(),
                hash,
                revision: revision.max(1),
            },
        );
        storage_arc
    }

    pub fn upsert_storage(&mut self, position: ChunkPos, storage: &ChunkStorage) -> StoreUpdate {
        let hash = compute_storage_hash(storage);
        match self.records.entry(position) {
            Entry::Occupied(mut entry) => {
                let record = entry.get_mut();
                if record.hash == hash {
                    return StoreUpdate::Unchanged;
                }

                record.storage = Arc::new(storage.clone());
                record.hash = hash;
                record.revision = record.revision.wrapping_add(1).max(1);
                StoreUpdate::Updated {
                    storage: record.storage.clone(),
                    revision: record.revision,
                }
            }
            Entry::Vacant(entry) => {
                let storage_arc = Arc::new(storage.clone());
                let record = ChunkRecord {
                    storage: storage_arc.clone(),
                    hash,
                    revision: 1,
                };
                entry.insert(record);
                StoreUpdate::Updated {
                    storage: storage_arc,
                    revision: 1,
                }
            }
        }
    }

    pub fn mark_dirty(&mut self, position: ChunkPos) {
        self.dirty.insert(position);
    }

    pub fn mark_clean(&mut self, position: &ChunkPos) {
        self.dirty.remove(position);
    }

    pub fn take_dirty(&mut self) -> Vec<ChunkPos> {
        let dirty = self.dirty.iter().copied().collect::<Vec<_>>();
        self.dirty.clear();
        dirty
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    pub fn encode_payload_bytes(&self, position: &ChunkPos) -> Option<Vec<u8>> {
        self.records
            .get(position)
            .map(|record| record.storage.encode_bytes())
    }

    pub fn contains(&self, position: &ChunkPos) -> bool {
        self.records.contains_key(position)
    }
}

#[derive(Debug, Clone)]
pub struct QueuedChunkPayload {
    pub position: ChunkPos,
    pub revision: u32,
    pub bytes: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct ChunkPayloadQueue {
    pending: Vec<QueuedChunkPayload>,
}

impl ChunkPayloadQueue {
    pub fn enqueue(&mut self, payload: QueuedChunkPayload) {
        self.pending.push(payload);
    }

    pub fn take_all(&mut self) -> Vec<QueuedChunkPayload> {
        std::mem::take(&mut self.pending)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[derive(Resource, Clone, Debug)]
pub struct PayloadDebugConfig {
    pub output_dir: PathBuf,
}

pub fn flush_queue_to_disk(
    queue: &mut ChunkPayloadQueue,
    output_dir: &Path,
) -> io::Result<Vec<PathBuf>> {
    if queue.is_empty() {
        return Ok(Vec::new());
    }

    create_dir_all(output_dir)?;

    let mut written = Vec::new();
    for payload in queue.take_all() {
        let filename = format!(
            "chunk_{}_{}_{}_rev{}.bin",
            payload.position.x, payload.position.y, payload.position.z, payload.revision
        );
        let path = output_dir.join(filename);
        let mut file = File::create(&path)?;
        file.write_all(&payload.bytes)?;
        written.push(path);
    }

    Ok(written)
}

pub fn snapshot_queue_to_disk(
    queue: &ChunkPayloadQueue,
    output_dir: &Path,
) -> io::Result<Vec<PathBuf>> {
    if queue.is_empty() {
        return Ok(Vec::new());
    }

    create_dir_all(output_dir)?;

    let mut written = Vec::new();
    for payload in &queue.pending {
        let filename = format!(
            "chunk_{}_{}_{}_rev{}.bin",
            payload.position.x, payload.position.y, payload.position.z, payload.revision
        );
        let path = output_dir.join(filename);
        if path.exists() {
            continue;
        }

        let mut file = File::create(&path)?;
        file.write_all(&payload.bytes)?;
        written.push(path);
    }

    Ok(written)
}

fn setup_payload_debug(mut commands: Commands) {
    if let Ok(dir) = std::env::var("FORGE_DEBUG_CHUNK_PAYLOADS_DIR") {
        if !dir.trim().is_empty() {
            commands.insert_resource(PayloadDebugConfig {
                output_dir: PathBuf::from(dir.trim()),
            });
        }
    }
}

pub fn flush_chunk_payload_queue(
    queue: Res<ChunkPayloadQueue>,
    config: Option<Res<PayloadDebugConfig>>,
) {
    let Some(config) = config else {
        // No debug capture configured; nothing to do.
        return;
    };

    if let Err(error) = snapshot_queue_to_disk(queue.as_ref(), &config.output_dir) {
        warn!(
            "Failed to flush chunk payloads to {:?}: {}",
            config.output_dir, error
        );
    }
}

pub struct PayloadDebugPlugin;

impl Plugin for PayloadDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_payload_debug)
            .add_systems(Update, flush_chunk_payload_queue);
    }
}
