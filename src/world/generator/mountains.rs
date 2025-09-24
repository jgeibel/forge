use bevy::math::Vec2;
use noise::Perlin;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::TAU;

use crate::world::config::WorldGenConfig;

use super::continents::ContinentSite;
use super::plates::PlateSample;
use super::util::{
    rotate_vec2, torus_delta, torus_distance, torus_noise, wrap_index, wrap_index_isize, wrap_vec2,
};

#[derive(Clone)]
pub(super) struct MountainRangeMap {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) data: Vec<f32>,
}

#[derive(Clone, Copy)]
pub(super) struct RangeParams {
    pub(super) spur_chance: f32,
    pub(super) spur_strength: f32,
    pub(super) roughness: f32,
}

impl MountainRangeMap {
    pub(super) fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            data: vec![0.0],
        }
    }

    pub(super) fn generate(
        config: &WorldGenConfig,
        sites: &[ContinentSite],
        plate_sampler: &dyn Fn(f32, f32) -> PlateSample,
    ) -> Self {
        let planet_size = config.planet_size.max(1) as f32;
        let mut resolution = (planet_size / 32.0).round() as usize;
        if resolution < 128 {
            resolution = 128;
        }
        if resolution > 4096 {
            resolution = 4096;
        }

        let width = resolution;
        let height = resolution;
        let mut map = Self {
            width,
            height,
            data: vec![0.0; width * height],
        };

        let count = config.mountain_range_count as usize;
        if count == 0 || map.data.is_empty() {
            return map;
        }

        let mut rng = StdRng::seed_from_u64(config.seed.wrapping_add(17));
        let base_half_width =
            (config.mountain_range_width.max(8.0) / planet_size * 0.5).clamp(0.002, 0.25);
        let base_strength = config.mountain_range_strength.max(0.0);
        let range_params = RangeParams {
            spur_chance: config.mountain_range_spur_chance.clamp(0.0, 1.0),
            spur_strength: config.mountain_range_spur_strength.clamp(0.0, 2.0),
            roughness: config.mountain_range_roughness.clamp(0.0, 2.5),
        };
        let roughness_noise = Perlin::new(config.seed.wrapping_add(91) as u32);
        let erosion_iterations = config.mountain_erosion_iterations as usize;

        if sites.is_empty() {
            for _ in 0..count {
                let mut points = Vec::new();
                let mut current = Vec2::new(rng.gen::<f32>(), rng.gen::<f32>());
                let mut heading = rng.gen::<f32>() * TAU;

                let segments = rng.gen_range(6..12);
                let total_length = rng.gen_range(0.18..0.42);
                let step = total_length / segments as f32;
                points.push(current);

                for _ in 0..segments {
                    let bend = (rng.gen::<f32>() - 0.5) * 0.4;
                    heading = (heading + bend).rem_euclid(TAU);
                    let lateral = (rng.gen::<f32>() - 0.5) * 0.35 * step;
                    let forward = Vec2::new(heading.cos(), heading.sin());
                    let normal = Vec2::new(-forward.y, forward.x);
                    current += forward * step + normal * lateral;
                    current.x = current.x.rem_euclid(1.0);
                    current.y = current.y.rem_euclid(1.0);
                    points.push(current);
                }

                let width_variation = rng.gen_range(0.75..1.35);
                let strength_variation = rng.gen_range(0.7..1.3);
                let half_width = (base_half_width * width_variation).clamp(0.002, 0.3);
                let strength = base_strength * strength_variation;
                map.paint_range(
                    &points,
                    half_width,
                    strength,
                    &roughness_noise,
                    range_params,
                    &mut rng,
                    true,
                    plate_sampler,
                    config,
                );
            }
            map.normalize();
            if erosion_iterations > 0 {
                map.apply_erosion(erosion_iterations);
            }
            return map;
        }

        let base_radius = config.continent_radius.max(0.01);
        let total_site_weight: f32 = sites.iter().map(|s| s.weight.max(0.05)).sum();
        let average_site_weight = total_site_weight / sites.len() as f32;

        let mut spawn_for_site = |site: &ContinentSite, rng: &mut StdRng| {
            let axis = site.axis_ratio.max(0.2);
            let major = (base_radius * site.radius_scale * axis).max(0.02);
            let minor = (base_radius * site.radius_scale / axis).max(0.01);

            let along = Vec2::new(site.ridge_angle.cos(), site.ridge_angle.sin());
            let across = Vec2::new(-along.y, along.x);

            let mut points = Vec::new();
            let mut current = {
                let offset_along = rng.gen_range(-0.35..0.35) * major;
                let offset_across = rng.gen_range(-0.55..0.55) * minor;
                wrap_vec2(site.position + along * offset_along + across * offset_across)
            };

            points.push(current);

            let total_length = major * rng.gen_range(0.9..1.6);
            let segments =
                ((total_length * map.width as f32 * 1.1).clamp(6.0, 20.0)).round() as usize;
            let step = (total_length / segments.max(1) as f32).max(0.005);

            let mut heading = rotate_vec2(along, rng.gen_range(-0.35..0.35));

            for _ in 0..segments {
                let bend = rng.gen_range(-0.28..0.28);
                heading = rotate_vec2(heading, bend * 0.45).normalize_or_zero();
                if heading.length_squared() <= f32::EPSILON {
                    heading = along;
                }

                let lateral_bias = rng.gen_range(-0.4..0.4) * minor;
                let advance = heading * step + across * (lateral_bias * 0.35);
                let mut candidate = wrap_vec2(current + advance);

                let local = Vec2::new(
                    torus_delta(site.position.x, candidate.x),
                    torus_delta(site.position.y, candidate.y),
                );
                let cos_o = site.orientation.cos();
                let sin_o = site.orientation.sin();
                let rotated_x = local.x * cos_o + local.y * sin_o;
                let rotated_y = -local.x * sin_o + local.y * cos_o;
                let normalized = ((rotated_x / (major * 1.1)).powi(2)
                    + (rotated_y / (minor * 1.1)).powi(2))
                .sqrt();
                if normalized > 1.25 {
                    let clamped = local / normalized * 1.25;
                    candidate = wrap_vec2(site.position + clamped);
                }

                current = candidate;
                points.push(current);
            }

            if points.len() >= 2 {
                let plate = plate_sampler(site.position.x, site.position.y);
                let site_weight = (site.weight / average_site_weight).sqrt().clamp(0.6, 2.2);
                let width_adjust = (1.0 + plate.divergence * config.mountain_divergence_penalty
                    - plate.convergence * config.mountain_convergence_boost)
                    .clamp(0.3, 2.0);
                let width_variation =
                    rng.gen_range(0.85..1.25) * (1.0 / axis).sqrt() * width_adjust;
                let strength_adjust = (1.0 + plate.convergence * config.mountain_convergence_boost
                    - plate.divergence * config.mountain_divergence_penalty)
                    .clamp(0.3, 3.0);
                let strength_variation = rng.gen_range(0.85..1.25) * site_weight * strength_adjust;
                let half_width = (base_half_width * width_variation).clamp(0.0025, 0.35);
                let strength = (base_strength * strength_variation).max(base_strength * 0.4);

                map.paint_range(
                    &points,
                    half_width,
                    strength,
                    &roughness_noise,
                    range_params,
                    rng,
                    true,
                    plate_sampler,
                    config,
                );
            }
        };

        for _ in 0..count {
            let roll = if total_site_weight > f32::EPSILON {
                rng.gen::<f32>() * total_site_weight
            } else {
                -1.0
            };

            if roll >= 0.0 {
                let mut accum = 0.0_f32;
                let mut chosen = None;
                for site in sites {
                    accum += site.weight.max(0.05);
                    if roll <= accum {
                        chosen = Some(site);
                        break;
                    }
                }
                if let Some(site) = chosen {
                    spawn_for_site(site, &mut rng);
                    continue;
                }
            }

            // Fallback in case weights are invalid
            let index = rng.gen_range(0..sites.len());
            let site = &sites[index];
            spawn_for_site(site, &mut rng);
        }

        map.normalize();
        if erosion_iterations > 0 {
            map.apply_erosion(erosion_iterations);
        }
        map
    }

    pub(super) fn sample(&self, u: f32, v: f32) -> f32 {
        if self.data.is_empty() || self.width == 0 || self.height == 0 {
            return 0.0;
        }

        let x = u.rem_euclid(1.0) * self.width as f32;
        let y = v.rem_euclid(1.0) * self.height as f32;

        let x0 = x.floor() as isize;
        let y0 = y.floor() as isize;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        let v0 = v00 + (v10 - v00) * tx;
        let v1 = v01 + (v11 - v01) * tx;
        (v0 + (v1 - v0) * ty).clamp(0.0, 1.0)
    }

    fn paint_range(
        &mut self,
        points: &[Vec2],
        half_width: f32,
        strength: f32,
        roughness_noise: &Perlin,
        params: RangeParams,
        rng: &mut StdRng,
        allow_spurs: bool,
        plate_sampler: &dyn Fn(f32, f32) -> PlateSample,
        config: &WorldGenConfig,
    ) {
        if points.len() < 2 {
            return;
        }

        for segment in points.windows(2) {
            let start = segment[0];
            let end = segment[1];
            self.paint_segment(
                start,
                end,
                half_width,
                strength,
                roughness_noise,
                params,
                plate_sampler,
                config,
            );

            if allow_spurs && rng.gen::<f32>() < params.spur_chance {
                if let Some(spur_points) = self.generate_spur(start, end, half_width, params, rng) {
                    let spur_half = (half_width * 0.6).clamp(0.001, half_width);
                    let spur_strength = strength * params.spur_strength * rng.gen_range(0.6..1.35);
                    self.paint_range(
                        &spur_points,
                        spur_half,
                        spur_strength,
                        roughness_noise,
                        params,
                        rng,
                        false,
                        plate_sampler,
                        config,
                    );
                }
            }
        }
    }

    fn paint_segment(
        &mut self,
        start: Vec2,
        end: Vec2,
        half_width: f32,
        strength: f32,
        roughness_noise: &Perlin,
        params: RangeParams,
        plate_sampler: &dyn Fn(f32, f32) -> PlateSample,
        config: &WorldGenConfig,
    ) {
        let dx = torus_delta(start.x, end.x);
        let dy = torus_delta(start.y, end.y);
        let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
        let steps = (distance * self.width as f32 * 2.4).ceil() as usize;
        let tangent = Vec2::new(dx, dy).normalize_or_zero();
        let lateral = Vec2::new(-tangent.y, tangent.x);
        let rough_freq = 4.0 + params.roughness * 6.0;

        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let point = Vec2::new(
                (start.x + dx * t).rem_euclid(1.0),
                (start.y + dy * t).rem_euclid(1.0),
            );

            let noise_value = if params.roughness > 0.01 {
                torus_noise(roughness_noise, point.x, point.y, rough_freq, t)
            } else {
                0.0
            };

            let width_mod = (1.0 + noise_value * params.roughness * 0.5).clamp(0.35, 2.8);
            let strength_mod = (1.0 + noise_value * params.roughness * 0.4).clamp(0.3, 2.6);
            let local_half = (half_width * width_mod).clamp(0.0005, 0.35);
            let plate = plate_sampler(point.x, point.y);
            let boundary_boost = (1.0 + plate.convergence * config.mountain_convergence_boost
                - plate.divergence * config.mountain_divergence_penalty)
                .clamp(0.25, 3.5);
            let shear_boost = (1.0 + plate.shear * config.mountain_shear_boost).clamp(0.5, 2.0);
            let local_strength = strength * strength_mod * boundary_boost * shear_boost;

            self.splat(point, local_half, local_strength);

            if params.roughness > 0.2 && tangent.length_squared() > 0.0 {
                let along_offset = (noise_value * 0.5 + 0.5) * local_half * 0.6;
                let side_offset = (noise_value * 0.5) * local_half * 0.5;

                let crest_point = wrap_vec2(point + tangent * along_offset);
                self.splat(crest_point, local_half * 0.55, local_strength * 0.55);

                let spur_point = wrap_vec2(point + lateral * side_offset);
                self.splat(spur_point, local_half * 0.45, local_strength * 0.4);
            }

            if plate.convergence > config.mountain_arc_threshold {
                let arc_dir = plate.drift.normalize_or_zero();
                if arc_dir.length_squared() > 0.0 && config.mountain_arc_strength > 0.0 {
                    let arc_point = wrap_vec2(point + arc_dir * (local_half * 0.8));
                    let arc_width = (local_half
                        * config.mountain_arc_width_factor.clamp(0.05, 1.0))
                    .clamp(0.0004, 0.35);
                    let arc_strength =
                        local_strength * config.mountain_arc_strength.clamp(0.05, 1.5);
                    self.splat(arc_point, arc_width, arc_strength);
                }
            }
        }
    }

    fn generate_spur(
        &self,
        start: Vec2,
        end: Vec2,
        half_width: f32,
        params: RangeParams,
        rng: &mut StdRng,
    ) -> Option<Vec<Vec2>> {
        let dx = torus_delta(start.x, end.x);
        let dy = torus_delta(start.y, end.y);
        let base = Vec2::new(dx, dy);
        let base_length = base.length();
        if base_length <= f32::EPSILON {
            return None;
        }

        let dir = base / base_length;
        let normal = Vec2::new(-dir.y, dir.x);
        if normal.length_squared() <= f32::EPSILON {
            return None;
        }

        let anchor_t = rng.gen_range(0.15..0.85);
        let anchor = Vec2::new(
            (start.x + dx * anchor_t).rem_euclid(1.0),
            (start.y + dy * anchor_t).rem_euclid(1.0),
        );

        let mut heading = normal * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        heading = heading.normalize_or_zero();
        if heading.length_squared() <= f32::EPSILON {
            return None;
        }

        let spur_segments = rng.gen_range(3..6);
        let rough_factor = params.roughness.max(0.2);
        let base_length = (half_width * rng.gen_range(1.8..3.6)).max(0.005);
        let step = (base_length / spur_segments as f32).max(0.002);

        let mut points = Vec::with_capacity(spur_segments + 1);
        points.push(anchor);
        let mut current = anchor;

        for _ in 0..spur_segments {
            let bend = (rng.gen::<f32>() - 0.5) * 0.6 * rough_factor;
            heading = rotate_vec2(heading, bend);
            let mix = rng.gen_range(-0.35..0.35);
            heading = (heading + dir * mix).normalize_or_zero();
            if heading.length_squared() <= f32::EPSILON {
                break;
            }

            current = wrap_vec2(current + heading * step);
            points.push(current);
        }

        if points.len() > 2 {
            Some(points)
        } else {
            None
        }
    }

    fn splat(&mut self, center: Vec2, half_width: f32, strength: f32) {
        let radius = (half_width * self.width as f32 * 3.0).ceil() as i32;
        if radius <= 0 {
            return;
        }

        let cx = (center.x * self.width as f32).floor() as i32;
        let cy = (center.y * self.height as f32).floor() as i32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = wrap_index(cx + dx, self.width as i32);
                let y = wrap_index(cy + dy, self.height as i32);

                let sample_u = (x as f32 + 0.5) / self.width as f32;
                let sample_v = (y as f32 + 0.5) / self.height as f32;
                let du = torus_distance(center.x, sample_u);
                let dv = torus_distance(center.y, sample_v);
                let dist = (du * du + dv * dv).sqrt();
                if dist > half_width * 3.0 {
                    continue;
                }

                let norm = (dist / half_width).min(3.0);
                let falloff = (-norm * norm * 0.7).exp();
                let idx = y as usize * self.width + x as usize;
                self.data[idx] += falloff * strength;
            }
        }
    }

    fn normalize(&mut self) {
        let mut max_value = 0.0_f32;
        for value in &self.data {
            if *value > max_value {
                max_value = *value;
            }
        }

        if max_value <= f32::EPSILON {
            self.data.fill(0.0);
            return;
        }

        for value in &mut self.data {
            *value = (*value / max_value).clamp(0.0, 1.0);
        }
    }

    fn get(&self, x: isize, y: isize) -> f32 {
        let xi = wrap_index_isize(x, self.width as isize) as usize;
        let yi = wrap_index_isize(y, self.height as isize) as usize;
        self.data[yi * self.width + xi]
    }

    fn apply_erosion(&mut self, iterations: usize) {
        if self.width == 0 || self.height == 0 || self.data.is_empty() {
            return;
        }

        let mut buffer = vec![0.0_f32; self.data.len()];
        let width = self.width as isize;
        let height = self.height as isize;

        for _ in 0..iterations.max(1) {
            for y in 0..height {
                for x in 0..width {
                    let center = self.get(x, y);
                    let mut sum = center;
                    let mut weight_sum = 1.0_f32;

                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = wrap_index_isize(x + dx, width);
                            let ny = wrap_index_isize(y + dy, height);
                            let neighbor = self.get(nx, ny);
                            let weight = if dx == 0 || dy == 0 { 0.9 } else { 0.7 };
                            sum += neighbor * weight;
                            weight_sum += weight;
                        }
                    }

                    let average = sum / weight_sum;
                    let diff = center - average;
                    let eroded = center - diff * 0.38;
                    buffer[(y as usize) * self.width + x as usize] = eroded.max(0.0);
                }
            }

            std::mem::swap(&mut self.data, &mut buffer);

            // Re-normalize after each pass to keep the field within [0, 1].
            let mut max_value = 0.0_f32;
            for value in &self.data {
                if *value > max_value {
                    max_value = *value;
                }
            }
            if max_value > f32::EPSILON {
                for value in &mut self.data {
                    *value = (*value / max_value).clamp(0.0, 1.0);
                }
            }
        }
    }
}
