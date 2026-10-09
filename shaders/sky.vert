#version 450

layout(location = 0) in vec3 in_position;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(location = 0) out vec3 out_ray_dir;

void main() {
    out_ray_dir = in_position;

    // Centrer le dôme de ciel sur la caméra
    vec4 world_pos = vec4(in_position * 800.0 + camera.camera_pos.xyz, 1.0);
    vec4 clip_pos = camera.view_proj * world_pos;

    // Placer le ciel sur le plan de profondeur lointain (depth = 1.0 en coordonnées Vulkan)
    gl_Position = clip_pos.xyww;
}
