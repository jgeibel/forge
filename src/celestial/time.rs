use bevy::prelude::*;
use std::f32::consts::PI;

// Constants for time configuration
pub const SECONDS_PER_DAY: f32 = 24.0 * 60.0;  // 24 real minutes = 1 game day
pub const DAYS_PER_YEAR: u32 = 365;
pub const AXIAL_TILT: f32 = 23.5 * PI / 180.0;  // Earth's axial tilt in radians

#[derive(Resource, Debug, Clone)]
pub struct GameTime {
    // Core time tracking
    pub total_seconds: f64,
    pub time_speed: f32,  // Multiplier for time passage
    
    // Derived values (cached for performance)
    pub current_hour: f32,      // 0-24
    pub current_day: u32,       // Day of year (0-364)
    pub current_year: u32,      // Years since world creation
    
    // Useful calculations
    pub sun_angle: f32,         // Angle of sun in sky (radians)
    pub day_progress: f32,      // 0.0 = midnight, 0.5 = noon, 1.0 = next midnight
    pub year_progress: f32,     // 0.0 = start of year, 1.0 = end of year
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            total_seconds: 6.0 * 3600.0,  // Start at 6 AM
            time_speed: 60.0,  // 1 real second = 1 game minute
            current_hour: 6.0,
            current_day: 90,  // Start in spring (day 90)
            current_year: 0,
            sun_angle: 0.0,
            day_progress: 0.25,  // 6 AM is 25% through the day
            year_progress: 90.0 / 365.0,
        }
    }
}

impl GameTime {
    pub fn update(&mut self, delta: f32) {
        // Update total time
        self.total_seconds += delta as f64 * self.time_speed as f64;
        
        // Calculate current time values
        let seconds_in_day = 24.0 * 3600.0;
        let seconds_in_year = seconds_in_day * DAYS_PER_YEAR as f64;
        
        // Current position in day
        let day_seconds = self.total_seconds % seconds_in_day;
        self.current_hour = (day_seconds / 3600.0) as f32;
        self.day_progress = (day_seconds / seconds_in_day) as f32;
        
        // Current day and year
        let total_days = (self.total_seconds / seconds_in_day) as u32;
        self.current_day = total_days % DAYS_PER_YEAR;
        self.current_year = total_days / DAYS_PER_YEAR;
        self.year_progress = self.current_day as f32 / DAYS_PER_YEAR as f32;
        
        // Calculate sun angle (0 = horizon at dawn, PI/2 = noon, PI = horizon at dusk)
        self.sun_angle = self.day_progress * 2.0 * PI;
    }
    
    pub fn is_daytime(&self) -> bool {
        self.current_hour >= 6.0 && self.current_hour < 18.0
    }
    
    pub fn is_night(&self) -> bool {
        !self.is_daytime()
    }
    
    pub fn get_season(&self) -> Season {
        // Northern hemisphere seasons
        match self.current_day {
            0..=78 => Season::Winter,      // Dec 21 - Mar 20
            79..=171 => Season::Spring,    // Mar 20 - Jun 21
            172..=265 => Season::Summer,   // Jun 21 - Sep 23
            266..=354 => Season::Fall,     // Sep 23 - Dec 21
            _ => Season::Winter,
        }
    }
    
    pub fn get_season_southern(&self) -> Season {
        // Southern hemisphere has opposite seasons
        match self.get_season() {
            Season::Winter => Season::Summer,
            Season::Spring => Season::Fall,
            Season::Summer => Season::Winter,
            Season::Fall => Season::Spring,
        }
    }
    
    // Get the sun's declination angle based on day of year (for seasonal variation)
    pub fn get_sun_declination(&self) -> f32 {
        // Maximum declination at summer solstice, minimum at winter solstice
        let day_angle = 2.0 * PI * (self.current_day as f32 - 79.0) / 365.0;  // 79 = spring equinox
        AXIAL_TILT * day_angle.sin()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GameTime>()
            .add_systems(Update, update_game_time);
    }
}

fn update_game_time(
    time: Res<Time>,
    mut game_time: ResMut<GameTime>,
) {
    game_time.update(time.delta_seconds());
    
    // Log time occasionally for debugging
    static mut LAST_LOG: f64 = 0.0;
    unsafe {
        if game_time.total_seconds - LAST_LOG > 60.0 {  // Log every game hour
            LAST_LOG = game_time.total_seconds;
            debug!(
                "Game Time: Day {} Year {} - {:02}:{:02} ({:?})",
                game_time.current_day,
                game_time.current_year,
                game_time.current_hour as u32,
                ((game_time.current_hour % 1.0) * 60.0) as u32,
                game_time.get_season()
            );
        }
    }
}