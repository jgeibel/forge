use bevy::prelude::*;
use bevy::window::PresentMode;

mod camera;
mod block;
mod chunk;
mod render;
mod input;
mod interaction;
mod inventory;
mod items;
mod tools;
mod particles;
mod planet;
mod fog;
mod ui;
mod texture;
mod world;
mod loading;
mod minimap;
mod physics;
mod celestial;

use camera::CameraPlugin;
use block::BlockPlugin;
use chunk::ChunkPlugin;
use render::RenderPlugin;
use input::InputPlugin;
use inventory::InventoryPlugin;
use planet::PlanetPlugin;
use fog::FogPlugin;
use ui::UIPlugin;
use texture::TexturePlugin;
use world::WorldPlugin;
use loading::{LoadingPlugin, GameState};
use minimap::MinimapPlugin;
use physics::PhysicsPlugin;
use celestial::CelestialPlugin;

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
            PhysicsPlugin,  // Add physics after camera
            BlockPlugin,
            WorldPlugin,  // Add before ChunkPlugin since chunks depend on world gen
            ChunkPlugin,
            RenderPlugin,
            InputPlugin,
            InventoryPlugin,  // Add inventory system
            PlanetPlugin,
            CelestialPlugin,  // Add celestial system for day/night cycle
            FogPlugin,
            UIPlugin,
            TexturePlugin,
            MinimapPlugin,
        ))
        .init_resource::<interaction::SelectedBlock>()
        .init_resource::<interaction::BlockExtractionState>()
        .add_systems(OnEnter(GameState::Playing), setup)
        .add_systems(Update, (
            interaction::block_interaction_system,
            interaction::update_extraction_visual,
            interaction::draw_selection_box,
            items::update_dropped_items,
            items::apply_item_collisions,  // Add collision system after physics update
            items::collect_items,
            particles::update_particles,
        ).run_if(in_state(GameState::Playing)))
        .run();
}

fn setup() {
    // Lighting is now handled by the CelestialPlugin
    // The sun and ambient light are dynamically updated based on time of day
}
