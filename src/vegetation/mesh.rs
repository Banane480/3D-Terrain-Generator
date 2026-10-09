use ash::vk;
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VegetationVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color_wind: [f32; 4], // rgb = couleur, a = amplitude de balancement au vent (0.0=base tronc, 1.0=cime)
}

impl VegetationVertex {
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
            // location 2: color_wind vec4
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(24),
        ]
    }
}

pub struct VegetationMesh {
    pub vertices: Vec<VegetationVertex>,
    pub indices: Vec<u32>,
}

impl VegetationMesh {
    /// Crée un maillage 3D complet de Sapin conifère réaliste
    pub fn create_pine_tree() -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let trunk_color = [0.32, 0.21, 0.13, 0.05];
        let pine_foliage = [0.12, 0.35, 0.14, 0.85];

        // 1. Tronc (cylindre à 6 côtés)
        let segments = 6;
        let trunk_height = 5.5;
        let r_base = 0.25;
        let r_top = 0.12;

        let base_idx = vertices.len() as u32;
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let cos = angle.cos();
            let sin = angle.sin();

            // Sommet base
            vertices.push(VegetationVertex {
                position: [cos * r_base, 0.0, sin * r_base],
                normal: [cos, 0.0, sin],
                color_wind: [trunk_color[0], trunk_color[1], trunk_color[2], 0.0],
            });

            // Sommet haut du tronc
            vertices.push(VegetationVertex {
                position: [cos * r_top, trunk_height, sin * r_top],
                normal: [cos, 0.0, sin],
                color_wind: [trunk_color[0], trunk_color[1], trunk_color[2], 0.20],
            });
        }

        for i in 0..segments {
            let next = (i + 1) % segments;
            let b0 = base_idx + i * 2;
            let t0 = b0 + 1;
            let b1 = base_idx + next * 2;
            let t1 = b1 + 1;

            indices.push(b0);
            indices.push(b1);
            indices.push(t0);

            indices.push(t0);
            indices.push(b1);
            indices.push(t1);
        }

        // 2. Étages de feuillage conique à branches étoilées (4 niveaux)
        let layers = [
            (2.2, 5.2, 2.3, 0.85),
            (4.0, 7.0, 1.8, 0.90),
            (5.8, 8.6, 1.3, 0.95),
            (7.4, 10.5, 0.8, 1.0),
        ];

        let fol_segments = 8;
        for &(y_base, y_tip, radius, wind) in &layers {
            let tip_idx = vertices.len() as u32;
            // Sommet de la pointe du cône
            vertices.push(VegetationVertex {
                position: [0.0, y_tip, 0.0],
                normal: [0.0, 1.0, 0.0],
                color_wind: [pine_foliage[0] * 1.1, pine_foliage[1] * 1.1, pine_foliage[2] * 1.1, wind],
            });

            let ring_base = vertices.len() as u32;
            for i in 0..fol_segments {
                let angle = (i as f32 / fol_segments as f32) * std::f32::consts::TAU;
                // Alternance de rayon pour former des branches en étoile
                let r = if i % 2 == 0 { radius } else { radius * 0.72 };
                let x = angle.cos() * r;
                let z = angle.sin() * r;
                let norm = Vec3::new(x, (y_tip - y_base) * 0.4, z).normalize();

                vertices.push(VegetationVertex {
                    position: [x, y_base, z],
                    normal: [norm.x, norm.y, norm.z],
                    color_wind: [pine_foliage[0], pine_foliage[1], pine_foliage[2], wind * 0.75],
                });
            }

            for i in 0..fol_segments {
                let next = (i + 1) % fol_segments;
                let v0 = ring_base + i;
                let v1 = ring_base + next;

                // Triangle cône vers le haut
                indices.push(tip_idx);
                indices.push(v0);
                indices.push(v1);

                // Triangle sous-face (pour visibilité depuis le dessous)
                indices.push(tip_idx);
                indices.push(v1);
                indices.push(v0);
            }
        }

        Self { vertices, indices }
    }

    /// Crée un maillage 3D d'Arbre feuillu (Chêne / Bouleau)
    pub fn create_broadleaf_tree() -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let trunk_color = [0.38, 0.25, 0.16, 0.0];
        let foliage_base = [0.22, 0.52, 0.18, 0.85];

        // 1. Tronc trapu robuste
        let segments = 6;
        let trunk_height = 3.6;
        let r_base = 0.32;
        let r_top = 0.18;

        let base_idx = vertices.len() as u32;
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let cos = angle.cos();
            let sin = angle.sin();

            vertices.push(VegetationVertex {
                position: [cos * r_base, 0.0, sin * r_base],
                normal: [cos, 0.0, sin],
                color_wind: trunk_color,
            });

            vertices.push(VegetationVertex {
                position: [cos * r_top, trunk_height, sin * r_top],
                normal: [cos, 0.0, sin],
                color_wind: [trunk_color[0], trunk_color[1], trunk_color[2], 0.15],
            });
        }

        for i in 0..segments {
            let next = (i + 1) % segments;
            let b0 = base_idx + i * 2;
            let t0 = b0 + 1;
            let b1 = base_idx + next * 2;
            let t1 = b1 + 1;

            indices.push(b0);
            indices.push(b1);
            indices.push(t0);

            indices.push(t0);
            indices.push(b1);
            indices.push(t1);
        }

        // 2. Canopée volumétrique touffue (grappes de couronnes de feuillage)
        let crowns = [
            (Vec3::new(0.0, 5.2, 0.0), 2.5, 0.95),
            (Vec3::new(1.1, 4.4, 0.6), 1.9, 0.85),
            (Vec3::new(-0.9, 4.2, -0.7), 1.8, 0.85),
            (Vec3::new(0.3, 4.3, -1.1), 1.7, 0.85),
        ];

        for (center, radius, wind) in crowns {
            let c_base = vertices.len() as u32;
            let rings = 5;
            let sectors = 7;

            for r_idx in 0..=rings {
                let v = r_idx as f32 / rings as f32;
                let phi = v * std::f32::consts::PI;

                for s_idx in 0..=sectors {
                    let u = s_idx as f32 / sectors as f32;
                    let theta = u * std::f32::consts::TAU;

                    let x = theta.cos() * phi.sin();
                    let y = phi.cos();
                    let z = theta.sin() * phi.sin();

                    let pos = center + Vec3::new(x, y, z) * radius;
                    let norm = Vec3::new(x, y, z).normalize();

                    let color = [
                        foliage_base[0] * (0.85 + (y + 1.0) * 0.15),
                        foliage_base[1] * (0.85 + (y + 1.0) * 0.15),
                        foliage_base[2] * (0.85 + (y + 1.0) * 0.15),
                        wind * (0.6 + (y + 1.0) * 0.2),
                    ];

                    vertices.push(VegetationVertex {
                        position: [pos.x, pos.y, pos.z],
                        normal: [norm.x, norm.y, norm.z],
                        color_wind: color,
                    });
                }
            }

            for r_idx in 0..rings {
                for s_idx in 0..sectors {
                    let cur = c_base + (r_idx * (sectors + 1) + s_idx) as u32;
                    let next = cur + (sectors + 1) as u32;

                    indices.push(cur);
                    indices.push(next);
                    indices.push(cur + 1);

                    indices.push(cur + 1);
                    indices.push(next);
                    indices.push(next + 1);
                }
            }
        }

        Self { vertices, indices }
    }

    /// Crée un maillage 3D de buisson dense pour la végétation basse
    pub fn create_bush() -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let bush_color = [0.18, 0.44, 0.16, 0.65];
        let center = Vec3::new(0.0, 0.7, 0.0);
        let radius = 1.15;
        let rings = 4;
        let sectors = 6;
        let c_base = vertices.len() as u32;

        for r_idx in 0..=rings {
            let v = r_idx as f32 / rings as f32;
            let phi = v * std::f32::consts::PI;

            for s_idx in 0..=sectors {
                let u = s_idx as f32 / sectors as f32;
                let theta = u * std::f32::consts::TAU;

                let x = theta.cos() * phi.sin();
                let y = phi.cos().max(-0.2); // Plat au sol
                let z = theta.sin() * phi.sin();

                let pos = center + Vec3::new(x, y, z) * radius;
                let norm = Vec3::new(x, y, z).normalize();

                vertices.push(VegetationVertex {
                    position: [pos.x, pos.y.max(0.0), pos.z],
                    normal: [norm.x, norm.y, norm.z],
                    color_wind: bush_color,
                });
            }
        }

        for r_idx in 0..rings {
            for s_idx in 0..sectors {
                let cur = c_base + (r_idx * (sectors + 1) + s_idx) as u32;
                let next = cur + (sectors + 1) as u32;

                indices.push(cur);
                indices.push(next);
                indices.push(cur + 1);

                indices.push(cur + 1);
                indices.push(next);
                indices.push(next + 1);
            }
        }

        Self { vertices, indices }
    }
}
