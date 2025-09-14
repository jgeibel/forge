use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct CelestialData {
    pub name: String,

    // Orbital characteristics
    pub orbital_radius: f64,           // Distance from sun in AU (Astronomical Units)
    pub orbital_period: f64,           // Year length in Earth days
    pub orbital_eccentricity: f32,     // How elliptical the orbit is (0 = circle, 1 = parabola)
    pub orbital_inclination: f32,      // Tilt of orbit relative to ecliptic plane (degrees)

    // Rotational characteristics
    pub rotation_period: f64,          // Day length in Earth hours (24 = Earth-like)
    pub axial_tilt: f32,              // Tilt of planet's axis (degrees, 23.5 = Earth-like)
    pub rotation_direction: RotationDirection,  // Prograde or retrograde

    // Physical characteristics
    pub radius: f64,                   // Planet radius in km
    pub mass: f64,                     // Planet mass in Earth masses
    pub surface_gravity: f32,          // Surface gravity in g (1.0 = Earth)
    pub escape_velocity: f32,          // km/s needed to escape gravity

    // Atmospheric properties
    pub has_atmosphere: bool,
    pub atmospheric_pressure: f32,     // Surface pressure in Earth atmospheres
    pub atmospheric_composition: AtmosphericComposition,
    pub greenhouse_effect: f32,        // Temperature increase from atmosphere (K)

    // Temperature and climate
    pub base_temperature: f32,         // Average temperature at surface (Kelvin)
    pub temperature_variance: f32,     // Day/night temperature difference
    pub albedo: f32,                  // Reflectivity (0-1, affects temperature)

    // Magnetic field (affects auroras, radiation protection)
    pub magnetic_field_strength: f32,  // Relative to Earth (1.0 = Earth-like)

    // Visual characteristics
    pub sky_color: Color,              // Atmosphere scattering color
    pub sunset_color: Color,           // Color during sunrise/sunset
    pub star_visibility: f32,          // How visible stars are during day (0-1)

    // Gameplay-relevant derived values
    pub solar_constant: f32,           // Solar energy at this distance (W/m²)
    pub day_length_seconds: f32,       // In-game seconds for one day
    pub year_length_days: f32,         // Number of planet days per year
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationDirection {
    Prograde,   // Normal rotation (like Earth)
    Retrograde, // Backwards rotation (like Venus)
}

#[derive(Debug, Clone)]
pub struct AtmosphericComposition {
    pub nitrogen: f32,
    pub oxygen: f32,
    pub carbon_dioxide: f32,
    pub water_vapor: f32,
    pub methane: f32,
    pub other: f32,
}

impl Default for AtmosphericComposition {
    fn default() -> Self {
        // Earth-like atmosphere
        Self {
            nitrogen: 0.78,
            oxygen: 0.21,
            carbon_dioxide: 0.0004,
            water_vapor: 0.01,
            methane: 0.0,
            other: 0.0096,
        }
    }
}

impl CelestialData {
    pub fn earth_like(name: String) -> Self {
        Self {
            name,

            // Orbital characteristics
            orbital_radius: 1.0,              // 1 AU from sun
            orbital_period: 365.25,           // Earth year
            orbital_eccentricity: 0.0167,     // Nearly circular
            orbital_inclination: 0.0,         // Reference plane

            // Rotational characteristics
            rotation_period: 24.0,            // 24 hour days
            axial_tilt: 23.5,                // Earth's tilt (causes seasons)
            rotation_direction: RotationDirection::Prograde,

            // Physical characteristics
            radius: 6371.0,                   // Earth radius in km
            mass: 1.0,                        // 1 Earth mass
            surface_gravity: 1.0,             // 1g
            escape_velocity: 11.2,            // Earth's escape velocity

            // Atmospheric properties
            has_atmosphere: true,
            atmospheric_pressure: 1.0,
            atmospheric_composition: AtmosphericComposition::default(),
            greenhouse_effect: 33.0,          // Earth's greenhouse warming

            // Temperature and climate
            base_temperature: 288.0,          // 15°C average
            temperature_variance: 20.0,       // Typical day/night difference
            albedo: 0.3,                     // Earth's albedo

            // Magnetic field
            magnetic_field_strength: 1.0,

            // Visual characteristics
            sky_color: Color::srgb(0.53, 0.81, 0.92),    // Sky blue
            sunset_color: Color::srgb(1.0, 0.6, 0.3),     // Orange sunset
            star_visibility: 0.0,             // No stars during day

            // Derived values
            solar_constant: 1361.0,           // Solar energy at Earth's distance
            day_length_seconds: 120.0,        // 2 minutes per day in-game
            year_length_days: 365.25,
        }
    }

    pub fn mars_like(name: String) -> Self {
        Self {
            name,

            // Orbital characteristics
            orbital_radius: 1.524,             // 1.5 AU from sun
            orbital_period: 687.0,            // Mars year in Earth days
            orbital_eccentricity: 0.0934,     // More eccentric than Earth
            orbital_inclination: 1.85,        // Slight tilt

            // Rotational characteristics
            rotation_period: 24.6,            // Sol (Mars day)
            axial_tilt: 25.19,               // Similar to Earth
            rotation_direction: RotationDirection::Prograde,

            // Physical characteristics
            radius: 3389.5,                   // Mars radius
            mass: 0.107,                      // ~11% Earth mass
            surface_gravity: 0.38,            // 38% Earth gravity
            escape_velocity: 5.03,

            // Atmospheric properties
            has_atmosphere: true,
            atmospheric_pressure: 0.006,      // Very thin
            atmospheric_composition: AtmosphericComposition {
                nitrogen: 0.027,
                oxygen: 0.0013,
                carbon_dioxide: 0.951,
                water_vapor: 0.0003,
                methane: 0.0,
                other: 0.0204,
            },
            greenhouse_effect: 5.0,           // Minimal greenhouse effect

            // Temperature and climate
            base_temperature: 210.0,          // -63°C average
            temperature_variance: 60.0,       // Large day/night difference
            albedo: 0.25,

            // Magnetic field
            magnetic_field_strength: 0.0,     // No global magnetic field

            // Visual characteristics
            sky_color: Color::srgb(0.9, 0.7, 0.5),       // Butterscotch
            sunset_color: Color::srgb(0.4, 0.6, 0.8),     // Blue sunset (opposite of Earth!)
            star_visibility: 0.1,             // Some stars visible during day

            // Derived values
            solar_constant: 586.0,            // Less solar energy
            day_length_seconds: 123.0,        // Slightly longer than Earth day
            year_length_days: 668.6,          // Mars sols per Mars year
        }
    }

    pub fn venus_like(name: String) -> Self {
        Self {
            name,

            // Orbital characteristics
            orbital_radius: 0.723,            // Closer to sun
            orbital_period: 224.7,
            orbital_eccentricity: 0.0067,     // Very circular
            orbital_inclination: 3.39,

            // Rotational characteristics
            rotation_period: 5832.5,          // 243 Earth days! (Very slow)
            axial_tilt: 177.4,               // Upside down
            rotation_direction: RotationDirection::Retrograde,

            // Physical characteristics
            radius: 6051.8,
            mass: 0.815,
            surface_gravity: 0.9,
            escape_velocity: 10.36,

            // Atmospheric properties
            has_atmosphere: true,
            atmospheric_pressure: 92.0,       // Crushing pressure
            atmospheric_composition: AtmosphericComposition {
                nitrogen: 0.035,
                oxygen: 0.0,
                carbon_dioxide: 0.965,
                water_vapor: 0.0,
                methane: 0.0,
                other: 0.0,
            },
            greenhouse_effect: 510.0,         // Extreme greenhouse effect

            // Temperature and climate
            base_temperature: 737.0,          // 464°C - hot enough to melt lead
            temperature_variance: 5.0,        // Almost no variation
            albedo: 0.75,                    // Very reflective clouds

            // Magnetic field
            magnetic_field_strength: 0.0,

            // Visual characteristics
            sky_color: Color::srgb(0.9, 0.8, 0.5),       // Yellowish
            sunset_color: Color::srgb(0.8, 0.6, 0.4),
            star_visibility: 0.0,             // Too bright to see stars

            // Derived values
            solar_constant: 2601.0,           // Much more solar energy
            day_length_seconds: 14562.5,     // Super long days (4 hours in-game)
            year_length_days: 1.92,           // Less than 2 Venus days per year!
        }
    }

    pub fn custom_planet(name: String, distance_au: f64, day_hours: f64) -> Self {
        let solar_constant = (1361.0 / (distance_au * distance_au)) as f32;
        let estimated_temp = (278.0 * (solar_constant as f64 / 1361.0).powf(0.25)) as f32;

        Self {
            name,
            orbital_radius: distance_au,
            orbital_period: 365.25 * distance_au.powf(1.5), // Kepler's third law approximation
            rotation_period: day_hours,
            solar_constant,
            base_temperature: estimated_temp,
            day_length_seconds: (day_hours / 24.0 * 120.0) as f32,
            ..Self::earth_like("Custom".to_string())
        }
    }

    // Calculate current solar angle based on time of day
    pub fn get_solar_angle(&self, time_of_day: f32) -> f32 {
        // time_of_day: 0.0 = midnight, 0.5 = noon, 1.0 = midnight again
        let angle = time_of_day * std::f32::consts::TAU;
        angle - std::f32::consts::PI // Adjust so noon is at top (PI/2)
    }

    // Calculate solar intensity based on angle (for realistic lighting)
    pub fn get_solar_intensity(&self, solar_angle: f32) -> f32 {
        // Simple atmospheric scattering model
        let elevation = solar_angle.sin();
        if elevation <= 0.0 {
            0.0 // Night
        } else {
            // Account for atmospheric thickness at low angles
            let atmospheric_factor = if self.has_atmosphere {
                (elevation * 2.0).min(1.0) // More scattering at horizon
            } else {
                elevation // No atmosphere = sharp transition
            };
            atmospheric_factor * (self.solar_constant / 1361.0)
        }
    }

    // Calculate temperature at current time and location
    pub fn get_temperature(&self, time_of_day: f32, latitude: f32) -> f32 {
        let solar_angle = self.get_solar_angle(time_of_day);
        let solar_intensity = self.get_solar_intensity(solar_angle);

        // Base temperature with day/night variation
        let day_factor = solar_intensity;
        let temp_variation = self.temperature_variance * (day_factor - 0.5);

        // Latitude affects temperature (colder at poles)
        let latitude_factor = (latitude.to_radians().cos() * 0.3 + 0.7).max(0.3);

        self.base_temperature + temp_variation * latitude_factor + self.greenhouse_effect
    }

    // Get apparent sun size (for rendering)
    pub fn get_sun_angular_diameter(&self) -> f32 {
        // Sun's actual diameter is ~1.39 million km
        // Angular diameter = 2 * arctan(radius / distance)
        let sun_radius_au = 0.00465; // Sun radius in AU
        2.0 * (sun_radius_au / self.orbital_radius).atan() as f32
    }
}

impl Default for CelestialData {
    fn default() -> Self {
        Self::earth_like("Terra".to_string())
    }
}