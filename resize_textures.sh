#!/bin/bash

# Resize all texture images to 32x32 pixels
echo "Resizing textures to 32x32..."

# Check if sips is available (macOS built-in tool)
if ! command -v sips &> /dev/null; then
    echo "Error: sips command not found. This script requires macOS."
    exit 1
fi

# Find all PNG files in the textures directory
for file in assets/textures/blocks/*/*.png; do
    if [ -f "$file" ]; then
        # Get dimensions
        width=$(sips -g pixelWidth "$file" | awk '/pixelWidth:/{print $2}')
        height=$(sips -g pixelHeight "$file" | awk '/pixelHeight:/{print $2}')
        
        if [ "$width" != "32" ] || [ "$height" != "32" ]; then
            echo "Resizing: $file (${width}x${height} -> 32x32)"
            # Resize to 32x32 using nearest neighbor (preserves pixel art)
            sips -z 32 32 "$file" --out "$file" -s format png
        else
            echo "Already 32x32: $file"
        fi
    fi
done

echo "Done! All textures have been resized to 32x32."