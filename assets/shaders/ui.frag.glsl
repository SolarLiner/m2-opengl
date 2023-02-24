#version 330
uniform sampler2D u_sampler;

in vec4 v_rgba_in_gamma;
in vec2 v_tc;
out vec4 f_color;

// 0-1 sRGB gamma  from  0-1 linear
float gamma_from_linear(float col) {
//    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
    bool cutoff = col < 0.0031308;
    float lower = col * 12.92;
    float higher = 1.055 * pow(col, 1.0 / 2.4) - 0.055;
    return mix(higher, lower, cutoff);
}
//vec3 srgb_gamma_from_linear(vec3 rgb) {
//    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
//    vec3 lower = rgb * vec3(12.92);
//    vec3 higher = vec3(1.055) * pow(rgb, vec3(1.0 / 2.4)) - vec3(0.055);
//    return mix(higher, lower, vec3(cutoff));
//}

void main() {
//    vec4 texture_in_gamma = srgba_gamma_from_linear(texture(u_sampler, v_tc).r);
    float texture_in_gamma = gamma_from_linear(texture(u_sampler, v_tc).r);

    // We multiply the colors in gamma space, because that's the only way to get text to look right.
    f_color = v_rgba_in_gamma * vec4(vec3(texture_in_gamma), 1);
}