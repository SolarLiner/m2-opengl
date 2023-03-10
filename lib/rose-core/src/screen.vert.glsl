#version 330

in vec2 pos;
in vec2 uv;
out vec2 v_uv;

void main() {
    vec2 pos;
    switch(gl_VertexID) {
        case 0:
            pos = vec2(-1);
            v_uv = vec2(0);
            break;
        case 1:
            pos = vec2(-1, 1);
            v_uv = vec2(0, 1);
            break;
        case 2:
            pos = vec2(1);
            v_uv = vec2(1);
            break;
        case 3:
            pos = vec2(1, -1);
            v_uv = vec2(1, 0);
            break;
        default:
            pos = vec2(0);
            v_uv = vec2(0);
            return;
    }
    gl_Position = vec4(pos, 0, 1);
}
