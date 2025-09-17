use bevy::prelude::*;
use bevy::window::PresentMode;

mod block;
mod camera;
mod celestial;
mod chunk;
mod fog;
mod input;
mod interaction;
mod inventory;
mod items;
mod loading;
mod particles;
mod physics;
mod planet;
mod render;
mod texture;
mod tools;
mod ui;
mod world;

use block::BlockPlugin;
use camera::CameraPlugin;
use celestial::CelestialPlugin;
use chunk::ChunkPlugin;
use fog::FogPlugin;
use input::InputPlugin;
use inventory::InventoryPlugin;
use loading::{GameState, LoadingPlugin};
use physics::PhysicsPlugin;
use planet::PlanetPlugin;
use render::RenderPlugin;
use texture::TexturePlugin;
use ui::UIPlugin;
use world::WorldPlugin;

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
            LoadingPlugin, // Add loading first to manage states
            CameraPlugin,
            PhysicsPlugin, // Add physics after camera
            BlockPlugin,
            WorldPlugin, // Add before ChunkPlugin since chunks depend on world gen
            ChunkPlugin,
            RenderPlugin,
            InputPlugin,
            InventoryPlugin, // Add inventory system
            PlanetPlugin,
            CelestialPlugin, // Add celestial system for day/night cycle
            FogPlugin,
            UIPlugin,
            TexturePlugin,
        ))
        .init_resource::<interaction::SelectedBlock>()
        .init_resource::<interaction::BlockExtractionState>()
        .add_systems(OnEnter(GameState::Playing), setup)
        .add_systems(
            Update,
            (
                interaction::block_interaction_system,
                interaction::update_extraction_visual,
                interaction::draw_selection_box,
                items::update_dropped_items,
                items::apply_item_collisions, // Add collision system after physics update
                items::collect_items,
                particles::update_particles,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .run();
}

fn setup() {
    // Lighting is now handled by the CelestialPlugin
    // The sun and ambient light are dynamically updated based on time of day
}
