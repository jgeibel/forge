use crate::planet::config::*;
use bevy::prelude::*;
use std::f32::consts::PI;

/// Wrapping coordinates for a spherical planet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanetPos {
    pub x: i32,
    pub y: i32, // Height doesn't wrap
    pub z: i32,
}

/// Geographic coordinates on the planet
#[derive(Debug, Clone, Copy)]
pub struct GeographicCoords {
    pub latitude: f32,  // -90 to 90 degrees (south to north)
    pub longitude: f32, // -180 to 180 degrees (west to east)
}

impl PlanetPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Get the wrapped position within planet bounds
    pub fn wrapped(&self) -> Self {
        Self {
            x: self.x.rem_euclid(PLANET_SIZE_BLOCKS),
            y: self.y.clamp(0, PLANET_HEIGHT_BLOCKS - 1),
            z: self.z.rem_euclid(PLANET_SIZE_BLOCKS),
        }
    }

    /// Convert to world position for rendering
    pub fn to_world_pos(&self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }

    /// Create from world position
    pub fn from_world_pos(pos: Vec3) -> Self {
        Self {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32,
            z: pos.z.floor() as i32,
        }
    }

    /// Get the chunk this position is in
    pub fn to_chunk_pos(&self) -> WrappedChunkPos {
        WrappedChunkPos::new(
            self.x >> 5, // Divide by 32
            self.y >> 5,
            self.z >> 5,
        )
    }

    /// Convert to geographic coordinates (latitude/longitude)
    pub fn to_geographic(&self) -> GeographicCoords {
        // Map X coordinate to longitude (-180 to 180 degrees)
        let longitude = (self.x as f32 / PLANET_SIZE_BLOCKS as f32) * 360.0 - 180.0;

        // Map Z coordinate to latitude (-90 to 90 degrees)
        // Z=0 is south pole, Z=PLANET_SIZE is north pole
        let latitude = (self.z as f32 / PLANET_SIZE_BLOCKS as f32) * 180.0 - 90.0;

        GeographicCoords {
            latitude,
            longitude,
        }
    }

    /// Create from geographic coordinates
    pub fn from_geographic(coords: &GeographicCoords) -> Self {
        // Convert longitude to X coordinate
        let x = ((coords.longitude + 180.0) / 360.0 * PLANET_SIZE_BLOCKS as f32) as i32;

        // Convert latitude to Z coordinate
        let z = ((coords.latitude + 90.0) / 180.0 * PLANET_SIZE_BLOCKS as f32) as i32;

        Self::new(x, 0, z)
    }
}

impl GeographicCoords {
    pub fn new(latitude: f32, longitude: f32) -> Self {
        Self {
            latitude: latitude.clamp(-90.0, 90.0),
            longitude: ((longitude + 180.0) % 360.0) - 180.0, // Wrap to -180 to 180
        }
    }

    /// Convert to radians for calculations
    pub fn to_radians(&self) -> (f32, f32) {
        (self.latitude * PI / 180.0, self.longitude * PI / 180.0)
    }

    /// Calculate day length at this latitude for a given day of year
    pub fn day_length_hours(&self, day_of_year: u32) -> f32 {
        use crate::celestial::time::AXIAL_TILT;

        let (lat_rad, _) = self.to_radians();

        // Calculate sun declination for this day
        let day_angle = 2.0 * PI * (day_of_year as f32 - 79.0) / 365.0;
        let declination = AXIAL_TILT * day_angle.sin();

        // Calculate sunrise/sunset hour angle
        let cos_hour_angle = -(lat_rad.tan() * declination.tan());

        if cos_hour_angle >= 1.0 {
            // Polar night
            0.0
        } else if cos_hour_angle <= -1.0 {
            // Polar day
            24.0
        } else {
            // Normal day/night cycle
            let hour_angle = cos_hour_angle.acos();
            2.0 * hour_angle * 12.0 / PI // Convert radians to hours
        }
    }
}

/// Chunk position with automatic wrapping
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WrappedChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl WrappedChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: x.rem_euclid(PLANET_SIZE_CHUNKS),
            y: y.clamp(0, PLANET_HEIGHT_CHUNKS - 1),
            z: z.rem_euclid(PLANET_SIZE_CHUNKS),
        }
    }

    /// Get neighbor chunk with wrapping
    pub fn neighbor(&self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }

    /// Get all 6 adjacent chunks
    pub fn adjacent(&self) -> [Self; 6] {
        [
            self.neighbor(-1, 0, 0), // Left
            self.neighbor(1, 0, 0),  // Right
            self.neighbor(0, -1, 0), // Down
            self.neighbor(0, 1, 0),  // Up
            self.neighbor(0, 0, -1), // Front
            self.neighbor(0, 0, 1),  // Back
        ]
    }

    /// Convert to world position
    pub fn to_world_pos(&self) -> Vec3 {
        Vec3::new(
            (self.x * 32) as f32,
            (self.y * 32) as f32,
            (self.z * 32) as f32,
        )
    }

    /// Distance to another chunk (accounting for wrapping)
    pub fn wrapped_distance(&self, other: &Self) -> i32 {
        let dx = wrapped_diff(self.x, other.x, PLANET_SIZE_CHUNKS);
        let dy = (self.y - other.y).abs();
        let dz = wrapped_diff(self.z, other.z, PLANET_SIZE_CHUNKS);

        dx.max(dy).max(dz)
    }
}

/// Calculate wrapped difference between two coordinates
fn wrapped_diff(a: i32, b: i32, size: i32) -> i32 {
    let direct = (a - b).abs();
    let wrapped = size - direct;
    direct.min(wrapped)
}

/// Component for entities that exist on the planet surface
#[derive(Component)]
pub struct PlanetPosition {
    pub logical: PlanetPos, // Can exceed bounds for smooth movement
    pub wrapped: PlanetPos, // Always within bounds
}

impl PlanetPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let pos = PlanetPos::new(x, y, z);
        Self {
            logical: pos,
            wrapped: pos.wrapped(),
        }
    }

    pub fn update(&mut self, delta: Vec3) {
        self.logical.x += delta.x as i32;
        self.logical.y += delta.y as i32;
        self.logical.z += delta.z as i32;
        self.wrapped = self.logical.wrapped();
    }
}
