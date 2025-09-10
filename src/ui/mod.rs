mod loading_screen;
mod debug_overlay;

use bevy::prelude::*;
use loading_screen::LoadingScreenPlugin;
use debug_overlay::DebugOverlayPlugin;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LoadingScreenPlugin)
            .add_plugins(DebugOverlayPlugin);
    }
}