use bevy::prelude::*;

/// Game states for managing loading and gameplay flow
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Loading,
    GeneratingWorld,
    Playing,
}

/// Tracks the current phase of world generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadingPhase {
    Initializing,
    GeneratingTerrain,
    PreparingSpawn,
    Complete,
}

impl LoadingPhase {
    pub fn description(&self) -> &str {
        match self {
            LoadingPhase::Initializing => "Initializing world generator...",
            LoadingPhase::GeneratingTerrain => "Generating base terrain...",
            LoadingPhase::PreparingSpawn => "Preparing spawn area...",
            LoadingPhase::Complete => "World ready!",
        }
    }
}

/// Resource tracking loading progress
#[derive(Resource)]
pub struct LoadingProgress {
    pub chunks_generated: u32,
    pub total_chunks: u32,
    pub current_phase: LoadingPhase,
    pub phase_start_time: f32,
    pub total_start_time: f32,
    pub spawn_position: Option<Vec3>, // Determined spawn position
}

impl Default for LoadingProgress {
    fn default() -> Self {
        Self {
            chunks_generated: 0,
            total_chunks: 0,
            current_phase: LoadingPhase::Initializing,
            phase_start_time: 0.0,
            total_start_time: 0.0,
            spawn_position: None,
        }
    }
}

impl LoadingProgress {
    pub fn progress_percentage(&self) -> f32 {
        // Calculate progress based on current phase
        match self.current_phase {
            LoadingPhase::Initializing => 0.0,
            LoadingPhase::GeneratingTerrain => 40.0,
            LoadingPhase::PreparingSpawn => 80.0,
            LoadingPhase::Complete => 100.0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_phase == LoadingPhase::Complete
    }

    pub fn advance_phase(&mut self, new_phase: LoadingPhase, time: f32) {
        info!(
            "Loading phase: {:?} -> {:?} ({}%)",
            self.current_phase,
            new_phase,
            self.progress_percentage()
        );
        self.current_phase = new_phase;
        self.phase_start_time = time;
    }
}

/// Plugin for managing loading states
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<LoadingProgress>()
            .add_systems(OnEnter(GameState::Loading), setup_loading)
            .add_systems(Update, update_loading.run_if(in_state(GameState::Loading)))
            .add_systems(OnExit(GameState::Loading), cleanup_loading)
            .add_systems(OnEnter(GameState::GeneratingWorld), setup_world_generation)
            .add_systems(
                Update,
                update_world_generation.run_if(in_state(GameState::GeneratingWorld)),
            )
            .add_systems(OnExit(GameState::GeneratingWorld), cleanup_world_generation)
            .add_systems(OnEnter(GameState::Playing), setup_gameplay);
    }
}

fn setup_loading(mut loading_progress: ResMut<LoadingProgress>, time: Res<Time>) {
    loading_progress.total_start_time = time.elapsed_seconds();
    loading_progress.phase_start_time = time.elapsed_seconds();
    info!("Entering loading state");
}

fn update_loading(
    mut next_state: ResMut<NextState<GameState>>,
    time: Res<Time>,
    mut loading_progress: ResMut<LoadingProgress>,
    world_gen: Option<Res<crate::world::WorldGenerator>>,
) {
    // Simulate world generation phases with brief delays
    let phase_duration = 0.3; // Each early phase takes 0.3 seconds
    let elapsed_since_phase = time.elapsed_seconds() - loading_progress.phase_start_time;

    match loading_progress.current_phase {
        LoadingPhase::Initializing if world_gen.is_some() && elapsed_since_phase > 0.5 => {
            loading_progress.advance_phase(LoadingPhase::GeneratingTerrain, time.elapsed_seconds());
        }
        LoadingPhase::GeneratingTerrain if elapsed_since_phase > phase_duration => {
            next_state.set(GameState::GeneratingWorld);
        }
        _ => {}
    }
}

fn cleanup_loading() {
    info!("Exiting loading state");
}

fn setup_world_generation(
    mut loading_progress: ResMut<LoadingProgress>,
    world_gen: Res<crate::world::WorldGenerator>,
    planet_config: Res<crate::planet::PlanetConfig>,
    time: Res<Time>,
) {
    // Reset progress counter when entering world generation
    loading_progress.chunks_generated = 0;

    // Determine spawn position before generating chunks
    let planet_size = planet_config.size_chunks as f32 * 32.0;
    let spawn_pos = crate::camera::find_guaranteed_land_spawn(&world_gen, planet_size);
    loading_progress.spawn_position = Some(spawn_pos);

    loading_progress.advance_phase(LoadingPhase::PreparingSpawn, time.elapsed_seconds());
    loading_progress.advance_phase(LoadingPhase::Complete, time.elapsed_seconds());

    info!(
        "Starting world generation at spawn position: {:?}",
        spawn_pos
    );
}

fn update_world_generation(
    mut next_state: ResMut<NextState<GameState>>,
    loading_progress: Res<LoadingProgress>,
) {
    // This will be replaced with actual chunk generation progress
    if loading_progress.is_complete() {
        next_state.set(GameState::Playing);
    }
}

fn cleanup_world_generation() {
    info!("World generation complete");
}

fn setup_gameplay() {
    info!("Starting gameplay");
}
