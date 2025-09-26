use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::camera::PlayerCamera;
use crate::world::config::WorldGenConfig;
use crate::world::generator::WorldGenerator;

const MAP_WIDTH: u32 = 512;
const MAP_HEIGHT: u32 = 256;
const ARROW_SIZE: f32 = 24.0;

pub struct MiniMapPlugin;

impl Plugin for MiniMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiniMapState>()
            .add_systems(OnEnter(crate::loading::GameState::Playing), setup_minimap)
            .add_systems(OnExit(crate::loading::GameState::Playing), cleanup_minimap)
            .add_systems(
                Update,
                (update_minimap, handle_minimap_clicks)
                    .run_if(in_state(crate::loading::GameState::Playing)),
            );
    }
}

#[derive(Resource, Default)]
struct MiniMapState {
    map_handle: Option<Handle<Image>>,
    arrow_handle: Option<Handle<Image>>,
    root_entity: Option<Entity>,
    arrow_entity: Option<Entity>,
    planet_size: f32,
    width: f32,
    height: f32,
}

#[derive(Component)]
struct MiniMapArrow;

#[derive(Component)]
struct MiniMapSurface;

fn setup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<MiniMapState>,
    generator: Res<WorldGenerator>,
) {
    let map_handle = images.add(build_map_image(&generator, MAP_WIDTH, MAP_HEIGHT));
    let arrow_handle = images.add(build_arrow_image(32));

    let width = MAP_WIDTH as f32;
    let height = MAP_HEIGHT as f32;

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.82)),
            border_color: BorderColor(Color::srgba(0.4, 0.5, 0.6, 0.9)),
            ..default()
        })
        .id();

    let mut arrow_entity: Option<Entity> = None;

    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(width),
                        height: Val::Px(height),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ..default()
                },
                MiniMapSurface,
            ))
            .with_children(|map_parent| {
                map_parent.spawn(ImageBundle {
                    style: Style {
                        width: Val::Px(width),
                        height: Val::Px(height),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    image: UiImage::new(map_handle.clone()),
                    ..default()
                });

                let arrow = map_parent
                    .spawn(ImageBundle {
                        style: Style {
                            width: Val::Px(ARROW_SIZE),
                            height: Val::Px(ARROW_SIZE),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        image: UiImage::new(arrow_handle.clone()),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                        ..default()
                    })
                    .insert(MiniMapArrow)
                    .id();

                arrow_entity = Some(arrow);
            });
    });

    let arrow_entity = arrow_entity.expect("minimap arrow spawned");

    state.map_handle = Some(map_handle);
    state.arrow_handle = Some(arrow_handle);
    state.root_entity = Some(root);
    state.arrow_entity = Some(arrow_entity);
    state.planet_size = generator.planet_size() as f32;
    state.width = width;
    state.height = height;
}

fn cleanup_minimap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<MiniMapState>,
) {
    if let Some(root) = state.root_entity.take() {
        commands.entity(root).despawn_recursive();
    }

    if let Some(handle) = state.map_handle.take() {
        images.remove(&handle);
    }

    if let Some(handle) = state.arrow_handle.take() {
        images.remove(&handle);
    }

    state.arrow_entity = None;
}

fn update_minimap(
    mut arrow_query: Query<
        (&mut Style, &mut Transform),
        (With<MiniMapArrow>, Without<PlayerCamera>),
    >,
    player_query: Query<&Transform, With<PlayerCamera>>,
    state: Res<MiniMapState>,
) {
    let Some(arrow_entity) = state.arrow_entity else {
        return;
    };

    let Ok((mut style, mut transform)) = arrow_query.get_mut(arrow_entity) else {
        return;
    };

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let planet_size = state.planet_size.max(1.0);
    let x = player_transform.translation.x.rem_euclid(planet_size);
    let z = player_transform.translation.z.rem_euclid(planet_size);

    if x.is_nan() || z.is_nan() {
        return;
    }

    let u = (x / planet_size).clamp(0.0, 1.0);
    let v = (z / planet_size).clamp(0.0, 1.0);

    let map_x = u * state.width;
    let map_y = (1.0 - v) * state.height;

    style.left = Val::Px(map_x - ARROW_SIZE * 0.5);
    style.top = Val::Px(map_y - ARROW_SIZE * 0.5);

    let forward = player_transform.forward();
    let yaw = forward.x.atan2(forward.z);
    transform.rotation = Quat::from_rotation_z(-yaw as f32);
}

fn handle_minimap_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    state: Res<MiniMapState>,
    generator: Res<WorldGenerator>,
    mut player_query: Query<&mut Transform, With<PlayerCamera>>,
    surface_query: Query<(&Node, &GlobalTransform), With<MiniMapSurface>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((node, transform)) = surface_query.get_single() else {
        return;
    };

    let rect = node.logical_rect(transform);
    if !rect.contains(cursor_position) {
        return;
    }

    let local = cursor_position - rect.min;
    let width = rect.width().max(1.0);
    let height = rect.height().max(1.0);
    let u = (local.x / width).clamp(0.0, 1.0);
    let v = (local.y / height).clamp(0.0, 1.0);

    let planet_size = state.planet_size.max(1.0);
    let world_x = u * planet_size;
    let world_z = (1.0 - v) * planet_size;
    let world_y = generator.get_height(world_x, world_z) + 5.0;

    if let Ok(mut transform) = player_query.get_single_mut() {
        transform.translation = Vec3::new(world_x, world_y, world_z);
        info!(
            "Teleported player to minimap location ({:.1}, {:.1}, {:.1})",
            world_x, world_y, world_z
        );
    }
}

fn build_map_image(generator: &WorldGenerator, width: u32, height: u32) -> Image {
    let usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (width * height * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        usage,
    );

    let planet_size = generator.planet_size() as f32;
    let data = &mut image.data;

    let config = generator.config();

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = 1.0 - y as f32 / height as f32;
            let world_x = u * planet_size;
            let world_z = v * planet_size;

            let height_value = generator.get_height(world_x, world_z);
            let biome = generator.get_biome(world_x, world_z);
            let base_color = generator.preview_color(world_x, world_z, biome, height_value);
            let color = apply_height_shading(base_color, height_value, config);

            let idx = ((y * width + x) * 4) as usize;
            data[idx..idx + 4].copy_from_slice(&color);
        }
    }

    image
}

fn apply_height_shading(base_color: [u8; 4], height: f32, config: &WorldGenConfig) -> [u8; 4] {
    let sea_level = config.sea_level;

    if height <= sea_level {
        return base_color;
    }

    let elevation = height - sea_level;
    let max_elevation = config.mountain_height + config.highland_bonus;

    let shade_factor = if elevation < 2.0 {
        0.5 + (elevation / 2.0) * 0.1
    } else if elevation < 5.0 {
        0.6 + (elevation - 2.0) / 3.0 * 0.15
    } else if elevation < 10.0 {
        0.75 + (elevation - 5.0) / 5.0 * 0.25
    } else if elevation < 20.0 {
        1.0 + (elevation - 10.0) / 10.0 * 0.3
    } else if elevation < 40.0 {
        1.3 + (elevation - 20.0) / 20.0 * 0.3
    } else if elevation < 80.0 {
        1.6 + (elevation - 40.0) / 40.0 * 0.2
    } else {
        let normalized = ((elevation - 80.0) / (max_elevation - 80.0)).clamp(0.0, 1.0);
        1.8 + normalized * 0.7
    };

    let mut shaded = [0u8; 4];
    for i in 0..3 {
        let value = base_color[i] as f32 * shade_factor;
        shaded[i] = value.clamp(0.0, 255.0) as u8;
    }
    shaded[3] = base_color[3];
    shaded
}

fn build_arrow_image(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let apex = (size / 2, 0);
    let left = (0, size - 1);
    let right = (size - 1, size - 1);

    for y in 0..size {
        for x in 0..size {
            if point_in_triangle((x, y), apex, left, right) {
                let idx = ((y * size + x) * 4) as usize;
                data[idx] = 220;
                data[idx + 1] = 60;
                data[idx + 2] = 60;
                data[idx + 3] = 255;
            }
        }
    }

    let usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        usage,
    )
}

fn point_in_triangle(p: (u32, u32), a: (u32, u32), b: (u32, u32), c: (u32, u32)) -> bool {
    let pa = ((a.0 as f32 - p.0 as f32), (a.1 as f32 - p.1 as f32));
    let pb = ((b.0 as f32 - p.0 as f32), (b.1 as f32 - p.1 as f32));
    let pc = ((c.0 as f32 - p.0 as f32), (c.1 as f32 - p.1 as f32));

    let cross_ab = pa.0 * pb.1 - pa.1 * pb.0;
    let cross_bc = pb.0 * pc.1 - pb.1 * pc.0;
    let cross_ca = pc.0 * pa.1 - pc.1 * pa.0;

    let has_neg = cross_ab < 0.0 || cross_bc < 0.0 || cross_ca < 0.0;
    let has_pos = cross_ab > 0.0 || cross_bc > 0.0 || cross_ca > 0.0;

    !(has_neg && has_pos)
}
