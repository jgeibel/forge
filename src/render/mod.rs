use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, _app: &mut App) {
        // Wireframe toggling removed - requires additional feature flag
        // Can be re-enabled by adding wireframe feature to bevy in Cargo.toml
    }
}
