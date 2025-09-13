mod loading_screen;
mod debug_overlay;
mod crosshair;
mod command_prompt;

use bevy::prelude::*;
use loading_screen::LoadingScreenPlugin;
use debug_overlay::DebugOverlayPlugin;
use command_prompt::CommandPromptPlugin;
pub use command_prompt::CommandPromptState;
use crate::loading::GameState;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LoadingScreenPlugin)
            .add_plugins(DebugOverlayPlugin)
            .add_plugins(CommandPromptPlugin)
            .add_systems(OnEnter(GameState::Playing), crosshair::setup_crosshair);
    }
}