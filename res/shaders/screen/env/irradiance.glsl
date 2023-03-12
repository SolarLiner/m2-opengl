#include "../../common/math.glsl"

in vec2 v_uv;

uniform sampler2D env_map;
uniform float angular_delta = 5e-3;

out vec4 out_color;

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
