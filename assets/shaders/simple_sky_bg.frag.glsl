#version 330 core

in vec2 v_uv;

uniform mat4 view_proj_inv;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 zenith_color;

out vec3 out_color;

void main() {
    vec4 screen_dir = normalize(vec4(v_uv * 2 - 1, 1, 0));
    vec4 world_posw = view_proj_inv * screen_dir;
    vec3 dir = world_posw.xyz;// Unnecessary ?

    if (dir.y > 0) out_color = mix(horizon_color, zenith_color, dir.y);
    else out_color = mix(ground_color + horizon_color, vec3(0), -dir.y);
}
