# Phase 1: Core Voxel Engine

## Overview
The foundation phase where we build the core voxel rendering system, chunk management, and basic interaction mechanics. This phase establishes the technical base that all other features will build upon.

## Goals
- Efficient voxel rendering with 60+ FPS
- Chunk-based world management
- Block placement and removal
- Basic player movement and camera

## Technical Implementation

### 1. Project Structure
```
forge/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── world/
│   │   ├── mod.rs
│   │   ├── chunk.rs         # Chunk data structure
│   │   ├── block.rs         # Block types and properties
│   │   └── voxel.rs         # Voxel operations
│   ├── rendering/
│   │   ├── mod.rs
│   │   ├── mesh.rs          # Mesh generation
│   │   ├── greedy.rs        # Greedy meshing algorithm
│   │   └── shaders.rs       # Shader management
│   ├── player/
│   │   ├── mod.rs
│   │   ├── controller.rs    # Movement and input
│   │   └── camera.rs        # First-person camera
│   └── utils/
│       ├── mod.rs
│       └── math.rs          # Vector math helpers
├── assets/
│   ├── textures/
│   │   └── blocks.png       # Block texture atlas
│   └── shaders/
│       ├── voxel.vert       # Vertex shader
│       └── voxel.frag       # Fragment shader
└── Cargo.toml
```

### 2. Block System

```rust
// Block types (expandable)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockType {
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
    Wood = 4,
    Leaves = 5,
    Sand = 6,
    Water = 7,
    // ... more blocks
}

impl BlockType {
    pub fn is_solid(&self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water)
    }
    
    pub fn is_transparent(&self) -> bool {
        matches!(self, BlockType::Air | BlockType::Water | BlockType::Leaves)
    }
    
    pub fn get_texture_indices(&self) -> [u32; 6] {
        // [top, bottom, north, south, east, west]
        match self {
            BlockType::Grass => [0, 2, 1, 1, 1, 1],
            BlockType::Wood => [4, 4, 3, 3, 3, 3],
            // ...
        }
    }
}
```

### 3. Chunk Structure

```rust
pub const CHUNK_SIZE: usize = 32;

pub struct Chunk {
    blocks: [[[BlockType; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    position: IVec3,
    mesh: Option<Mesh>,
    dirty: bool,
}

impl Chunk {
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        self.blocks[x][y][z]
    }
    
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockType) {
        self.blocks[x][y][z] = block;
        self.dirty = true;
    }
}
```

### 4. Greedy Meshing Algorithm

The greedy meshing algorithm combines adjacent blocks of the same type into larger quads, significantly reducing triangle count:

```rust
// Pseudocode for greedy meshing
for each layer in chunk {
    for each block in layer {
        if block is visible {
            find maximum rectangle of same blocks
            create single quad for entire rectangle
            mark blocks as processed
        }
    }
}
```

Benefits:
- Reduces vertices by 60-90%
- Improves GPU performance
- Maintains visual quality

### 5. Rendering Pipeline

```rust
// Bevy system for chunk rendering
fn render_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunks: Query<(&mut Chunk, Option<&Handle<Mesh>>)>,
) {
    for (mut chunk, mesh_handle) in chunks.iter_mut() {
        if chunk.dirty {
            let new_mesh = generate_chunk_mesh(&chunk);
            // Update or create mesh
            chunk.dirty = false;
        }
    }
}
```

### 6. Ray Casting for Block Selection

```rust
pub fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    world: &World,
) -> Option<(IVec3, IVec3)> {
    // DDA algorithm for voxel traversal
    let mut current = origin;
    let step = direction.normalize() * 0.01;
    
    while current.distance(origin) < max_distance {
        let block_pos = world_to_block_pos(current);
        if world.get_block(block_pos).is_solid() {
            let previous = world_to_block_pos(current - step);
            return Some((block_pos, previous));
        }
        current += step;
    }
    None
}
```

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
bevy = "0.14"
bevy_egui = "0.28"  # For debug UI
nalgebra = "0.33"   # Additional math
noise = "0.9"       # For future terrain generation

[profile.dev]
opt-level = 1  # Better performance in debug

[profile.release]
lto = true
codegen-units = 1
```

## Performance Optimizations

### Immediate Optimizations
1. **Frustum Culling**: Only render chunks in view
2. **Face Culling**: Don't render faces between solid blocks
3. **Texture Atlas**: Single texture for all blocks
4. **Instanced Rendering**: For repeated elements

### Future Optimizations
1. **Level of Detail (LOD)**: Simpler meshes for distant chunks
2. **Occlusion Culling**: Don't render chunks behind others
3. **Mesh Caching**: Save generated meshes
4. **Multithreaded Meshing**: Generate meshes in parallel

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_chunk_indexing() {
        let chunk = Chunk::new(IVec3::ZERO);
        chunk.set_block(0, 0, 0, BlockType::Stone);
        assert_eq!(chunk.get_block(0, 0, 0), BlockType::Stone);
    }
    
    #[test]
    fn test_greedy_meshing() {
        // Test that adjacent blocks are combined
    }
}
```

### Integration Tests
- Spawn 1000 chunks, verify memory usage
- Place/remove 1000 blocks, verify performance
- Camera movement across chunk boundaries

### Performance Benchmarks
```rust
#[bench]
fn bench_chunk_generation(b: &mut Bencher) {
    b.iter(|| {
        let chunk = generate_chunk(IVec3::ZERO);
        generate_mesh(&chunk);
    });
}
```

## Common Issues & Solutions

### Issue: Seams between chunks
**Solution**: Ensure chunk meshes include boundary checks with neighbor chunks

### Issue: Z-fighting on block faces
**Solution**: Offset selection box slightly, use depth bias

### Issue: Poor performance with many chunks
**Solution**: Implement view distance, LOD, chunk unloading

### Issue: Memory usage grows unbounded
**Solution**: Implement chunk cache with LRU eviction

## Deliverables Checklist

- [ ] Basic voxel rendering working
- [ ] Chunk system implemented
- [ ] Greedy meshing optimizing triangle count
- [ ] Block placement/removal functional
- [ ] First-person camera controls
- [ ] Ray casting for block selection
- [ ] Selection highlight rendering
- [ ] Basic lighting (ambient + directional)
- [ ] 60+ FPS with 16 chunk view distance
- [ ] Memory usage under 1GB

## Next Phase Preparation

Before moving to Phase 2 (Inventory & Crafting):
1. Ensure chunk system is stable
2. Verify block data structure supports metadata
3. Confirm rendering pipeline is extensible
4. Document any technical debt
5. Create performance baseline metrics