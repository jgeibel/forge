use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

const ANIMATION_FPS: f32 = 8.0;

#[derive(Component)]
pub struct AnimatedTexture {
    pub frames: Vec<usize>,
    pub current_frame: usize,
    pub timer: Timer,
}

impl AnimatedTexture {
    pub fn new(frames: Vec<usize>) -> Self {
        Self {
            frames,
            current_frame: 0,
            timer: Timer::from_seconds(1.0 / ANIMATION_FPS, TimerMode::Repeating),
        }
    }
    
    pub fn update(&mut self, delta: f32) {
        self.timer.tick(Duration::from_secs_f32(delta));
        
        if self.timer.just_finished() {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }
    
    pub fn current_frame_index(&self) -> usize {
        self.frames[self.current_frame]
    }
}

#[derive(Resource)]
pub struct AnimationManager {
    pub animations: HashMap<String, AnimatedTexture>,
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self {
            animations: HashMap::new(),
        }
    }
}

pub fn update_texture_animations(
    time: Res<Time>,
    mut manager: ResMut<AnimationManager>,
) {
    let delta = time.delta_seconds();
    
    for (_, animation) in manager.animations.iter_mut() {
        animation.update(delta);
    }
}