use bevy::prelude::*;
use crate::loading::{GameState, LoadingProgress, LoadingPhase};
use crate::planet::PlanetConfig;

/// Marker component for the loading screen root
#[derive(Component)]
pub struct LoadingScreen;

/// Marker component for the loading screen camera
#[derive(Component)]
pub struct LoadingCamera;

/// Marker for the progress bar fill
#[derive(Component)]
pub struct LoadingProgressBar;

/// Marker for the loading status text
#[derive(Component)]
pub struct LoadingStatusText;

/// Marker for the progress percentage text
#[derive(Component)]
pub struct LoadingPercentageText;

/// Marker for the tips text
#[derive(Component)]
pub struct LoadingTipText;

/// Resource for cycling through loading tips
#[derive(Resource)]
pub struct LoadingTips {
    tips: Vec<String>,
    current_index: usize,
    time_per_tip: f32,
    timer: f32,
}

impl Default for LoadingTips {
    fn default() -> Self {
        Self {
            tips: vec![
                "Tip: Hold Shift to sprint and explore faster!".to_string(),
                "Tip: Press F to toggle flight mode for building.".to_string(),
                "Tip: Different biomes contain unique resources.".to_string(),
                "Tip: Caves often contain valuable ores deep underground.".to_string(),
                "Tip: The deeper you dig, the rarer the materials!".to_string(),
                "Tip: Your planet wraps around - keep walking to return home!".to_string(),
                "Tip: Press Tab to see your coordinates and altitude.".to_string(),
                "Tip: Mountains often have exposed ore veins.".to_string(),
                "Tip: Ocean floors hide ancient treasures.".to_string(),
                "Tip: Each planet has a unique seed for generation.".to_string(),
            ],
            current_index: 0,
            time_per_tip: 3.0,
            timer: 0.0,
        }
    }
}

impl LoadingTips {
    pub fn update(&mut self, delta: f32) {
        self.timer += delta;
        if self.timer >= self.time_per_tip {
            self.timer = 0.0;
            self.current_index = (self.current_index + 1) % self.tips.len();
        }
    }
    
    pub fn current_tip(&self) -> &str {
        &self.tips[self.current_index]
    }
}

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<LoadingTips>()
            .add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(OnEnter(GameState::GeneratingWorld), show_generation_screen)
            .add_systems(
                Update,
                (update_loading_screen, update_tips)
                    .run_if(in_state(GameState::Loading).or_else(in_state(GameState::GeneratingWorld)))
            )
            .add_systems(OnExit(GameState::GeneratingWorld), cleanup_loading_screen);
    }
}

fn setup_loading_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn a camera for the UI
    commands.spawn((
        Camera2dBundle::default(),
        LoadingCamera,
    ));
    
    // Create full-screen loading overlay
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.05, 0.05, 0.08)),
                ..default()
            },
            LoadingScreen,
        ))
        .with_children(|parent| {
            // Logo image
            parent.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(500.0),
                    height: Val::Auto,
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
                image: UiImage::new(asset_server.load("images/forge_logo.png")),
                background_color: BackgroundColor(Color::NONE),
                ..default()
            });
            
            // Loading status text
            parent.spawn((
                TextBundle::from_section(
                    "Initializing...",
                    TextStyle {
                        font_size: 24.0,
                        color: Color::srgb(0.7, 0.7, 0.7),
                        ..default()
                    },
                ).with_style(Style {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                }),
                LoadingStatusText,
            ));
            
            // Progress bar container
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(400.0),
                    height: Val::Px(30.0),
                    margin: UiRect::bottom(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                border_color: BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                ..default()
            }).with_children(|parent| {
                // Progress bar fill
                parent.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                        ..default()
                    },
                    LoadingProgressBar,
                ));
            });
            
            // Percentage text
            parent.spawn((
                TextBundle::from_section(
                    "0%",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::srgb(0.6, 0.6, 0.6),
                        ..default()
                    },
                ).with_style(Style {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                }),
                LoadingPercentageText,
            ));
            
            // Tips text
            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 18.0,
                        color: Color::srgb(0.5, 0.5, 0.6),
                        ..default()
                    },
                ).with_style(Style {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                }),
                LoadingTipText,
            ));
        });
}

fn show_generation_screen(
    mut commands: Commands,
    planet_config: Res<PlanetConfig>,
    loading_screen_query: Query<Entity, With<LoadingScreen>>,
) {
    // Add planet info to the loading screen
    if let Ok(screen_entity) = loading_screen_query.get_single() {
        commands.entity(screen_entity).with_children(|parent| {
            // Planet info container in bottom-left corner
            parent.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(20.0),
                    left: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            }).with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    format!("Planet: {}", planet_config.name),
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.4, 0.4, 0.5),
                        ..default()
                    },
                ));
                parent.spawn(TextBundle::from_section(
                    format!("Size: {}x{} blocks", 
                        planet_config.size_chunks * 32, 
                        planet_config.size_chunks * 32),
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.4, 0.4, 0.5),
                        ..default()
                    },
                ));
                parent.spawn(TextBundle::from_section(
                    format!("Seed: {}", planet_config.seed),
                    TextStyle {
                        font_size: 16.0,
                        color: Color::srgb(0.4, 0.4, 0.5),
                        ..default()
                    },
                ));
            });
        });
    }
}

fn update_loading_screen(
    loading_progress: Res<LoadingProgress>,
    mut progress_bar_query: Query<&mut Style, With<LoadingProgressBar>>,
    mut status_text_query: Query<&mut Text, (With<LoadingStatusText>, Without<LoadingPercentageText>)>,
    mut percentage_text_query: Query<&mut Text, (With<LoadingPercentageText>, Without<LoadingStatusText>)>,
) {
    // Update progress bar width
    if let Ok(mut style) = progress_bar_query.get_single_mut() {
        style.width = Val::Percent(loading_progress.progress_percentage());
    }
    
    // Update status text
    if let Ok(mut text) = status_text_query.get_single_mut() {
        text.sections[0].value = loading_progress.current_phase.description().to_string();
    }
    
    // Update percentage text
    if let Ok(mut text) = percentage_text_query.get_single_mut() {
        text.sections[0].value = format!("{:.0}%", loading_progress.progress_percentage());
    }
}

fn update_tips(
    time: Res<Time>,
    mut tips: ResMut<LoadingTips>,
    mut tip_text_query: Query<&mut Text, With<LoadingTipText>>,
) {
    tips.update(time.delta_seconds());
    
    if let Ok(mut text) = tip_text_query.get_single_mut() {
        text.sections[0].value = tips.current_tip().to_string();
    }
}

fn cleanup_loading_screen(
    mut commands: Commands,
    loading_screen_query: Query<Entity, With<LoadingScreen>>,
    loading_camera_query: Query<Entity, With<LoadingCamera>>,
) {
    // Remove the loading screen
    for entity in loading_screen_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    
    // Remove the loading camera
    for entity in loading_camera_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}