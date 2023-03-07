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

uniform sampler2D normal_map;
uniform sampler2D env_map;

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

void main() {
    float a = texture(normal_map, v_uv).a;
    if (a <= 0.5) {
        discard;
        //        out_color = vec4(vec3(0), 1);
        //        return;
    }

    vec3 normal = texture(normal_map, v_uv).rgb;
    vec4 ray_clip = vec4(v_uv * 2 - 1, -1, 1);
    vec4 ray_eye = view.inv_proj * ray_clip;
    ray_eye.zw = vec2(-1, 0);
    vec3 ray_world = (view.inv_view * ray_eye).xyz;
    ray_world = normalize(ray_world);

    vec3 reflected_ray = reflect(normal, ray_world);
    vec2 uv = spherical_to_polar(reflected_ray);
    //    vec3 color = texture(env_map, uv).rgb;
    vec3 color = textureLod(env_map, uv, 10).rgb;
    out_color = vec4(color, 1);
}
