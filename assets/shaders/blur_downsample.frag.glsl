#version 330 core

in vec2 v_uv;

uniform sampler2D in_texture;
uniform vec2 screen_size;

layout (location = 0) out vec3 downsample;

void main() {
    vec2 texel_size = 1 / screen_size;
    float x = texel_size.x;
    float y = texel_size.y;
    vec3 a = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y + 2*y)).rgb;
    vec3 b = texture(in_texture, vec2(v_uv.x,       v_uv.y + 2*y)).rgb;
    vec3 c = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y + 2*y)).rgb;

    vec3 d = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y)).rgb;
    vec3 e = texture(in_texture, vec2(v_uv.x,       v_uv.y)).rgb;
    vec3 f = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y)).rgb;

    vec3 g = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y - 2*y)).rgb;
    vec3 h = texture(in_texture, vec2(v_uv.x,       v_uv.y - 2*y)).rgb;
    vec3 i = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y - 2*y)).rgb;

    vec3 j = texture(in_texture, vec2(v_uv.x - x, v_uv.y + y)).rgb;
    vec3 k = texture(in_texture, vec2(v_uv.x + x, v_uv.y + y)).rgb;
    vec3 l = texture(in_texture, vec2(v_uv.x - x, v_uv.y - y)).rgb;
    vec3 m = texture(in_texture, vec2(v_uv.x + x, v_uv.y - y)).rgb;

    downsample = e*0.125;
    downsample += (a+c+g+i)*0.03125;
    downsample += (b+d+f+h)*0.0625;
    downsample += (j+k+l+m)*0.125;
}
