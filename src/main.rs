use bevy::prelude::*;
use bevy::window::PresentMode;

mod camera;
mod block;
mod chunk;
mod render;
mod input;
mod interaction;
mod planet;
mod fog;
mod ui;
mod texture;
mod world;
mod loading;
mod minimap;

use camera::CameraPlugin;
use block::BlockPlugin;
use chunk::ChunkPlugin;
use render::RenderPlugin;
use input::InputPlugin;
use planet::PlanetPlugin;
use fog::FogPlugin;
use ui::UIPlugin;
use texture::TexturePlugin;
use world::WorldPlugin;
use loading::{LoadingPlugin, GameState};
use minimap::MinimapPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Forge".into(),
                resolution: (1280., 720.).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            LoadingPlugin,  // Add loading first to manage states
            CameraPlugin,
            BlockPlugin,
            WorldPlugin,  // Add before ChunkPlugin since chunks depend on world gen
            ChunkPlugin,
            RenderPlugin,
            InputPlugin,
            PlanetPlugin,
            FogPlugin,
            UIPlugin,
            TexturePlugin,
            MinimapPlugin,
        ))
        .init_resource::<interaction::SelectedBlock>()
        .add_systems(OnEnter(GameState::Playing), setup)
        .add_systems(Update, (
            interaction::block_interaction_system,
            interaction::draw_selection_box,
        ).run_if(in_state(GameState::Playing)))
        .run();
}

fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,
) {
    ambient_light.brightness = 150.0;
    
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
