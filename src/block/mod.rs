use bevy::prelude::*;

pub mod types;
pub use types::BlockType;

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, _app: &mut App) {
        // Block-related systems will be added here
    }
}