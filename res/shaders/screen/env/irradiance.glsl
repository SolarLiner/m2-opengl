#include "../../common/math.glsl"

in vec2 v_uv;

uniform sampler2D env_map;
uniform float angular_delta = 5e-3;

out vec4 out_color;

vec2 normal_to_polar(vec3 sph) {
    const vec2 inv_atan = vec2(0.1591, 0.3183);
    vec2 uv = vec2(atan(sph.z, sph.x), asin(sph.y));
    uv *= inv_atan;
    uv += 0.5;
    return uv;
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

vec3 irradiance(vec3 normal) {
    vec3 up = vec3(0, 1, 0);
    vec3 right = normalize(cross(up, normal));
    up = normalize(cross(normal, right));
    float nr_samples = 0;
    vec3 ret = vec3(0);

    for (float phi = 0; phi < M_TAU; phi += angular_delta) {
        for (float theta = 0; theta < 0.5 * M_PI; theta += angular_delta) {
            vec3 tangent = vec3(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
            vec3 sample_vec = tangent.x * right + tangent.y * up + tangent.z * normal;
            ret += texture(env_map, normal_to_polar(sample_vec)).rgb * cos(theta) * sin(theta);
            nr_samples++;
        }
    }

    return ret * M_PI * (1 / nr_samples);
}

void main() {
    vec3 n = uv_to_normal(v_uv);
    out_color.a = 1;
    out_color.rgb = irradiance(n);
}
