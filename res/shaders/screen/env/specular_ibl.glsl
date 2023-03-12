#include "../../common/math.glsl"
#include "../../common/random.glsl"
#include "../../common/pbr.glsl"

in vec2 v_uv;

uniform sampler2D env_map;
uniform float roughness;

out vec4 out_color;

void main() {
    vec3 n = uv_to_normal(v_uv);
    vec3 r = n;
    vec3 v = r;

    const uint SAMPLE_COUNT = 1024u;
    float total_weight = 0;
    vec3 prefiltered = vec3(0);
    for (uint i = 0u; i < SAMPLE_COUNT; ++i) {
        vec2 xi = rand_hammersley(i, SAMPLE_COUNT);
        vec3 H = ggx_importance_sample(xi, n, roughness);
        vec3 L = normalize(2 * dot(v, H) * H - v);

        float NdotL = max(dot(n, L), 0);
        prefiltered += texture(env_map, normal_to_polar(L)).rgb * NdotL;
        total_weight += NdotL;
    }
    prefiltered /= total_weight;
    out_color = vec4(prefiltered, 1);
}