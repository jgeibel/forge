use bevy::math::Vec2;
use noise::{NoiseFn, Perlin};

pub(super) fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub(super) fn celsius_to_fahrenheit(c: f32) -> f32 {
    c * 9.0 / 5.0 + 32.0
}

pub(super) fn lerp_color(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        lerp_f32(a[0] as f32, b[0] as f32, t) as u8,
        lerp_f32(a[1] as f32, b[1] as f32, t) as u8,
        lerp_f32(a[2] as f32, b[2] as f32, t) as u8,
    ]
}

pub(super) fn wrap_vec2(v: Vec2) -> Vec2 {
    Vec2::new(v.x.rem_euclid(1.0), v.y.rem_euclid(1.0))
}

pub(super) fn rotate_vec2(vec: Vec2, radians: f32) -> Vec2 {
    let (sin_a, cos_a) = radians.sin_cos();
    Vec2::new(vec.x * cos_a - vec.y * sin_a, vec.x * sin_a + vec.y * cos_a)
}

pub(super) fn torus_noise(noise: &Perlin, u: f32, v: f32, cycles: f32, extra: f32) -> f32 {
    if cycles <= f32::EPSILON {
        return 0.0;
    }

    let cycles = cycles.max(0.01) as f64;
    let theta = (u as f64 * cycles) * std::f64::consts::TAU;
    let phi = (v as f64 * cycles) * std::f64::consts::TAU;
    let extra_angle = (extra as f64) * std::f64::consts::TAU;

    noise.get([
        theta.sin(),
        theta.cos(),
        phi.sin() + extra_angle.sin() * 0.35,
        phi.cos() + extra_angle.cos() * 0.35,
    ]) as f32
}

pub(super) fn wrap_index(value: i32, size: i32) -> i32 {
    let mut result = value % size;
    if result < 0 {
        result += size;
    }
    result
}

pub(super) fn wrap_index_isize(value: isize, size: isize) -> isize {
    let mut result = value % size;
    if result < 0 {
        result += size;
    }
    result
}

pub(super) fn torus_delta(a: f32, b: f32) -> f32 {
    let mut diff = b - a;
    if diff > 0.5 {
        diff -= 1.0;
    } else if diff < -0.5 {
        diff += 1.0;
    }
    diff
}

pub(super) fn torus_distance(a: f32, b: f32) -> f32 {
    let diff = (a - b).abs().fract();
    if diff > 0.5 {
        1.0 - diff
    } else {
        diff
    }
}
