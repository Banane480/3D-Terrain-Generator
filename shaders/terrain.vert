#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_biome_weights;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(location = 0) out vec3 out_world_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec2 out_uv;
layout(location = 3) out vec4 out_biome_weights;

void main() {
    out_world_pos = in_position;
    out_normal = in_normal;
    out_uv = in_uv;
    out_biome_weights = in_biome_weights;

    gl_Position = camera.view_proj * vec4(in_position, 1.0);
}
