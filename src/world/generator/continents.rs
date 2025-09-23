use bevy::math::Vec2;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::TAU;

use super::util::torus_distance;
use super::WorldGenerator;

#[derive(Clone)]
pub(super) struct ContinentSite {
    pub(super) position: Vec2,
    pub(super) ridge_angle: f32,
}

pub(super) fn generate_continent_sites(seed: u64, count: u32) -> Vec<ContinentSite> {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = count.max(1);
    let grid_len = (n as f32).sqrt().ceil() as u32;
    let cell_size = 1.0 / grid_len as f32;
    let jitter = cell_size * 0.6;

    let mut sites = Vec::with_capacity(n as usize);
    let mut index = 0u32;

    let offset_u = rng.gen::<f32>() * cell_size;
    let offset_v = rng.gen::<f32>() * cell_size;

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
            let angle = rng.gen::<f32>() * TAU;
            sites.push(ContinentSite {
                position: Vec2::new(u, v),
                ridge_angle: angle,
            });
        }
        if index >= n {
            break;
        }
    }

    sites
}

impl WorldGenerator {
    pub(super) fn continent_site_mask(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let radius = self.config.continent_radius.max(0.01);
        let radius_sq = radius * radius;
        let edge_power = self.config.continent_edge_power.max(0.1);
        let mut best = 0.0_f32;

        for site in &self.continent_sites {
            let du = torus_distance(u, site.position.x);
            let dv = torus_distance(v, site.position.y);
            let dist_sq = du * du + dv * dv;

            if dist_sq <= radius_sq {
                let influence = 1.0 - (dist_sq / radius_sq);
                best = best.max(influence);
            }
        }

        if best == 0.0 {
            0.0
        } else {
            best.powf(edge_power)
        }
    }

    pub(super) fn continent_ridge_factor(&self, u: f32, v: f32) -> f32 {
        if self.continent_sites.is_empty() {
            return 1.0;
        }

        let radius = self.config.continent_radius.max(0.01);
        let ridge_width = (radius * 0.3).max(0.02);
        let mut strongest = 0.0_f32;

        for site in &self.continent_sites {
            let du = torus_distance(u, site.position.x);
            let dv = torus_distance(v, site.position.y);

            let dist_sq = du * du + dv * dv;
            if dist_sq > radius * radius {
                continue;
            }

            let cos_a = site.ridge_angle.cos();
            let sin_a = site.ridge_angle.sin();
            let along = du * cos_a + dv * sin_a;
            let across = -du * sin_a + dv * cos_a;

            let longitudinal = (1.0 - (along.abs() / radius)).max(0.0);
            let transverse = (1.0 - (across.abs() / ridge_width)).max(0.0);
            strongest = strongest.max(longitudinal * transverse);
        }

        strongest.clamp(0.0, 1.0)
    }
}
