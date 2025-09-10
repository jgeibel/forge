mod loading_screen;
mod debug_overlay;
mod crosshair;

use bevy::prelude::*;
use loading_screen::LoadingScreenPlugin;
use debug_overlay::DebugOverlayPlugin;
use crate::loading::GameState;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LoadingScreenPlugin)
            .add_plugins(DebugOverlayPlugin)
            .add_systems(OnEnter(GameState::Playing), crosshair::setup_crosshair);
    }
}