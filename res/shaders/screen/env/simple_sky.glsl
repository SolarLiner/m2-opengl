#include "../../common/math.glsl"
#include "../../common/uniforms/view.glsl"

in vec2 v_uv;

uniform sampler2D albedo;
uniform sampler2D normal_map;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 zenith_color;
uniform bool is_illumination;

out vec3 out_color;

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

vec3 gradient(float t) {
    if (t > 0) return mix(horizon_color, zenith_color, t);
    else return mix(ground_color + horizon_color * 0.5, vec3(0), -t);
}

void main() {
    vec4 nc = texture(normal_map, v_uv);
    if (nc.a <= 0.5) {
        vec3 ray_world = get_ray_dir();
        float lat_pc = ray_world.y / M_PI;
        out_color = gradient(lat_pc);
    } else {
        vec3 albedo = texture(albedo, v_uv).rgb;
        vec3 normal = nc.xyz;
        vec3 refl_dir = reflect(get_ray_dir(), normal);
        float lat_pc = refl_dir.y / M_PI;
        out_color = albedo * gradient(lat_pc);
    }
}
