#include "../common/uniforms/light.glsl"
#include "../common/uniforms/view.glsl"
#include "../common/pbr.glsl"

in vec2 v_uv;

uniform sampler2D frame_position;
uniform sampler2D frame_albedo;
uniform sampler2D frame_normal;
uniform sampler2D frame_rough_metal;
uniform sampler2D frame_emission;

out vec4 out_color;

void main() {
    vec4 nc = texture(frame_normal, v_uv);
    if (nc.a <= 0.5) discard;

    vec3 position = texture(frame_position, v_uv).rgb;
    vec3 albedo = texture(frame_albedo, v_uv).rgb;
    vec3 normal = texture(frame_normal, v_uv).rgb;
    vec3 rough_metal = texture(frame_rough_metal, v_uv).rgb;

    float roughness = rough_metal.r;
    float metallic = rough_metal.g;

    if (light.kind == LIGHT_KIND_AMBIENT) {
        out_color = vec4(light.color * albedo, 1.0);
        return;
    }

    LightSource src;
    if (light.kind == LIGHT_KIND_POINT) {
        float d = distance(light.pos_dir, position);// <- nominal
        vec3 dir = normalize(light.pos_dir - position);// <- nominal, world space
        src = create_light_source(dir, light.color, d);
    } else {
        src = create_light_source(light.pos_dir, light.color, 1);
    }

    LightingMaterial mat = create_material(metallic, roughness);
    Lighting l = create_lighting(src, mat, normalize(view.camera_pos - position), normal, albedo);

    vec3 reflectance = get_lighting(l) + texture(frame_emission, v_uv).rgb;
    out_color = vec4(reflectance, 1.0);
}