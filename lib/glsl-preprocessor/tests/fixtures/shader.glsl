#version 330 core

#include "common.glsl"

out vec4 out_color;

void main() {
    out_color = vec4(gen_color(), 1);
}
