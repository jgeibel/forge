use crate::block::BlockType;
use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;

const CHUNK_PAYLOAD_MAGIC: [u8; 4] = *b"FBCH";
pub const CHUNK_PAYLOAD_VERSION: u8 = 1;

#[derive(Debug)]
pub enum ChunkPayloadError {
    InvalidMagic,
    UnsupportedVersion(u8),
    UnexpectedEof,
    PaletteTooLarge(usize),
    UnknownBlock(u8),
    PaletteIndexOutOfBounds(u16),
    RunOverflow,
    LengthMismatch { expected: usize, actual: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct VoxelRun {
    pub palette_index: u16,
    pub length: u16,
}

#[derive(Clone, Debug)]
pub struct ChunkPayload {
    pub version: u8,
    pub palette: Vec<BlockType>,
    pub runs: Vec<VoxelRun>,
}

/// Compact storage for chunk voxel data in X-major linear order.
#[derive(Clone, Debug)]
pub struct ChunkStorage {
    voxels: Box<[BlockType; CHUNK_VOLUME]>,
}

impl ChunkStorage {
    pub fn new() -> Self {
        Self::filled(BlockType::Air)
    }

    pub fn filled(block_type: BlockType) -> Self {
        Self {
            voxels: Box::new([block_type; CHUNK_VOLUME]),
        }
    }

    pub fn from_fn<F>(mut f: F) -> Self
    where
        F: FnMut(usize, usize, usize) -> BlockType,
    {
        let mut voxels = Box::new([BlockType::Air; CHUNK_VOLUME]);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let idx = Self::linear_index(x, y, z);
                    voxels[idx] = f(x, y, z);
                }
            }
        }
        Self { voxels }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockType {
        let idx = Self::linear_index(x, y, z);
        self.voxels[idx]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block_type: BlockType) {
        let idx = Self::linear_index(x, y, z);
        self.voxels[idx] = block_type;
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = BlockType> + '_ {
        self.voxels.iter().copied()
    }

    #[inline]
    pub fn as_slice(&self) -> &[BlockType; CHUNK_VOLUME] {
        &self.voxels
    }

    #[inline]
    pub fn linear_index(x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        x + CHUNK_SIZE * (y + CHUNK_SIZE * z)
    }

    pub fn encode_payload(&self) -> ChunkPayload {
        use std::convert::TryFrom;

        let mut palette = Vec::new();
        let mut palette_lookup: HashMap<BlockType, u16> = HashMap::new();
        let mut runs: Vec<VoxelRun> = Vec::new();

        let mut current_index: Option<u16> = None;
        let mut current_length: u16 = 0;

        for block in self.iter() {
            let palette_index = if let Some(&index) = palette_lookup.get(&block) {
                index
            } else {
                let index = u16::try_from(palette.len()).expect("palette exceeds u16 range");
                palette.push(block);
                palette_lookup.insert(block, index);
                index
            };

            match current_index {
                Some(idx) if idx == palette_index && current_length < u16::MAX => {
                    current_length = current_length.saturating_add(1);
                }
                Some(idx) => {
                    runs.push(VoxelRun {
                        palette_index: idx,
                        length: current_length,
                    });
                    current_index = Some(palette_index);
                    current_length = 1;
                }
                None => {
                    current_index = Some(palette_index);
                    current_length = 1;
                }
            }
        }

        if let Some(idx) = current_index {
            runs.push(VoxelRun {
                palette_index: idx,
                length: current_length.max(1),
            });
        }

        ChunkPayload {
            version: CHUNK_PAYLOAD_VERSION,
            palette,
            runs,
        }
    }

    pub fn encode_bytes(&self) -> Vec<u8> {
        self.encode_payload().to_bytes()
    }

    pub fn from_payload(payload: &ChunkPayload) -> Result<Self, ChunkPayloadError> {
        if payload.version != CHUNK_PAYLOAD_VERSION {
            return Err(ChunkPayloadError::UnsupportedVersion(payload.version));
        }

        if payload.palette.len() > u16::MAX as usize {
            return Err(ChunkPayloadError::PaletteTooLarge(payload.palette.len()));
        }

        let mut voxels = Box::new([BlockType::Air; CHUNK_VOLUME]);
        let mut offset = 0usize;

        for run in &payload.runs {
            let palette_index = run.palette_index as usize;
            let block = payload.palette.get(palette_index).ok_or(
                ChunkPayloadError::PaletteIndexOutOfBounds(run.palette_index),
            )?;
            let length = run.length as usize;

            if length == 0 {
                continue;
            }

            if offset + length > CHUNK_VOLUME {
                return Err(ChunkPayloadError::RunOverflow);
            }

            for idx in offset..offset + length {
                voxels[idx] = *block;
            }
            offset += length;
        }

        if offset != CHUNK_VOLUME {
            return Err(ChunkPayloadError::LengthMismatch {
                expected: CHUNK_VOLUME,
                actual: offset,
            });
        }

        Ok(Self { voxels })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChunkPayloadError> {
        let payload = ChunkPayload::from_bytes(bytes)?;
        Self::from_payload(&payload)
    }
}

impl ChunkPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        use std::convert::TryFrom;

        let mut bytes = Vec::with_capacity(
            4 + 1 + 2 + self.palette.len() + 4 + self.runs.len() * std::mem::size_of::<VoxelRun>(),
        );
        bytes.extend_from_slice(&CHUNK_PAYLOAD_MAGIC);
        bytes.push(self.version);

        let palette_len = u16::try_from(self.palette.len()).expect("palette exceeds u16 range");
        bytes.extend_from_slice(&palette_len.to_le_bytes());
        for block in &self.palette {
            bytes.push(block.to_u8());
        }

        let run_len = u32::try_from(self.runs.len()).expect("run count exceeds u32 range");
        bytes.extend_from_slice(&run_len.to_le_bytes());
        for run in &self.runs {
            bytes.extend_from_slice(&run.palette_index.to_le_bytes());
            bytes.extend_from_slice(&run.length.to_le_bytes());
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChunkPayloadError> {
        let mut cursor = 0usize;

        if bytes.len() < 7 {
            return Err(ChunkPayloadError::UnexpectedEof);
        }

        if bytes[..4] != CHUNK_PAYLOAD_MAGIC {
            return Err(ChunkPayloadError::InvalidMagic);
        }
        cursor += 4;

        let version = bytes[cursor];
        cursor += 1;

        if cursor + 2 > bytes.len() {
            return Err(ChunkPayloadError::UnexpectedEof);
        }
        let palette_len = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
        cursor += 2;

        if cursor + palette_len > bytes.len() {
            return Err(ChunkPayloadError::UnexpectedEof);
        }

        let mut palette = Vec::with_capacity(palette_len);
        for _ in 0..palette_len {
            let block_byte = bytes[cursor];
            cursor += 1;
            let block = BlockType::from_u8(block_byte)
                .ok_or(ChunkPayloadError::UnknownBlock(block_byte))?;
            palette.push(block);
        }

        if cursor + 4 > bytes.len() {
            return Err(ChunkPayloadError::UnexpectedEof);
        }
        let run_len = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        cursor += 4;

        let mut runs = Vec::with_capacity(run_len);
        for _ in 0..run_len {
            if cursor + 4 > bytes.len() {
                return Err(ChunkPayloadError::UnexpectedEof);
            }
            let palette_index = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            let length = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
            cursor += 4;
            runs.push(VoxelRun {
                palette_index,
                length,
            });
        }

        Ok(ChunkPayload {
            version,
            palette,
            runs,
        })
    }

    pub fn total_voxels(&self) -> usize {
        self.runs
            .iter()
            .map(|run| run.length as usize)
            .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_roundtrip_preserves_voxels() {
        let mut storage = ChunkStorage::new();
        storage.set(0, 0, 0, BlockType::Stone);
        storage.set(1, 0, 0, BlockType::Stone);
        storage.set(0, 1, 0, BlockType::Grass);
        storage.set(10, 10, 10, BlockType::Water);

        let payload = storage.encode_payload();
        assert_eq!(payload.version, CHUNK_PAYLOAD_VERSION);
        assert_eq!(payload.total_voxels(), CHUNK_VOLUME);

        let decoded = ChunkStorage::from_payload(&payload).expect("decode payload");
        assert_eq!(decoded.get(0, 0, 0), BlockType::Stone);
        assert_eq!(decoded.get(1, 0, 0), BlockType::Stone);
        assert_eq!(decoded.get(0, 1, 0), BlockType::Grass);
        assert_eq!(decoded.get(10, 10, 10), BlockType::Water);
        assert_eq!(decoded.get(31, 31, 31), BlockType::Air);
    }

    #[test]
    fn payload_bytes_roundtrip() {
        let mut storage = ChunkStorage::filled(BlockType::Dirt);
        storage.set(0, 0, 0, BlockType::Bedrock);
        storage.set(31, 31, 31, BlockType::Grass);

        let bytes = storage.encode_bytes();
        let decoded = ChunkStorage::from_bytes(&bytes).expect("decode bytes");
        assert_eq!(decoded.get(0, 0, 0), BlockType::Bedrock);
        assert_eq!(decoded.get(31, 31, 31), BlockType::Grass);
        assert_eq!(decoded.get(2, 2, 2), BlockType::Dirt);
    }
}

impl Default for ChunkStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world_pos(pos: Vec3) -> Self {
        Self {
            x: (pos.x / CHUNK_SIZE_F32).floor() as i32,
            y: (pos.y / CHUNK_SIZE_F32).floor() as i32,
            z: (pos.z / CHUNK_SIZE_F32).floor() as i32,
        }
    }

    pub fn to_world_pos(&self) -> Vec3 {
        Vec3::new(
            self.x as f32 * CHUNK_SIZE_F32,
            self.y as f32 * CHUNK_SIZE_F32,
            self.z as f32 * CHUNK_SIZE_F32,
        )
    }
}

#[derive(Component)]
pub struct Chunk {
    pub storage: ChunkStorage,
    pub position: ChunkPos,
    pub dirty: bool,
}

impl Chunk {
    pub fn new(position: ChunkPos) -> Self {
        Self {
            storage: ChunkStorage::new(),
            position,
            dirty: true,
        }
    }

    pub fn new_filled(position: ChunkPos, block_type: BlockType) -> Self {
        Self {
            storage: ChunkStorage::filled(block_type),
            position,
            dirty: true,
        }
    }

    pub fn from_storage(position: ChunkPos, storage: ChunkStorage) -> Self {
        Self {
            storage,
            position,
            dirty: true,
        }
    }

    pub fn generate_terrain(position: ChunkPos) -> Self {
        // This is now a simple placeholder - actual generation will use WorldGenerator
        let mut chunk = Self::new(position);
        let world_offset = position.to_world_pos();

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = world_offset.x + x as f32;
                let world_z = world_offset.z + z as f32;

                // Use wrapped coordinates for seamless terrain
                let wrapped_x = world_x.rem_euclid(2048.0); // Planet size
                let wrapped_z = world_z.rem_euclid(2048.0);

                // Generate rolling hills with sine waves
                let height = 16.0
                    + (wrapped_x * 0.05).sin() * 8.0
                    + (wrapped_z * 0.05).cos() * 8.0
                    + (wrapped_x * 0.01).sin() * 16.0;

                // Create landmarks at specific positions
                let is_origin = wrapped_x < 32.0 && wrapped_z < 32.0;
                let is_pillar = wrapped_x as i32 % 256 == 0 && wrapped_z as i32 % 256 == 0;

                for y in 0..CHUNK_SIZE {
                    let world_y = world_offset.y + y as f32;

                    // Add bedrock at the bottom
                    chunk.storage.set(
                        x,
                        y,
                        z,
                        if world_y < 3.0 {
                            BlockType::Bedrock
                        } else if is_pillar && world_y < 50.0 {
                            // Create tall stone pillars every 256 blocks
                            BlockType::Stone
                        } else if is_origin && world_y < height + 2.0 && world_y >= height - 1.0 {
                            // Mark origin area with stone platform
                            BlockType::Stone
                        } else if world_y < height - 3.0 {
                            BlockType::Stone
                        } else if world_y < height - 1.0 {
                            BlockType::Dirt
                        } else if world_y < height {
                            if is_origin {
                                BlockType::Stone // Stone surface at origin
                            } else {
                                BlockType::Grass
                            }
                        } else {
                            BlockType::Air
                        },
                    );
                }
            }
        }

        chunk
    }

    pub fn generate_with_world_gen(
        position: ChunkPos,
        world_gen: &crate::world::WorldGenerator,
    ) -> Self {
        let mut storage = world_gen.bake_chunk(position);
        apply_lithology_layers(position, &mut storage, world_gen);
        carve_caves_and_ores(position, &mut storage, world_gen);
        Self::from_storage(position, storage)
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.storage.get(x, y, z)
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_type: BlockType) {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            self.storage.set(x, y, z, block_type);
            self.dirty = true;
        }
    }

    pub fn is_block_visible(&self, x: usize, y: usize, z: usize) -> bool {
        let block = self.get_block(x, y, z);
        if !block.is_visible() {
            return false;
        }

        if x == 0 || !self.get_block(x - 1, y, z).is_solid() {
            return true;
        }
        if x == CHUNK_SIZE - 1 || !self.get_block(x + 1, y, z).is_solid() {
            return true;
        }
        if y == 0 || !self.get_block(x, y - 1, z).is_solid() {
            return true;
        }
        if y == CHUNK_SIZE - 1 || !self.get_block(x, y + 1, z).is_solid() {
            return true;
        }
        if z == 0 || !self.get_block(x, y, z - 1).is_solid() {
            return true;
        }
        if z == CHUNK_SIZE - 1 || !self.get_block(x, y, z + 1).is_solid() {
            return true;
        }

        false
    }
}

fn apply_lithology_layers(
    chunk_pos: ChunkPos,
    storage: &mut ChunkStorage,
    world_gen: &crate::world::WorldGenerator,
) {
    let world_origin = chunk_pos.to_world_pos();

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let profile = world_gen
                .lithology_profile_at(world_origin.x + x as f32, world_origin.z + z as f32);
            let mut remaining = profile.surface_depth.max(1) as i32;

            for y in (0..CHUNK_SIZE).rev() {
                let mut block = storage.get(x, y, z);

                if block == crate::block::BlockType::Air {
                    continue;
                }

                if remaining > 0 {
                    block = profile.surface_block;
                    remaining -= 1;
                } else {
                    let mut depth = profile.surface_depth as i32;
                    let mut replaced = false;
                    for layer in &profile.strata {
                        depth += layer.thickness as i32;
                        if world_origin.y as i32 + CHUNK_SIZE as i32 - (y as i32 + 1) < depth {
                            block = layer.block;
                            replaced = true;
                            break;
                        }
                    }

                    if !replaced {
                        block = profile.basement_block;
                    }
                }

                storage.set(x, y, z, block);
            }
        }
    }
}

fn carve_caves_and_ores(
    chunk_pos: ChunkPos,
    storage: &mut ChunkStorage,
    world_gen: &crate::world::WorldGenerator,
) {
    let world_origin = chunk_pos.to_world_pos();
    let seed = world_gen.config().seed;

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let profile = world_gen
                .lithology_profile_at(world_origin.x + x as f32, world_origin.z + z as f32);

            let column_seed = seed
                ^ ((world_origin.x as i64 + x as i64) as u64).wrapping_mul(0x9E3779B97F4A7C15)
                ^ ((world_origin.z as i64 + z as i64) as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
                ^ ((chunk_pos.y as i64) as u64).wrapping_mul(0x165667B19E3779F9);
            let mut rng = StdRng::seed_from_u64(column_seed);

            let cave_threshold = (profile.cave_bias * 0.015).clamp(0.0, 0.35);
            let ore_threshold = (profile.ore_bias * 0.01).clamp(0.0, 0.25);

            for y in (0..CHUNK_SIZE).rev() {
                let block = storage.get(x, y, z);
                if !block.is_solid() || matches!(block, BlockType::Bedrock) {
                    continue;
                }

                if y > 2 && rng.gen::<f32>() < cave_threshold {
                    storage.set(x, y, z, BlockType::Air);
                    continue;
                }

                if matches!(block, BlockType::Stone | BlockType::Cobblestone)
                    && rng.gen::<f32>() < ore_threshold
                {
                    storage.set(x, y, z, BlockType::Cobblestone);
                }
            }
        }
    }
}
