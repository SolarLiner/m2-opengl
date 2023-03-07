#version 330 core

const float M_PI = 3.1415926535;

in vec2 v_uv;

layout(std140) uniform View {
    mat4 mat_view;
    mat4 mat_proj;
    mat4 inv_view;
    mat4 inv_proj;
    vec4 viewport;
    vec3 camera_pos;
} view;

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

void main() {
    float a = texture(normal_map, v_uv).a;
    if (a <= 0.5 && is_illumination) {
        discard;
        //        out_color = vec4(vec3(0), 1);
        //        return;
    }

    vec4 ray_clip = vec4(v_uv * 2 - 1, -1, 1);
    vec4 ray_eye = view.inv_proj * ray_clip;
    ray_eye.zw = vec2(-1, 0);
    vec3 ray_world = (view.inv_view * ray_eye).xyz;
    ray_world = normalize(ray_world);

    float lat_pc = ray_world.y / M_PI;
    if (is_illumination) {
        if (lat_pc > 0) out_color = mix(horizon_color, zenith_color, lat_pc);
        else out_color = mix(horizon_color, ground_color, -lat_pc);
    } else {
        vec3 normal = texture(normal_map, v_uv).rgb;
        ray_world = reflect(ray_world, normal);
        if (lat_pc > 0) out_color = mix(horizon_color, zenith_color, lat_pc);
        else out_color = mix(ground_color + horizon_color, vec3(0), -lat_pc);
    }
}
