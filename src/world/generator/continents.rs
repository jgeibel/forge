use bevy::math::Vec2;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::TAU;

use crate::world::config::WorldGenConfig;

use super::util::{torus_delta, torus_distance, wrap_vec2};
use super::WorldGenerator;

#[derive(Clone)]
pub(super) struct ContinentSite {
    pub(super) position: Vec2,
    pub(super) ridge_angle: f32,
    pub(super) orientation: f32,
    pub(super) axis_ratio: f32,
    pub(super) radius_scale: f32,
    pub(super) edge_power: f32,
    pub(super) weight: f32,
    pub(super) drift: Vec2,
}

#[derive(Clone, Copy, Default)]
pub(super) struct PlateSample {
    pub(super) drift: Vec2,
    pub(super) convergence: f32,
    pub(super) divergence: f32,
    pub(super) shear: f32,
}

pub(super) fn generate_continent_sites(config: &WorldGenConfig) -> Vec<ContinentSite> {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let n = config.continent_count.max(1);
    let grid_len = (n as f32).sqrt().ceil() as u32;
    let cell_size = 1.0 / grid_len as f32;
    let jitter = cell_size * 0.6;

    let mut sites = Vec::with_capacity(n as usize);
    let mut index = 0u32;

    let offset_u = rng.gen::<f32>() * cell_size;
    let offset_v = rng.gen::<f32>() * cell_size;
    let belt_center = rng.gen::<f32>();
    let belt_half_width = config.continent_belt_width.clamp(0.05, 0.45);

    for row in 0..grid_len {
        for col in 0..grid_len {
            if index >= n {
                break;
            }
            index += 1;
            let base_u = (col as f32 + offset_u).rem_euclid(grid_len as f32) * cell_size;
            let base_v = (row as f32 + offset_v).rem_euclid(grid_len as f32) * cell_size;
            let jitter_u = (rng.gen::<f32>() - 0.5) * jitter;
            let jitter_v = (rng.gen::<f32>() - 0.5) * jitter;
            let u = (base_u + jitter_u).rem_euclid(1.0);
            let v = (base_v + jitter_v).rem_euclid(1.0);
            let orientation = rng.gen::<f32>() * TAU;
            let axis_ratio = rng.gen_range(0.55_f32..1.45_f32);
            let lat = v;
            let mut belt_offset = lat - belt_center;
            belt_offset = (belt_offset + 0.5).rem_euclid(1.0) - 0.5;
            let belt_intensity =
                (1.0 - (belt_offset.abs() / belt_half_width).clamp(0.0, 1.0)).powi(2);
            let radius_scale = {
                let tier = rng.gen::<f32>();
                if tier < 0.1 + belt_intensity * 0.12 {
                    rng.gen_range(1.55_f32..2.2_f32)
                } else if tier < 0.28 + belt_intensity * 0.15 {
                    rng.gen_range(1.1_f32..1.55_f32)
                } else if tier < 0.62 {
                    rng.gen_range(0.8_f32..1.08_f32)
                } else {
                    rng.gen_range(0.45_f32..0.85_f32)
                }
            };
            let edge_power = rng.gen_range(0.6_f32..1.4_f32);
            let weight = (radius_scale * radius_scale).clamp(0.35_f32, 4.5_f32);
            let ridge_angle = (orientation + rng.gen_range(-0.7_f32..0.7_f32)).rem_euclid(TAU);
            sites.push(ContinentSite {
                position: Vec2::new(u, v),
                ridge_angle,
                orientation,
                axis_ratio,
                radius_scale,
                edge_power,
                weight,
                drift: Vec2::ZERO,
            });
        }
        if index >= n {
            break;
        }
    }

    let mut segment_has_major = [false; 3];
    for site in &sites {
        let segment = ((site.position.y * 3.0).floor() as usize).min(2);
        if site.radius_scale >= 1.2 {
            segment_has_major[segment] = true;
        }
    }

    for segment in 0..3 {
        if segment_has_major[segment] {
            continue;
        }
        let start = segment as f32 / 3.0;
        let end = start + 1.0 / 3.0;
        if let Some((index, _)) = sites
            .iter()
            .enumerate()
            .filter(|(_, site)| {
                let y = site.position.y;
                y >= start && y < end
            })
            .max_by(|a, b| a.1.radius_scale.partial_cmp(&b.1.radius_scale).unwrap())
        {
            let boost = rng.gen_range(1.18_f32..1.48_f32);
            sites[index].radius_scale = boost;
            sites[index].weight = (boost * boost).clamp(0.45_f32, 4.5_f32);
            sites[index].edge_power = (sites[index].edge_power * 0.6 + 0.8).clamp(0.5, 1.4);
        }
    }

    let base_spacing = (1.0 / grid_len.max(1) as f32).max(0.02);
    for _ in 0..2 {
        let mut adjustments = vec![Vec2::ZERO; sites.len()];

        for i in 0..sites.len() {
            let mut displacement = Vec2::ZERO;
            for j in 0..sites.len() {
                if i == j {
                    continue;
                }
                let dx = torus_delta(sites[i].position.x, sites[j].position.x);
                let dy = torus_delta(sites[i].position.y, sites[j].position.y);
                let diff = Vec2::new(dx, dy);
                let distance = diff.length();
                if distance <= f32::EPSILON {
                    continue;
                }

                let desired = base_spacing
                    * (0.55
                        + (sites[i].radius_scale + sites[j].radius_scale).max(0.4) * 0.22
                        + (sites[i].weight + sites[j].weight).sqrt() * 0.02);

                if distance < desired {
                    let push = (desired - distance) / desired;
                    let dir = diff.normalize_or_zero();
                    let weight_factor = ((sites[i].weight + sites[j].weight) * 0.25).sqrt();
                    displacement -= dir
                        * push
                        * weight_factor
                        * config.continent_repulsion_strength.clamp(0.0, 0.3);
                }
            }
            adjustments[i] = displacement;
        }

        let max_delta = base_spacing * 0.28;
        for (site, delta) in sites.iter_mut().zip(adjustments.iter()) {
            let mut adjustment = *delta;
            let len_sq = adjustment.length_squared();
            if len_sq > max_delta * max_delta {
                adjustment = adjustment.normalize_or_zero() * max_delta;
            }
            site.position = wrap_vec2(site.position + adjustment);
        }
    }

    let belt_axis = Vec2::from_angle((rng.gen::<f32>() - 0.5) * 0.9);

    for site in &mut sites {
        let along = Vec2::new(site.ridge_angle.cos(), site.ridge_angle.sin());
        let across = Vec2::new(-along.y, along.x);
        let belt_bias = belt_axis.dot(across).clamp(-1.0, 1.0);
        let magnitude =
            (config.continent_drift_gain + site.radius_scale * 0.08 + site.weight.sqrt() * 0.03)
                * (1.0 + belt_bias.abs() * config.continent_drift_belt_gain);
        let direction = if belt_bias >= 0.0 { across } else { -across };
        site.drift = direction.normalize_or_zero() * magnitude;
    }

    sites
}

impl WorldGenerator {
    pub(super) fn plate_sample(&self, u: f32, v: f32) -> PlateSample {
        if self.continent_sites.is_empty() {
            return PlateSample::default();
        }

        let base_radius = self.config.continent_radius.max(0.01_f32);
        let global_edge = self.config.continent_edge_power.max(0.1_f32);
        let coastal_cycles = (self.config.continent_frequency * 0.35).max(0.05);
        let coastal_noise =
            self.periodic_noise(&self.continent_noise, u as f64, v as f64, coastal_cycles) as f32;
        let jitter = base_radius * 0.1_f32 * coastal_noise;

        let mut total_weight = 0.0_f32;
        let mut combined_drift = Vec2::ZERO;

        let mut best = (0.0_f32, None);
        let mut second = (0.0_f32, None);

        for (index, site) in self.continent_sites.iter().enumerate() {
            let mut du = torus_distance(u, site.position.x);
            let mut dv = torus_distance(v, site.position.y);

            if jitter.abs() > f32::EPSILON {
                let dir = Vec2::new(site.ridge_angle.cos(), site.ridge_angle.sin());
                du += dir.x * jitter;
                dv += dir.y * jitter;
            }

            let delta = Vec2::new(du, dv);

            let cos_o = site.orientation.cos();
            let sin_o = site.orientation.sin();
            let rotated_x = delta.x * cos_o + delta.y * sin_o;
            let rotated_y = -delta.x * sin_o + delta.y * cos_o;

            let axis = site.axis_ratio.max(0.2_f32);
            let major = (base_radius * site.radius_scale * axis).max(0.01_f32);
            let minor = (base_radius * site.radius_scale / axis).max(0.01_f32);

            let normalized = ((rotated_x / major).powi(2) + (rotated_y / minor).powi(2)).sqrt();
            let interior = (1.0 - normalized).max(0.0);
            let edge = (global_edge * site.edge_power).clamp(0.2_f32, 4.0_f32);
            let core = interior.powf(edge);
            let feather = (-normalized.powf(1.35_f32) * 1.25_f32).exp();
            let influence = (core * 0.85_f32 + feather * 0.35_f32).clamp(0.0, 1.0);

            if influence <= 0.0005_f32 {
                continue;
            }

            let weighted = influence * site.weight;
            total_weight += weighted;
            combined_drift += site.drift * weighted;

            if influence > best.0 {
                second = best;
                best = (influence, Some(index));
            } else if influence > second.0 {
                second = (influence, Some(index));
            }
        }

        if total_weight <= f32::EPSILON {
            return PlateSample::default();
        }

        let drift = combined_drift / total_weight;

        let mut convergence = 0.0_f32;
        let mut divergence = 0.0_f32;
        let mut shear = 0.0_f32;

        if let (Some(a), Some(b)) = (best.1, second.1) {
            let site_a = &self.continent_sites[a];
            let site_b = &self.continent_sites[b];

            let mut sep = Vec2::new(
                torus_delta(site_a.position.x, site_b.position.x),
                torus_delta(site_a.position.y, site_b.position.y),
            );
            let distance = sep.length().max(0.0001);
            sep /= distance;

            let drift_a = site_a.drift;
            let drift_b = site_b.drift;

            let secondary_weight = second.0;
            let toward = (drift_a.dot(sep).max(0.0) + drift_b.dot(-sep).max(0.0))
                * secondary_weight.powf(1.2);
            let away = (-drift_a.dot(sep)).max(0.0) + (-drift_b.dot(-sep)).max(0.0);
            convergence = toward;
            divergence = away * secondary_weight.powf(1.1);

            let perp = Vec2::new(-sep.y, sep.x);
            let shear_a = drift_a.dot(perp);
            let shear_b = drift_b.dot(perp);
            shear = (shear_a - shear_b).abs() * secondary_weight.powf(1.15);
        }

        PlateSample {
            drift,
            convergence,
            divergence,
            shear,
        }
    }
    pub(super) fn continent_site_mask(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let base_radius = self.config.continent_radius.max(0.01_f32);
        let global_edge = self.config.continent_edge_power.max(0.1_f32);

        let mut accum = 0.0_f32;
        let mut total_weight = 0.0_f32;
        let mut best = 0.0_f32;
        let mut second_best = 0.0_f32;

        let coastal_cycles = (self.config.continent_frequency * 0.35).max(0.05);
        let coastal_noise =
            self.periodic_noise(&self.continent_noise, u as f64, v as f64, coastal_cycles) as f32;
        let jitter = base_radius * 0.1_f32 * coastal_noise;

        for site in &self.continent_sites {
            let mut du = torus_distance(u, site.position.x);
            let mut dv = torus_distance(v, site.position.y);

            if jitter.abs() > f32::EPSILON {
                let dir = Vec2::new(site.ridge_angle.cos(), site.ridge_angle.sin());
                du += dir.x * jitter;
                dv += dir.y * jitter;
            }

            let delta = Vec2::new(du, dv);

            let cos_o = site.orientation.cos();
            let sin_o = site.orientation.sin();
            let rotated_x = delta.x * cos_o + delta.y * sin_o;
            let rotated_y = -delta.x * sin_o + delta.y * cos_o;

            let axis = site.axis_ratio.max(0.2_f32);
            let major = (base_radius * site.radius_scale * axis).max(0.01_f32);
            let minor = (base_radius * site.radius_scale / axis).max(0.01_f32);

            let normalized = ((rotated_x / major).powi(2) + (rotated_y / minor).powi(2)).sqrt();
            let interior = (1.0 - normalized).max(0.0);
            let edge = (global_edge * site.edge_power).clamp(0.2_f32, 4.0_f32);
            let core = interior.powf(edge);
            let feather = (-normalized.powf(1.35_f32) * 1.25_f32).exp();
            let influence = (core * 0.85_f32 + feather * 0.35_f32).clamp(0.0, 1.0);

            if influence <= 0.0005_f32 {
                continue;
            }

            accum += influence * site.weight;
            total_weight += site.weight;
            if influence > best {
                second_best = best;
                best = influence;
            } else if influence > second_best {
                second_best = influence;
            }
        }

        if total_weight <= f32::EPSILON {
            return 0.0;
        }

        let base = (accum / total_weight).clamp(0.0, 1.0);

        let mut mask =
            (best * 0.6_f32 + base * 0.3_f32 + (best - base).max(0.0) * 0.1_f32).clamp(0.0, 1.0);

        if second_best > 0.001_f32 {
            let overlap = best.min(second_best);
            mask += overlap.powf(1.35_f32) * 0.05_f32;
        }

        mask = mask.powf(0.95_f32);

        let noise_scale =
            (1.0_f32 + coastal_noise.clamp(-0.9_f32, 0.9_f32) * 0.04_f32).clamp(0.85_f32, 1.15_f32);
        mask *= noise_scale;

        mask.clamp(0.0, 1.0)
    }

    pub(super) fn continent_ridge_factor(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let base_radius = self.config.continent_radius.max(0.01_f32);
        let mut strongest = 0.0_f32;

        for site in &self.continent_sites {
            let du = torus_distance(u, site.position.x);
            let dv = torus_distance(v, site.position.y);

            let delta = Vec2::new(du, dv);
            let axis = site.axis_ratio.max(0.2_f32);
            let major = (base_radius * site.radius_scale * axis).max(0.01_f32);
            let minor = (base_radius * site.radius_scale / axis).max(0.01_f32);

            let cos_o = site.orientation.cos();
            let sin_o = site.orientation.sin();
            let rotated_x = delta.x * cos_o + delta.y * sin_o;
            let rotated_y = -delta.x * sin_o + delta.y * cos_o;

            let normalized_sq = (rotated_x / major).powi(2) + (rotated_y / minor).powi(2);
            if normalized_sq > 1.6_f32 {
                continue;
            }

            let crest_dir = Vec2::new(site.ridge_angle.cos(), site.ridge_angle.sin());
            let along = du * crest_dir.x + dv * crest_dir.y;
            let across = -du * crest_dir.y + dv * crest_dir.x;

            let longitudinal = (1.0_f32 - (along.abs() / (major.max(minor) * 0.95_f32))).max(0.0);
            let transverse_span = (minor * 0.9_f32 + major * 0.35_f32).max(0.02_f32);
            let transverse = (1.0_f32 - (across.abs() / transverse_span)).max(0.0);
            let interior = (1.0_f32 - normalized_sq.sqrt()).max(0.0).powf(0.85_f32);
            let weight = site.weight.sqrt().clamp(0.6_f32, 1.8_f32);

            strongest = strongest
                .max(longitudinal.powf(0.7_f32) * transverse.powf(0.85_f32) * interior * weight);
        }

        strongest.clamp(0.0, 1.0)
    }
}
