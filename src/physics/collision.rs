use crate::chunk::{Chunk, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_F32};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl AABB {
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self {
            center,
            half_extents: size / 2.0,
        }
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.half_extents
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        let self_min = self.min();
        let self_max = self.max();
        let other_min = other.min();
        let other_max = other.max();

        self_min.x <= other_max.x
            && self_max.x >= other_min.x
            && self_min.y <= other_max.y
            && self_max.y >= other_min.y
            && self_min.z <= other_max.z
            && self_max.z >= other_min.z
    }

    pub fn intersects_block(&self, block_pos: IVec3) -> bool {
        let block_aabb = AABB::new(block_pos.as_vec3() + Vec3::splat(0.5), Vec3::ONE);
        self.intersects(&block_aabb)
    }
}

pub fn check_collision_with_world(aabb: &AABB, chunk_query: &Query<(&Chunk, &ChunkPos)>) -> bool {
    let min = aabb.min();
    let max = aabb.max();

    // Check all block positions that the AABB could be touching
    let min_block = IVec3::new(
        min.x.floor() as i32,
        min.y.floor() as i32,
        min.z.floor() as i32,
    );
    let max_block = IVec3::new(
        max.x.floor() as i32,
        max.y.floor() as i32,
        max.z.floor() as i32,
    );

    // Removed debug logging for performance

    for y in min_block.y..=max_block.y {
        for x in min_block.x..=max_block.x {
            for z in min_block.z..=max_block.z {
                let block_pos = IVec3::new(x, y, z);

                // Check if AABB intersects with this block position
                if aabb.intersects_block(block_pos) {
                    // Get the block at this position
                    if let Some(block) = get_block_at(block_pos, chunk_query) {
                        if block.is_solid() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

fn get_block_at(
    block_pos: IVec3,
    chunk_query: &Query<(&Chunk, &ChunkPos)>,
) -> Option<crate::block::BlockType> {
    // Handle negative Y (below world)
    if block_pos.y < 0 {
        // Treat everything below Y=0 as solid bedrock
        return Some(crate::block::BlockType::Bedrock);
    }

    let chunk_pos = ChunkPos::new(
        (block_pos.x as f32 / CHUNK_SIZE_F32).floor() as i32,
        (block_pos.y as f32 / CHUNK_SIZE_F32).floor() as i32,
        (block_pos.z as f32 / CHUNK_SIZE_F32).floor() as i32,
    );

    for (chunk, pos) in chunk_query.iter() {
        if *pos == chunk_pos {
            let local_x = block_pos.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_y = block_pos.y.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = block_pos.z.rem_euclid(CHUNK_SIZE as i32) as usize;

            return Some(chunk.get_block(local_x, local_y, local_z));
        }
    }

    None
}
