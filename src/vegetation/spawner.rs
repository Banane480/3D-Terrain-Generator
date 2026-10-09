use glam::{Mat4, Quat, Vec3};
use crate::terrain::generator::TerrainGenerator;
use crate::vegetation::instance::VegetationInstance;

/// Fonction de hachage pseudo-aléatoire 2D rapide et déterministe
fn hash2d(x: i32, z: i32, seed: u32) -> f32 {
    let mut n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((z as u32).wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(1013904223));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n & 0x007fffff) as f32 / 8388607.0
}

pub struct VegetationSpawner;

pub struct SpawnedVegetation {
    pub pines: Vec<VegetationInstance>,
    pub broadleafs: Vec<VegetationInstance>,
    pub bushes: Vec<VegetationInstance>,
}

impl VegetationSpawner {
    pub fn spawn_for_world(
        generator: &TerrainGenerator,
        center_x: f32,
        center_z: f32,
        view_radius: f32,
        seed: u32,
    ) -> SpawnedVegetation {
        let mut pines = Vec::new();
        let mut broadleafs = Vec::new();
        let mut bushes = Vec::new();

        // Grille de sous-cellules pour répartition organique dense
        let cell_size = 6.2; // Densité naturelle pour forêts et vallées
        let min_x = ((center_x - view_radius) / cell_size).floor() as i32;
        let max_x = ((center_x + view_radius) / cell_size).ceil() as i32;
        let min_z = ((center_z - view_radius) / cell_size).floor() as i32;
        let max_z = ((center_z + view_radius) / cell_size).ceil() as i32;

        for cz in min_z..=max_z {
            for cx in min_x..=max_x {
                let r1 = hash2d(cx, cz, seed);
                let r2 = hash2d(cx, cz, seed.wrapping_add(17));
                let r3 = hash2d(cx, cz, seed.wrapping_add(43));
                let r4 = hash2d(cx, cz, seed.wrapping_add(79));

                // Position continue avec gigue (jitter)
                let x = (cx as f32 + 0.10 + r1 * 0.80) * cell_size;
                let z = (cz as f32 + 0.10 + r2 * 0.80) * cell_size;

                // Distance au centre
                let dist_sq = (x - center_x) * (x - center_x) + (z - center_z) * (z - center_z);
                if dist_sq > view_radius * view_radius {
                    continue;
                }

                let y = generator.sample_height(x, z);
                let normal = generator.sample_normal(x, z);
                let biomes = generator.sample_biome_weights(x, z, y, normal);

                // 1. Pas d'arbres sous l'eau ou sur plage immédiate (y > 2.0)
                // 2. Pas d'arbres au-delà de la limite des neiges éternelles (y < 78.0)
                if y <= 2.2 || y >= 78.0 {
                    continue;
                }

                // Orientation aléatoire et variation
                let rotation_angle = r4 * std::f32::consts::TAU;
                let rotation = Quat::from_rotation_y(rotation_angle);
                let wind_phase = r1 * std::f32::consts::TAU;
                let tint_var = 0.90 + r2 * 0.20;
                let sway_amp = 0.85 + r3 * 0.35;

                // A. Zone Alpine / Collines / Crêtes : Sapins & Conifères
                // Les sapins poussent sur pentes plus rocheuses et plus hautes
                if (y > 22.0 || r3 > 0.65) && normal.y > 0.76 && biomes[3] < 0.45 && biomes[0] < 0.35 {
                    if r1 > 0.28 {
                        let scale_val = 0.85 + r2 * 0.50;
                        let scale = Vec3::splat(scale_val);
                        let transform = Mat4::from_scale_rotation_translation(scale, rotation, Vec3::new(x, y, z));
                        let cols = transform.to_cols_array_2d();

                        pines.push(VegetationInstance {
                            model_col0: cols[0],
                            model_col1: cols[1],
                            model_col2: cols[2],
                            model_col3: cols[3],
                            variation: [wind_phase, tint_var, sway_amp, 0.0],
                        });
                        continue;
                    }
                }

                // B. Zone Plaines & Vallées : Chênes / Arbres feuillus et Buissons
                if y <= 35.0 && normal.y > 0.84 && biomes[1] > 0.25 && biomes[2] < 0.45 {
                    if r3 > 0.40 {
                        // Arbres feuillus
                        let scale_val = 0.80 + r1 * 0.45;
                        let scale = Vec3::splat(scale_val);
                        let transform = Mat4::from_scale_rotation_translation(scale, rotation, Vec3::new(x, y, z));
                        let cols = transform.to_cols_array_2d();

                        broadleafs.push(VegetationInstance {
                            model_col0: cols[0],
                            model_col1: cols[1],
                            model_col2: cols[2],
                            model_col3: cols[3],
                            variation: [wind_phase, tint_var, sway_amp, 1.0],
                        });
                    } else if r4 > 0.30 {
                        // Buissons et sous-bois
                        let scale_val = 0.70 + r2 * 0.50;
                        let scale = Vec3::splat(scale_val);
                        let transform = Mat4::from_scale_rotation_translation(scale, rotation, Vec3::new(x, y, z));
                        let cols = transform.to_cols_array_2d();

                        bushes.push(VegetationInstance {
                            model_col0: cols[0],
                            model_col1: cols[1],
                            model_col2: cols[2],
                            model_col3: cols[3],
                            variation: [wind_phase, tint_var, sway_amp, 2.0],
                        });
                    }
                }
            }
        }

        SpawnedVegetation {
            pines,
            broadleafs,
            bushes,
        }
    }
}
