#version 450

layout(location = 0) in vec3 out_ray_dir;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(location = 0) out vec4 out_color;

void main() {
    vec3 dir = normalize(out_ray_dir);
    vec3 sun_dir = normalize(camera.sun_dir.xyz);

    // Dégradé atmosphérique de Rayleigh (Zénith bleu profond -> Horizon chaud et lumineux)
    float height = clamp(dir.y, 0.0, 1.0);
    vec3 zenith_color = vec3(0.16, 0.38, 0.78);
    vec3 horizon_color = vec3(0.72, 0.84, 0.95);
    vec3 ground_haze = vec3(0.55, 0.65, 0.75);

    vec3 sky = (dir.y >= 0.0)
        ? mix(horizon_color, zenith_color, pow(height, 0.55))
        : mix(horizon_color, ground_haze, clamp(-dir.y * 3.0, 0.0, 1.0));

    // Lueur solaire et disque du soleil (diffusion de Mie)
    float sun_dot = max(dot(dir, sun_dir), 0.0);
    float sun_disk = smoothstep(0.9982, 0.9996, sun_dot);
    vec3 sun_disk_color = vec3(1.0, 0.98, 0.90) * 12.0;

    // Halo solaire doux et halo étendu
    float sun_halo = pow(sun_dot, 32.0) * 0.70 + pow(sun_dot, 6.0) * 0.30;
    vec3 halo_color = vec3(1.0, 0.85, 0.65) * 1.5;

    // Brume chaude près du soleil sur l'horizon
    float horizon_sun = pow(sun_dot, 3.0) * (1.0 - height) * 0.5;
    vec3 horizon_glow = vec3(1.0, 0.70, 0.45) * horizon_sun;

    vec3 final_color = sky + sun_disk_color * sun_disk + halo_color * sun_halo + horizon_glow;

    // ACES filmic tonemapping
    vec3 a = final_color * (2.51 * final_color + 0.03);
    vec3 b = final_color * (2.43 * final_color + 0.59) + 0.14;
    vec3 mapped = clamp(a / b, 0.0, 1.0);

    out_color = vec4(mapped, 1.0);
}
