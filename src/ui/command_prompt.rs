use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Component)]
pub struct CommandPrompt {
    pub visible: bool,
    pub input_buffer: String,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub output_lines: Vec<String>,
    pub max_output_lines: usize,
}

impl Default for CommandPrompt {
    fn default() -> Self {
        Self {
            visible: false,
            input_buffer: String::new(),
            history: Vec::new(),
            history_index: None,
            output_lines: Vec::new(),
            max_output_lines: 20,
        }
    }
}

#[derive(Component)]
struct CommandPromptUI;

#[derive(Component)]
struct CommandInputText;

#[derive(Component)]
struct CommandOutputText;

#[derive(Resource, Clone)]
pub struct CommandRegistry {
    commands: HashMap<String, Arc<CommandHandler>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        
        registry.register_default_commands();
        registry
    }
}

pub struct CommandHandler {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub permission_level: PermissionLevel,
    pub execute: Box<dyn Fn(&[&str], &mut World) -> Result<String, String> + Send + Sync>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    Player,
    Moderator,
    Admin,
}

#[derive(Resource)]
pub struct PlayerPermissions {
    pub level: PermissionLevel,
}

#[derive(Resource, Default)]
pub struct CommandPromptState {
    pub is_open: bool,
}

impl Default for PlayerPermissions {
    fn default() -> Self {
        Self {
            level: PermissionLevel::Admin,  // Default to Admin for development
        }
    }
}

impl CommandRegistry {
    pub fn register_command(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        usage: impl Into<String>,
        permission_level: PermissionLevel,
        execute: impl Fn(&[&str], &mut World) -> Result<String, String> + Send + Sync + 'static,
    ) {
        let name = name.into();
        self.commands.insert(
            name.clone(),
            Arc::new(CommandHandler {
                name,
                description: description.into(),
                usage: usage.into(),
                permission_level,
                execute: Box::new(execute),
            }),
        );
    }
    
    fn register_default_commands(&mut self) {
        self.register_command(
            "help",
            "Show available commands",
            "/help [command]",
            PermissionLevel::Player,
            |args, world| {
                let registry = world.resource::<CommandRegistry>();
                let permissions = world.resource::<PlayerPermissions>();
                
                if args.len() > 1 {
                    if let Some(cmd) = registry.commands.get(args[1]) {
                        Ok(format!(
                            "{}: {}\nUsage: {}",
                            cmd.name, cmd.description, cmd.usage
                        ))
                    } else {
                        Err(format!("Unknown command: {}", args[1]))
                    }
                } else {
                    let mut output = String::from("Available commands:\n");
                    for (name, cmd) in &registry.commands {
                        if permissions.level as u8 >= cmd.permission_level as u8 {
                            output.push_str(&format!("  /{} - {}\n", name, cmd.description));
                        }
                    }
                    Ok(output)
                }
            },
        );
        
        self.register_command(
            "clear",
            "Clear the command output",
            "/clear",
            PermissionLevel::Player,
            |_, _| Ok("Console cleared".to_string()),
        );
        
        self.register_command(
            "time",
            "Set or display the time of day",
            "/time [set <hour>|add <hours>]",
            PermissionLevel::Admin,
            |args, world| {
                // If no arguments, show current time
                if args.len() < 2 {
                    if let Some(time) = world.get_resource::<crate::celestial::time::GameTime>() {
                        return Ok(format!("Current time: {:.2}:00 (Day {} of Year {})",
                            time.current_hour, time.current_day, time.current_year));
                    } else {
                        return Err("Time system not available".to_string());
                    }
                }

                match args[1] {
                    "query" => {
                        // Keep for backwards compatibility
                        if let Some(time) = world.get_resource::<crate::celestial::time::GameTime>() {
                            Ok(format!("Current time: {:.2}:00 (Day {} of Year {})",
                                time.current_hour, time.current_day, time.current_year))
                        } else {
                            Err("Time system not available".to_string())
                        }
                    }
                    "set" => {
                        if args.len() < 3 {
                            return Err("Usage: /time set <hour>".to_string());
                        }
                        if let Ok(hour) = args[2].parse::<f32>() {
                            if let Some(mut time) = world.get_resource_mut::<crate::celestial::time::GameTime>() {
                                let new_hour = hour % 24.0;
                                let hours_diff = new_hour - time.current_hour;
                                time.total_seconds += (hours_diff * 3600.0) as f64;
                                time.current_hour = new_hour;
                                time.day_progress = new_hour / 24.0;
                                Ok(format!("Time set to {:.2}:00", new_hour))
                            } else {
                                Err("Time system not available".to_string())
                            }
                        } else {
                            Err("Invalid hour value".to_string())
                        }
                    }
                    "add" => {
                        if args.len() < 3 {
                            return Err("Usage: /time add <hours>".to_string());
                        }
                        if let Ok(hours) = args[2].parse::<f32>() {
                            if let Some(mut time) = world.get_resource_mut::<crate::celestial::time::GameTime>() {
                                time.total_seconds += (hours * 3600.0) as f64;
                                time.update(0.0);  // Recalculate derived values
                                Ok(format!("Time advanced by {} hours to {:.2}:00", hours, time.current_hour))
                            } else {
                                Err("Time system not available".to_string())
                            }
                        } else {
                            Err("Invalid hours value".to_string())
                        }
                    }
                    _ => Err("Unknown subcommand. Use 'set <hour>' or 'add <hours>', or no arguments to display current time".to_string()),
                }
            },
        );
        
        self.register_command(
            "teleport",
            "Teleport to coordinates",
            "/teleport <x> <y> <z>",
            PermissionLevel::Moderator,
            |args, world| {
                if args.len() < 4 {
                    return Err("Usage: /teleport <x> <y> <z>".to_string());
                }
                
                let x = args[1].parse::<f32>().map_err(|_| "Invalid X coordinate")?;
                let y = args[2].parse::<f32>().map_err(|_| "Invalid Y coordinate")?;
                let z = args[3].parse::<f32>().map_err(|_| "Invalid Z coordinate")?;
                
                let mut query = world.query_filtered::<&mut Transform, With<crate::camera::PlayerCamera>>();
                for mut transform in query.iter_mut(world) {
                    transform.translation = Vec3::new(x, y, z);
                    return Ok(format!("Teleported to ({}, {}, {})", x, y, z));
                }
                
                Err("No player entity found".to_string())
            },
        );
        
        self.register_command(
            "permission",
            "Set player permission level",
            "/permission <level>",
            PermissionLevel::Admin,
            |args, world| {
                if args.len() < 2 {
                    return Err("Usage: /permission <player|moderator|admin>".to_string());
                }
                
                let level = match args[1].to_lowercase().as_str() {
                    "player" => PermissionLevel::Player,
                    "moderator" => PermissionLevel::Moderator,
                    "admin" => PermissionLevel::Admin,
                    _ => return Err("Invalid permission level. Use: player, moderator, or admin".to_string()),
                };
                
                if let Some(mut permissions) = world.get_resource_mut::<PlayerPermissions>() {
                    permissions.level = level;
                    Ok(format!("Permission level set to {:?}", level))
                } else {
                    Err("Permission system not available".to_string())
                }
            },
        );
    }
    
    pub fn execute_command(&self, input: &str, world: &mut World) -> Result<String, String> {
        let input = input.trim();
        if !input.starts_with('/') {
            return Err("Commands must start with '/'".to_string());
        }
        
        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }
        
        let command_name = parts[0];
        
        if let Some(handler) = self.commands.get(command_name) {
            let permissions = world.resource::<PlayerPermissions>();
            if permissions.level as u8 >= handler.permission_level as u8 {
                (handler.execute)(&parts, world)
            } else {
                Err("Insufficient permissions".to_string())
            }
        } else {
            Err(format!("Unknown command: {}", command_name))
        }
    }
}

pub fn setup_command_prompt(mut commands: Commands) {
    
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(60.0),
                    height: Val::Percent(40.0),
                    left: Val::Percent(20.0),
                    bottom: Val::Percent(30.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                visibility: Visibility::Hidden,
                ..default()
            },
            CommandPromptUI,
            CommandPrompt::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    TextBundle {
                        style: Style {
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        text: Text::from_section(
                            "",
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ),
                        ..default()
                    },
                    CommandOutputText,
                ));
            });
            
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 1.0)),
                    border_color: BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "/_",
                            TextStyle {
                                font_size: 16.0,
                                color: Color::srgb(1.0, 1.0, 1.0),
                                ..default()
                            },
                        ),
                        ..default()
                    },
                    CommandInputText,
                ));
            });
        });
}

pub fn toggle_command_prompt(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut CommandPrompt, &mut Visibility), With<CommandPromptUI>>,
    mut state: ResMut<CommandPromptState>,
    mut window_query: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut events: EventReader<KeyboardInput>,
) {
    // Check if we should open the prompt
    let should_open_with_slash = keyboard.just_pressed(KeyCode::Slash);
    let should_open_for_chat = keyboard.just_pressed(KeyCode::KeyT);

    if should_open_with_slash || should_open_for_chat {
        for (mut prompt, mut visibility) in query.iter_mut() {
            // Don't toggle if already open
            if prompt.visible {
                continue;
            }

            prompt.visible = true;
            state.is_open = true;
            *visibility = Visibility::Visible;

            // Clear buffer and set initial content
            prompt.input_buffer.clear();
            if should_open_with_slash {
                prompt.input_buffer.push('/');
            }
            // For 't' key, we start with empty buffer (chat mode)

            // Clear the keyboard events to prevent the triggering key from being processed as input
            events.clear();

            // Update cursor visibility
            if let Ok(mut window) = window_query.get_single_mut() {
                window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
                window.cursor.visible = true;
            }
        }
    }
}

pub fn handle_command_input(
    mut events: EventReader<KeyboardInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut prompt_query: Query<&mut CommandPrompt, With<CommandPromptUI>>,
    mut text_query: Query<&mut Text, With<CommandInputText>>,
    mut output_query: Query<&mut Text, (With<CommandOutputText>, Without<CommandInputText>)>,
    mut world_commands: Commands,
    mut state: ResMut<CommandPromptState>,
    mut window_query: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut visibility_query: Query<&mut Visibility, With<CommandPromptUI>>,
) {
    let mut should_close = false;
    
    for mut prompt in prompt_query.iter_mut() {
        if !prompt.visible {
            continue;
        }
        
        for event in events.read() {
            if !event.state.is_pressed() {
                continue;
            }
            
            let key_code = event.key_code;
            match key_code {
                    KeyCode::Enter => {
                        if !prompt.input_buffer.is_empty() {
                            let input = prompt.input_buffer.clone();
                            prompt.history.push(input.clone());
                            prompt.history_index = None;

                            // Check if it's a command (starts with /) or chat message
                            if input.starts_with('/') && input.len() > 1 {
                                // Execute as command
                                let cmd = input.clone();
                                world_commands.add(move |world: &mut World| {
                                    // Clone registry to avoid borrow issues
                                    let registry_clone = {
                                        world.resource::<CommandRegistry>().clone()
                                    };

                                    let result = registry_clone.execute_command(&cmd, world);

                                    // Convert result to string immediately
                                    let (is_clear, output_str) = match result {
                                        Ok(msg) => {
                                            let is_clear = msg.contains("Console cleared");
                                            (is_clear, format!("> {}\n{}", cmd, msg))
                                        },
                                        Err(msg) => (false, format!("> {}\nError: {}", cmd, msg)),
                                    };

                                    // Now update the prompt
                                    let mut prompt_query = world.query::<&mut CommandPrompt>();
                                    for mut prompt in prompt_query.iter_mut(world) {
                                        if is_clear {
                                            prompt.output_lines.clear();
                                        } else {
                                            prompt.output_lines.push(output_str.clone());
                                            if prompt.output_lines.len() > prompt.max_output_lines {
                                                prompt.output_lines.remove(0);
                                            }
                                        }
                                    }
                                });
                            } else if !input.is_empty() {
                                // Handle as chat message
                                let chat_msg = format!("[Chat] {}", input);
                                prompt.output_lines.push(chat_msg);
                                if prompt.output_lines.len() > prompt.max_output_lines {
                                    prompt.output_lines.remove(0);
                                }
                                info!("Chat message sent: {}", input);
                            }

                            prompt.input_buffer.clear();
                        }
                    }
                    KeyCode::Backspace => {
                        if !prompt.input_buffer.is_empty() {
                            prompt.input_buffer.pop();
                        }
                    }
                    KeyCode::Escape => {
                        prompt.visible = false;
                        should_close = true;
                    }
                    KeyCode::ArrowUp => {
                        if !prompt.history.is_empty() {
                            let index = match prompt.history_index {
                                Some(i) if i > 0 => i - 1,
                                None => prompt.history.len() - 1,
                                _ => continue,
                            };
                            prompt.history_index = Some(index);
                            prompt.input_buffer = prompt.history[index].clone();
                        }
                    }
                    KeyCode::ArrowDown => {
                        if let Some(index) = prompt.history_index {
                            if index < prompt.history.len() - 1 {
                                prompt.history_index = Some(index + 1);
                                prompt.input_buffer = prompt.history[index + 1].clone();
                            } else {
                                prompt.history_index = None;
                                prompt.input_buffer.clear();
                            }
                        }
                    }
                    _ => {
                        // Handle alphanumeric and special characters
                        // Check if shift is pressed for uppercase
                        let shift_pressed = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
                        
                        let character = match key_code {
                            KeyCode::KeyA => Some(if shift_pressed { 'A' } else { 'a' }),
                            KeyCode::KeyB => Some(if shift_pressed { 'B' } else { 'b' }),
                            KeyCode::KeyC => Some(if shift_pressed { 'C' } else { 'c' }),
                            KeyCode::KeyD => Some(if shift_pressed { 'D' } else { 'd' }),
                            KeyCode::KeyE => Some(if shift_pressed { 'E' } else { 'e' }),
                            KeyCode::KeyF => Some(if shift_pressed { 'F' } else { 'f' }),
                            KeyCode::KeyG => Some(if shift_pressed { 'G' } else { 'g' }),
                            KeyCode::KeyH => Some(if shift_pressed { 'H' } else { 'h' }),
                            KeyCode::KeyI => Some(if shift_pressed { 'I' } else { 'i' }),
                            KeyCode::KeyJ => Some(if shift_pressed { 'J' } else { 'j' }),
                            KeyCode::KeyK => Some(if shift_pressed { 'K' } else { 'k' }),
                            KeyCode::KeyL => Some(if shift_pressed { 'L' } else { 'l' }),
                            KeyCode::KeyM => Some(if shift_pressed { 'M' } else { 'm' }),
                            KeyCode::KeyN => Some(if shift_pressed { 'N' } else { 'n' }),
                            KeyCode::KeyO => Some(if shift_pressed { 'O' } else { 'o' }),
                            KeyCode::KeyP => Some(if shift_pressed { 'P' } else { 'p' }),
                            KeyCode::KeyQ => Some(if shift_pressed { 'Q' } else { 'q' }),
                            KeyCode::KeyR => Some(if shift_pressed { 'R' } else { 'r' }),
                            KeyCode::KeyS => Some(if shift_pressed { 'S' } else { 's' }),
                            KeyCode::KeyT => Some(if shift_pressed { 'T' } else { 't' }),
                            KeyCode::KeyU => Some(if shift_pressed { 'U' } else { 'u' }),
                            KeyCode::KeyV => Some(if shift_pressed { 'V' } else { 'v' }),
                            KeyCode::KeyW => Some(if shift_pressed { 'W' } else { 'w' }),
                            KeyCode::KeyX => Some(if shift_pressed { 'X' } else { 'x' }),
                            KeyCode::KeyY => Some(if shift_pressed { 'Y' } else { 'y' }),
                            KeyCode::KeyZ => Some(if shift_pressed { 'Z' } else { 'z' }),
                            KeyCode::Digit0 => Some(if shift_pressed { ')' } else { '0' }),
                            KeyCode::Digit1 => Some(if shift_pressed { '!' } else { '1' }),
                            KeyCode::Digit2 => Some(if shift_pressed { '@' } else { '2' }),
                            KeyCode::Digit3 => Some(if shift_pressed { '#' } else { '3' }),
                            KeyCode::Digit4 => Some(if shift_pressed { '$' } else { '4' }),
                            KeyCode::Digit5 => Some(if shift_pressed { '%' } else { '5' }),
                            KeyCode::Digit6 => Some(if shift_pressed { '^' } else { '6' }),
                            KeyCode::Digit7 => Some(if shift_pressed { '&' } else { '7' }),
                            KeyCode::Digit8 => Some(if shift_pressed { '*' } else { '8' }),
                            KeyCode::Digit9 => Some(if shift_pressed { '(' } else { '9' }),
                            KeyCode::Space => Some(' '),
                            KeyCode::Period => Some(if shift_pressed { '>' } else { '.' }),
                            KeyCode::Comma => Some(if shift_pressed { '<' } else { ',' }),
                            KeyCode::Minus => Some(if shift_pressed { '_' } else { '-' }),
                            KeyCode::Equal => Some(if shift_pressed { '+' } else { '=' }),
                            KeyCode::Slash => Some(if shift_pressed { '?' } else { '/' }),
                            KeyCode::Semicolon => Some(if shift_pressed { ':' } else { ';' }),
                            KeyCode::Quote => Some(if shift_pressed { '"' } else { '\'' }),
                            KeyCode::BracketLeft => Some(if shift_pressed { '{' } else { '[' }),
                            KeyCode::BracketRight => Some(if shift_pressed { '}' } else { ']' }),
                            KeyCode::Backslash => Some(if shift_pressed { '|' } else { '\\' }),
                            _ => None,
                        };
                        
                        if let Some(ch) = character {
                            prompt.input_buffer.push(ch);
                        }
                    }
            }
        }
        
        // Update input text with a blinking cursor
        let cursor = if (time.elapsed_seconds() * 2.0) as i32 % 2 == 0 { "_" } else { "" };
        // No prefix needed - just show the input buffer with cursor
        let display_text = format!("{}{}", prompt.input_buffer, cursor);
        
        for mut text in text_query.iter_mut() {
            text.sections[0].value = display_text.clone();
        }
        
        for mut text in output_query.iter_mut() {
            text.sections[0].value = prompt.output_lines.join("\n");
        }
    }
    
    if should_close {
        state.is_open = false;
        
        // Hide cursor and update window when closing
        if let Ok(mut visibility) = visibility_query.get_single_mut() {
            *visibility = Visibility::Hidden;
        }
        
        if let Ok(mut window) = window_query.get_single_mut() {
            window.cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            window.cursor.visible = false;
        }
    }
}

pub struct CommandPromptPlugin;

impl Plugin for CommandPromptPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CommandRegistry>()
            .init_resource::<PlayerPermissions>()
            .init_resource::<CommandPromptState>()
            .add_systems(Startup, setup_command_prompt)
            .add_systems(
                Update,
                (
                    toggle_command_prompt,
                    handle_command_input,
                ),
            );
    }
}