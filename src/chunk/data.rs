use bevy::prelude::*;
use crate::block::BlockType;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;

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
    pub blocks: Box<[[[BlockType; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]>,
    pub position: ChunkPos,
    pub dirty: bool,
}

impl Chunk {
    pub fn new(position: ChunkPos) -> Self {
        Self {
            blocks: Box::new([[[BlockType::Air; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]),
            position,
            dirty: true,
        }
    }
    
    pub fn new_filled(position: ChunkPos, block_type: BlockType) -> Self {
        Self {
            blocks: Box::new([[[block_type; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]),
            position,
            dirty: true,
        }
    }
    
    pub fn generate_terrain(position: ChunkPos) -> Self {
        let mut chunk = Self::new(position);
        let world_offset = position.to_world_pos();
        
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = world_offset.x + x as f32;
                let world_z = world_offset.z + z as f32;
                
                // Use wrapped coordinates for seamless terrain
                let wrapped_x = world_x.rem_euclid(2048.0);  // Planet size
                let wrapped_z = world_z.rem_euclid(2048.0);
                
                // Generate rolling hills with sine waves
                let height = 16.0 
                    + (wrapped_x * 0.05).sin() * 8.0 
                    + (wrapped_z * 0.05).cos() * 8.0
                    + (wrapped_x * 0.01).sin() * 16.0;
                
                // Create landmarks at specific positions
                let is_origin = wrapped_x < 32.0 && wrapped_z < 32.0;
                let is_pillar = (wrapped_x as i32 % 256 == 0 && wrapped_z as i32 % 256 == 0);
                
                for y in 0..CHUNK_SIZE {
                    let world_y = world_offset.y + y as f32;
                    
                    // Add bedrock at the bottom
                    chunk.blocks[x][y][z] = if world_y < 3.0 {
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
                            BlockType::Stone  // Stone surface at origin
                        } else {
                            BlockType::Grass
                        }
                    } else {
                        BlockType::Air
                    };
                }
            }
        }
        
        chunk
    }
    
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.blocks[x][y][z]
    }
    
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_type: BlockType) {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            self.blocks[x][y][z] = block_type;
            self.dirty = true;
        }
    }
    
    pub fn is_block_visible(&self, x: usize, y: usize, z: usize) -> bool {
        let block = self.get_block(x, y, z);
        if !block.is_visible() {
            return false;
        }
        
        if x == 0 || !self.get_block(x - 1, y, z).is_solid() { return true; }
        if x == CHUNK_SIZE - 1 || !self.get_block(x + 1, y, z).is_solid() { return true; }
        if y == 0 || !self.get_block(x, y - 1, z).is_solid() { return true; }
        if y == CHUNK_SIZE - 1 || !self.get_block(x, y + 1, z).is_solid() { return true; }
        if z == 0 || !self.get_block(x, y, z - 1).is_solid() { return true; }
        if z == CHUNK_SIZE - 1 || !self.get_block(x, y, z + 1).is_solid() { return true; }
        
        false
    }
}