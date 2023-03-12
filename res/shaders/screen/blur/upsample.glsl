in vec2 v_uv;

uniform sampler2D in_texture;
uniform float filter_radius;

layout (location = 0) out vec3 upsample;

void main() {
    float x = filter_radius;
    float y = filter_radius;
    vec3 a = texture(in_texture, vec2(v_uv.x - x, v_uv.y + y)).rgb;
    vec3 b = texture(in_texture, vec2(v_uv.x,     v_uv.y + y)).rgb;
    vec3 c = texture(in_texture, vec2(v_uv.x + x, v_uv.y + y)).rgb;

    vec3 d = texture(in_texture, vec2(v_uv.x - x, v_uv.y)).rgb;
    vec3 e = texture(in_texture, vec2(v_uv.x,     v_uv.y)).rgb;
    vec3 f = texture(in_texture, vec2(v_uv.x + x, v_uv.y)).rgb;

    vec3 g = texture(in_texture, vec2(v_uv.x - x, v_uv.y - y)).rgb;
    vec3 h = texture(in_texture, vec2(v_uv.x,     v_uv.y - y)).rgb;
    vec3 i = texture(in_texture, vec2(v_uv.x + x, v_uv.y - y)).rgb;

    // Apply weighted distribution, by using a 3x3 tent filter:
    //  1   | 1 2 1 |
    // -- * | 2 4 2 |
    // 16   | 1 2 1 |
    upsample = e*4.0;
    upsample += (b+d+f+h)*2.0;
    upsample += (a+c+g+i);
    upsample *= 1.0 / 16.0;
}
