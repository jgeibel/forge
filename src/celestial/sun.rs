use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use std::f32::consts::PI;
use crate::planet::config::PLANET_SIZE_BLOCKS;
use crate::celestial::time::GameTime;
use crate::camera::PlayerCamera;

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct SunDisc;

#[derive(Resource, Debug, Clone)]
pub struct SunPosition {
    pub direction: Vec3,  // Normalized direction TO the sun
    pub angle_from_horizon: f32,  // Angle above horizon (radians)
}

impl Default for SunPosition {
    fn default() -> Self {
        Self {
            direction: Vec3::new(1.0, 1.0, 0.0).normalize(),
            angle_from_horizon: PI / 4.0,
        }
    }
}

pub struct SunPlugin;

impl Plugin for SunPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SunPosition>()
            .add_systems(Startup, spawn_sun)
            .add_systems(Update, (
                update_sun_position,
                update_sun_light,
                update_sun_disc_position,
            ).chain());
    }
}

fn spawn_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the main directional light that acts as the sun
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 10000.0,
                shadows_enabled: false,  // Disabled due to persistent artifacts
                shadow_depth_bias: 0.5,
                shadow_normal_bias: 0.7,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 100.0, 0.0)
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
        Sun,
    ));
    
    // Spawn visual sun disc
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(100.0)),  // Larger sphere for better visibility
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.95, 0.7),
                emissive: LinearRgba::new(15.0, 14.0, 8.0, 1.0),  // Brighter glow
                unlit: true,
                alpha_mode: AlphaMode::Opaque,  // Opaque for better visibility
                ..default()
            }),
            transform: Transform::from_xyz(1000.0, 500.0, 0.0),  // Start further away and higher
            ..default()
        },
        SunDisc,
        NotShadowCaster,  // Sun disc shouldn't cast shadows
    ));
}

fn update_sun_position(
    game_time: Res<GameTime>,
    mut sun_position: ResMut<SunPosition>,
) {
    // Calculate sun position based on time of day
    // The sun moves from east to west (positive X to negative X)
    let hour_angle = (game_time.current_hour - 12.0) * 15.0 * PI / 180.0;  // 15 degrees per hour
    
    // Get sun declination for seasonal variation
    let declination = game_time.get_sun_declination();
    
    // Calculate sun direction (simplified model)
    // At equator, sun moves in a perfect arc
    // Declination adjusts the path for seasons
    let sun_altitude = PI / 2.0 - hour_angle.abs();  // Highest at noon
    let seasonal_adjustment = declination * (PI / 2.0 - hour_angle.abs()) / (PI / 2.0);
    let adjusted_altitude = (sun_altitude + seasonal_adjustment).max(0.0);
    
    // Convert to 3D direction
    sun_position.direction = Vec3::new(
        -hour_angle.sin(),  // East-West component
        adjusted_altitude.sin(),  // Vertical component
        -hour_angle.cos() * declination.cos(),  // North-South component (seasonal)
    ).normalize();
    
    sun_position.angle_from_horizon = adjusted_altitude;
    
    // Debug log sun position occasionally
    static mut LAST_LOG: f32 = 0.0;
    unsafe {
        if (game_time.current_hour - LAST_LOG).abs() > 1.0 {
            info!("Sun Update - Hour: {:.1}, Angle: {:.2} rad ({:.1}°), Dir: ({:.2}, {:.2}, {:.2})", 
                game_time.current_hour, 
                adjusted_altitude, 
                adjusted_altitude * 180.0 / PI,
                sun_position.direction.x,
                sun_position.direction.y,
                sun_position.direction.z
            );
            LAST_LOG = game_time.current_hour;
        }
    }
}

fn update_sun_light(
    sun_position: Res<SunPosition>,
    mut sun_query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    for (mut transform, mut light) in sun_query.iter_mut() {
        // Update sun direction
        *transform = Transform::from_translation(sun_position.direction * 1000.0)
            .looking_at(Vec3::ZERO, Vec3::Y);
        
        // Calculate light intensity and color based on sun angle
        let angle = sun_position.angle_from_horizon;
        
        if angle <= 0.0 {
            // Night time
            light.illuminance = 100.0;  // Moonlight
            light.color = Color::srgb(0.4, 0.4, 0.6);  // Bluish moonlight
            ambient_light.brightness = 20.0;
            ambient_light.color = Color::srgb(0.1, 0.1, 0.2);  // Dark blue night
        } else if angle < 0.1 {
            // Sunrise/sunset (golden hour)
            let t = angle / 0.1;  // 0 to 1 during transition
            light.illuminance = 100.0 + 9900.0 * t;
            light.color = Color::srgb(
                1.0,
                0.4 + 0.6 * t,
                0.2 + 0.8 * t,
            );  // Orange to white
            ambient_light.brightness = 20.0 + 130.0 * t;
            ambient_light.color = Color::srgb(
                0.3 + 0.7 * t,
                0.2 + 0.8 * t,
                0.2 + 0.8 * t,
            );
        } else {
            // Day time
            let intensity = (angle / (PI / 2.0)).min(1.0);  // Max at noon
            light.illuminance = 10000.0 + 5000.0 * intensity;
            light.color = Color::WHITE;
            ambient_light.brightness = 150.0 + 50.0 * intensity;
            ambient_light.color = Color::srgb(0.9, 0.95, 1.0);  // Slight blue tint for sky
        }
    }
}

// Helper function to calculate sun angle for a specific position on the planet
pub fn calculate_local_sun_angle(
    world_x: f32,
    world_z: f32,
    game_time: &GameTime,
) -> f32 {
    // Convert world position to longitude (0 to 2*PI)
    let longitude = (world_x / PLANET_SIZE_BLOCKS as f32) * 2.0 * PI;
    
    // Convert world position to latitude (-PI/2 to PI/2)
    let latitude = ((world_z / PLANET_SIZE_BLOCKS as f32) - 0.5) * PI;
    
    // Calculate local solar time based on longitude
    let local_hour = game_time.current_hour + (longitude / PI) * 12.0;
    let local_hour = if local_hour >= 24.0 { local_hour - 24.0 } else { local_hour };
    
    // Calculate sun elevation angle
    let hour_angle = (local_hour - 12.0) * 15.0 * PI / 180.0;
    let declination = game_time.get_sun_declination();
    
    // Solar elevation formula
    let elevation = (latitude.sin() * declination.sin() +
                    latitude.cos() * declination.cos() * hour_angle.cos()).asin();
    
    elevation
}

// Update visual sun disc position based on player location
fn update_sun_disc_position(
    player_query: Query<&Transform, With<PlayerCamera>>,
    mut sun_disc_query: Query<(&mut Transform, &mut Handle<StandardMaterial>, &mut Visibility), (With<SunDisc>, Without<PlayerCamera>)>,
    game_time: Res<GameTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    
    let Ok((mut sun_transform, material_handle, mut visibility)) = sun_disc_query.get_single_mut() else {
        return;
    };
    
    // Calculate local sun angle for player position
    let player_pos = player_transform.translation;
    let local_sun_angle = calculate_local_sun_angle(player_pos.x, player_pos.z, &game_time);
    
    // Calculate sun position relative to player
    let hour_angle = (game_time.current_hour - 12.0) * 15.0 * PI / 180.0;
    let distance = 2000.0;  // Distance from player to sun disc
    
    // Position sun based on actual elevation angle (allow negative for below horizon)
    // Sun moves from east (positive X) to west (negative X)
    let sun_x = -hour_angle.sin() * distance;
    let sun_y = local_sun_angle.sin() * distance;  // Use actual angle, can go negative
    let sun_z = -hour_angle.cos() * distance * 0.3;  // Reduced Z movement
    
    // Update sun position relative to player
    sun_transform.translation = player_pos + Vec3::new(sun_x, sun_y, sun_z);
    
    // Make sun billboard towards camera but maintain size
    // Calculate direction from sun to player for billboarding
    let to_player = (player_pos - sun_transform.translation).normalize();
    sun_transform.rotation = Quat::from_rotation_arc(Vec3::Z, to_player);
    
    // Debug log sun position occasionally
    static mut LAST_SUN_LOG: f32 = 0.0;
    unsafe {
        if (game_time.current_hour - LAST_SUN_LOG).abs() > 0.5 {
            info!("Sun Disc - Hour: {:.1}, Angle: {:.2}°, Pos: ({:.0}, {:.0}, {:.0}), Player: ({:.0}, {:.0}, {:.0})",
                game_time.current_hour,
                local_sun_angle * 180.0 / PI,
                sun_transform.translation.x,
                sun_transform.translation.y,
                sun_transform.translation.z,
                player_pos.x,
                player_pos.y,
                player_pos.z
            );
            LAST_SUN_LOG = game_time.current_hour;
        }
    }
    
    // Hide sun when below horizon
    *visibility = if local_sun_angle <= -0.1 {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    
    // Update sun color based on elevation angle
    if let Some(material) = materials.get_mut(&*material_handle) {
        let (base_color, emissive_strength) = if local_sun_angle <= -0.1 {
            // Well below horizon - completely hide sun (redundant with visibility but keeps material consistent)
            (Color::srgba(0.0, 0.0, 0.0, 0.0), 0.0)
        } else if local_sun_angle <= 0.0 {
            // Just below horizon - deep red
            let t = (local_sun_angle + 0.2) / 0.2;
            (Color::srgb(0.8, 0.2, 0.1).with_alpha(t), 2.0 * t)
        } else if local_sun_angle < 0.1 {
            // Sunrise/sunset - orange to yellow gradient
            let t = local_sun_angle / 0.1;
            (
                Color::srgb(
                    1.0,
                    0.2 + 0.7 * t,
                    0.1 + 0.6 * t,
                ),
                3.0 + 2.0 * t
            )
        } else {
            // Daytime - bright yellow-white
            let intensity = (local_sun_angle / (PI / 2.0)).min(1.0);
            (
                Color::srgb(
                    1.0,
                    0.95 + 0.05 * intensity,
                    0.7 + 0.3 * intensity,
                ),
                5.0 + 3.0 * intensity
            )
        };
        
        material.base_color = base_color;
        material.emissive = LinearRgba::from(base_color) * emissive_strength;
    }
}