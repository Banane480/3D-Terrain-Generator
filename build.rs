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
    println!("cargo:rerun-if-changed=shaders/terrain.vert");
    println!("cargo:rerun-if-changed=shaders/terrain.frag");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR non defini");
    let out_path = PathBuf::from(out_dir);

    // Vertex shader
    let vert_src = fs::read_to_string("shaders/terrain.vert")
        .expect("Impossible de lire shaders/terrain.vert");
    let vert_spv = compile_glsl_to_spv(&vert_src, naga::ShaderStage::Vertex, "terrain.vert");
    fs::write(out_path.join("terrain_vert.spv"), vert_spv)
        .expect("Impossible d'ecrire terrain_vert.spv");

    // Fragment shader
    let frag_src = fs::read_to_string("shaders/terrain.frag")
        .expect("Impossible de lire shaders/terrain.frag");
    let frag_spv = compile_glsl_to_spv(&frag_src, naga::ShaderStage::Fragment, "terrain.frag");
    fs::write(out_path.join("terrain_frag.spv"), frag_spv)
        .expect("Impossible d'ecrire terrain_frag.spv");
}
