use bevy::prelude::*;
use bevy_console::{ConsoleCommand, ConsolePlugin, ConsoleConfiguration, AddConsoleCommand, ConsoleOpen, ConsoleChatMessage, reply};
use clap::Parser;

/// Plugin to handle all console commands
pub struct ConsoleCommandsPlugin;

impl Plugin for ConsoleCommandsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(ConsolePlugin)
            .insert_resource(ConsoleConfiguration {
                // Empty keys - we'll handle opening manually, ESC will close by default
                keys: vec![],
                left_pos: 200.0,
                top_pos: 100.0,
                height: 400.0,
                width: 800.0,
                ..Default::default()
            })
            .add_console_command::<PlanetCommand, _>(planet_command)
            .add_console_command::<TimeCommand, _>(time_command)
            .add_console_command::<TeleportCommand, _>(teleport_command)
            .add_console_command::<ClearCommand, _>(clear_command)
            .add_console_command::<HelpCommand, _>(help_command)
            .add_systems(Update, (handle_console_open, handle_chat_messages));
    }
}

/// Custom system to handle opening console with T or slash key
fn handle_console_open(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut console_open: ResMut<ConsoleOpen>,
) {
    // Only handle opening if console is closed
    if !console_open.open {
        if keyboard.just_pressed(KeyCode::Slash) {
            // Open console with slash pre-populated
            console_open.open = true;
            console_open.open_with_slash = true;
        } else if keyboard.just_pressed(KeyCode::KeyT) {
            // Open console for chat (no slash)
            console_open.open = true;
            console_open.open_with_slash = false;
        }
    }
}

/// Display planet information
#[derive(Parser, ConsoleCommand)]
#[command(name = "planet")]
struct PlanetCommand;

fn planet_command(
    mut log: ConsoleCommand<PlanetCommand>,
    planet_data: Option<Res<crate::planet::CelestialData>>,
) {
    if let Some(Ok(PlanetCommand)) = log.take() {
        if let Some(planet) = planet_data {
            let temp_c = planet.base_temperature - 273.15;
            let temp_f = temp_c * 9.0 / 5.0 + 32.0;

            let rotation_dir = match planet.rotation_direction {
                crate::planet::RotationDirection::Prograde => "Prograde",
                crate::planet::RotationDirection::Retrograde => "Retrograde",
            };

            reply!(log, "=== Planet: {} ===", planet.name);
            reply!(log, "");
            reply!(log, "Orbital Characteristics:");
            reply!(log, "  Distance from sun: {:.3} AU", planet.orbital_radius);
            reply!(log, "  Year length: {:.1} Earth days", planet.orbital_period);
            reply!(log, "  Days per year: {:.0}", planet.year_length_days);
            reply!(log, "  Orbital eccentricity: {:.3}", planet.orbital_eccentricity);
            reply!(log, "  Orbital inclination: {:.1}°", planet.orbital_inclination);
            reply!(log, "");
            reply!(log, "Rotation:");
            reply!(log, "  Day length: {:.1} hours", planet.rotation_period);
            reply!(log, "  Axial tilt: {:.1}°", planet.axial_tilt);
            reply!(log, "  Rotation: {}", rotation_dir);
            reply!(log, "");
            reply!(log, "Physical Properties:");
            reply!(log, "  Radius: {:.0} km", planet.radius);
            reply!(log, "  Mass: {:.2} Earth masses", planet.mass);
            reply!(log, "  Surface gravity: {:.2}g", planet.surface_gravity);
            reply!(log, "  Escape velocity: {:.1} km/s", planet.escape_velocity);
            reply!(log, "");
            reply!(log, "Climate:");
            reply!(log, "  Temperature: {:.1}°C ({:.1}°F)", temp_c, temp_f);
            reply!(log, "  Day/night variance: {:.1}°C", planet.temperature_variance);
            reply!(log, "  Solar constant: {:.0} W/m²", planet.solar_constant);
            reply!(log, "  Albedo: {:.2}", planet.albedo);
            reply!(log, "");
            reply!(log, "Atmosphere:");
            if planet.has_atmosphere {
                reply!(log, "  Present: Yes");
                reply!(log, "  Pressure: {:.2} Earth atmospheres", planet.atmospheric_pressure);
                reply!(log, "  Greenhouse effect: +{:.1}°C", planet.greenhouse_effect);
            } else {
                reply!(log, "  Present: No");
            }
            reply!(log, "");
            reply!(log, "Other:");
            reply!(log, "  Magnetic field: {:.2}x Earth", planet.magnetic_field_strength);
            reply!(log, "  Star visibility (day): {:.0}%", planet.star_visibility * 100.0);
        } else {
            reply!(log, "error: Planet data not available");
        }
    }
}

/// Manage time
#[derive(Parser, ConsoleCommand)]
#[command(name = "time")]
struct TimeCommand {
    /// Subcommand (set/add)
    subcommand: Option<String>,
    /// Value for the subcommand
    value: Option<f32>,
}

fn time_command(
    mut log: ConsoleCommand<TimeCommand>,
    mut game_time: Option<ResMut<crate::celestial::time::GameTime>>,
) {
    if let Some(Ok(cmd)) = log.take() {
        if let Some(mut time) = game_time {
            match cmd.subcommand.as_deref() {
                None => {
                    reply!(log, "Current time: {:.2}:00 (Day {} of Year {})",
                        time.current_hour, time.current_day, time.current_year);
                }
                Some("set") => {
                    if let Some(hour) = cmd.value {
                        let new_hour = hour % 24.0;
                        let hours_diff = new_hour - time.current_hour;
                        time.total_seconds += (hours_diff * 3600.0) as f64;
                        time.current_hour = new_hour;
                        time.day_progress = new_hour / 24.0;
                        reply!(log, "Time set to {:.2}:00", new_hour);
                    } else {
                        reply!(log, "error: Usage: /time set <hour>");
                    }
                }
                Some("add") => {
                    if let Some(hours) = cmd.value {
                        time.total_seconds += (hours * 3600.0) as f64;
                        time.update(0.0);
                        reply!(log, "Time advanced by {} hours to {:.2}:00", hours, time.current_hour);
                    } else {
                        reply!(log, "error: Usage: /time add <hours>");
                    }
                }
                _ => {
                    reply!(log, "error: Unknown subcommand. Use 'set <hour>' or 'add <hours>'");
                }
            }
        } else {
            reply!(log, "error: Time system not available");
        }
    }
}

/// Teleport to coordinates
#[derive(Parser, ConsoleCommand)]
#[command(name = "teleport")]
#[command(alias = "tp")]
struct TeleportCommand {
    /// X coordinate
    x: f32,
    /// Y coordinate
    y: f32,
    /// Z coordinate
    z: f32,
}

fn teleport_command(
    mut log: ConsoleCommand<TeleportCommand>,
    mut query: Query<&mut Transform, With<crate::camera::PlayerCamera>>,
) {
    if let Some(Ok(cmd)) = log.take() {
        for mut transform in query.iter_mut() {
            transform.translation = Vec3::new(cmd.x, cmd.y, cmd.z);
            reply!(log, "Teleported to ({}, {}, {})", cmd.x, cmd.y, cmd.z);
            return;
        }
        reply!(log, "error: No player entity found");
    }
}

/// Clear the console
#[derive(Parser, ConsoleCommand)]
#[command(name = "clear")]
#[command(alias = "cls")]
struct ClearCommand;

fn clear_command(
    mut log: ConsoleCommand<ClearCommand>,
) {
    if let Some(Ok(ClearCommand)) = log.take() {
        // Console clearing is handled internally by bevy_console
        // Just output a newline to visually separate
        reply!(log, "");
    }
}

/// Show help for commands
#[derive(Parser, ConsoleCommand)]
#[command(name = "help")]
struct HelpCommand {
    /// Command to get help for
    command: Option<String>,
}

fn help_command(
    mut log: ConsoleCommand<HelpCommand>,
) {
    if let Some(Ok(cmd)) = log.take() {
        if let Some(command) = cmd.command {
            match command.as_str() {
                "planet" => reply!(log, "/planet - Display current planet information"),
                "time" => reply!(log, "/time [set <hour>|add <hours>] - Display or modify game time"),
                "teleport" | "tp" => reply!(log, "/teleport <x> <y> <z> - Teleport to coordinates"),
                "clear" | "cls" => reply!(log, "/clear - Clear the console"),
                "help" => reply!(log, "/help [command] - Show help information"),
                _ => reply!(log, "error: Unknown command: {}", command),
            }
        } else {
            reply!(log, "Available commands:");
            reply!(log, "  /planet - Display current planet information");
            reply!(log, "  /time - Display or modify game time");
            reply!(log, "  /teleport (or /tp) - Teleport to coordinates");
            reply!(log, "  /clear (or /cls) - Clear the console");
            reply!(log, "  /help - Show this help message");
            reply!(log, "");
            reply!(log, "All commands must start with /");
            reply!(log, "Text without / will be sent as chat messages");
        }
    }
}

/// Handle chat messages from the console
fn handle_chat_messages(
    mut chat_events: EventReader<ConsoleChatMessage>,
) {
    for event in chat_events.read() {
        // For now, just log the chat message
        // In the future, this would send to the server for team/global chat
        info!("[CHAT] {}", event.message);
    }
}