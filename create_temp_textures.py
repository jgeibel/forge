#!/usr/bin/env python3
"""
Create temporary colored textures for missing block types
"""

from PIL import Image
import os

# Define block colors (RGBA)
BLOCKS = {
    'water': (51, 102, 204, 180),  # Transparent blue
    'sand': (230, 204, 153, 255),  # Light tan
    'bedrock': (25, 25, 25, 255),  # Very dark gray
    'wood': (139, 90, 43, 255),    # Brown
    'leaves': (51, 153, 51, 200),  # Semi-transparent green
    'cobblestone': (128, 128, 128, 255),  # Gray
    'planks': (179, 128, 77, 255),  # Light brown
}

# Texture size (16x16 for Minecraft-style)
SIZE = 16

def create_solid_texture(color, filename):
    """Create a solid color texture"""
    img = Image.new('RGBA', (SIZE, SIZE), color)
    img.save(filename)
    print(f"Created {filename}")

def main():
    base_dir = "assets/textures/blocks"
    
    for block_name, color in BLOCKS.items():
        block_dir = os.path.join(base_dir, block_name)
        
        # Create directory if it doesn't exist
        if not os.path.exists(block_dir):
            os.makedirs(block_dir)
            print(f"Created directory: {block_dir}")
        
        # Create 'all.png' for blocks with same texture on all sides
        texture_path = os.path.join(block_dir, 'all.png')
        if not os.path.exists(texture_path):
            create_solid_texture(color, texture_path)
        else:
            print(f"Skipping {texture_path} (already exists)")

if __name__ == "__main__":
    main()