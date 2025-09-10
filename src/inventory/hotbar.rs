use bevy::prelude::*;
use crate::block::BlockType;

const HOTBAR_SLOTS: usize = 9;

#[derive(Resource)]
pub struct Hotbar {
    pub slots: [Option<BlockType>; HOTBAR_SLOTS],
    pub selected_slot: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        let mut slots = [None; HOTBAR_SLOTS];
        // Pre-fill with common blocks
        slots[0] = Some(BlockType::Stone);
        slots[1] = Some(BlockType::Dirt);
        slots[2] = Some(BlockType::Grass);
        slots[3] = Some(BlockType::Sand);
        slots[4] = Some(BlockType::Snow);
        slots[5] = Some(BlockType::Ice);
        slots[6] = Some(BlockType::PackedIce);
        slots[7] = Some(BlockType::Bedrock);
        slots[8] = Some(BlockType::Air); // For erasing
        
        Self {
            slots,
            selected_slot: 0,
        }
    }
}

impl Hotbar {
    pub fn get_selected_block(&self) -> Option<BlockType> {
        self.slots[self.selected_slot]
    }
}

#[derive(Component)]
pub struct HotbarUI;

#[derive(Component)]
pub struct HotbarSlot {
    pub index: usize,
}

#[derive(Component)]
pub struct HotbarSelector;

pub fn setup_hotbar_ui(
    mut commands: Commands,
) {
    // Root container for hotbar
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                width: Val::Px(360.0), // 9 slots * 40px
                height: Val::Px(40.0),
                margin: UiRect::left(Val::Px(-180.0)), // Center it
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        HotbarUI,
    ))
    .with_children(|parent| {
        // Create 9 hotbar slots
        for i in 0..HOTBAR_SLOTS {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        margin: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                    border_color: BorderColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
                    ..default()
                },
                HotbarSlot { index: i },
            ))
            .with_children(|slot| {
                // Slot number label
                slot.spawn(TextBundle::from_section(
                    format!("{}", i + 1),
                    TextStyle {
                        font_size: 12.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
            });
        }
        
        // Selection indicator
        parent.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(40.0),
                    height: Val::Px(40.0),
                    left: Val::Px(0.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::srgba(1.0, 1.0, 0.0, 1.0)),
                ..default()
            },
            HotbarSelector,
        ));
    });
}

pub fn hotbar_selection_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut hotbar: ResMut<Hotbar>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    // Number keys 1-9 select hotbar slots
    for i in 1..=9 {
        let key = match i {
            1 => KeyCode::Digit1,
            2 => KeyCode::Digit2,
            3 => KeyCode::Digit3,
            4 => KeyCode::Digit4,
            5 => KeyCode::Digit5,
            6 => KeyCode::Digit6,
            7 => KeyCode::Digit7,
            8 => KeyCode::Digit8,
            9 => KeyCode::Digit9,
            _ => continue,
        };
        
        if keyboard.just_pressed(key) {
            hotbar.selected_slot = i - 1;
        }
    }
    
    // Mouse wheel scrolling (simplified - would need proper event handling)
    // This is a placeholder - actual mouse wheel implementation would use events
}

pub fn update_hotbar_ui(
    hotbar: Res<Hotbar>,
    mut slot_query: Query<(&HotbarSlot, &mut BackgroundColor, &Children), Without<HotbarSelector>>,
    mut text_query: Query<&mut Text>,
    mut selector_query: Query<&mut Style, With<HotbarSelector>>,
) {
    // Update slot backgrounds and text
    for (slot, mut bg_color, children) in slot_query.iter_mut() {
        // Highlight if selected
        if slot.index == hotbar.selected_slot {
            bg_color.0 = Color::srgba(0.4, 0.4, 0.4, 0.9);
        } else {
            bg_color.0 = Color::srgba(0.2, 0.2, 0.2, 0.8);
        }
        
        // Update text to show block type
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                if let Some(block_type) = hotbar.slots[slot.index] {
                    text.sections[0].value = format!("{:?}", block_type)
                        .chars()
                        .take(3)
                        .collect::<String>()
                        .to_uppercase();
                } else {
                    text.sections[0].value = format!("{}", slot.index + 1);
                }
            }
        }
    }
    
    // Update selector position
    if let Ok(mut selector_style) = selector_query.get_single_mut() {
        selector_style.left = Val::Px(hotbar.selected_slot as f32 * 40.0);
    }
}