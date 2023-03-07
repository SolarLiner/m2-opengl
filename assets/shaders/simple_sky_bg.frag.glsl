#version 330 core

const float M_PI = 3.1415926535;

in vec2 v_uv;

uniform mat4 view_proj;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 zenith_color;

out vec3 out_color;

vec3 camera_dir(mat4 view_proj) {
    vec4 screen_dir = normalize(vec4(v_uv * 2 - 1, 1, 0));
    vec4 world_posw = view_proj * screen_dir;
    return normalize(world_posw.xyz);// Unnecessary ?
}

void main() {
    vec3 dir = camera_dir(view_proj);

    float lat_pc = dir.y / M_PI;
    if (lat_pc > 0) out_color = mix(horizon_color, zenith_color, lat_pc);
    else out_color = mix(ground_color + horizon_color, vec3(0), -lat_pc);
}
