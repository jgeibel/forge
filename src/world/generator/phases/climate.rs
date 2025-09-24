use super::super::{
    util::{celsius_to_fahrenheit, lerp_f32},
    WorldGenerator,
};

impl WorldGenerator {
    pub(crate) fn raw_rainfall(&self, world_x: f32, world_z: f32) -> f32 {
        let base = self.config.hydrology_rainfall.max(0.0);
        if base <= 0.0 {
            return 0.0;
        }

        let variance = self.config.hydrology_rainfall_variance.clamp(0.0, 3.0);
        let (u, v) = self.normalized_uv(world_x, world_z);
        let noise = self.fractal_periodic(
            &self.hydrology_rain_noise,
            u,
            v,
            self.config.hydrology_rainfall_frequency.max(0.05),
            3,
            2.1,
            0.55,
        ) as f32;
        if variance <= f32::EPSILON {
            return base;
        }

        let humidity = self.sample_moisture(world_x, world_z) * 2.0 - 1.0;
        let noise = noise.clamp(-1.0, 1.0);
        let combined = (humidity * 0.6 + noise * 0.4).clamp(-1.0, 1.0);
        let multiplier = (1.0 + combined * variance).max(0.0);
        base * multiplier
    }

    pub fn get_moisture(&self, world_x: f32, world_z: f32) -> f32 {
        self.sample_moisture(world_x, world_z)
    }

    pub fn get_temperature_c(&self, world_x: f32, world_z: f32) -> f32 {
        let height = self.get_height(world_x, world_z);
        self.temperature_at_height(world_x, world_z, height)
    }

    pub fn temperature_at_height(&self, world_x: f32, world_z: f32, height: f32) -> f32 {
        self.sample_temperature_c(world_x, world_z, height)
    }

    pub fn get_air_temperature(&self, world_x: f32, world_y: f32, world_z: f32) -> f32 {
        let temp_c = self.temperature_at_height(world_x, world_z, world_y);
        celsius_to_fahrenheit(temp_c)
    }

    pub(crate) fn sample_moisture(&self, world_x: f32, world_z: f32) -> f32 {
        let (u, v) = self.normalized_uv(world_x, world_z);
        let moisture = self.fractal_periodic(
            &self.moisture_noise,
            u,
            v,
            self.config.moisture_frequency,
            3,
            2.2,
            0.55,
        );
        ((moisture + 1.0) * 0.5) as f32
    }

    fn sample_temperature_c(&self, world_x: f32, world_z: f32, height: f32) -> f32 {
        let size = self.config.planet_size.max(1) as f32;
        let latitude = ((world_z / size).rem_euclid(1.0) - 0.5).abs();
        let lat_angle = (latitude * std::f32::consts::PI).clamp(0.0, std::f32::consts::FRAC_PI_2);
        let insolation = lat_angle.cos().clamp(0.0, 1.0);

        let base_temp = lerp_f32(
            self.config.pole_temp_c,
            self.config.equator_temp_c,
            insolation,
        );

        let elevation_above_sea = (height - self.config.sea_level).max(0.0);
        let lapse = elevation_above_sea * self.config.lapse_rate_c_per_block;

        let (u, v) = self.normalized_uv(world_x, world_z);
        let variation = self.fractal_periodic(&self.temperature_noise, u, v, 2.5, 3, 2.0, 0.6)
            as f32
            * self.config.temperature_variation;

        base_temp - lapse + variation
    }
}
