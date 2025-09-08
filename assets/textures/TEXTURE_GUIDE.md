# Forge Texture System Guide

## Directory Structure

All block textures should be placed in `assets/textures/blocks/[block_name]/`

Example:
```
assets/textures/blocks/
├── grass/
│   ├── top.png
│   ├── bottom.png
│   └── side.png
├── stone/
│   └── all.png
└── furnace/
    ├── top.png
    ├── bottom.png
    ├── side.png
    ├── front_off.png
    └── front_on.png
```

## Texture Resolution

- Standard texture size: **32x32 pixels**
- All textures must be the same size
- PNG format with optional transparency

## File Naming Convention

### Basic Textures

The system loads textures with this priority:

1. **`all.png`** - Used for all 6 faces of the block
2. **Individual face textures:**
   - `top.png` - Top face (Y+)
   - `bottom.png` - Bottom face (Y-)
   - `side.png` - All 4 side faces (default for front/back/left/right)
   - `front.png` - Front face (Z+) - overrides `side.png`
   - `back.png` - Back face (Z-) - overrides `side.png`
   - `left.png` - Left face (X-) - overrides `side.png`
   - `right.png` - Right face (X+) - overrides `side.png`

### Loading Priority

For each face, the system checks in this order:
1. Specific face file (`front.png`, `back.png`, etc.)
2. `side.png` (for front/back/left/right faces)
3. `all.png` (if no other textures found)

### Animated Textures

Add frame numbers with underscore suffix:
- `water/top_0.png`, `water/top_1.png`, `water/top_2.png` - Animation frames
- Frames are played in numerical order
- Animation speed is currently fixed at 8 FPS

### State Variants

For blocks with different states (on/off, powered, etc.):
- `front_off.png` - Default state
- `front_on.png` - Active state
- Can be combined with animation: `front_on_0.png`, `front_on_1.png`

## Examples

### Simple Block (Stone)
```
stone/
└── all.png          # Same texture on all 6 faces
```

### Standard Block (Grass)
```
grass/
├── top.png          # Green grass
├── bottom.png       # Dirt
└── side.png         # Dirt with grass edge
```

### Directional Block (Furnace)
```
furnace/
├── top.png          # Metal top
├── bottom.png       # Metal bottom  
├── side.png         # Metal sides (back, left, right)
├── front_off.png    # Front when inactive
└── front_on.png     # Front when smelting (could be animated)
```

### Animated Block (Water)
```
water/
├── top_0.png        # Animation frame 0
├── top_1.png        # Animation frame 1
├── top_2.png        # Animation frame 2
├── top_3.png        # Animation frame 3
└── side.png         # Static sides
```

### Complex Block (Command Block)
```
command_block/
├── top.png
├── bottom.png
├── side.png         # Default for all sides
├── front_off.png    # Override for front when off
├── front_on_0.png   # Animated front when active
├── front_on_1.png
└── front_on_2.png
```

## Special Considerations

### Transparent Blocks
- Use PNG alpha channel for transparency
- Examples: leaves, glass, water
- These blocks require special rendering order

### Emissive Blocks
- Currently uses same texture
- Future: may support `_emissive.png` suffix for glow maps

### Connected Textures
- Not yet supported
- Future: will detect adjacent blocks for seamless textures

## Adding New Blocks

1. Create a new directory under `assets/textures/blocks/` with your block name
2. Add PNG files following the naming convention above
3. The system will automatically detect and load your textures
4. No configuration files needed!

## Troubleshooting

- **Missing Texture**: System will use a purple placeholder texture
- **Wrong Size**: All textures must be 32x32 pixels
- **Not Loading**: Check file names match exactly (case-sensitive)
- **Animation Not Playing**: Ensure frames are numbered sequentially starting from 0