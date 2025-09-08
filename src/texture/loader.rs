use super::*;
use bevy::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use image::{ImageBuffer, Rgba};

const TEXTURE_SIZE: u32 = 32;
const PADDING: u32 = 2;
const BLOCKS_DIR: &str = "assets/textures/blocks";

#[derive(Debug)]
struct BlockTextures {
    name: String,
    faces: HashMap<(BlockFace, BlockState), Vec<PathBuf>>, // Vec for animation frames
}

pub fn load_block_textures(
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
) -> BlockTextureAtlas {
    info!("Loading block textures from {}", BLOCKS_DIR);
    let block_textures = scan_texture_directory();
    info!("Found {} block types", block_textures.len());
    for block in &block_textures {
        info!("Block '{}' has {} face configurations", block.name, block.faces.len());
    }
    build_atlas(block_textures, asset_server, images)
}

fn scan_texture_directory() -> Vec<BlockTextures> {
    let mut blocks = Vec::new();
    
    let blocks_path = Path::new(BLOCKS_DIR);
    if !blocks_path.exists() {
        warn!("Texture directory not found: {}", BLOCKS_DIR);
        return blocks;
    }
    
    info!("Scanning texture directory: {:?}", blocks_path);
    
    // Scan each block directory
    if let Ok(entries) = fs::read_dir(blocks_path) {
        for entry in entries.filter_map(Result::ok) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Some(block_name) = entry.file_name().to_str() {
                        info!("Found block directory: {}", block_name);
                        let block_textures = scan_block_directory(
                            &entry.path(),
                            block_name.to_string()
                        );
                        blocks.push(block_textures);
                    }
                }
            }
        }
    }
    
    blocks
}

fn scan_block_directory(path: &Path, name: String) -> BlockTextures {
    let mut faces = HashMap::new();
    let mut texture_files: Vec<PathBuf> = Vec::new();
    
    // Collect all PNG files
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(Result::ok) {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".png") {
                    info!("  Found texture file: {}", file_name);
                    texture_files.push(entry.path());
                }
            }
        }
    }
    
    // Parse texture files and organize by face/state/frame
    let mut parsed_textures: HashMap<(String, Option<String>, Option<usize>), PathBuf> = HashMap::new();
    
    for file_path in texture_files {
        if let Some(file_name) = file_path.file_stem().and_then(|s| s.to_str()) {
            let (base_name, state, frame) = parse_texture_name(file_name);
            parsed_textures.insert((base_name, state, frame), file_path);
        }
    }
    
    // Check for "all.png" first
    if parsed_textures.contains_key(&("all".to_string(), None, None)) {
        let all_path = parsed_textures[&("all".to_string(), None, None)].clone();
        for face in BlockFace::all() {
            faces.insert((face, BlockState::Normal), vec![all_path.clone()]);
        }
    } else {
        // Process each face type
        process_face_textures(&mut faces, &parsed_textures, "top", BlockFace::Top);
        process_face_textures(&mut faces, &parsed_textures, "bottom", BlockFace::Bottom);
        
        // Handle side textures with fallback
        if let Some(side_textures) = get_textures_for_base(&parsed_textures, "side") {
            for face in BlockFace::sides() {
                for (state, paths) in &side_textures {
                    faces.insert((face, *state), paths.clone());
                }
            }
        }
        
        // Override with specific face textures if they exist
        process_face_textures(&mut faces, &parsed_textures, "front", BlockFace::Front);
        process_face_textures(&mut faces, &parsed_textures, "back", BlockFace::Back);
        process_face_textures(&mut faces, &parsed_textures, "left", BlockFace::Left);
        process_face_textures(&mut faces, &parsed_textures, "right", BlockFace::Right);
    }
    
    BlockTextures { name, faces }
}

fn parse_texture_name(name: &str) -> (String, Option<String>, Option<usize>) {
    let parts: Vec<&str> = name.split('_').collect();
    
    if parts.is_empty() {
        return (name.to_string(), None, None);
    }
    
    let base_name = parts[0].to_string();
    let mut state = None;
    let mut frame = None;
    
    // Check if last part is a frame number
    if parts.len() > 1 {
        if let Ok(frame_num) = parts[parts.len() - 1].parse::<usize>() {
            frame = Some(frame_num);
            // Check for state in middle parts
            if parts.len() > 2 {
                state = Some(parts[1..parts.len()-1].join("_"));
            }
        } else {
            // No frame number, everything after base is state
            state = Some(parts[1..].join("_"));
        }
    }
    
    (base_name, state, frame)
}

fn get_textures_for_base(
    parsed: &HashMap<(String, Option<String>, Option<usize>), PathBuf>,
    base: &str,
) -> Option<HashMap<BlockState, Vec<PathBuf>>> {
    let mut result = HashMap::new();
    let mut found_any = false;
    
    // Group by state
    let mut by_state: HashMap<Option<String>, Vec<(usize, PathBuf)>> = HashMap::new();
    
    for ((b, state, frame), path) in parsed {
        if b == base {
            found_any = true;
            let frame_num = frame.unwrap_or(0);
            by_state.entry(state.clone())
                .or_insert_with(Vec::new)
                .push((frame_num, path.clone()));
        }
    }
    
    if !found_any {
        return None;
    }
    
    // Sort frames and convert states
    for (state_str, mut frames) in by_state {
        frames.sort_by_key(|(frame_num, _)| *frame_num);
        let paths: Vec<PathBuf> = frames.into_iter().map(|(_, path)| path).collect();
        
        let block_state = match state_str.as_deref() {
            Some("on") => BlockState::On,
            Some("off") => BlockState::Off,
            Some("active") => BlockState::Active,
            Some("powered") => BlockState::Powered,
            _ => BlockState::Normal,
        };
        
        result.insert(block_state, paths);
    }
    
    Some(result)
}

fn process_face_textures(
    faces: &mut HashMap<(BlockFace, BlockState), Vec<PathBuf>>,
    parsed: &HashMap<(String, Option<String>, Option<usize>), PathBuf>,
    base_name: &str,
    face: BlockFace,
) {
    if let Some(textures) = get_textures_for_base(parsed, base_name) {
        for (state, paths) in textures {
            faces.insert((face, state), paths);
        }
    }
}

fn build_atlas(
    blocks: Vec<BlockTextures>,
    _asset_server: &AssetServer,
    images: &mut Assets<Image>,
) -> BlockTextureAtlas {
    // Calculate atlas size
    let mut total_textures = 0;
    for block in &blocks {
        for (_, frames) in &block.faces {
            total_textures += frames.len();
        }
    }
    
    // Add space for missing texture
    total_textures += 1;
    
    // Calculate grid size (make it square-ish)
    let grid_size = (total_textures as f32).sqrt().ceil() as u32;
    let atlas_size = grid_size * (TEXTURE_SIZE + PADDING * 2);
    
    // Create atlas image
    let mut atlas_image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(atlas_size, atlas_size);
    
    // Create missing texture (purple)
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let px = x + PADDING;
            let py = y + PADDING;
            atlas_image.put_pixel(px, py, Rgba([255, 0, 255, 255]));
        }
    }
    
    let mut texture_map = HashMap::new();
    let mut current_index = 1usize; // Start at 1, 0 is missing texture
    
    // Load and place textures
    for block in blocks {
        for ((face, state), paths) in block.faces {
            let mut frame_indices = Vec::new();
            
            for path in paths {
                let grid_x = (current_index % grid_size as usize) as u32;
                let grid_y = (current_index / grid_size as usize) as u32;
                let x = grid_x * (TEXTURE_SIZE + PADDING * 2) + PADDING;
                let y = grid_y * (TEXTURE_SIZE + PADDING * 2) + PADDING;
                
                // Load texture file
                info!("Loading texture: {:?}", path);
                if let Ok(img) = image::open(&path) {
                    let rgba = img.to_rgba8();
                    
                    // Copy to atlas with padding
                    for dy in 0..TEXTURE_SIZE {
                        for dx in 0..TEXTURE_SIZE {
                            if dx < rgba.width() && dy < rgba.height() {
                                let pixel = rgba.get_pixel(dx, dy);
                                atlas_image.put_pixel(x + dx, y + dy, *pixel);
                            }
                        }
                    }
                    
                    // Add padding by extending edge pixels
                    for i in 0..TEXTURE_SIZE {
                        // Top padding
                        let top_pixel = *atlas_image.get_pixel(x + i, y);
                        for p in 1..=PADDING {
                            atlas_image.put_pixel(x + i, y - p, top_pixel);
                        }
                        // Bottom padding
                        let bottom_pixel = *atlas_image.get_pixel(x + i, y + TEXTURE_SIZE - 1);
                        for p in 1..=PADDING {
                            atlas_image.put_pixel(x + i, y + TEXTURE_SIZE + p - 1, bottom_pixel);
                        }
                        // Left padding
                        let left_pixel = *atlas_image.get_pixel(x, y + i);
                        for p in 1..=PADDING {
                            atlas_image.put_pixel(x - p, y + i, left_pixel);
                        }
                        // Right padding
                        let right_pixel = *atlas_image.get_pixel(x + TEXTURE_SIZE - 1, y + i);
                        for p in 1..=PADDING {
                            atlas_image.put_pixel(x + TEXTURE_SIZE + p - 1, y + i, right_pixel);
                        }
                    }
                    
                    frame_indices.push(current_index as usize);
                } else {
                    warn!("Failed to load texture: {:?}", path);
                    frame_indices.push(0); // Use missing texture
                }
                
                current_index += 1;
            }
            
            // Calculate UVs for first frame
            if !frame_indices.is_empty() {
                let first_frame = frame_indices[0];
                let grid_x = (first_frame % grid_size as usize) as f32;
                let grid_y = (first_frame / grid_size as usize) as f32;
                
                let uv_min = Vec2::new(
                    (grid_x * (TEXTURE_SIZE + PADDING * 2) as f32 + PADDING as f32) / atlas_size as f32,
                    (grid_y * (TEXTURE_SIZE + PADDING * 2) as f32 + PADDING as f32) / atlas_size as f32,
                );
                let uv_max = Vec2::new(
                    uv_min.x + (TEXTURE_SIZE as f32 / atlas_size as f32),
                    uv_min.y + (TEXTURE_SIZE as f32 / atlas_size as f32),
                );
                
                let animation_frames = if frame_indices.len() > 1 {
                    Some(frame_indices.clone())
                } else {
                    None
                };
                
                texture_map.insert(
                    (block.name.clone(), face, state),
                    TextureInfo {
                        atlas_index: first_frame,
                        uv_min,
                        uv_max,
                        animation_frames,
                    },
                );
            }
        }
    }
    
    // Convert to Bevy image
    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        atlas_image.into_raw(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );
    
    let handle = images.add(image);
    
    BlockTextureAtlas {
        texture: handle,
        atlas_size: Vec2::new(atlas_size as f32, atlas_size as f32),
        texture_size: TEXTURE_SIZE as f32,
        textures: texture_map,
    }
}