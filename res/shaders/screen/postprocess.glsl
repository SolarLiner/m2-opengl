#version 330

in vec2 v_uv;
out vec4 out_color;

uniform sampler2D frame;
uniform sampler2D bloom_tex;
uniform float luminance_average = 0.5;
uniform float bloom_strength = 1e-2;
uniform float lens_flare_strength = 4e-3;
uniform float lens_flare_threshold = 20;
uniform float distortion_amt = 2;
uniform float ghost_spacing = 0.8;
uniform int ghost_count = 5;

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

// taken from https://thebookofshaders.com/10/
float random (vec2 st) {
    return fract(sin(dot(st.xy,
    vec2(12.9898, 78.233)))*
    43758.5453123);
}

// Taken from https://john-chapman.github.io/2017/11/05/pseudo-lens-flare.html
vec3 threshold(vec3 rgb, float t) {
    return max(rgb - t, vec3(0));
}

vec2 dist_offset(vec2 uv)
{
    vec2 p = 2 * uv - 1;
    float theta  = atan(p.y, p.x);
    float radius = length(p);
    radius = pow(radius, distortion_amt);
    p.x = radius * cos(theta);
    p.y = radius * sin(theta);
    return 0.5 * (p + 1.0);
}

vec3 lens_flare() {
    vec2 uv = dist_offset(1 - v_uv);
    vec3 ghosts = vec3(0);
    vec2 ghost_vec = (0.5 - uv) * ghost_spacing;
    for (int i = 0; i < ghost_count;  ++i){
        vec2 suv = (uv + ghost_vec * vec2(i));
        float d = distance(suv, vec2(0.5));
        float weight = 1 - smoothstep(0, 0.5, d);
        vec3 s = threshold(texture(bloom_tex, suv).rgb, lens_flare_threshold);
        vec3 color = vec3(random(vec2(i, 0)), random(vec2(i, 1)), random(vec2(i, 3)));
        color = mix(vec3(1), color, 0.5);
        ghosts += s * weight * color;
    }
    //    vec2 halo_v = vec2(0.5) - uv;
    //    halo_v = normalize(halo_v);
    //    vec2 wuv = (uv - vec2(0.5, 0.0)) / vec2(1.0, 1.0) + vec2(0.5, 0.0);
    //    float d = distance(wuv, vec2(0.5));
    //    float halo_w = smoothstep(halo_radius, halo_thickness, d); // cubic window function
    //    halo_v *= halo_radius;
    //    vec3 halo = texture(bloom_tex, halo_v).rgb * halo_w;
    return ghosts;
}

void main() {
    vec3 blur = texture(bloom_tex, v_uv).rgb;
    vec3 flare = lens_flare();
    vec3 linear_out = texture(frame, v_uv).rgb + bloom_strength * blur + flare * lens_flare_strength;
    out_color = vec4(aces(scale_levels(linear_out)), 1);
}