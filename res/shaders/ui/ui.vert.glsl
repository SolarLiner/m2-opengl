#version 330
uniform vec2 u_screen_size;

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_tc;
layout(location = 2) in vec4 a_srgba;

out vec4 v_rgba_in_gamma;
out vec2 v_tc;

void main() {
    gl_Position = vec4(
                      2.0 * a_pos.x / u_screen_size.x - 1.0,
                      1.0 - 2.0 * a_pos.y / u_screen_size.y,
                      0.0,
                      1.0);
    v_rgba_in_gamma = a_srgba;
    v_tc = a_tc;
}