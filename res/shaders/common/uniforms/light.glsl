const uint LIGHT_KIND_POINT = 0u;
const uint LIGHT_KIND_DIRECTIONAL = 1u;
const uint LIGHT_KIND_AMBIENT = 2u;

layout(std140) uniform Light {
    uint kind;
    vec3 pos_dir;// <- world space
    vec3 color;
} light;
