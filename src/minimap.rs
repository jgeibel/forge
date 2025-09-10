use bevy::prelude::*;
use bevy::window::{WindowMode, PresentMode};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::sprite::{ColorMaterial, ColorMesh2dBundle};
use crate::world::{WorldGenerator, Biome};
use crate::camera::PlayerCamera;
use crate::loading::GameState;
use crate::planet::config::PLANET_SIZE_CHUNKS;

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Playing), setup_minimap)
            .add_systems(Update, (
                update_minimap_player_position,
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
    
    // Create minimap window
    let minimap_window = commands.spawn((
        Window {
            title: "Forge - World Map".to_string(),
            resolution: (PLANET_SIZE_CHUNKS as f32, PLANET_SIZE_CHUNKS as f32).into(),
            position: WindowPosition::Centered(MonitorSelection::Primary),
            mode: WindowMode::Windowed,
            present_mode: PresentMode::AutoVsync,
            ..default()
        },
        MinimapWindow,
    )).id();
    
    // Generate world map texture
    let map_size = PLANET_SIZE_CHUNKS as u32;
    let mut image_data = vec![0u8; (map_size * map_size * 4) as usize];
    
    // Sample biomes to verify they're working
    let mut biome_counts = std::collections::HashMap::new();
    
    // Sample world at chunk centers to create overview
    for chunk_z in 0..map_size {
        for chunk_x in 0..map_size {
            let world_x = (chunk_x * 32 + 16) as f32;
            let world_z = (chunk_z * 32 + 16) as f32;
            
            // Get biome at this position
            let biome = world_gen.get_biome(world_x, world_z);
            *biome_counts.entry(format!("{:?}", biome)).or_insert(0) += 1;
            let color = get_biome_color(biome);
            
            let pixel_index = ((chunk_z * map_size + chunk_x) * 4) as usize;
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
    let player_mesh = meshes.add(Circle::new(10.0));  // Circle is clearer than square
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
        Vec2::new(0.0, 15.0),    // Top point (front of arrow)
        Vec2::new(-8.0, -8.0),   // Bottom left
        Vec2::new(8.0, -8.0),    // Bottom right
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
    
    // Store minimap data
    commands.insert_resource(MinimapData {
        window_entity: Some(minimap_window),
        player_position: Vec2::ZERO,
    });
    
    info!("Minimap window created");
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
    
    // Debug: Check if we're getting correct coordinates
    static mut LAST_LOG: f32 = 0.0;
    unsafe {
        if (world_pos.y - LAST_LOG).abs() > 5.0 {
            info!("Player position: X={:.1}, Y={:.1}, Z={:.1}", world_pos.x, world_pos.y, world_pos.z);
            LAST_LOG = world_pos.y;
        }
    }
    
    // Convert world position to chunk position - ONLY X and Z
    let chunk_x = (world_pos.x / 32.0).floor();
    let chunk_z = (world_pos.z / 32.0).floor();
    
    // Wrap to planet bounds
    let wrapped_chunk_x = chunk_x.rem_euclid(PLANET_SIZE_CHUNKS as f32);
    let wrapped_chunk_z = chunk_z.rem_euclid(PLANET_SIZE_CHUNKS as f32);
    
    // Convert to minimap coordinates (centered origin)
    let map_x = wrapped_chunk_x - (PLANET_SIZE_CHUNKS as f32 / 2.0);
    let map_y = (PLANET_SIZE_CHUNKS as f32 / 2.0) - wrapped_chunk_z; // Flip Z for screen Y
    
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