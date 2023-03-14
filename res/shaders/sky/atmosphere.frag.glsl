#include "../common/uniforms/view.glsl"

layout(std140) uniform Atmosphere {
    vec3 center;
    float ground_radius;
    float atmosphere_radius;
    vec3 sun_dir;
    vec3 sun_color;
} atmosphere;

in vec3 vs_position;
in vec2 vs_uv;
in vec3 vs_normal;

layout(location=0) out vec3 frame_position;
layout(location=1) out vec3 frame_albedo;
layout(location=2) out vec4 frame_normal;
layout(location=3) out vec2 frame_rough_metal;
layout(location=4) out vec3 frame_emission;


void main() {
    float k = max(0, dot(vs_normal, atmosphere.sun_dir));
    frame_position = vs_position;
    frame_normal = vec4(-vs_normal, 1);
    frame_albedo = vec3(0);
    frame_rough_metal = vec2(1, 0);
    frame_emission = vec3(1, 0, 1) * k;
}