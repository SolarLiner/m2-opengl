uniform sampler2D in_texture;

in vec2 v_uv;

out vec3 out_color;

void main() {
    out_color = texture(in_texture, v_uv).rgb;
}