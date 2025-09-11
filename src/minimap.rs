use bevy::prelude::*;
use bevy::window::{WindowMode, PresentMode};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::sprite::{ColorMaterial, ColorMesh2dBundle};
use crate::world::{WorldGenerator, Biome};
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::config::PLANET_SIZE_CHUNKS;
use crate::celestial::time::GameTime;
use crate::celestial::sun::calculate_local_sun_angle;

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_minimap)
            .add_systems(Update, (
                update_minimap_player_position,
                update_day_night_overlay,
                handle_minimap_window_close,
            ).run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component)]
struct MinimapWindow;

#[derive(Component)]
struct MinimapCamera;

#[derive(Component)]
struct PlayerMarker;

#[derive(Component)]
struct WorldMap;

#[derive(Component)]
struct DayNightOverlay;

#[derive(Resource)]
struct MinimapData {
    window_entity: Option<Entity>,
    player_position: Vec2,
}

impl Default for MinimapData {
    fn default() -> Self {
        Self {
            window_entity: None,
            player_position: Vec2::ZERO,
        }
    }
}

fn setup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    world_gen: Res<WorldGenerator>,
) {
    info!("Setting up minimap window");
    
    // Limit minimap size to reasonable bounds
    // Use 1024x1024 for the window and downsample the map
    const MAX_MAP_SIZE: u32 = 1024;
    let scale_factor = (PLANET_SIZE_CHUNKS as u32 / MAX_MAP_SIZE).max(1);
    let map_size = (PLANET_SIZE_CHUNKS as u32 / scale_factor).min(MAX_MAP_SIZE);
    
    // Create minimap window with reasonable size
    let minimap_window = commands.spawn((
        Window {
            title: "Forge - World Map".to_string(),
            resolution: (map_size as f32, map_size as f32).into(),
            position: WindowPosition::Centered(MonitorSelection::Primary),
            mode: WindowMode::Windowed,
            present_mode: PresentMode::AutoVsync,
            ..default()
        },
        MinimapWindow,
    )).id();
    
    // Generate world map texture
    let mut image_data = vec![0u8; (map_size * map_size * 4) as usize];
    
    // Sample biomes to verify they're working
    let mut biome_counts = std::collections::HashMap::new();
    
    // Sample world at chunk centers to create overview
    for pixel_z in 0..map_size {
        for pixel_x in 0..map_size {
            // Scale up pixel coordinates to world chunk coordinates
            let chunk_x = pixel_x * scale_factor;
            let chunk_z = pixel_z * scale_factor;
            let world_x = (chunk_x * 32 + 16) as f32;
            let world_z = (chunk_z * 32 + 16) as f32;
            
            // Get biome at this position
            let biome = world_gen.get_biome(world_x, world_z);
            *biome_counts.entry(format!("{:?}", biome)).or_insert(0) += 1;
            let color = get_biome_color(biome);
            
            let pixel_index = ((pixel_z * map_size + pixel_x) * 4) as usize;
            let rgba = color.to_srgba();
            image_data[pixel_index] = (rgba.red * 255.0) as u8;
            image_data[pixel_index + 1] = (rgba.green * 255.0) as u8;
            image_data[pixel_index + 2] = (rgba.blue * 255.0) as u8;
            image_data[pixel_index + 3] = 255;
        }
    }
    
    // Log biome distribution
    info!("Biome distribution in minimap:");
    for (biome, count) in biome_counts.iter() {
        info!("  {}: {} chunks", biome, count);
    }
    
    let map_image = Image::new(
        Extent3d {
            width: map_size,
            height: map_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    
    let map_texture = images.add(map_image);
    
    // Create orthographic camera for minimap
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                target: bevy::render::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(minimap_window)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1000.0),
            ..default()
        },
        MinimapCamera,
    ));
    
    // Create world map sprite (no render layers for simplicity)
    commands.spawn((
        SpriteBundle {
            texture: map_texture,
            transform: Transform::from_scale(Vec3::splat(1.0)),
            sprite: Sprite {
                custom_size: Some(Vec2::new(map_size as f32, map_size as f32)),
                ..default()
            },
            ..default()
        },
        WorldMap,
    ));
    
    // Create a colored mesh for the player marker (more reliable than sprite color)
    let player_mesh = meshes.add(Circle::new(20.0));  // Increased size for better visibility on large map
    let player_material = materials.add(ColorMaterial::from(Color::srgb(1.0, 0.0, 0.0)));
    
    // Create player position marker (bigger and more visible)
    commands.spawn((
        ColorMesh2dBundle {
            mesh: player_mesh.clone().into(),
            material: player_material,
            transform: Transform::from_xyz(0.0, 0.0, 10.0),
            ..default()
        },
        PlayerMarker,
    ));
    
    // Create a proper arrow shape for direction indicator
    // Triangle pointing upward (will be rotated based on player direction)
    let arrow_mesh = meshes.add(Triangle2d::new(
        Vec2::new(0.0, 30.0),    // Top point (front of arrow) - doubled size
        Vec2::new(-16.0, -16.0),   // Bottom left - doubled size
        Vec2::new(16.0, -16.0),    // Bottom right - doubled size
    ));
    let arrow_material = materials.add(ColorMaterial::from(Color::srgb(1.0, 1.0, 0.0)));
    
    commands.spawn((
        ColorMesh2dBundle {
            mesh: arrow_mesh.into(),
            material: arrow_material,
            transform: Transform::from_xyz(0.0, 0.0, 11.0), // Same position as player, just higher Z
            ..default()
        },
        PlayerMarker,
    ));
    
    // Create day/night overlay texture
    let overlay_data = vec![0u8; (map_size * map_size * 4) as usize];
    let overlay_image = Image::new(
        Extent3d {
            width: map_size,
            height: map_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        overlay_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let overlay_texture = images.add(overlay_image);
    
    // Create day/night overlay sprite (higher Z than map, lower than markers)
    commands.spawn((
        SpriteBundle {
            texture: overlay_texture,
            transform: Transform::from_xyz(0.0, 0.0, 5.0),  // Between map (0) and player marker (10)
            sprite: Sprite {
                custom_size: Some(Vec2::new(map_size as f32, map_size as f32)),
                ..default()
            },
            ..default()
        },
        DayNightOverlay,
    ));
    
    // Store minimap data
    commands.insert_resource(MinimapData {
        window_entity: Some(minimap_window),
        player_position: Vec2::ZERO,
    });
    
    info!("Minimap window created with day/night overlay");
}

fn update_minimap_player_position(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut marker_query: Query<&mut Transform, (With<PlayerMarker>, Without<PlayerCamera>)>,
    mut minimap_data: ResMut<MinimapData>,
) {
    let Ok(player_transform) = camera_query.get_single() else {
        return;
    };
    
    // Only use X and Z coordinates, ignore Y (altitude)
    let world_pos = player_transform.translation;
    
    // Convert world position to chunk position - ONLY X and Z
    let chunk_x = (world_pos.x / 32.0).floor();
    let chunk_z = (world_pos.z / 32.0).floor();
    
    // Wrap to planet bounds
    let wrapped_chunk_x = chunk_x.rem_euclid(PLANET_SIZE_CHUNKS as f32);
    let wrapped_chunk_z = chunk_z.rem_euclid(PLANET_SIZE_CHUNKS as f32);
    
    // The minimap is scaled down - we need to account for this
    const MAX_MAP_SIZE: u32 = 1024;
    let scale_factor = (PLANET_SIZE_CHUNKS as u32 / MAX_MAP_SIZE).max(1) as f32;
    let map_size = (PLANET_SIZE_CHUNKS as u32 / scale_factor as u32).min(MAX_MAP_SIZE) as f32;
    
    // Convert to minimap coordinates (scaled and centered)
    let scaled_chunk_x = wrapped_chunk_x / scale_factor;
    let scaled_chunk_z = wrapped_chunk_z / scale_factor;
    
    // Convert to centered minimap coordinates
    let map_x = scaled_chunk_x - (map_size / 2.0);
    let map_y = (map_size / 2.0) - scaled_chunk_z; // Flip Z for screen Y
    
    // Debug: Check if we're getting correct coordinates
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 120 == 0 {  // Log every 2 seconds at 60fps
            info!("Minimap - World: ({:.0}, {:.0}) -> Chunk: ({:.0}, {:.0}) -> Scaled: ({:.0}, {:.0}) -> Map: ({:.0}, {:.0}), Scale: {}",
                world_pos.x, world_pos.z,
                chunk_x, chunk_z,
                scaled_chunk_x, scaled_chunk_z,
                map_x, map_y,
                scale_factor
            );
        }
    }
    
    // Get player's facing direction (yaw rotation around Y axis)
    let forward = player_transform.forward();
    // In Bevy, forward.x is right/left, forward.z is forward/backward
    // The minimap Y is flipped (world Z maps to -minimap Y)
    // So when player faces +Z (forward in world), arrow should point down on minimap
    // When player faces +X (right in world), arrow should point right on minimap
    // Add PI to flip 180 degrees, then add PI/2 for coordinate system alignment
    let angle = -(forward.z.atan2(forward.x)) + std::f32::consts::PI * 1.5;
    
    // Update all markers (both position dot and direction arrow)
    let mut index = 0;
    for mut marker_transform in marker_query.iter_mut() {
        // Update position for both markers
        marker_transform.translation.x = map_x;
        marker_transform.translation.y = map_y;
        
        // The second marker (arrow) gets rotation
        if index == 1 {
            // Arrow is at same position as player dot, just higher Z
            // Rotate based on facing direction
            marker_transform.rotation = Quat::from_rotation_z(angle);
        }
        
        index += 1;
    }
    
    minimap_data.player_position = Vec2::new(wrapped_chunk_x, wrapped_chunk_z);
}

fn handle_minimap_window_close(
    mut closed: EventReader<bevy::window::WindowClosed>,
    minimap_query: Query<Entity, With<MinimapWindow>>,
    mut minimap_data: ResMut<MinimapData>,
) {
    for event in closed.read() {
        if let Ok(_) = minimap_query.get(event.window) {
            info!("Minimap window closed");
            minimap_data.window_entity = None;
        }
    }
}

fn update_day_night_overlay(
    game_time: Res<GameTime>,
    mut overlay_query: Query<&Handle<Image>, With<DayNightOverlay>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(overlay_handle) = overlay_query.get_single_mut() else {
        return;
    };
    
    let Some(overlay_image) = images.get_mut(overlay_handle) else {
        return;
    };
    
    // Get image dimensions
    let width = overlay_image.texture_descriptor.size.width;
    let height = overlay_image.texture_descriptor.size.height;
    let scale_factor = (PLANET_SIZE_CHUNKS as u32 / width).max(1);
    
    // Update overlay data every frame (or less frequently if needed)
    for pixel_z in 0..height {
        for pixel_x in 0..width {
            // Scale up pixel coordinates to world chunk coordinates
            let chunk_x = pixel_x * scale_factor;
            let chunk_z = pixel_z * scale_factor;
            let world_x = (chunk_x * 32 + 16) as f32;
            let world_z = (chunk_z * 32 + 16) as f32;
            
            // Calculate local sun angle for this position
            let sun_angle = calculate_local_sun_angle(world_x, world_z, &game_time);
            
            // Determine shadow intensity based on sun angle
            let shadow_alpha = if sun_angle > 0.1 {
                // Full daylight
                0.0
            } else if sun_angle > 0.0 {
                // Sunrise/sunset transition
                (0.1 - sun_angle) / 0.1 * 0.7
            } else if sun_angle > -0.2 {
                // Twilight
                0.7 + ((-0.2 - sun_angle) / 0.2) * 0.2
            } else {
                // Night
                0.9
            };
            
            let pixel_index = ((pixel_z * width + pixel_x) * 4) as usize;
            overlay_image.data[pixel_index] = 0;      // R
            overlay_image.data[pixel_index + 1] = 0;  // G  
            overlay_image.data[pixel_index + 2] = 0;  // B
            overlay_image.data[pixel_index + 3] = (shadow_alpha * 255.0) as u8; // A
        }
    }
}

fn get_biome_color(biome: Biome) -> Color {
    // Use more distinct colors for debugging
    match biome {
        Biome::Ocean | Biome::DeepOcean => Color::srgb(0.0, 0.0, 0.8), // Bright blue
        Biome::FrozenOcean => Color::srgb(0.7, 0.85, 1.0), // Light blue
        Biome::Beach => Color::srgb(1.0, 1.0, 0.5), // Yellow
        Biome::Plains => Color::srgb(0.3, 0.8, 0.3), // Bright green
        Biome::Forest => Color::srgb(0.0, 0.4, 0.0), // Dark green
        Biome::Desert => Color::srgb(1.0, 0.8, 0.3), // Sandy yellow
        Biome::Mountains => Color::srgb(0.5, 0.5, 0.5), // Gray
        Biome::SnowyMountains => Color::srgb(1.0, 1.0, 1.0), // White
        Biome::Tundra => Color::srgb(0.8, 0.9, 1.0), // Very light blue
        Biome::Jungle => Color::srgb(0.0, 0.6, 0.0), // Medium green
        Biome::Swamp => Color::srgb(0.3, 0.4, 0.2), // Dark olive
        Biome::Savanna => Color::srgb(0.8, 0.7, 0.3), // Brown-yellow
        Biome::Mesa => Color::srgb(0.8, 0.4, 0.2), // Orange-brown
        Biome::IceSpikes => Color::srgb(0.85, 0.95, 1.0), // Ice blue
        Biome::IcePlains => Color::srgb(0.95, 0.95, 1.0), // Almost white
        Biome::Taiga => Color::srgb(0.1, 0.5, 0.3), // Pine green
    }
}