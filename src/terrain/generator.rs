use glam::Vec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, Simplex};

pub struct TerrainGenerator {
    base_noise: Fbm<Simplex>,
    warp_noise: Perlin,
    detail_noise: Fbm<Perlin>,
    moisture_noise: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        let base_noise = Fbm::<Simplex>::new(seed)
            .set_octaves(6)
            .set_frequency(0.0018)
            .set_lacunarity(2.05)
            .set_persistence(0.5);

        let warp_noise = Perlin::new(seed.wrapping_add(101));

        let detail_noise = Fbm::<Perlin>::new(seed.wrapping_add(202))
            .set_octaves(4)
            .set_frequency(0.015)
            .set_persistence(0.45);

        let moisture_noise = Perlin::new(seed.wrapping_add(303));

        Self {
            base_noise,
            warp_noise,
            detail_noise,
            moisture_noise,
        }
    }

    /// Calcule la hauteur d'altitude continue h(x, z)
    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        let x64 = x as f64;
        let z64 = z as f64;

        // Domain warping pour des reliefs et vallées géologiques naturels
        let warp_scale = 0.0035;
        let warp_amp = 60.0;
        let wx = x64 + self.warp_noise.get([x64 * warp_scale, z64 * warp_scale]) * warp_amp;
        let wz = z64 + self.warp_noise.get([x64 * warp_scale + 73.1, z64 * warp_scale + 91.7]) * warp_amp;

        // Bruit de base continental
        let base = self.base_noise.get([wx, wz]) as f32; // Entre -1.0 et 1.0

        // Reliefs montagneux accentués
        let mountain_shape = if base > 0.05 {
            let normalized = (base - 0.05) / 0.95;
            normalized.powf(1.6) * 110.0
        } else {
            base * 30.0
        };

        // Micro-reliefs et variations de terrain haute définition
        let detail = self.detail_noise.get([x64 * 0.008, z64 * 0.008]) as f32 * 6.5;

        // Altitude finale continue
        mountain_shape + detail
    }

    /// Calcule la normale unitaire analytique par différences centrales
    pub fn sample_normal(&self, x: f32, z: f32) -> Vec3 {
        const EPS: f32 = 0.5;
        let h_l = self.sample_height(x - EPS, z);
        let h_r = self.sample_height(x + EPS, z);
        let h_d = self.sample_height(x, z - EPS);
        let h_u = self.sample_height(x, z + EPS);

        let dx = (h_l - h_r) / (2.0 * EPS);
        let dz = (h_d - h_u) / (2.0 * EPS);

        Vec3::new(dx, 1.0, dz).normalize()
    }

    /// Détermine les poids de mélange des biomes [sand, grass/forest, rock, snow]
    pub fn sample_biome_weights(&self, x: f32, z: f32, height: f32, normal: Vec3) -> [f32; 4] {
        let slope = 1.0 - normal.y.clamp(0.0, 1.0);
        let moisture = ((self.moisture_noise.get([x as f64 * 0.0012, z as f64 * 0.0012]) as f32) + 1.0) * 0.5;

        // 1. Plage / Sable à basse altitude ou zones très arides
        let sand = (1.0 - ((height - 1.0) / 4.5).clamp(0.0, 1.0)).powf(1.5)
            + if moisture < 0.25 && height < 30.0 { (0.25 - moisture) * 1.5 } else { 0.0 };

        // 2. Plaine / Forêt verdoyante modulée par l'humidité
        let grass_range = if height >= 2.0 && height < 55.0 {
            let up = ((height - 2.0) / 6.0).clamp(0.0, 1.0);
            let down = (1.0 - ((height - 35.0) / 20.0)).clamp(0.0, 1.0);
            up * down * (0.4 + moisture * 0.6)
        } else {
            0.0
        };

        // 3. Roche alpine / Falaises
        let height_rock = ((height - 30.0) / 45.0).clamp(0.0, 1.0);
        let slope_rock = (slope / 0.45).clamp(0.0, 1.0).powf(2.0);
        let rock = (height_rock + slope_rock * 1.5).clamp(0.0, 1.0);

        // 4. Sommets enneigés
        let snow = ((height - 65.0) / 30.0).clamp(0.0, 1.0).powf(1.8);

        // Normalisation de la somme des poids à 1.0
        let total = sand + grass_range + rock + snow + 0.0001;
        [sand / total, grass_range / total, rock / total, snow / total]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_height() {
        let gen = TerrainGenerator::new(1337);
        let h0 = gen.sample_height(0.0, 0.0);
        let n0 = gen.sample_normal(0.0, 0.0);
        let b0 = gen.sample_biome_weights(0.0, 0.0, h0, n0);
        println!("Height at (0, 0): {}", h0);
        println!("Normal at (0, 0): {:?}", n0);
        println!("Biomes at (0, 0): {:?}", b0);
    }
}

