#include "../common/uniforms/view.glsl"
#include "../common/uniforms/bone.glsl"

const int MAX_BONES = 32;

in vec3 position;
in vec3 normal;
in vec2 uv;
in ivec4 bone_ix;
in vec4 bone_w;

layout(std140) uniform Bones {
    Bone bones[MAX_BONES];
};
uniform mat4 model;

out vec3 vs_position;
out vec2 vs_uv;
out vec3 vs_normal;

vec4 bone_transform_pos() {
    vec4 p = vec4(position, 1);
    return bones[0].transform * p * bone_w[0]
    + bones[1].transform * p * bone_w[1]
    + bones[2].transform * p * bone_w[2]
    + bones[3].transform * p * bone_w[3];
}

vec4 bone_transform_normal() {
    vec4 n = vec4(normal, 0);
    return bones[0].transform * n * bone_w[0]
    + bones[1].transform * n * bone_w[1]
    + bones[2].transform * n * bone_w[2]
    + bones[3].transform * n * bone_w[3];
}

void main() {
    mat4 view_proj = view.mat_proj * view.mat_view;
    mat4 transform = view_proj * model;
    gl_Position = model * bone_transform_pos();
    vs_position = gl_Position.xyz/gl_Position.w;// <- world space
    vs_uv = uv;
    vec4 pnormal = model * normalize(bone_transform_normal());
    gl_Position = view_proj * gl_Position;
    vs_normal = pnormal.xyz;
}
