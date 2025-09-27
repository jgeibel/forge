use std::path::{Path, PathBuf};

use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use super::chunk_store::{ChunkPayloadQueue, QueuedChunkPayload};
use crate::chunk::ChunkPos;

pub fn chunk_filename(position: &ChunkPos, revision: u32) -> String {
    format!(
        "chunk_{}_{}_{}_rev{}.bin",
        position.x, position.y, position.z, revision
    )
}

pub trait ChunkPersistence: Send + Sync + 'static {
    fn persist(&mut self, payload: &QueuedChunkPayload) -> io::Result<()>;
    fn load(&self, position: ChunkPos) -> io::Result<Option<(u32, Vec<u8>)>>;
}

#[derive(Clone)]
pub struct DiskChunkPersistence {
    root: PathBuf,
    index: Arc<RwLock<HashMap<ChunkPos, (u32, PathBuf)>>>,
}

impl DiskChunkPersistence {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let root_path = root.as_ref().to_path_buf();
        let index = Arc::new(RwLock::new(HashMap::new()));

        if let Ok(entries) = std::fs::read_dir(&root_path) {
            if let Ok(mut map) = index.write() {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    if let Some((position, revision)) = parse_chunk_filename(&entry.file_name()) {
                        match map.get(&position) {
                            Some((current_rev, _)) if *current_rev >= revision => {}
                            _ => {
                                map.insert(position, (revision, path));
                            }
                        }
                    }
                }
            }
        }

        Self {
            root: root_path,
            index,
        }
    }

    fn refresh_chunk_entry(&self, position: ChunkPos) -> io::Result<Option<(u32, PathBuf)>> {
        if !self.root.exists() {
            return Ok(None);
        }

        let mut latest: Option<(u32, PathBuf)> = None;
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some((pos, revision)) = parse_chunk_filename(&entry.file_name()) {
                if pos != position {
                    continue;
                }

                match latest {
                    Some((current_rev, _)) if current_rev >= revision => {}
                    _ => {
                        latest = Some((revision, path));
                    }
                }
            }
        }

        if let Some((revision, path)) = latest {
            if let Ok(mut map) = self.index.write() {
                map.insert(position, (revision, path.clone()));
            }
            return Ok(Some((revision, path)));
        }

        Ok(None)
    }
}

fn parse_chunk_filename(filename: &std::ffi::OsStr) -> Option<(ChunkPos, u32)> {
    let name = filename.to_str()?;
    if !name.starts_with("chunk_") || !name.ends_with(".bin") {
        return None;
    }

    let trimmed = &name[6..name.len() - 4];
    let mut parts = trimmed.split('_');

    let x = parts.next()?.parse::<i32>().ok()?;
    let y = parts.next()?.parse::<i32>().ok()?;
    let z = parts.next()?.parse::<i32>().ok()?;
    let rev_part = parts.next()?;

    if parts.next().is_some() {
        return None;
    }

    let revision = rev_part.strip_prefix("rev")?.parse::<u32>().ok()?;
    Some((ChunkPos::new(x, y, z), revision))
}

impl ChunkPersistence for DiskChunkPersistence {
    fn persist(&mut self, payload: &QueuedChunkPayload) -> io::Result<()> {
        create_dir_all(&self.root)?;
        let path = self
            .root
            .join(chunk_filename(&payload.position, payload.revision));
        let mut file = File::create(&path)?;
        file.write_all(&payload.bytes)?;

        if let Ok(mut map) = self.index.write() {
            match map.get(&payload.position) {
                Some((current_rev, _)) if *current_rev >= payload.revision => {}
                _ => {
                    map.insert(payload.position, (payload.revision, path));
                }
            }
        }

        Ok(())
    }

    fn load(&self, position: ChunkPos) -> io::Result<Option<(u32, Vec<u8>)>> {
        if !self.root.exists() {
            return Ok(None);
        }

        let mut entry = {
            let map = self.index.read().expect("chunk index poisoned");
            map.get(&position).cloned()
        };

        if entry.is_none() {
            entry = self.refresh_chunk_entry(position)?;
        }

        if let Some((revision, path)) = entry {
            let bytes = std::fs::read(path)?;
            return Ok(Some((revision, bytes)));
        }

        Ok(None)
    }
}

#[derive(Resource, Clone)]
pub struct PersistenceConfig {
    pub enabled: bool,
}

#[derive(Resource)]
pub struct PersistenceHandler<T: ChunkPersistence> {
    handler: T,
}

impl<T: ChunkPersistence> PersistenceHandler<T> {
    pub fn new(handler: T) -> Self {
        Self { handler }
    }

    pub fn handler_mut(&mut self) -> &mut T {
        &mut self.handler
    }

    pub fn handler(&self) -> &T {
        &self.handler
    }
}

pub fn flush_queue_to_persistence<T: ChunkPersistence>(
    mut queue: ResMut<ChunkPayloadQueue>,
    mut handler: ResMut<PersistenceHandler<T>>,
    config: Option<Res<PersistenceConfig>>,
) {
    if let Some(config) = config {
        if !config.enabled {
            queue.take_all();
            return;
        }
    }

    let payloads = queue.take_all();
    if payloads.is_empty() {
        return;
    }

    let start = Instant::now();
    let mut total_bytes = 0_usize;
    let mut processed = 0_usize;
    let mut failures = 0_usize;

    for payload in payloads {
        total_bytes += payload.bytes.len();
        processed += 1;

        if let Err(error) = handler.handler_mut().persist(&payload) {
            failures += 1;
            warn!(
                "Failed to persist chunk payload {:?} rev {}: {}",
                payload.position, payload.revision, error
            );
        }
    }

    let duration_ms = start.elapsed().as_secs_f32() * 1000.0;
    info!(
        "chunk-payload persist: count={} total_ms={:.2} avg_ms={:.2} total_bytes={} failures={}",
        processed,
        duration_ms,
        duration_ms / processed.max(1) as f32,
        total_bytes,
        failures
    );
}

pub struct ChunkPersistencePlugin<T: ChunkPersistence + Clone> {
    handler: T,
    config: PersistenceConfig,
}

impl<T: ChunkPersistence + Clone> ChunkPersistencePlugin<T> {
    pub fn new(handler: T, config: PersistenceConfig) -> Self {
        Self { handler, config }
    }
}

impl<T: ChunkPersistence + Clone> Plugin for ChunkPersistencePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(PersistenceHandler::new(self.handler.clone()))
            .insert_resource(self.config.clone())
            .add_systems(Update, flush_queue_to_persistence::<T>);
    }
}
