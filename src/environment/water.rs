use ash::vk;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl WaterVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
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
        ]
    }
}

pub struct WaterMesh {
    pub vertices: Vec<WaterVertex>,
    pub indices: Vec<u32>,
}

impl WaterMesh {
    /// Crée une nappe d'eau maillée plane au niveau de la mer
    pub fn create_ocean_plane(center_x: f32, center_z: f32, size: f32, resolution: usize, water_height: f32) -> Self {
        let verts_per_edge = resolution + 1;
        let mut vertices = Vec::with_capacity(verts_per_edge * verts_per_edge);
        let step = size / resolution as f32;
        let start_x = center_x - size * 0.5;
        let start_z = center_z - size * 0.5;

        for z_idx in 0..verts_per_edge {
            let z = start_z + (z_idx as f32) * step;
            for x_idx in 0..verts_per_edge {
                let x = start_x + (x_idx as f32) * step;

                vertices.push(WaterVertex {
                    position: [x, water_height, z],
                    normal: [0.0, 1.0, 0.0],
                    uv: [x * 0.05, z * 0.05],
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

                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        Self { vertices, indices }
    }
}
