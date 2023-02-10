#version 330

in vec3 position;
in vec3 normal;
in vec2 uv;

uniform mat4 model;
uniform mat4 view_proj;

out vec3 v_position;
out vec2 v_uv;
out vec3 v_normal;

void main() {
    mat4 transform = view_proj * model;
    gl_Position = model * vec4(position, 1.0);
    v_position = gl_Position.xyz/gl_Position.w;// <- world space
    v_uv = uv;
    vec4 pnormal = model * vec4(normal, 0.0);
    gl_Position = view_proj * gl_Position;
    v_normal = pnormal.xyz;
}
