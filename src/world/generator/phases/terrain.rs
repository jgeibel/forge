use noise::{NoiseFn, Perlin};

use super::super::{hydrology::HydrologySample, util::lerp_f32, WorldGenerator};

#[derive(Debug, Clone, Copy)]
pub struct HydrologyDebugSample {
    pub base_height: f32,
    pub terrain_height: f32,
    pub channel_depth: f32,
    pub water_level: f32,
    pub river_intensity: f32,
    pub pond_intensity: f32,
    pub coastal_factor: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct TerrainComponents {
    pub(crate) base_height: f32,
}

impl WorldGenerator {
    pub(crate) fn terrain_components(&self, world_x: f32, world_z: f32) -> TerrainComponents {
        let (u, v) = self.normalized_uv(world_x, world_z);

        let border_width = 0.03;
        let ocean_border_factor = if u < border_width {
            (u / border_width).clamp(0.0, 1.0)
        } else if u > (1.0 - border_width) {
            ((1.0 - u) / border_width).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let continent = self.fractal_periodic(
            &self.continent_noise,
            u,
            v,
            self.config.continent_frequency,
            4,
            2.0,
            0.45,
        );

        let continent_mask = ((continent + 1.0) * 0.5).powf(self.config.continent_power as f64);
        let mut land_factor = ((continent_mask as f32)
            - (self.config.continent_threshold - self.config.continent_bias))
            .max(0.0)
            / (1.0 - self.config.continent_threshold);
        land_factor = land_factor.clamp(0.0, 1.0);

        let site_mask = self.continent_site_mask(u as f32, v as f32);
        land_factor = (land_factor * site_mask * ocean_border_factor as f32).clamp(0.0, 1.0);

        let ocean_factor: f32 = 1.0 - land_factor;
        let sea_level = self.config.sea_level;
        let deep_floor = sea_level - self.config.deep_ocean_depth;
        let shallow_floor = sea_level - self.config.ocean_depth;

        let ocean_height = lerp_f32(
            deep_floor,
            shallow_floor,
            (continent_mask as f32).clamp(0.0, 1.0),
        );

        let detail_scale = 50.0;
        let detail1 = self.world_noise(&self.detail_noise, world_x, world_z, detail_scale) as f32;
        let detail2 = self.world_noise(
            &self.detail_noise,
            world_x + 1000.0,
            world_z + 1000.0,
            detail_scale * 2.0,
        ) as f32
            * 0.5;
        let detail3 = self.world_noise(
            &self.detail_noise,
            world_x + 2000.0,
            world_z + 2000.0,
            detail_scale * 4.0,
        ) as f32
            * 0.25;
        let detail =
            (detail1 + detail2 + detail3) / 1.75 * self.config.detail_amplitude * land_factor;

        let micro_scale = self.config.micro_detail_scale.max(1.0);
        let persistence = self.config.micro_detail_roughness.clamp(0.1, 0.95);
        let mut micro_total = 0.0f32;
        let mut micro_weight = 0.0f32;
        let mut current_scale = micro_scale;
        let mut amplitude = 1.0f32;
        for octave in 0..3 {
            let offset = 3000.0 * (octave as f32 + 1.0);
            let sample = self.world_noise(
                &self.micro_detail_noise,
                world_x + offset,
                world_z - offset,
                current_scale.max(1.0),
            ) as f32;
            micro_total += sample * amplitude;
            micro_weight += amplitude;
            amplitude *= persistence;
            current_scale = (current_scale * 0.5).max(2.0);
        }
        let micro_base = if micro_weight > 0.0 {
            (micro_total / micro_weight).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let land_blend = self.config.micro_detail_land_blend.max(0.05);
        let mask = land_factor.powf(land_blend);
        let micro_detail = micro_base * self.config.micro_detail_amplitude * mask;

        let mountain_scale = 200.0;
        let mountain1 = self.world_noise(&self.mountain_noise, world_x, world_z, mountain_scale);
        let mountain2 = self.world_noise(
            &self.mountain_noise,
            world_x + 5000.0,
            world_z + 5000.0,
            mountain_scale * 2.0,
        ) * 0.5;
        let mountain_raw = (mountain1 + mountain2) / 1.5;

        let mountain_mask = ((mountain_raw + 1.0) * 0.5).powf(1.8);
        let mountain_bonus = if mountain_mask as f32 > self.config.mountain_threshold {
            (mountain_mask as f32 - self.config.mountain_threshold)
                / (1.0 - self.config.mountain_threshold)
        } else {
            0.0
        };
        let ridge_factor = self.continent_ridge_factor(u as f32, v as f32);
        let range_factor = self
            .mountain_ranges
            .sample(u as f32, v as f32)
            .clamp(0.0, 1.0);
        let land_weight = land_factor.powf(0.65);
        let base_mountain = (mountain_bonus * ridge_factor + land_factor * 0.1).clamp(0.0, 1.0)
            * self.config.mountain_height
            * land_factor;
        let range_bonus = range_factor
            * self.config.mountain_height
            * self.config.mountain_range_strength
            * land_weight;
        let mountains = base_mountain + range_bonus;

        let interior_mask = land_factor.powf(1.4);
        let range_highlands = range_factor * self.config.highland_bonus * 0.6 * interior_mask;
        let highlands = ((ridge_factor * 0.9 + interior_mask * 0.4).clamp(0.0, 1.0)
            * self.config.highland_bonus
            * interior_mask)
            + range_highlands;

        let land_height =
            sea_level + detail + micro_detail + highlands + mountains + land_factor * 16.0;
        let island_raw = self.fractal_periodic(
            &self.island_noise,
            u,
            v,
            self.config.island_frequency,
            3,
            2.3,
            0.55,
        );
        let island_mask = ((island_raw + 1.0) * 0.5) as f32;
        let island_strength = ((island_mask - self.config.island_threshold)
            / (1.0 - self.config.island_threshold))
            .max(0.0)
            .clamp(0.0, 1.0);
        let ocean_only = ocean_factor.powf(self.config.island_falloff.max(0.1));
        let island_bonus = island_strength * ocean_only * self.config.island_height;

        let base_height = ocean_height * ocean_factor + land_height * land_factor + island_bonus;

        TerrainComponents { base_height }
    }

    pub fn get_height(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let hydro = self.sample_hydrology(world_x, world_z, components.base_height);
        let floodplain = self.config.hydrology_floodplain_radius.max(0.0);
        let mut height = components.base_height - hydro.channel_depth;
        if hydro.pond_intensity > 0.05 {
            let soften = (floodplain * 0.1).clamp(0.0, 6.0);
            let shore_level = (hydro.water_level - soften).min(height);
            height = height.min(shore_level);
        } else if hydro.river_intensity > 0.05 {
            let soften = (floodplain * 0.2).clamp(0.5, 12.0);
            let blend = soften * (1.0 - hydro.river_intensity).clamp(0.0, 1.0);
            height = height.min(hydro.water_level - blend);
        }

        if hydro.coastal_factor > 0.01 {
            let blend_strength = hydro.coastal_factor.clamp(0.0, 1.0);
            let max_elevation = (self.config.hydrology_estuary_length * 0.05).clamp(4.0, 18.0);
            let relative = height - self.config.sea_level;
            let clamped = relative.clamp(-max_elevation, max_elevation);
            let target = self.config.sea_level + clamped;
            height = lerp_f32(height, target, (blend_strength * 0.5).clamp(0.0, 1.0));
            height = height.max(self.config.sea_level + 0.05);
        }
        height.max(4.0)
    }

    pub(crate) fn normalized_uv(&self, world_x: f32, world_z: f32) -> (f64, f64) {
        let size = self.config.planet_size.max(1) as f32;
        let u = (world_x / size).rem_euclid(1.0) as f64;
        let v = (world_z / size).rem_euclid(1.0) as f64;
        (u, v)
    }

    pub(crate) fn periodic_noise(&self, noise: &Perlin, u: f64, v: f64, cycles: f64) -> f64 {
        const TAU: f64 = std::f64::consts::PI * 2.0;
        let theta = (u * cycles) * TAU;
        let phi = (v * cycles) * TAU;
        noise.get([theta.sin(), theta.cos(), phi.sin(), phi.cos()])
    }

    pub(crate) fn world_noise(
        &self,
        noise: &Perlin,
        world_x: f32,
        world_z: f32,
        scale: f32,
    ) -> f64 {
        let x = world_x as f64 / scale as f64;
        let z = world_z as f64 / scale as f64;

        let planet_size = self.config.planet_size as f64;
        const TAU: f64 = std::f64::consts::PI * 2.0;
        let theta = (world_x as f64 / planet_size) * TAU;
        let phi = (world_z as f64 / planet_size) * TAU;

        noise.get([
            theta.sin() + x * 0.1,
            theta.cos() + x * 0.1,
            phi.sin() + z * 0.1,
            phi.cos() + z * 0.1,
        ])
    }

    pub(crate) fn fractal_periodic(
        &self,
        noise: &Perlin,
        u: f64,
        v: f64,
        base_cycles: f64,
        octaves: usize,
        lacunarity: f64,
        gain: f64,
    ) -> f64 {
        let mut frequency = base_cycles.max(0.0001);
        let mut amplitude = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;

        for _ in 0..octaves {
            sum += self.periodic_noise(noise, u, v, frequency) * amplitude;
            norm += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }

        if norm == 0.0 {
            0.0
        } else {
            sum / norm
        }
    }

    pub(crate) fn sample_hydrology(
        &self,
        world_x: f32,
        world_z: f32,
        base_height: f32,
    ) -> HydrologySample {
        let mut sample = self.hydrology.sample(world_x, world_z);

        if base_height <= self.config.sea_level {
            sample.channel_depth = 0.0;
            sample.water_level = self.config.sea_level;
            sample.river_intensity = 0.0;
            sample.pond_intensity = 0.0;
            sample.coastal_factor = 0.0;
            return sample;
        }

        if sample.channel_depth > 0.0 {
            let bankfull_cap = (self.config.hydrology_river_depth_scale * 2.0).max(3.0);
            let max_carve = (base_height - 4.0).max(0.0).min(bankfull_cap);
            sample.channel_depth = sample.channel_depth.min(max_carve);
        }

        if sample.water_level <= self.config.sea_level {
            sample.water_level = base_height - sample.channel_depth;
            sample.water_level = sample.water_level.max(self.config.sea_level);
        }

        sample
    }

    pub fn hydrology_debug_sample(&self, world_x: f32, world_z: f32) -> HydrologyDebugSample {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        let terrain_height = self.get_height(world_x, world_z);

        HydrologyDebugSample {
            base_height: components.base_height,
            terrain_height,
            channel_depth: sample.channel_depth,
            water_level: sample.water_level,
            river_intensity: sample.river_intensity,
            pond_intensity: sample.pond_intensity,
            coastal_factor: sample.coastal_factor,
        }
    }
}
