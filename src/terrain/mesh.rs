use ash::vk;
use crate::terrain::generator::TerrainGenerator;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_weights: [f32; 4],
}

impl TerrainVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        [
            // location 0: position vec3
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // location 1: normal vec3
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            // location 2: uv vec2
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
            // location 3: biome_weights vec4
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(32),
        ]
    }
}

pub struct TerrainMesh {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
}

impl TerrainMesh {
    /// Génère un maillage continu régulier pour une zone carrée (chunk)
    pub fn generate_chunk(
        generator: &TerrainGenerator,
        origin_x: f32,
        origin_z: f32,
        size: f32,
        resolution: usize, // Nombre de quads par arête
    ) -> Self {
        let verts_per_edge = resolution + 1;
        let mut vertices = Vec::with_capacity(verts_per_edge * verts_per_edge);
        let step = size / resolution as f32;

        for z_idx in 0..verts_per_edge {
            let z = origin_z + (z_idx as f32) * step;
            for x_idx in 0..verts_per_edge {
                let x = origin_x + (x_idx as f32) * step;
                let y = generator.sample_height(x, z);
                let normal = generator.sample_normal(x, z);
                let biome_weights = generator.sample_biome_weights(x, z, y, normal);

                vertices.push(TerrainVertex {
                    position: [x, y, z],
                    normal: [normal.x, normal.y, normal.z],
                    uv: [x * 0.05, z * 0.05],
                    biome_weights,
                });
            }
        }

        let mut indices = Vec::with_capacity(resolution * resolution * 6);
        for z_idx in 0..resolution {
            for x_idx in 0..resolution {
                let top_left = (z_idx * verts_per_edge + x_idx) as u32;
                let top_right = top_left + 1;
                let bottom_left = ((z_idx + 1) * verts_per_edge + x_idx) as u32;
                let bottom_right = bottom_left + 1;

                // Triangle 1
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Triangle 2
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        Self { vertices, indices }
    }
}
