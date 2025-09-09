use noise::{NoiseFn, Perlin, Simplex, Fbm, RidgedMulti, Billow, MultiFractal};
use std::sync::Arc;

#[derive(Clone)]
pub struct NoiseLayer {
    noise: Arc<dyn NoiseFn<f64, 3> + Send + Sync>,
    scale: f64,
    amplitude: f64,
}

impl NoiseLayer {
    pub fn new_perlin(seed: u32, frequency: f64, amplitude: f64) -> Self {
        let perlin = Perlin::new(seed);
        Self {
            noise: Arc::new(perlin),
            scale: frequency,
            amplitude,
        }
    }
    
    pub fn new_simplex(seed: u32, frequency: f64, amplitude: f64) -> Self {
        let simplex = Simplex::new(seed);
        Self {
            noise: Arc::new(simplex),
            scale: frequency,
            amplitude,
        }
    }
    
    pub fn new_fbm(seed: u32, frequency: f64, amplitude: f64, octaves: usize) -> Self {
        let fbm = Fbm::<Perlin>::new(seed)
            .set_frequency(frequency)
            .set_octaves(octaves);
        Self {
            noise: Arc::new(fbm),
            scale: 1.0, // Frequency is already in the Fbm
            amplitude,
        }
    }
    
    pub fn new_ridged(seed: u32, frequency: f64, amplitude: f64, octaves: usize) -> Self {
        let ridged = RidgedMulti::<Perlin>::new(seed)
            .set_frequency(frequency)
            .set_octaves(octaves);
        Self {
            noise: Arc::new(ridged),
            scale: 1.0,
            amplitude,
        }
    }
    
    pub fn new_billow(seed: u32, frequency: f64, amplitude: f64, octaves: usize) -> Self {
        let billow = Billow::<Perlin>::new(seed)
            .set_frequency(frequency)
            .set_octaves(octaves);
        Self {
            noise: Arc::new(billow),
            scale: 1.0,
            amplitude,
        }
    }
    
    pub fn sample_2d(&self, x: f64, z: f64) -> f64 {
        let scaled_x = x * self.scale;
        let scaled_z = z * self.scale;
        self.noise.get([scaled_x, 0.0, scaled_z]) * self.amplitude
    }
    
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let scaled_x = x * self.scale;
        let scaled_y = y * self.scale;
        let scaled_z = z * self.scale;
        self.noise.get([scaled_x, scaled_y, scaled_z]) * self.amplitude
    }
}

#[derive(Clone)]
pub struct LayeredNoise {
    layers: Vec<NoiseLayer>,
}

impl LayeredNoise {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
        }
    }
    
    pub fn add_layer(mut self, layer: NoiseLayer) -> Self {
        self.layers.push(layer);
        self
    }
    
    pub fn sample_2d(&self, x: f64, z: f64) -> f64 {
        self.layers.iter()
            .map(|layer| layer.sample_2d(x, z))
            .sum()
    }
    
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        self.layers.iter()
            .map(|layer| layer.sample_3d(x, y, z))
            .sum()
    }
}