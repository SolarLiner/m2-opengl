#version 330

in vec2 v_uv;
out vec4 out_color;

uniform sampler2D frame;
uniform sampler2D bloom_tex;
uniform float luminance_average = 0.5;
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
    vec3 blur = texture(bloom_tex, v_uv).rgb;
    vec3 linear_out = texture(frame, v_uv).rgb + bloom_strength * blur;
    out_color = vec4(aces(scale_levels(linear_out)), 1);
}