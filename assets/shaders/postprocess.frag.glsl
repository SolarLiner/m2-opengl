#version 330

in vec2 v_uv;
out vec4 out_color;

uniform sampler2D frame;
uniform float exposure;

float desaturate(vec3 col) {
    return dot(col, vec3(0.2126, 0.7152, 0.0722));
}

vec3 reinhard(vec3 col) {
    return col / (1.0 + desaturate(col));
}

void main() {
    out_color = vec4(reinhard(exposure * texture(frame, v_uv).rgb), 1);
}