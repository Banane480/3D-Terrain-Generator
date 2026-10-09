use ash::vk;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyVertex {
    pub position: [f32; 3],
}

impl SkyVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
        ]
    }
}

pub struct SkyMesh {
    pub vertices: Vec<SkyVertex>,
    pub indices: Vec<u32>,
}

impl SkyMesh {
    /// Crée une demi-sphère de ciel atmosphérique enveloppant la scène
    pub fn create_sky_dome() -> Self {
        let rings = 16;
        let sectors = 24;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for r in 0..=rings {
            let v = r as f32 / rings as f32;
            let phi = v * std::f32::consts::FRAC_PI_2; // 0 à PI/2 (haut vers horizon)

            for s in 0..=sectors {
                let u = s as f32 / sectors as f32;
                let theta = u * std::f32::consts::TAU;

                let x = theta.cos() * phi.sin();
                let y = phi.cos(); // 1.0 au zénith, 0.0 à l'horizon
                let z = theta.sin() * phi.sin();

                vertices.push(SkyVertex {
                    position: [x, y, z],
                });
            }
        }

        // Ajouter une jupe sous l'horizon pour éviter tout trou noir
        let base_idx = vertices.len() as u32;
        for s in 0..=sectors {
            let u = s as f32 / sectors as f32;
            let theta = u * std::f32::consts::TAU;
            vertices.push(SkyVertex {
                position: [theta.cos(), -0.25, theta.sin()],
            });
        }

        for r in 0..rings {
            for s in 0..sectors {
                let cur = (r * (sectors + 1) + s) as u32;
                let next = cur + (sectors + 1) as u32;

                indices.push(cur);
                indices.push(cur + 1);
                indices.push(next);

                indices.push(cur + 1);
                indices.push(next + 1);
                indices.push(next);
            }
        }

        // Triangles de la jupe inférieure
        let last_ring = (rings * (sectors + 1)) as u32;
        for s in 0..sectors {
            let cur = last_ring + s as u32;
            let next = base_idx + s as u32;

            indices.push(cur);
            indices.push(cur + 1);
            indices.push(next);

            indices.push(cur + 1);
            indices.push(next + 1);
            indices.push(next);
        }

        Self { vertices, indices }
    }
}
