use std::path::{Path, PathBuf};

use bevy::prelude::*;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};

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

#[derive(Default, Clone)]
pub struct DiskChunkPersistence {
    root: PathBuf,
}

impl DiskChunkPersistence {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl ChunkPersistence for DiskChunkPersistence {
    fn persist(&mut self, payload: &QueuedChunkPayload) -> io::Result<()> {
        create_dir_all(&self.root)?;
        let path = self
            .root
            .join(chunk_filename(&payload.position, payload.revision));
        let mut file = File::create(path)?;
        file.write_all(&payload.bytes)
    }

    fn load(&self, position: ChunkPos) -> io::Result<Option<(u32, Vec<u8>)>> {
        if !self.root.exists() {
            return Ok(None);
        }

        let pattern = format!("chunk_{}_{}_{}_rev", position.x, position.y, position.z);

        let mut latest: Option<(u32, PathBuf)> = None;
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            if !filename.starts_with(&pattern) {
                continue;
            }

            if let Some(rev_str) = filename.split("rev").nth(1) {
                if let Ok(revision) = rev_str.trim_end_matches(".bin").parse::<u32>() {
                    match latest {
                        Some((current, _)) if current >= revision => {}
                        _ => {
                            latest = Some((revision, entry.path()));
                        }
                    }
                }
            }
        }

        if let Some((revision, path)) = latest {
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

    for payload in queue.take_all() {
        if let Err(error) = handler.handler_mut().persist(&payload) {
            warn!(
                "Failed to persist chunk payload {:?} rev {}: {}",
                payload.position, payload.revision, error
            );
        }
    }
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
