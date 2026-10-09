use std::path::Path;

use glam::Vec3;

use super::VegetationVertex;

pub struct LoadedVegetation {
    pub vertices: Vec<VegetationVertex>,
    pub lod_indices: [Vec<u32>; 3],
}

pub fn load_quaternius_lods(
    path: &Path,
    target_height: f32,
) -> Result<LoadedVegetation, Box<dyn std::error::Error>> {
    let (document, buffers, _images) = gltf::import(path)?;
    let mut vertices = Vec::new();
    let mut source_indices = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                return Err(format!("{}: primitive non triangulaire", path.display()).into());
            }
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
            let positions: Vec<_> = reader.read_positions().ok_or("positions glTF absentes")?.collect();
            let normals: Vec<_> = reader.read_normals().ok_or("normales glTF absentes")?.collect();
            let pbr = primitive.material().pbr_metallic_roughness();
            let factor = pbr.base_color_factor();
            let (layer, texcoords) = pbr.base_color_texture().map(|info| {
                let source = info.texture().source().index();
                let texcoords: Vec<_> = reader.read_tex_coords(info.tex_coord())
                    .expect("coordonnees UV glTF absentes").into_f32().collect();
                (texture_layer(&document.images().nth(source).unwrap()), texcoords)
            }).ok_or("texture de couleur glTF absente")?;
            let layer = layer.ok_or("texture de vegetation inconnue")?;
            if normals.len() != positions.len() || texcoords.len() != positions.len() {
                return Err(format!("{}: attributs de sommets incoherents", path.display()).into());
            }

            let base = vertices.len() as u32;
            for (i, position) in positions.into_iter().enumerate() {
                vertices.push(VegetationVertex {
                    position,
                    normal: normals[i],
                    color_wind: [factor[0], factor[1], factor[2], 0.0],
                    uv_layer: [texcoords[i][0], texcoords[i][1], layer, 0.2],
                });
            }
            let mut indices: Vec<u32> = reader.read_indices()
                .map(|indices| indices.into_u32().collect())
                .unwrap_or_else(|| (0..vertices.len() as u32 - base).collect());
            if indices.is_empty() || indices.len() % 3 != 0 || indices.iter().any(|&i| i >= vertices.len() as u32 - base) {
                return Err(format!("{}: indices glTF invalides", path.display()).into());
            }
            source_indices.extend(indices.drain(..).map(|i| i + base));
        }
    }

    let min = vertices.iter().fold(Vec3::splat(f32::INFINITY), |v, p| v.min(Vec3::from_array(p.position)));
    let max = vertices.iter().fold(Vec3::splat(f32::NEG_INFINITY), |v, p| v.max(Vec3::from_array(p.position)));
    if !min.is_finite() || max.y - min.y <= 0.0001 {
        return Err(format!("{}: modele vide", path.display()).into());
    }
    let scale = target_height / (max.y - min.y);
    let origin = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
    for vertex in &mut vertices {
        let position = (Vec3::from_array(vertex.position) - origin) * scale;
        vertex.position = position.to_array();
        vertex.color_wind[3] = (position.y / target_height).clamp(0.0, 1.0).powi(2);
    }

    let adapter = meshopt::VertexDataAdapter::new(
        bytemuck::cast_slice(&vertices),
        std::mem::size_of::<VegetationVertex>(),
        0,
    )?;
    let mut lod_indices = [source_indices.clone(), Vec::new(), Vec::new()];
    for (lod, ratio, error) in [(1, 0.5, 0.03), (2, 0.2, 0.1)] {
        let target = ((source_indices.len() as f32 * ratio) as usize / 3 * 3).max(3);
        lod_indices[lod] = meshopt::simplify(
            &source_indices,
            &adapter,
            target,
            error,
            meshopt::SimplifyOptions::Permissive | meshopt::SimplifyOptions::Prune,
            None,
        );
        if lod_indices[lod].len() > target + target / 5 {
            lod_indices[lod] = meshopt::simplify_sloppy(
                &source_indices,
                &adapter,
                target,
                1.0,
                None,
            );
        }
        if lod_indices[lod].is_empty() {
            let step = if lod == 1 { 2 } else { 5 };
            lod_indices[lod] = source_indices
                .chunks_exact(3)
                .step_by(step)
                .flatten()
                .copied()
                .collect();
        }
    }
    for indices in &mut lod_indices {
        meshopt::optimize_vertex_cache_in_place(indices, vertices.len());
    }
    Ok(LoadedVegetation {
        vertices,
        lod_indices,
    })
}

fn texture_layer(image: &gltf::Image<'_>) -> Option<f32> {
    let gltf::image::Source::Uri { uri, .. } = image.source() else { return None };
    match Path::new(uri).file_name()?.to_str()? {
        "Bark_NormalTree.png" => Some(4.0),
        "Leaf_Pine_C.png" => Some(5.0),
        "Leaves_NormalTree_C.png" => Some(6.0),
        "Leaves_TwistedTree_C.png" => Some(7.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_models_have_three_valid_lods() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/models/vegetation/quaternius/glTF");
        for (name, height) in [("Pine_1", 10.5), ("CommonTree_1", 8.0), ("Bush_Common", 1.8)] {
            let loaded = load_quaternius_lods(&root.join(format!("{name}.gltf")), height).unwrap();
            assert!(loaded.vertices.iter().all(|v| v.position[1] >= -0.001));
            assert!(loaded.lod_indices[1].len() < loaded.lod_indices[0].len(), "{name}");
            assert!(loaded.lod_indices[2].len() < loaded.lod_indices[1].len(), "{name}");
            assert!(loaded.lod_indices.iter().all(|lod| !lod.is_empty()), "{name}");
            assert!(loaded.lod_indices.iter().flatten().all(|&i| i < loaded.vertices.len() as u32));
            assert!(loaded.vertices.iter().all(|v| (4.0..=7.0).contains(&v.uv_layer[2])));
        }
    }
}
