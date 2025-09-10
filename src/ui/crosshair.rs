use bevy::prelude::*;

#[derive(Component)]
pub struct Crosshair;

pub fn setup_crosshair(mut commands: Commands) {
    // Root container for crosshair
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
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        Crosshair,
    ))
    .with_children(|parent| {
        // Crosshair container
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(20.0),
                height: Val::Px(20.0),
                position_type: PositionType::Relative,
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|crosshair| {
            // Horizontal line
            crosshair.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    left: Val::Px(0.0),
                    top: Val::Px(9.0), // Center vertically
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                ..default()
            });
            
            // Vertical line
            crosshair.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    left: Val::Px(9.0), // Center horizontally
                    top: Val::Px(0.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                ..default()
            });
            
            // Center dot (optional, for better visibility)
            crosshair.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(2.0),
                    height: Val::Px(2.0),
                    left: Val::Px(9.0),
                    top: Val::Px(9.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                ..default()
            });
        });
    });
}