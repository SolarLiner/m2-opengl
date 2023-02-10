#version 330

uniform vec3 stroke;
out vec4 color;

void main() {
    color = vec4(stroke, 1.0);
}
