#include "../common/uniforms/view.glsl"

in vec3 position;
in vec3 normal;
in vec2 uv;

uniform mat4 model;

out vec3 vs_position;
out vec2 vs_uv;
out vec3 vs_normal;

void main() {
    mat4 view_proj = view.mat_proj * view.mat_view;
    mat4 transform = view_proj * model;
    gl_Position = model * vec4(position, 1.0);
    vs_position = gl_Position.xyz/gl_Position.w;// <- world space
    vs_uv = uv;
    vec4 pnormal = model * vec4(normal, 0.0);
    gl_Position = view_proj * gl_Position;
    vs_normal = pnormal.xyz;
}
