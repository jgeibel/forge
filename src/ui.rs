use bevy::prelude::*;
use crate::camera::PlayerCamera;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_ui)
            .add_systems(Update, update_coordinates_display);
    }
}

#[derive(Component)]
struct CoordinatesText;

fn setup_ui(mut commands: Commands) {
    // Crosshair
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        Name::new("Crosshair Container"),
    ))
    .with_children(|parent| {
        // Horizontal line
        parent.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Px(16.0),
                height: Val::Px(2.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
            ..default()
        });
        
        // Vertical line
        parent.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Px(2.0),
                height: Val::Px(16.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
            ..default()
        });
    });
    
    // Coordinates display
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Position: ",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::new(
                "X: 0, Y: 0, Z: 0",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ),
            TextSection::new(
                "\nWrapped: ",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::new(
                "X: 0, Z: 0",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(0.9, 0.9, 0.2),
                    ..default()
                },
            ),
            TextSection::new(
                "\n[Origin at (0,0) marked with stone platform]",
                TextStyle {
                    font_size: 16.0,
                    color: Color::srgb(0.6, 0.6, 0.6),
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        })
        .with_background_color(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        CoordinatesText,
    ));
}

fn update_coordinates_display(
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut text_query: Query<&mut Text, With<CoordinatesText>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };
    
    let pos = camera_transform.translation;
    
    // Calculate wrapped coordinates (planet is 2048x2048)
    let wrapped_x = pos.x.rem_euclid(2048.0);
    let wrapped_z = pos.z.rem_euclid(2048.0);
    
    // Update actual position
    text.sections[1].value = format!(
        "X: {:.0}, Y: {:.0}, Z: {:.0}",
        pos.x, pos.y, pos.z
    );
    
    // Update wrapped position
    text.sections[3].value = format!(
        "X: {:.0}, Z: {:.0}",
        wrapped_x, wrapped_z
    );
}