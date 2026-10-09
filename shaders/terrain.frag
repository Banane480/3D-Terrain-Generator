#version 450

layout(location = 0) in vec3 in_world_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_biome_weights;

layout(set = 0, binding = 0) uniform CameraData {
    mat4 view_proj;
    vec4 camera_pos;
    vec4 sun_dir;
} camera;

layout(set = 0, binding = 1) uniform texture2DArray u_textures;
layout(set = 0, binding = 2) uniform sampler u_sampler;

layout(location = 0) out vec4 out_color;

// Projection triplanaire pour les roches et falaises (elimine tout etirement vertical)
vec3 sample_triplanar(float layer, vec3 world_pos, vec3 normal, float scale) {
    vec3 blending = abs(normal);
    blending = normalize(max(blending, 0.00001));
    float b_sum = blending.x + blending.y + blending.z;
    blending /= b_sum;

    vec3 xaxis = texture(sampler2DArray(u_textures, u_sampler), vec3(world_pos.zy * scale, layer)).rgb;
    vec3 yaxis = texture(sampler2DArray(u_textures, u_sampler), vec3(world_pos.xz * scale, layer)).rgb;
    vec3 zaxis = texture(sampler2DArray(u_textures, u_sampler), vec3(world_pos.xy * scale, layer)).rgb;

    return xaxis * blending.x + yaxis * blending.y + zaxis * blending.z;
}

void main() {
    vec3 N = normalize(in_normal);
    vec3 L = normalize(camera.sun_dir.xyz);
    vec3 V = normalize(camera.camera_pos.xyz - in_world_pos);

    // Echantillonnage des vraies textures PBR 1K
    // Layer 0: Sand, Layer 1: Grass, Layer 2: Rock, Layer 3: Snow
    vec2 uv_sand  = in_world_pos.xz * 0.08;
    vec2 uv_grass = in_world_pos.xz * 0.05;
    vec2 uv_snow  = in_world_pos.xz * 0.06;

    vec3 tex_sand  = texture(sampler2DArray(u_textures, u_sampler), vec3(uv_sand, 0.0)).rgb;
    vec3 tex_grass = texture(sampler2DArray(u_textures, u_sampler), vec3(uv_grass, 1.0)).rgb;
    vec3 tex_rock  = sample_triplanar(2.0, in_world_pos, N, 0.04);
    vec3 tex_snow  = texture(sampler2DArray(u_textures, u_sampler), vec3(uv_snow, 3.0)).rgb;

    // Calcul de la pente pour projeter automatiquement la roche sur les parois abruptes
    float slope = 1.0 - clamp(dot(N, vec3(0.0, 1.0, 0.0)), 0.0, 1.0);
    float rock_factor = smoothstep(0.28, 0.58, slope);

    // Melange continu selon les poids des biomes
    vec3 albedo = in_biome_weights.x * tex_sand
                + in_biome_weights.y * tex_grass
                + in_biome_weights.z * tex_rock
                + in_biome_weights.w * tex_snow;

    // Remplacement par la roche sur forte pente
    albedo = mix(albedo, tex_rock, rock_factor * (1.0 - in_biome_weights.w * 0.7));

    // Eclairage solaire directionnel + rebond atmospherique du ciel
    float NdotL = max(dot(N, L), 0.0);
    vec3 sun_color = vec3(1.0, 0.96, 0.90) * 1.45;
    vec3 sky_ambient = vec3(0.24, 0.35, 0.50) * 0.65;
    vec3 diffuse = sun_color * NdotL + sky_ambient;

    // Speculaire Blinn-Phong pour reflets d'eau / sable humide / neige
    vec3 H = normalize(L + V);
    float NdotH = max(dot(N, H), 0.0);
    float spec_power = mix(24.0, 80.0, in_biome_weights.w + in_biome_weights.x);
    float spec_intensity = (in_biome_weights.w * 0.35 + in_biome_weights.x * 0.2);
    vec3 specular = sun_color * pow(NdotH, spec_power) * spec_intensity * NdotL;

    vec3 final_color = albedo * diffuse + specular;

    // Brouillard atmospherique doux sur l'horizon
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
