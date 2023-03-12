in vec2 v_uv;

uniform sampler2D in_texture;
uniform vec2 screen_size;
uniform bool first_mip = false;

layout (location = 0) out vec3 downsample;

vec3 powvf(vec3 v, float p)
{
    return vec3(pow(v.x, p), pow(v.y, p), pow(v.z, p));
}

const float inv_gamma = 1.0 / 2.2;
vec3 srgb(vec3 v) { return powvf(v, inv_gamma); }

float rgb2luminance(vec3 col)
{
    return dot(col, vec3(0.2126f, 0.7152f, 0.0722f));
}

float avg_karis(vec3 col)
{
    // Formula is 1 / (1 + luma)
    float luma = rgb2luminance(srgb(col)) * 0.25f;
    return 1.0f / (1.0f + luma);
}

void main() {
    vec2 texel_size = 1 / screen_size;
    float x = texel_size.x;
    float y = texel_size.y;
    vec3 a = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y + 2*y)).rgb;
    vec3 b = texture(in_texture, vec2(v_uv.x, v_uv.y + 2*y)).rgb;
    vec3 c = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y + 2*y)).rgb;

    vec3 d = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y)).rgb;
    vec3 e = texture(in_texture, vec2(v_uv.x,       v_uv.y)).rgb;
    vec3 f = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y)).rgb;

    vec3 g = texture(in_texture, vec2(v_uv.x - 2*x, v_uv.y - 2*y)).rgb;
    vec3 h = texture(in_texture, vec2(v_uv.x, v_uv.y - 2*y)).rgb;
    vec3 i = texture(in_texture, vec2(v_uv.x + 2*x, v_uv.y - 2*y)).rgb;

    vec3 j = texture(in_texture, vec2(v_uv.x - x, v_uv.y + y)).rgb;
    vec3 k = texture(in_texture, vec2(v_uv.x + x, v_uv.y + y)).rgb;
    vec3 l = texture(in_texture, vec2(v_uv.x - x, v_uv.y - y)).rgb;
    vec3 m = texture(in_texture, vec2(v_uv.x + x, v_uv.y - y)).rgb;

    if (first_mip) {
        vec3 groups[5];
        groups[0] = (a+b+d+e) * (0.125f/4.0f);
        groups[1] = (b+c+e+f) * (0.125f/4.0f);
        groups[2] = (d+e+g+h) * (0.125f/4.0f);
        groups[3] = (e+f+h+i) * (0.125f/4.0f);
        groups[4] = (j+k+l+m) * (0.5f/4.0f);
        groups[0] *= avg_karis(groups[0]);
        groups[1] *= avg_karis(groups[1]);
        groups[2] *= avg_karis(groups[2]);
        groups[3] *= avg_karis(groups[3]);
        groups[4] *= avg_karis(groups[4]);
        downsample = groups[0]+groups[1]+groups[2]+groups[3]+groups[4];
    } else {
        downsample = e*0.125;
        downsample += (a+c+g+i)*0.03125;
        downsample += (b+d+f+h)*0.0625;
        downsample += (j+k+l+m)*0.125;
    }
}
