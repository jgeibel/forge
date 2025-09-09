use noise::{NoiseFn, Perlin, Seedable};

#[derive(Clone)]
pub struct ClimateMap {
    temperature_noise: Perlin,
    moisture_noise: Perlin,
    planet_size: f32,
}

impl ClimateMap {
    pub fn new(seed: u64, planet_size: f32) -> Self {
        let temperature_noise = Perlin::new((seed + 1000) as u32);
        let moisture_noise = Perlin::new((seed + 2000) as u32);
        
        Self {
            temperature_noise,
            moisture_noise,
            planet_size,
        }
    }
    
    pub fn get_temperature(&self, x: f64, z: f64) -> f32 {
        // Calculate distance from equator (0 = equator, 1 = pole)
        let equator = (self.planet_size / 2.0) as f64;
        let distance_from_equator = ((z - equator).abs() / equator).min(1.0);
        
        // Create temperature bands for more realistic climate zones
        // The closer to 1.0, the closer to the poles (colder)
        // The closer to 0.0, the closer to the equator (warmer)
        
        let base_temp = if distance_from_equator > 0.85 {
            // Polar region (very cold)
            0.05 + (1.0 - distance_from_equator) * 2.0
        } else if distance_from_equator > 0.70 {
            // Sub-polar/Taiga (cold)
            0.25 + (0.85 - distance_from_equator) * 2.0
        } else if distance_from_equator > 0.45 {
            // Temperate (moderate)
            0.50 + (0.70 - distance_from_equator) * 1.5
        } else if distance_from_equator > 0.20 {
            // Sub-tropical (warm)
            0.70 + (0.45 - distance_from_equator) * 0.8
        } else {
            // Tropical/Equatorial (hot)
            0.85 + (0.20 - distance_from_equator) * 0.75
        };
        
        // Add noise variation for more natural transitions
        let noise_scale = 0.001;
        let temp_variation = self.temperature_noise.get([x * noise_scale, 0.0, z * noise_scale]) * 0.15;
        
        ((base_temp + temp_variation) as f32).clamp(0.0, 1.0)
    }
    
    pub fn get_moisture(&self, x: f64, z: f64, distance_to_water: f32) -> f32 {
        // Base moisture from distance to water
        let water_influence = (1.0 - (distance_to_water / 100.0).min(1.0)) * 0.5;
        
        // Add noise variation
        let noise_scale = 0.0015;
        let moisture_variation = self.moisture_noise.get([x * noise_scale, 0.0, z * noise_scale]) * 0.5 + 0.5;
        
        ((water_influence + moisture_variation as f32 * 0.5) as f32).clamp(0.0, 1.0)
    }
    
    pub fn adjust_temperature_for_altitude(&self, base_temp: f32, altitude: f32, sea_level: f32) -> f32 {
        if altitude <= sea_level {
            return base_temp;
        }
        
        // Temperature drops with altitude
        let altitude_above_sea = altitude - sea_level;
        let temp_drop = (altitude_above_sea / 100.0) * 0.5; // 0.5 temp units per 100 blocks
        
        (base_temp - temp_drop).max(0.0)
    }
}