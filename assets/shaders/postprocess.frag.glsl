#version 330

in vec2 v_uv;
out vec4 out_color;

uniform sampler2D frame;
uniform float luminance_average = 0.5;
uniform float bloom_size = 5e-2;
uniform float bloom_strength = 1e-2;

float desaturate(vec3 col) {
    return dot(col, vec3(0.2126, 0.7152, 0.0722));
}

vec3 reinhard(vec3 col) {
    return col / (1.0 + desaturate(col));
}

vec3 scale_levels(vec3 color) {
    return color / (9.6 * luminance_average);
}

vec3 aces(vec3 x)
{
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), 0, 1);
}

void main() {
    vec3 blur = vec3(0);
    for (int y = -3; y < 3; ++y)
        for(int x = -3; x < 3; ++x) {
            vec2 offset = vec2(x, y) * bloom_size;
            blur += texture(frame, v_uv + offset).rgb;
        }
    vec3 linear_out = texture(frame, v_uv).rgb + bloom_strength * blur;
    out_color = vec4(aces(scale_levels(linear_out)), 1);
}