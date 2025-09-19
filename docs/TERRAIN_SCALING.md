# Terrain Scaling Design Principles

## Core Concept

When generating terrain for different world sizes, features must be explicitly classified as either **scale-invariant** or **scale-dependent** to ensure consistent gameplay and visual appearance across all world sizes.

## Definitions

### Scale-Invariant Features
Features that maintain constant physical dimensions (in blocks) regardless of world size. These features should look and feel the same whether you're playing on a tiny world or a massive continental world.

**Examples:**
- A beach is always 2-6 blocks of elevation above sea level
- A river is always 10-30 blocks wide
- A hill is always 50-200 blocks in diameter
- A mountain peak is always 100-400 blocks wide at its base

### Scale-Dependent Features
Features that scale proportionally with world size to maintain similar relative proportions when viewing the world map.

**Examples:**
- Continents cover roughly the same percentage of the map
- Ocean basins maintain similar relative sizes
- Climate zones span similar latitudinal ranges

## Feature Classification

### Scale-Invariant (Physical Features)
| Feature | Block Dimensions | Rationale |
|---------|-----------------|-----------|
| Ocean Depth | 50-200 blocks | Continental shelf to deep ocean |
| Beach Elevation | 2-6 blocks above sea | Vertical range for beach classification |
| River Width | 10-30 blocks | Rivers have physical constraints |
| River Depth | 3-18 blocks | Physical water depth |
| Lake Depth | 10-20 blocks | Natural lake depths |
| Lake Size | 30-100 blocks | Natural lake dimensions |
| Individual Hills | 50-200 blocks wide | Hill size is independent of world |
| Mountain Base | 100-400 blocks | Individual mountain dimensions |
| Mountain Height | 200-300 blocks | Realistic mountain elevation |
| Biome Transitions | 20-50 blocks | Gradual ecosystem changes |
| Valley Width | 30-150 blocks | Physical valley dimensions |
| Cliff Height | 10-40 blocks | Natural cliff formations |

### Scale-Dependent (Geographic Features)
| Feature | Scaling Method | Rationale |
|---------|---------------|-----------|
| Continent Count | Logarithmic | More continents on larger worlds, but not linearly |
| Continent Size | Proportional | Maintains map appearance |
| Ocean Basin Size | Proportional | Relative to world size |
| Climate Zones | Proportional | Latitude bands scale with world |
| Tectonic Plates | Square root | Geological scaling |

## Implementation Guidelines

### Frequency Scaling Formula
For noise-based terrain generation, frequencies must scale inversely with world size to maintain constant feature dimensions:

```rust
// Base frequency for a "standard" world (16384 blocks)
const STANDARD_WORLD_SIZE: f32 = 16384.0;

// Calculate frequency scaling factor
let frequency_scale = STANDARD_WORLD_SIZE / actual_world_size;

// Apply to all scale-invariant noise features
let terrain_detail_frequency = BASE_DETAIL_FREQ / frequency_scale;
let mountain_frequency = BASE_MOUNTAIN_FREQ / frequency_scale;
let hill_frequency = BASE_HILL_FREQ / frequency_scale;
```

### Absolute Dimensions
For non-noise features, use absolute block counts:

```rust
// SCALE-INVARIANT: Always the same number of blocks
const BEACH_WIDTH_BLOCKS: f32 = 3.0;
const RIVER_WIDTH_BLOCKS: f32 = 20.0;
const OCEAN_DEPTH_BLOCKS: f32 = 25.0;

// SCALE-DEPENDENT: Scales with world
let continent_radius = world_size * 0.15; // 15% of world width
```

## Visual Examples

### Tiny World (2048x2048 blocks)
- **Mountains**: ~10-20 individual peaks
- **Rivers**: 2-3 major rivers, clearly visible on map
- **Beaches**: Visible strips along coasts
- **Continents**: 2-3 major landmasses

### Default World (16384x16384 blocks)
- **Mountains**: ~500-1000 individual peaks
- **Rivers**: 20-40 major rivers, thin lines on map
- **Beaches**: Thin lines along coasts
- **Continents**: 4-6 major landmasses

### Continental World (2097152x2097152 blocks)
- **Mountains**: ~100,000+ individual peaks
- **Rivers**: 1000+ major rivers, invisible on world map
- **Beaches**: Invisible at map scale
- **Continents**: 12-20 major landmasses

## Testing Checklist

When adding new terrain features, verify:

1. **Classification**: Is this feature scale-invariant or scale-dependent?
2. **Documentation**: Add to the appropriate table above
3. **Implementation**: Use frequency scaling or proportional scaling as appropriate
4. **Testing**: Generate at 3+ different world sizes and measure:
   - Scale-invariant features: Measure in blocks, should be constant
   - Scale-dependent features: Measure as % of world, should be constant

## Common Mistakes to Avoid

❌ **DON'T** scale physical dimensions with world size:
```rust
// WRONG: Makes beaches wider on larger worlds
beach_width = 30.0 * world_scale;
```

❌ **DON'T** use constant frequencies for scale-invariant features:
```rust
// WRONG: Mountains get stretched on larger worlds
mountain_noise_freq = 2.5; // Same for all world sizes
```

✅ **DO** keep physical dimensions constant:
```rust
// CORRECT: Beaches always same width range
beach_width = 15.0 + noise * 35.0; // Always 15-50 blocks
```

✅ **DO** scale frequencies inversely:
```rust
// CORRECT: More mountains on larger worlds, same size each
mountain_noise_freq = 2.5 / frequency_scale;
```

## Future Features

When implementing new terrain features, you MUST:

1. Explicitly classify as scale-invariant or scale-dependent
2. Document the classification and rationale
3. Add appropriate scaling in the implementation
4. Test at multiple world sizes
5. Update this document

This ensures consistent and predictable terrain generation across all world sizes.