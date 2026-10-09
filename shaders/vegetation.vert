#version 450

// Attributs du sommet (Binding 0)
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_color_wind; // rgb = couleur de base, a = sensibilite vent (0=tronc, 1=cime)
layout(location = 3) in vec4 in_uv_layer; // xy = UV, z = couche texture, w = seuil alpha

// Attributs de l'instance (Binding 1)
layout(location = 4) in vec4 in_model_col0;
layout(location = 5) in vec4 in_model_col1;
layout(location = 6) in vec4 in_model_col2;
layout(location = 7) in vec4 in_model_col3;
layout(location = 8) in vec4 in_variation; // x = phase vent, y = variation teinte, z = amplitude, w = unused

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir; // xyz = sun direction, w = time
} camera;

layout(location = 0) out vec3 out_world_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_color;
layout(location = 3) out vec4 out_uv_layer;

void main() {
    mat4 model = mat4(in_model_col0, in_model_col1, in_model_col2, in_model_col3);
    vec4 world_pos = model * vec4(in_position, 1.0);

    float time = camera.sun_dir.w;
    float wind_factor = in_color_wind.w;
    float wind_phase = in_variation.x;

    // Déplacement dynamique du vent sur la cime et les feuilles
    float sway = (sin(time * 2.2 + world_pos.x * 0.12 + world_pos.z * 0.08 + wind_phase) * 0.45
                + sin(time * 4.1 + world_pos.z * 0.25) * 0.18) * in_variation.z;
    world_pos.x += sway * wind_factor;
    world_pos.z += sway * 0.65 * wind_factor;

    mat3 normal_matrix = mat3(model);
    vec3 world_normal = normalize(normal_matrix * in_normal);

    // Teinte avec variation par instance
    vec3 color = in_color_wind.rgb * in_variation.y;

    out_world_pos = world_pos.xyz;
    out_normal = world_normal;
    out_color = vec4(color, in_color_wind.w);
    out_uv_layer = in_uv_layer;

    gl_Position = camera.view_proj * world_pos;
}
