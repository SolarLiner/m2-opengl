#version 330

uniform sampler2D in_texture;

in vec2 v_uv;
out float out_color;

/* Ideas taken adapted from https://bruop.github.io/exposure/ */
float desaturate(vec3 color) {
    const vec3 to_luma = vec3(0.2125, 0.7154, 0.0721);
    return dot(color, to_luma);
}

void main() {
    out_color = desaturate(texture(in_texture, v_uv).rgb);
}
