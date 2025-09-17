pub mod animation;
pub mod atlas;
pub mod loader;
pub mod test_textures;

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFace {
    Top,
    Bottom,
    Front,
    Back,
    Left,
    Right,
}

impl BlockFace {
    pub fn all() -> [BlockFace; 6] {
        [
            BlockFace::Top,
            BlockFace::Bottom,
            BlockFace::Front,
            BlockFace::Back,
            BlockFace::Left,
            BlockFace::Right,
        ]
    }

    pub fn sides() -> [BlockFace; 4] {
        [
            BlockFace::Front,
            BlockFace::Back,
            BlockFace::Left,
            BlockFace::Right,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockState {
    Normal,
    Active,
    Powered,
    On,
    Off,
}

impl Default for BlockState {
    fn default() -> Self {
        BlockState::Normal
    }
}

#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub atlas_index: usize,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub animation_frames: Option<Vec<usize>>,
}

#[derive(Resource)]
pub struct BlockTextureAtlas {
    pub texture: Handle<Image>,
    pub atlas_size: Vec2,
    pub texture_size: f32,
    pub textures: HashMap<(String, BlockFace, BlockState), TextureInfo>,
}

impl BlockTextureAtlas {
    pub fn get_uv(&self, block_type: &str, face: BlockFace, state: BlockState) -> (Vec2, Vec2) {
        let key = (block_type.to_string(), face, state);

        if let Some(info) = self.textures.get(&key) {
            return (info.uv_min, info.uv_max);
        }

        // Fallback to normal state if specific state not found
        let fallback_key = (block_type.to_string(), face, BlockState::Normal);
        if let Some(info) = self.textures.get(&fallback_key) {
            return (info.uv_min, info.uv_max);
        }

        // Return missing texture UV (purple)
        (
            Vec2::ZERO,
            Vec2::new(32.0 / self.atlas_size.x, 32.0 / self.atlas_size.y),
        )
    }

    /// Get a representative texture path for UI display purposes
    /// Returns the most representative texture for a block type
    pub fn get_display_texture_path(block_type: &str) -> String {
        let base_path = format!("assets/textures/blocks/{}", block_type);

        // Check actual file existence
        use std::path::Path;

        // Try faces in order of preference for UI display
        let face_options = vec![
            "side",   // Side view is often most recognizable
            "all",    // Universal texture
            "top",    // Top view as fallback
            "front",  // Front face if available
            "bottom", // Last resort
        ];

        for face_name in face_options {
            let file_path = format!("{}/{}.png", base_path, face_name);
            if Path::new(&file_path).exists() {
                // Return without "assets/" prefix for asset_server.load()
                return format!("textures/blocks/{}/{}.png", block_type, face_name);
            }
        }

        // Default fallback (assume all.png exists)
        format!("textures/blocks/{}/all.png", block_type)
    }
}

pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_textures);
    }
}

fn setup_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    // This will be implemented in loader.rs
    info!("Setting up texture system...");

    // For now, create a placeholder atlas
    let atlas = loader::load_block_textures(&asset_server, &mut images);
    commands.insert_resource(atlas);
}
