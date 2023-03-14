#include "../common/uniforms/view.glsl"

in vec2 v_uv;

uniform sampler2D frame_position;
uniform sampler2D frame_normal;
uniform sampler2D noise;
uniform int kernel_size = 64;
uniform float radius = 0.5;
uniform vec3 samples[64];

out float out_ssao;

void main() {
    vec4 nc = texture(frame_normal, v_uv);
    if(nc.a <= 0.5) {
        out_ssao = 1;
        return;
    }
    vec2 noise_scale = view.viewport.zw / 4;
    vec3 pos = texture(frame_position, v_uv).xyz;
    vec3 normal = texture(frame_normal, v_uv).xyz;
    vec3 random_vec = texture(noise, v_uv * noise_scale).xyz;

    vec3 tangent = normalize(random_vec - normal * dot(random_vec, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 tbn = mat3(tangent, bitangent, normal);

    float occlusion = 0;
    const float bias = 2e-3;
    for(int i = 0; i < kernel_size; ++i) {
        vec3 sample_pos = tbn * samples[i];
        sample_pos = pos + sample_pos * radius;

        vec4 offset = vec4(sample_pos, 1);
        offset = view.mat_proj * view.mat_view * offset;
        offset.xyz /= offset.w;
        offset.xyz = offset.xyz * 0.5 + 0.5;
        
        float sample_depth = texture(frame_position, offset.xy).z;
        float range_check = smoothstep(0, 1, radius / abs(pos.z - sample_depth));
        occlusion += (sample_depth >= sample_pos.z + bias ? 1 : 0) * range_check;
    }

    out_ssao = 1 - (occlusion / kernel_size);
}
