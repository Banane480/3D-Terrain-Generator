#version 450

layout(location = 0) in vec3 in_world_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(location = 0) out vec4 out_color;

void main() {
    vec3 N = normalize(in_normal);
    vec3 L = normalize(camera.sun_dir.xyz);
    vec3 V = normalize(camera.camera_pos.xyz - in_world_pos);

    // Effet de Fresnel (plus on regarde vers l'horizon, plus l'eau reflète le ciel)
    float NdotV = max(dot(N, V), 0.0);
    float fresnel = 0.03 + 0.97 * pow(1.0 - NdotV, 4.0);

    // Couleurs de l'eau : turquoise tropicale dans les hauts-fonds -> bleu saphir profond
    vec3 shallow_color = vec3(0.08, 0.48, 0.58);
    vec3 deep_color = vec3(0.03, 0.14, 0.32);
    vec3 sky_reflection = vec3(0.65, 0.80, 0.95);

    vec3 water_base = mix(deep_color, shallow_color, 0.45);
    vec3 diffuse = mix(water_base, sky_reflection, fresnel * 0.75);

    // Reflet spéculaire étincelant du soleil
    vec3 H = normalize(L + V);
    float NdotH = max(dot(N, H), 0.0);
    float sun_spec = pow(NdotH, 128.0) * 2.2;
    float sun_sheen = pow(NdotH, 16.0) * 0.25;
    vec3 specular = vec3(1.0, 0.96, 0.85) * (sun_spec + sun_sheen);

    // Écume sur la crête des vagues
    float wave_crest = smoothstep(0.30, 0.45, in_world_pos.y - 1.0);
    vec3 foam_color = vec3(0.92, 0.96, 1.0);

    vec3 final_color = diffuse + specular;
    final_color = mix(final_color, foam_color, wave_crest * 0.6);

    // Brouillard atmosphérique
    float dist = length(camera.camera_pos.xyz - in_world_pos);
    float fog_density = 0.0022;
    float fog_factor = 1.0 - exp(-pow(dist * fog_density, 1.4));
    vec3 fog_color = vec3(0.68, 0.78, 0.90);
    final_color = mix(final_color, fog_color, clamp(fog_factor, 0.0, 0.95));

    // ACES filmic tonemapping
    vec3 a = final_color * (2.51 * final_color + 0.03);
    vec3 b = final_color * (2.43 * final_color + 0.59) + 0.14;
    vec3 mapped = clamp(a / b, 0.0, 1.0);

    // Semi-transparence élégante
    out_color = vec4(mapped, 0.88);
}
