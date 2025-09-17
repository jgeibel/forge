use image::{Rgba, RgbaImage};
use std::path::Path;

pub fn generate_test_textures() {
    let texture_dir = Path::new("assets/textures/blocks");

    // Generate grass textures
    generate_grass_textures(&texture_dir.join("grass"));

    // Generate stone texture
    generate_stone_texture(&texture_dir.join("stone"));

    // Generate dirt texture
    generate_dirt_texture(&texture_dir.join("dirt"));

    // Generate missing block textures
    generate_water_texture(&texture_dir.join("water"));
    generate_sand_texture(&texture_dir.join("sand"));
    generate_bedrock_texture(&texture_dir.join("bedrock"));
    generate_wood_texture(&texture_dir.join("wood"));
    generate_leaves_texture(&texture_dir.join("leaves"));
    generate_cobblestone_texture(&texture_dir.join("cobblestone"));
    generate_planks_texture(&texture_dir.join("planks"));

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

fn generate_water_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut water = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            // Very saturated ocean blue
            let noise = ((x * 3 + y * 5) % 7) as f32 / 7.0;
            let blue = 255;
            let green = 100 + (noise * 20.0) as u8;
            let red = 0;
            water.put_pixel(x, y, Rgba([red, green, blue, 255])); // Fully opaque bright blue
        }
    }
    water.save(dir.join("all.png")).unwrap();
}

fn generate_sand_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut sand = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 7 + y * 5) % 11) as f32 / 11.0;
            let r = 210 + (noise * 30.0) as u8;
            let g = 180 + (noise * 30.0) as u8;
            let b = 120 + (noise * 30.0) as u8;
            sand.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    sand.save(dir.join("all.png")).unwrap();
}

fn generate_bedrock_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut bedrock = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 2 + y * 3) % 5) as f32 / 5.0;
            let gray = 20 + (noise * 30.0) as u8;
            bedrock.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }
    bedrock.save(dir.join("all.png")).unwrap();
}

fn generate_wood_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut wood = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            // Wood grain pattern
            let grain = (y % 8 < 4) as u8;
            let noise = ((x * 3 + y * 5) % 7) as f32 / 7.0;
            let r = 139 + grain * 20 + (noise * 20.0) as u8;
            let g = 90 + grain * 10 + (noise * 10.0) as u8;
            let b = 43 + (noise * 10.0) as u8;
            wood.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    wood.save(dir.join("all.png")).unwrap();
}

fn generate_leaves_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut leaves = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let noise = ((x * 5 + y * 7) % 13) as f32 / 13.0;
            let g = 120 + (noise * 60.0) as u8;
            // Semi-transparent for leaves
            leaves.put_pixel(x, y, Rgba([51, g, 51, 200]));
        }
    }
    leaves.save(dir.join("all.png")).unwrap();
}

fn generate_cobblestone_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut cobble = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            // Create a cobblestone pattern
            let is_gap = (x % 8 == 0 || x % 8 == 7) || (y % 8 == 0 || y % 8 == 7);
            let noise = ((x * 3 + y * 5) % 11) as f32 / 11.0;
            let gray = if is_gap {
                60 + (noise * 20.0) as u8
            } else {
                100 + (noise * 40.0) as u8
            };
            cobble.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }
    cobble.save(dir.join("all.png")).unwrap();
}

fn generate_planks_texture(dir: &Path) {
    std::fs::create_dir_all(dir).ok();

    let mut planks = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            // Vertical plank lines
            let is_gap = x % 8 == 0;
            let noise = ((x * 3 + y * 5) % 7) as f32 / 7.0;
            let r = if is_gap { 120 } else { 179 } + (noise * 20.0) as u8;
            let g = if is_gap { 80 } else { 128 } + (noise * 15.0) as u8;
            let b = if is_gap { 40 } else { 77 } + (noise * 10.0) as u8;
            planks.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    planks.save(dir.join("all.png")).unwrap();
}
