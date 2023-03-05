#version 330 core

in vec2 v_uv;

uniform mat4 view_proj_inv;
uniform sampler2D normal;
uniform vec3 horizon_color;
uniform vec3 ground_color;
uniform vec3 zenith_color;

out vec3 out_color;

void main() {
    vec3 world_normal = texture(normal, v_uv).xyz;

    vec4 screen_dir = normalize(vec4(v_uv * vec2(1, -1) * 2 - 1, 1, 0));
    vec4 world_posw = view_proj_inv * screen_dir;
    vec3 dir = normalize(world_posw.xyz);// Unnecessary ?

    vec3 env_dir = reflect(world_normal, dir);
    if (env_dir.y > 0) out_color = mix(horizon_color, zenith_color, env_dir.y);
    else out_color = mix(horizon_color, ground_color, -env_dir.y);
}