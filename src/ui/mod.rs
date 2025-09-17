mod crosshair;
mod debug_overlay;
mod loading_screen;
// mod console_commands;  // Disabled - conflicts with command_prompt
pub mod command_prompt; // Made public so other modules can access CommandPromptState

use bevy::prelude::*;
use debug_overlay::DebugOverlayPlugin;
use loading_screen::LoadingScreenPlugin;
// use console_commands::ConsoleCommandsPlugin;  // Disabled - using CommandPromptPlugin instead
use crate::loading::GameState;
use command_prompt::CommandPromptPlugin;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoadingScreenPlugin)
            .add_plugins(DebugOverlayPlugin)
            // .add_plugins(ConsoleCommandsPlugin)  // Disabled - conflicts with CommandPromptPlugin
            .add_plugins(CommandPromptPlugin) // Use custom command prompt instead
            .add_systems(OnEnter(GameState::Playing), crosshair::setup_crosshair);
    }
}
