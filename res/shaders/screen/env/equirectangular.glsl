#include "../../common/math.glsl"
#include "../../common/pbr.glsl"
#include "../../common/uniforms/view.glsl"

in vec2 v_uv;

uniform sampler2D frame_albedo;
uniform sampler2D frame_normal;
uniform sampler2D frame_rough_metal;
uniform sampler2D env_map;
uniform sampler2D irradiance_map;
uniform sampler2D specular_map;

out vec4 out_color;

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
    vec3 diffuse_color = texture(irradiance_map, normal_to_polar(normal)).rgb;
    vec3 specular_color = textureLod(specular_map, normal_to_polar(light), (rough_metal.r)*10).rgb;

    return albedo * ((1 - rough_metal.g)*diffuse_color + specular_color);
}

void main() {
    vec4 nc = texture(frame_normal, v_uv);
    vec3 color = nc.a <= 0.5 ? background() : illuminate(nc.xyz);
    out_color = vec4(color, 1);
}
