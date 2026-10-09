use crate::terrain::generator::TerrainGenerator;
use crate::terrain::mesh::{TerrainMesh, TerrainVertex};

pub struct TerrainWorld {
    pub generator: TerrainGenerator,
    pub chunk_size: f32,
    pub chunk_resolution: usize,
    pub view_distance_chunks: i32,
}

impl TerrainWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            generator: TerrainGenerator::new(seed),
            chunk_size: 64.0,
            chunk_resolution: 32, // 32x32 quads par chunk pour fluidite immediate
            view_distance_chunks: 3, // Rayon de 3 chunks => grille de 7x7 chunks = 448m x 448m
        }
    }

    pub fn sample_height(&self, x: f32, z: f32) -> f32 {
        self.generator.sample_height(x, z)
    }

    /// Génère un maillage global continu unifié pour le secteur autour du joueur
    pub fn build_world_mesh(&self, center_x: f32, center_z: f32) -> (Vec<TerrainVertex>, Vec<u32>) {
        let center_chunk_x = (center_x / self.chunk_size).floor() as i32;
        let center_chunk_z = (center_z / self.chunk_size).floor() as i32;

        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        let r = self.view_distance_chunks;
        for cz in (center_chunk_z - r)..=(center_chunk_z + r) {
            for cx in (center_chunk_x - r)..=(center_chunk_x + r) {
                let origin_x = (cx as f32) * self.chunk_size;
                let origin_z = (cz as f32) * self.chunk_size;

                let chunk_mesh = TerrainMesh::generate_chunk(
                    &self.generator,
                    origin_x,
                    origin_z,
                    self.chunk_size,
                    self.chunk_resolution,
                );

                let base_index = all_vertices.len() as u32;
                all_vertices.extend(chunk_mesh.vertices);
                all_indices.extend(chunk_mesh.indices.into_iter().map(|idx| idx + base_index));
            }
        }

        (all_vertices, all_indices)
    }
}
