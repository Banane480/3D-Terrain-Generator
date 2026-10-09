use std::env;
use std::fs;
use std::path::PathBuf;

fn compile_glsl_to_spv(source: &str, stage: naga::ShaderStage, name: &str) -> Vec<u8> {
    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options::from(stage);
    let module = frontend
        .parse(&options, source)
        .unwrap_or_else(|e| panic!("Erreur de parsing GLSL dans {}: {:?}", name, e));

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .unwrap_or_else(|e| panic!("Erreur de validation shader pour {}: {:?}", name, e));

    let spv_opts = naga::back::spv::Options::default();
    let mut words = Vec::new();
    let mut writer = naga::back::spv::Writer::new(&spv_opts)
        .expect("Echec de creation du writer SPIR-V");
    writer
        .write(&module, &info, None, &None, &mut words)
        .unwrap_or_else(|e| panic!("Echec d'emission SPIR-V pour {}: {:?}", name, e));

    let mut bytes = Vec::with_capacity(words.len() * 4);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn main() {
    let shaders = [
        ("shaders/terrain.vert", naga::ShaderStage::Vertex, "terrain_vert.spv"),
        ("shaders/terrain.frag", naga::ShaderStage::Fragment, "terrain_frag.spv"),
        ("shaders/vegetation.vert", naga::ShaderStage::Vertex, "vegetation_vert.spv"),
        ("shaders/vegetation.frag", naga::ShaderStage::Fragment, "vegetation_frag.spv"),
        ("shaders/sky.vert", naga::ShaderStage::Vertex, "sky_vert.spv"),
        ("shaders/sky.frag", naga::ShaderStage::Fragment, "sky_frag.spv"),
        ("shaders/water.vert", naga::ShaderStage::Vertex, "water_vert.spv"),
        ("shaders/water.frag", naga::ShaderStage::Fragment, "water_frag.spv"),
    ];

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR non defini");
    let out_path = PathBuf::from(out_dir);

    for (file_path, stage, out_name) in shaders {
        println!("cargo:rerun-if-changed={}", file_path);
        let src = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Impossible de lire {}", file_path));
        let spv = compile_glsl_to_spv(&src, stage, file_path);
        fs::write(out_path.join(out_name), spv)
            .unwrap_or_else(|_| panic!("Impossible d'ecrire {}", out_name));
    }
}
