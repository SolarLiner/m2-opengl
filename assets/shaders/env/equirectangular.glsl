#version 330 core

in vec2 v_uv;

layout(std140) uniform View {
    mat4 mat_view;
    mat4 mat_proj;
    mat4 inv_view;
    mat4 inv_proj;
    vec4 viewport;
    vec3 camera_pos;
} view;

uniform sampler2D frame_albedo;
uniform sampler2D frame_normal;
uniform sampler2D frame_rough_metal;
uniform sampler2D env_map;
uniform bool is_illuminate;

out vec4 out_color;

vec4 ndc2clip(vec4 ndc_pos) {
    vec4 clip_pos;
    clip_pos.w = view.mat_proj[3][2] / (ndc_pos.z - (view.mat_proj[2][2] / view.mat_proj[2][3]));
    clip_pos.xyz = ndc_pos.xyz * clip_pos.w;
    return clip_pos;
}

vec4 ndc2world(vec4 ndc) {
    return view.inv_view * ndc2clip(ndc);
}

vec2 spherical_to_polar(vec3 sph) {
    const vec2 inv_atan = vec2(0.1591, 0.3183);
    vec2 uv = vec2(atan(sph.z, sph.x), asin(sph.y));
    uv *= inv_atan;
    uv += 0.5;
    return uv;
}

vec3 get_ray_dir() {
    vec4 ray_clip = vec4(v_uv * 2 - 1, -1, 1);
    vec4 ray_eye = view.inv_proj * ray_clip;
    ray_eye.zw = vec2(-1, 0);
    vec3 ray_world = (view.inv_view * ray_eye).xyz;
    return normalize(ray_world);
}

vec3 background() {
    vec3 ray = get_ray_dir();
    vec2 uv = spherical_to_polar(ray);
    return texture(env_map, uv).rgb;
}

vec3 illuminate(vec3 normal) {
    vec3 albedo = texture(frame_albedo, v_uv).rgb;
    vec2 rough_metal = texture(frame_rough_metal, v_uv).rg;

    vec3 reflected_ray = reflect(get_ray_dir(), normal);
    vec2 uv = spherical_to_polar(reflected_ray);
    return albedo * textureLod(env_map, uv, sqrt(rough_metal.r) * 15).rgb;
}

void main() {
    vec4 nc = texture(frame_normal, v_uv);
    vec3 color = nc.a <= 0.5 ? background() : illuminate(nc.xyz);
    out_color = vec4(color, 1);
}

