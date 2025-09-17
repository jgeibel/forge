use crate::block::BlockType;
use bevy::prelude::*;
use rand::prelude::*;

const PARTICLE_LIFETIME: f32 = 1.5;
const PARTICLE_GRAVITY: f32 = -9.8;

#[derive(Component)]
pub struct ExtractionParticle {
    velocity: Vec3,
    lifetime: f32,
    particle_type: ParticleType,
}

#[derive(Clone, Copy)]
pub enum ParticleType {
    Dirt,
    Stone,
    Wood,
    Sand,
    Ice,
}

impl ParticleType {
    pub fn from_block_type(block: BlockType) -> Option<Self> {
        match block {
            BlockType::Dirt | BlockType::Grass => Some(ParticleType::Dirt),
            BlockType::Stone | BlockType::Cobblestone => Some(ParticleType::Stone),
            BlockType::Wood | BlockType::Planks => Some(ParticleType::Wood),
            BlockType::Sand => Some(ParticleType::Sand),
            BlockType::Ice | BlockType::PackedIce | BlockType::Snow => Some(ParticleType::Ice),
            _ => None,
        }
    }

    fn get_color(&self) -> Color {
        match self {
            ParticleType::Dirt => Color::srgb(0.4, 0.3, 0.2),
            ParticleType::Stone => Color::srgb(0.7, 0.7, 0.7),
            ParticleType::Wood => Color::srgb(0.5, 0.35, 0.2),
            ParticleType::Sand => Color::srgb(0.9, 0.8, 0.6),
            ParticleType::Ice => Color::srgb(0.8, 0.9, 1.0),
        }
    }

    fn is_spark(&self) -> bool {
        matches!(self, ParticleType::Stone)
    }
}

pub fn spawn_extraction_particles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    block_type: BlockType,
    intensity: f32,
) {
    let Some(particle_type) = ParticleType::from_block_type(block_type) else {
        return;
    };

    let mut rng = thread_rng();
    let num_particles = ((3.0 + intensity * 5.0) as usize).min(8);

    // Spawn particles from edges where cutting is happening
    for _ in 0..num_particles {
        // Random edge position
        let edge_offset = match rng.gen_range(0..12) {
            // Bottom edges
            0 => Vec3::new(rng.gen_range(-0.5..0.5), -0.5, -0.5),
            1 => Vec3::new(rng.gen_range(-0.5..0.5), -0.5, 0.5),
            2 => Vec3::new(-0.5, -0.5, rng.gen_range(-0.5..0.5)),
            3 => Vec3::new(0.5, -0.5, rng.gen_range(-0.5..0.5)),
            // Top edges
            4 => Vec3::new(rng.gen_range(-0.5..0.5), 0.5, -0.5),
            5 => Vec3::new(rng.gen_range(-0.5..0.5), 0.5, 0.5),
            6 => Vec3::new(-0.5, 0.5, rng.gen_range(-0.5..0.5)),
            7 => Vec3::new(0.5, 0.5, rng.gen_range(-0.5..0.5)),
            // Vertical edges
            8 => Vec3::new(-0.5, rng.gen_range(-0.5..0.5), -0.5),
            9 => Vec3::new(0.5, rng.gen_range(-0.5..0.5), -0.5),
            10 => Vec3::new(-0.5, rng.gen_range(-0.5..0.5), 0.5),
            _ => Vec3::new(0.5, rng.gen_range(-0.5..0.5), 0.5),
        };

        let spawn_pos = position + edge_offset;

        // Velocity based on particle type
        let base_velocity = if particle_type.is_spark() {
            // Sparks fly outward quickly
            Vec3::new(
                rng.gen_range(-3.0..3.0),
                rng.gen_range(2.0..5.0),
                rng.gen_range(-3.0..3.0),
            )
        } else {
            // Debris falls more naturally
            Vec3::new(
                rng.gen_range(-1.5..1.5),
                rng.gen_range(0.5..2.0),
                rng.gen_range(-1.5..1.5),
            )
        };

        let particle_size = if particle_type.is_spark() {
            rng.gen_range(0.01..0.02) // Tiny sparks
        } else {
            rng.gen_range(0.02..0.04) // Small debris particles
        };

        let mesh = meshes.add(Cuboid::new(particle_size, particle_size, particle_size));

        // Add emissive for sparks
        let material = if particle_type.is_spark() {
            materials.add(StandardMaterial {
                base_color: particle_type.get_color(),
                emissive: LinearRgba::from(Color::srgb(1.0, 0.8, 0.3)),
                emissive_exposure_weight: 0.5,
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: particle_type.get_color(),
                ..default()
            })
        };

        commands.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_translation(spawn_pos),
                ..default()
            },
            ExtractionParticle {
                velocity: base_velocity,
                lifetime: PARTICLE_LIFETIME,
                particle_type,
            },
        ));
    }
}

pub fn update_particles(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut ExtractionParticle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in particles.iter_mut() {
        let dt = time.delta_seconds();

        // Update lifetime
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Apply physics
        if !particle.particle_type.is_spark() {
            // Gravity for non-spark particles
            particle.velocity.y += PARTICLE_GRAVITY * dt;
        } else {
            // Sparks slow down but don't fall as much
            particle.velocity *= 0.95;
        }

        // Update position
        transform.translation += particle.velocity * dt;

        // Fade out
        let alpha = particle.lifetime / PARTICLE_LIFETIME;
        transform.scale = Vec3::splat(transform.scale.x * alpha.sqrt());

        // Simple ground collision
        if transform.translation.y < 0.0 {
            transform.translation.y = 0.0;
            particle.velocity.y *= -0.3; // Small bounce
            particle.velocity.x *= 0.7; // Friction
            particle.velocity.z *= 0.7;
        }
    }
}
