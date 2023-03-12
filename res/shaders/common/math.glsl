const float M_PI = 3.141592653589793238462;
const float M_TAU = 2 * M_PI;

vec4 ndc2clip(mat4 proj, vec4 ndc_pos) {
    vec4 clip_pos;
    clip_pos.w = proj[3][2] / (ndc_pos.z - (proj[2][2] / proj[2][3]));
    clip_pos.xyz = ndc_pos.xyz * clip_pos.w;
    return clip_pos;
}

vec4 ndc2world(mat4 inv_view, mat4 proj, vec4 ndc) {
    return inv_view * ndc2clip(proj, ndc);
}

vec3 uv_to_normal(vec2 uv) {
    vec2 polar = uv - 0.5;
    polar *= vec2(M_TAU, M_PI);
    vec3 n;
    n.x = sin(polar.x) * cos(polar.y);
    n.y = sin(polar.y) * sin(polar.y);
    n.z = cos(polar.x);
    return normalize(n);
}

vec2 normal_to_polar(vec3 sph) {
    const vec2 inv_atan = vec2(0.1591, 0.3183);
    vec2 uv = vec2(atan(sph.z, sph.x), asin(sph.y));
    uv *= inv_atan;
    uv += 0.5;
    return uv;
}
