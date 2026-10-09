#version 450

layout(location = 0) in vec3 in_world_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_color;
layout(location = 3) in vec4 in_uv_layer;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(set = 0, binding = 1) uniform texture2DArray u_textures;
layout(set = 0, binding = 2) uniform sampler u_sampler;

layout(location = 0) out vec4 out_color;

void main() {
    vec4 albedo = texture(sampler2DArray(u_textures, u_sampler), vec3(in_uv_layer.xy, in_uv_layer.z));
    if (albedo.a < in_uv_layer.w) discard;

    vec3 N = normalize(in_normal);
    vec3 L = normalize(camera.sun_dir.xyz);
    vec3 V = normalize(camera.camera_pos.xyz - in_world_pos);

    // Eclairage double face pour le feuillage (effet de translucidite / subsurface scattering)
    float NdotL = max(dot(N, L), 0.0);
    float back_scatter = max(dot(-N, L), 0.0) * 0.45 * in_color.a;

    vec3 sun_color = vec3(1.0, 0.95, 0.88) * 1.45;
    vec3 sky_ambient = vec3(0.25, 0.38, 0.52) * 0.70;

    // Gradient d'occlusion ambiante selon la hauteur dans le feuillage
    float ao = mix(0.60, 1.0, in_color.a);
    vec3 diffuse = (sun_color * (NdotL + back_scatter) + sky_ambient) * ao;

    // Reflet speculaire leger
    vec3 H = normalize(L + V);
    float NdotH = max(dot(N, H), 0.0);
    float spec = pow(NdotH, 32.0) * 0.20 * in_color.a;

    vec3 final_color = in_color.rgb * albedo.rgb * diffuse + sun_color * spec;

    // Brouillard atmospherique
    float dist = length(camera.camera_pos.xyz - in_world_pos);
    float fog_density = 0.0022;
    float fog_factor = 1.0 - exp(-pow(dist * fog_density, 1.4));
    vec3 fog_color = vec3(0.68, 0.78, 0.90);
    final_color = mix(final_color, fog_color, clamp(fog_factor, 0.0, 0.95));

    // ACES filmic tonemapping
    vec3 a = final_color * (2.51 * final_color + 0.03);
    vec3 b = final_color * (2.43 * final_color + 0.59) + 0.14;
    vec3 mapped = clamp(a / b, 0.0, 1.0);

    out_color = vec4(mapped, 1.0);
}
