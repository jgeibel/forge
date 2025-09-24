use bevy::math::Vec2;
use std::collections::{HashMap, HashSet};

use crate::world::config::WorldGenConfig;

use super::continents::ContinentSite;
use super::util::{torus_delta, torus_distance, wrap_vec2};

#[derive(Clone, Copy, Default)]
pub(crate) struct PlateSample {
    pub(super) plate_id: usize,
    pub(super) drift: Vec2,
    pub(super) convergence: f32,
    pub(super) divergence: f32,
    pub(super) shear: f32,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(super) struct PlateInfo {
    pub(super) id: usize,
    pub(super) site_index: usize,
    pub(super) centroid: Vec2,
    pub(super) drift: Vec2,
    pub(super) rotation_rate: f32,
    pub(super) area: f32,
    pub(super) neighbors: Vec<usize>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(super) struct PlateBoundary {
    pub(super) plates: (usize, usize),
    pub(super) length: f32,
    pub(super) normal: Vec2,
    pub(super) relative_drift: Vec2,
}

#[derive(Clone)]
pub(super) struct PlateMap {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) assignment: Vec<usize>,
    pub(super) plates: Vec<PlateInfo>,
    pub(super) _boundaries: Vec<PlateBoundary>,
}

impl PlateMap {
    pub(super) fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            assignment: Vec::new(),
            plates: Vec::new(),
            _boundaries: Vec::new(),
        }
    }

    pub(super) fn generate(config: &WorldGenConfig, sites: &[ContinentSite]) -> Self {
        if sites.is_empty() {
            return Self::empty();
        }

        let resolution = ((config.planet_size as f32 / 96.0).clamp(64.0, 512.0)).round() as usize;
        let width = resolution.max(4);
        let height = resolution.max(4);

        let mut assignment = vec![0usize; width * height];
        let mut accum_sum = vec![Vec2::ZERO; sites.len()];
        let mut accum_count = vec![0.0_f32; sites.len()];

        for y in 0..height {
            let v = (y as f32 + 0.5) / height as f32;
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let best = nearest_site(u, v, sites);
                let idx = y * width + x;
                assignment[idx] = best;
                accum_sum[best] += wrap_vec2(Vec2::new(u, v));
                accum_count[best] += 1.0;
            }
        }

        let mut neighbor_sets: Vec<HashSet<usize>> = vec![HashSet::new(); sites.len()];
        let mut boundary_map: HashMap<(usize, usize), BoundaryAccumulator> = HashMap::new();

        for y in 0..height {
            for x in 0..width {
                let a = assignment[y * width + x];
                let current = cell_position(width, height, x as isize, y as isize);

                let neighbors = [((x + 1) % width, y), (x, (y + 1) % height)];

                for &(nx, ny) in &neighbors {
                    let b = assignment[ny * width + nx];
                    if a == b {
                        continue;
                    }

                    let key = ordered_pair(a, b);
                    let entry = boundary_map
                        .entry(key)
                        .or_insert_with(BoundaryAccumulator::default);

                    let neighbor_pos = cell_position(width, height, nx as isize, ny as isize);
                    let mut edge = Vec2::new(
                        torus_delta(current.x, neighbor_pos.x),
                        torus_delta(current.y, neighbor_pos.y),
                    );

                    let length = edge.length().max(f32::EPSILON);
                    edge /= length;
                    let normal = Vec2::new(-edge.y, edge.x);

                    entry.length += length;
                    entry.normal += normal;

                    neighbor_sets[a].insert(b);
                    neighbor_sets[b].insert(a);
                }
            }
        }

        let mut plates = Vec::with_capacity(sites.len());
        for (index, site) in sites.iter().enumerate() {
            let count = accum_count[index].max(1.0);
            let centroid = wrap_vec2(accum_sum[index] / count);
            let neighbors = neighbor_sets[index].iter().copied().collect();
            plates.push(PlateInfo {
                id: index,
                site_index: index,
                centroid,
                drift: site.drift,
                rotation_rate: 0.0,
                area: count / (width as f32 * height as f32),
                neighbors,
            });
        }

        let mut boundaries = Vec::with_capacity(boundary_map.len());
        for (key, accum) in boundary_map {
            let normal = if accum.normal.length_squared() > f32::EPSILON {
                accum.normal.normalize()
            } else {
                Vec2::new(0.0, 1.0)
            };
            let drift_a = plates[key.0].drift;
            let drift_b = plates[key.1].drift;
            boundaries.push(PlateBoundary {
                plates: key,
                length: accum.length,
                normal,
                relative_drift: drift_b - drift_a,
            });
        }

        Self {
            width,
            height,
            assignment,
            plates,
            _boundaries: boundaries,
        }
    }

    pub(super) fn sample(&self, u: f32, v: f32) -> PlateSample {
        if self.plates.is_empty() || self.width == 0 || self.height == 0 {
            return PlateSample::default();
        }

        let xf = (u.rem_euclid(1.0)) * self.width as f32;
        let yf = (v.rem_euclid(1.0)) * self.height as f32;
        let x = xf.floor() as isize;
        let y = yf.floor() as isize;

        let width = self.width as isize;
        let height = self.height as isize;

        let xi = wrap_index(x, width) as usize;
        let yi = wrap_index(y, height) as usize;
        let primary = self.assignment[yi * self.width + xi];
        let primary_info = &self.plates[primary];

        let current = cell_position(self.width, self.height, x, y);

        let mut convergence = 0.0_f32;
        let mut divergence = 0.0_f32;
        let mut shear = 0.0_f32;
        let mut best_weight = 0.0_f32;

        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap_index(x + dx, width) as usize;
                let ny = wrap_index(y + dy, height) as usize;
                let other = self.assignment[ny * self.width + nx];
                if other == primary {
                    continue;
                }

                let neighbor_pos = cell_position(self.width, self.height, x + dx, y + dy);
                let mut edge = Vec2::new(
                    torus_delta(current.x, neighbor_pos.x),
                    torus_delta(current.y, neighbor_pos.y),
                );

                let distance = edge.length();
                if distance <= f32::EPSILON {
                    continue;
                }

                let weight = 1.0 / distance;
                if weight <= best_weight {
                    continue;
                }

                edge /= distance;
                let normal = Vec2::new(-edge.y, edge.x);

                let drift_primary = primary_info.drift;
                let drift_other = self.plates[other].drift;
                let relative = drift_other - drift_primary;

                convergence = relative.dot(normal).max(0.0);
                divergence = (-relative.dot(normal)).max(0.0);
                shear = relative.dot(edge).abs();
                best_weight = weight;
            }
        }

        PlateSample {
            plate_id: primary,
            drift: primary_info.drift,
            convergence,
            divergence,
            shear,
        }
    }

    pub(super) fn plate_index(&self, u: f32, v: f32) -> usize {
        if self.plates.is_empty() || self.width == 0 || self.height == 0 {
            return 0;
        }

        let xf = (u.rem_euclid(1.0)) * self.width as f32;
        let yf = (v.rem_euclid(1.0)) * self.height as f32;
        let x = wrap_index(xf.floor() as isize, self.width as isize) as usize;
        let y = wrap_index(yf.floor() as isize, self.height as isize) as usize;
        self.assignment[y * self.width + x]
    }
}

fn wrap_index(value: isize, size: isize) -> isize {
    let mut v = value % size;
    if v < 0 {
        v += size;
    }
    v
}

fn nearest_site(u: f32, v: f32, sites: &[ContinentSite]) -> usize {
    let mut best = 0;
    let mut best_dist = f32::MAX;
    for (index, site) in sites.iter().enumerate() {
        let du = torus_distance(u, site.position.x);
        let dv = torus_distance(v, site.position.y);
        let dist = du * du + dv * dv;
        if dist < best_dist {
            best_dist = dist;
            best = index;
        }
    }
    best
}

fn cell_position(width: usize, height: usize, x: isize, y: isize) -> Vec2 {
    let xf = (wrap_index(x, width as isize) as f32 + 0.5) / width as f32;
    let yf = (wrap_index(y, height as isize) as f32 + 0.5) / height as f32;
    Vec2::new(xf, yf)
}

fn ordered_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Default)]
struct BoundaryAccumulator {
    length: f32,
    normal: Vec2,
}
