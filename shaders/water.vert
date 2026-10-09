#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(location = 0) out vec3 out_world_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec2 out_uv;

void main() {
    vec3 pos = in_position;
    float time = camera.sun_dir.w;

    // Vagues animées (somme d'ondes sinusoïdales à différentes fréquences)
    float w1 = sin(pos.x * 0.15 + time * 2.2) * 0.22;
    float w2 = cos(pos.z * 0.18 + time * 1.8) * 0.18;
    float w3 = sin((pos.x + pos.z) * 0.25 + time * 2.8) * 0.10;
    pos.y += w1 + w2 + w3;

    // Calcul de la normale perturbée par les vagues
    float dx = (cos(pos.x * 0.15 + time * 2.2) * 0.15 * 0.22)
             + (cos((pos.x + pos.z) * 0.25 + time * 2.8) * 0.25 * 0.10);
    float dz = (-sin(pos.z * 0.18 + time * 1.8) * 0.18 * 0.18)
             + (cos((pos.x + pos.z) * 0.25 + time * 2.8) * 0.25 * 0.10);
    vec3 normal = normalize(vec3(-dx, 1.0, -dz));

    out_world_pos = pos;
    out_normal = normal;
    out_uv = in_uv;

    gl_Position = camera.view_proj * vec4(pos, 1.0);
}
