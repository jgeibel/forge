use image::{ImageBuffer, Rgba};
use std::path::Path;

fn main() {
    fix_texture_borders();
}

fn fix_texture_borders() {
    let paths = [
        "assets/textures/blocks/grass/top.png",
        "assets/textures/blocks/grass/side.png",
        "assets/textures/blocks/grass/bottom.png",
        "assets/textures/blocks/stone/all.png",
        "assets/textures/blocks/dirt/all.png",
    ];

    for path_str in &paths {
        let path = Path::new(path_str);
        if path.exists() {
            println!("Checking {}", path_str);
            
            if let Ok(img) = image::open(path) {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                
                // Check if edges are dark
                let mut has_dark_border = false;
                
                // Check top and bottom edges
                for x in 0..width {
                    let top_pixel = rgba.get_pixel(x, 0);
                    let bottom_pixel = rgba.get_pixel(x, height - 1);
                    
                    if is_dark(top_pixel) || is_dark(bottom_pixel) {
                        has_dark_border = true;
                        break;
                    }
                }
                
                // Check left and right edges
                for y in 0..height {
                    let left_pixel = rgba.get_pixel(0, y);
                    let right_pixel = rgba.get_pixel(width - 1, y);
                    
                    if is_dark(left_pixel) || is_dark(right_pixel) {
                        has_dark_border = true;
                        break;
                    }
                }
                
                if has_dark_border {
                    println!("  Found dark borders, fixing...");
                    let fixed = remove_borders(&rgba);
                    fixed.save(path).unwrap();
                    println!("  Fixed!");
                } else {
                    println!("  No dark borders found");
                }
            }
        }
    }
}

fn is_dark(pixel: &Rgba<u8>) -> bool {
    let [r, g, b, a] = pixel.0;
    // Check if pixel is very dark (but not transparent)
    a > 128 && (r as u32 + g as u32 + b as u32) < 100
}

fn remove_borders(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let mut result = img.clone();
    
    // Copy second row/column pixels to edges to remove borders
    for x in 0..width {
        // Top edge - copy from second row
        if height > 1 {
            let pixel = *img.get_pixel(x, 1);
            result.put_pixel(x, 0, pixel);
        }
        // Bottom edge - copy from second-to-last row
        if height > 1 {
            let pixel = *img.get_pixel(x, height - 2);
            result.put_pixel(x, height - 1, pixel);
        }
    }
    
    for y in 0..height {
        // Left edge - copy from second column
        if width > 1 {
            let pixel = *img.get_pixel(1, y);
            result.put_pixel(0, y, pixel);
        }
        // Right edge - copy from second-to-last column
        if width > 1 {
            let pixel = *img.get_pixel(width - 2, y);
            result.put_pixel(width - 1, y, pixel);
        }
    }
    
    result
}