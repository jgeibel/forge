use forge::chunk::{Chunk, ChunkPos};
use forge::world::{WorldGenConfig, WorldGenerator};

fn main() {
    let config = WorldGenConfig::default();
    let generator = WorldGenerator::new(config);
    let origin = ChunkPos::new(0, 0, 0);
    let chunk = Chunk::generate_with_world_gen(origin, &generator);

    let mut counts = std::collections::HashMap::new();
    for x in 0..forge::chunk::CHUNK_SIZE {
        for y in 0..forge::chunk::CHUNK_SIZE {
            for z in 0..forge::chunk::CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);
                *counts.entry(block).or_insert(0) += 1;
            }
        }
    }

    println!("Block histogram for chunk {:?}:", origin);
    let mut entries: Vec<_> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    for (block, count) in entries {
        println!("  {:?}: {}", block, count);
    }
}
