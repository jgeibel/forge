pub mod collision;
pub mod player;

pub use player::PlayerPhysics;

use bevy::prelude::*;
use player::PlayerPhysicsPlugin;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlayerPhysicsPlugin);
    }
}
