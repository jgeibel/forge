use image::{RgbaImage, Rgba};
use std::path::Path;

pub fn generate_test_textures() {
    let texture_dir = Path::new("assets/textures/blocks");
    
    // Generate grass textures
    generate_grass_textures(&texture_dir.join("grass"));
    
    // Generate stone texture
    generate_stone_texture(&texture_dir.join("stone"));
    
    // Generate dirt texture
    generate_dirt_texture(&texture_dir.join("dirt"));
    
    println!("Test textures generated successfully!");
}

fn generate_grass_textures(dir: &Path) {
    std::fs::create_dir_all(dir).ok();
    
    // Grass top - green with variation
    let mut top = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 7 + y * 11) % 17) as f32 / 17.0;
            let g = 120 + (noise * 60.0) as u8;
            top.put_pixel(x, y, Rgba([60, g, 30, 255]));
        }
    }
    top.save(dir.join("top.png")).unwrap();
    
    // Grass side - dirt with grass on top
    let mut side = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 5 + y * 7) % 13) as f32 / 13.0;
            if y < 8 {
                // Grass part
                let g = 120 + (noise * 60.0) as u8;
                side.put_pixel(x, y, Rgba([60, g, 30, 255]));
            } else {
                // Dirt part
                let brown = 100 + (noise * 40.0) as u8;
                side.put_pixel(x, y, Rgba([brown, brown / 2, 20, 255]));
            }
        }
    }
    side.save(dir.join("side.png")).unwrap();
    
    // Grass bottom - just dirt
    let mut bottom = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 5 + y * 7) % 13) as f32 / 13.0;
            let brown = 100 + (noise * 40.0) as u8;
            bottom.put_pixel(x, y, Rgba([brown, brown / 2, 20, 255]));
        }
    }
    bottom.save(dir.join("bottom.png")).unwrap();
}

fn generate_stone_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();
    
    let mut stone = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise1 = ((x * 3 + y * 5) % 11) as f32 / 11.0;
            let noise2 = ((x * 7 + y * 2) % 13) as f32 / 13.0;
            let gray = 100 + (noise1 * 50.0 + noise2 * 30.0) as u8;
            stone.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }
    stone.save(dir.join("all.png")).unwrap();
}

fn generate_dirt_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();
    
    let mut dirt = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 5 + y * 7) % 13) as f32 / 13.0;
            let brown = 100 + (noise * 40.0) as u8;
            dirt.put_pixel(x, y, Rgba([brown, brown / 2, 20, 255]));
        }
    }
    dirt.save(dir.join("all.png")).unwrap();
}