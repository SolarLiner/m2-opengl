#include "../../common/pbr.glsl"
#include "../../common/uniforms/view.glsl"

in vec2 v_uv;

uniform sampler2D frame_albedo;
uniform sampler2D frame_normal;
uniform sampler2D frame_rough_metal;
uniform sampler2D env_map;
uniform sampler2D irradiance_map;

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

vec2 normal_to_polar(vec3 sph) {
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
    vec2 uv = normal_to_polar(ray);
    return texture(env_map, uv).rgb;
}

vec3 illuminate(vec3 normal) {
    vec3 albedo = texture(frame_albedo, v_uv).rgb;
    vec2 rough_metal = texture(frame_rough_metal, v_uv).rg;

    vec3 view = get_ray_dir();
    vec3 light = reflect(view, normal);
    vec2 uv = normal_to_polar(light);
    vec3 diffuse_color = texture(irradiance_map, uv).rgb;

    LightSource light_source = create_light_source(light, diffuse_color, 1);
    light_source.specular = texture(env_map, uv).rgb;

    LightingMaterial light_mat = create_material(rough_metal.r, rough_metal.g);
    Lighting l = create_lighting(light_source, light_mat, view, normal, albedo);
    return get_lighting(l);
}

void main() {
    vec4 nc = texture(frame_normal, v_uv);
    vec3 color = nc.a <= 0.5 ? background() : illuminate(nc.xyz);
    out_color = vec4(color, 1);
}
