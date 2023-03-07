#version 330 core

const float M_PI = 3.1415926535;

in vec2 v_uv;

uniform mat4 view_proj;
uniform sampler2D normal;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 zenith_color;

out vec3 out_color;

void main() {
    vec4 nc = texture(normal, v_uv);
    if (nc.a <= 0.5) discard;
    vec3 world_normal = nc.xyz;

    vec4 screen_dir = normalize(vec4(v_uv * vec2(1, -1) * 2 - 1, 1, 0));
    vec4 world_posw = view_proj * screen_dir;
    vec3 dir = normalize(world_posw.xyz);// Unnecessary ?

    vec3 env_dir = reflect(world_normal, dir);
    float lat_pc = env_dir.y / M_PI;
    if (lat_pc > 0) out_color = mix(horizon_color, zenith_color, lat_pc);
    else out_color = mix(horizon_color, ground_color, -lat_pc);
}