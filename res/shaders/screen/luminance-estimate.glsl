#include "../common/color.glsl"

uniform sampler2D in_texture;

in vec2 v_uv;
out float out_color;

/* Ideas taken adapted from https://bruop.github.io/exposure/ */
void main() {
    out_color = desaturate(texture(in_texture, v_uv).rgb);
}
