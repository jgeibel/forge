use crate::block::BlockType;
use bevy::prelude::*;

const HOTBAR_SLOTS: usize = 9;
const MAX_STACK_SIZE: u32 = 64;

#[derive(Clone, Copy, Debug)]
pub struct InventorySlot {
    pub block_type: Option<BlockType>,
    pub quantity: u32,
}

impl Default for InventorySlot {
    fn default() -> Self {
        Self {
            block_type: None,
            quantity: 0,
        }
    }
}

impl InventorySlot {
    pub fn is_empty(&self) -> bool {
        self.block_type.is_none() || self.quantity == 0
    }

    pub fn can_add(&self, block_type: BlockType) -> bool {
        self.is_empty() || (self.block_type == Some(block_type) && self.quantity < MAX_STACK_SIZE)
    }

    pub fn add(&mut self, block_type: BlockType, amount: u32) -> u32 {
        if self.is_empty() {
            self.block_type = Some(block_type);
            self.quantity = amount.min(MAX_STACK_SIZE);
            return amount.saturating_sub(MAX_STACK_SIZE);
        }

        if self.block_type == Some(block_type) {
            let space_available = MAX_STACK_SIZE - self.quantity;
            let to_add = amount.min(space_available);
            self.quantity += to_add;
            return amount - to_add;
        }

        amount
    }

    pub fn remove(&mut self, amount: u32) -> u32 {
        let to_remove = amount.min(self.quantity);
        self.quantity -= to_remove;

        if self.quantity == 0 {
            self.block_type = None;
        }

        to_remove
    }
}

#[derive(Resource)]
pub struct Hotbar {
    pub slots: [InventorySlot; HOTBAR_SLOTS],
    pub selected_slot: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        // Start with all empty slots
        let slots = [InventorySlot::default(); HOTBAR_SLOTS];

        Self {
            slots,
            selected_slot: 0,
        }
    }
}

impl Hotbar {
    pub fn get_selected_block(&self) -> Option<BlockType> {
        self.slots[self.selected_slot].block_type
    }

    pub fn add_item(&mut self, block_type: BlockType, quantity: u32) -> u32 {
        let mut remaining = quantity;

        // First try to add to existing stacks of the same type
        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if slot.block_type == Some(block_type) && slot.quantity < MAX_STACK_SIZE {
                remaining = slot.add(block_type, remaining);
            }
        }

        // Then try to add to empty slots
        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if slot.is_empty() {
                remaining = slot.add(block_type, remaining);
            }
        }

        remaining
    }

    pub fn use_selected_item(&mut self) -> bool {
        let slot = &mut self.slots[self.selected_slot];
        if !slot.is_empty() {
            slot.remove(1);
            return true;
        }
        false
    }
}

#[derive(Component)]
pub struct HotbarUI;

#[derive(Component)]
pub struct HotbarSlot {
    pub index: usize,
}

#[derive(Component)]
pub struct HotbarSlotIcon;

#[derive(Component)]
pub struct HotbarSlotText;

#[derive(Component)]
pub struct HotbarSelector;

pub fn setup_hotbar_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
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
                // Block icon/image (initially invisible)
                slot.spawn((
                    ImageBundle {
                        style: Style {
                            width: Val::Px(28.0),
                            height: Val::Px(28.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        image: UiImage {
                            texture: asset_server.load(
                                &crate::texture::BlockTextureAtlas::get_display_texture_path("stone")
                            ), // Default placeholder
                            ..default()
                        },
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    HotbarSlotIcon,
                ));

                // Quantity text (bottom-right corner)
                slot.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "",
                            TextStyle {
                                font_size: 14.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ),
                        style: Style {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(0.0),
                            right: Val::Px(2.0),
                            ..default()
                        },
                        ..default()
                    },
                    HotbarSlotText,
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
    command_prompt_state: Option<Res<crate::ui::command_prompt::CommandPromptState>>,
) {
    // Don't process input if command prompt is open
    if let Some(prompt_state) = command_prompt_state {
        if prompt_state.is_open {
            return;
        }
    }

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
    mut icon_query: Query<
        (&mut UiImage, &mut Visibility),
        (With<HotbarSlotIcon>, Without<HotbarSlotText>),
    >,
    mut text_query: Query<&mut Text, With<HotbarSlotText>>,
    mut selector_query: Query<&mut Style, With<HotbarSelector>>,
    asset_server: Res<AssetServer>,
    texture_atlas: Option<Res<crate::texture::BlockTextureAtlas>>,
) {
    // Update slot backgrounds, icons, and text
    for (slot, mut bg_color, children) in slot_query.iter_mut() {
        let inventory_slot = &hotbar.slots[slot.index];

        // Highlight if selected
        if slot.index == hotbar.selected_slot {
            bg_color.0 = Color::srgba(0.4, 0.4, 0.4, 0.9);
        } else if inventory_slot.is_empty() {
            bg_color.0 = Color::srgba(0.15, 0.15, 0.15, 0.6);
        } else {
            bg_color.0 = Color::srgba(0.2, 0.2, 0.2, 0.8);
        }

        // Find and update icon and text children
        for &child in children.iter() {
            // Update icon visibility and texture
            if let Ok((mut ui_image, mut visibility)) = icon_query.get_mut(child) {
                if let Some(block_type) = inventory_slot.block_type {
                    // Show the block icon
                    *visibility = Visibility::Visible;

                    // Load the appropriate texture for this block type
                    let block_name = format!("{:?}", block_type).to_lowercase();

                    // Skip air blocks
                    if block_name == "air" {
                        *visibility = Visibility::Hidden;
                        continue;
                    }

                    // Use "all.png" for uniform blocks or specific face textures
                    let texture_path = match block_name.as_str() {
                        "grass" => "textures/blocks/grass/top.png".to_string(),
                        "dirt" | "stone" | "sand" | "cobblestone" | "bedrock" | "planks"
                        | "wood" | "leaves" | "water" => {
                            format!("textures/blocks/{}/all.png", block_name)
                        }
                        "snow" | "ice" | "packedice" => {
                            // Use a white/blue tinted stone texture as fallback for ice blocks
                            "textures/blocks/stone/all.png".to_string()
                        }
                        _ => {
                            // Fallback to a default texture
                            "textures/blocks/stone/all.png".to_string()
                        }
                    };
                    ui_image.texture = asset_server.load(&texture_path);
                } else {
                    // Hide icon for empty slots
                    *visibility = Visibility::Hidden;
                }
            }

            // Update quantity text
            if let Ok(mut text) = text_query.get_mut(child) {
                if let Some(_block_type) = inventory_slot.block_type {
                    if inventory_slot.quantity > 1 {
                        // Show quantity for stacks
                        text.sections[0].value = format!("{}", inventory_slot.quantity);
                        text.sections[0].style.font_size = 14.0;
                        text.sections[0].style.color = Color::WHITE;
                    } else {
                        // Don't show text for single items (icon is enough)
                        text.sections[0].value = String::new();
                    }
                } else {
                    // Empty slot - don't show text (slot is visually empty)
                    text.sections[0].value = String::new();
                }
            }
        }
    }

    // Update selector position
    if let Ok(mut selector_style) = selector_query.get_single_mut() {
        selector_style.left = Val::Px(hotbar.selected_slot as f32 * 40.0);
    }
}
