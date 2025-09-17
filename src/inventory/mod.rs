use crate::block::BlockType;
use crate::loading::GameState;
use bevy::prelude::*;

pub mod hotbar;

pub use hotbar::{Hotbar, HotbarUI};

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            .add_systems(OnEnter(GameState::Playing), hotbar::setup_hotbar_ui)
            .add_systems(
                Update,
                (hotbar::hotbar_selection_system, hotbar::update_hotbar_ui)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// Player's selected block type
#[derive(Resource)]
pub struct SelectedBlockType {
    pub block_type: BlockType,
}

impl Default for SelectedBlockType {
    fn default() -> Self {
        Self {
            block_type: BlockType::Stone,
        }
    }
}
