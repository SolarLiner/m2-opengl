#version 330

in vec3 vs_position;
in vec2 vs_uv;
in vec3 vs_normal;

layout(location=0) out vec3 frame_position;
layout(location=1) out vec3 frame_albedo;
layout(location=2) out vec3 frame_normal;
layout(location=3) out vec2 frame_rough_metal;

#ifdef HAS_COLOR_TEXTURE
uniform sampler2D color;
#else
uniform vec3 color;
#endif
#ifdef HAS_NORMAL_TEXTURE
uniform sampler2D normal_map;
uniform float normal_amount = 1.0;
#endif
#ifdef HAS_ROUGH_METAL_TEXTURE
uniform sampler2D rough_metal;
#else
uniform vec2 rough_metal;
#endif

mat3 cotangent_frame(vec3 pos, vec3 normal, vec2 uv) {
    vec3 dp1 = dFdx(pos);
    vec3 dp2 = dFdy(pos);
    vec2 duv1 = dFdx(uv);
    vec2 duv2 = dFdy(uv);
    vec3 dp2perp = cross(dp2, normal);
    vec3 dp1perp = cross(normal, dp1);
    vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;
    float invmax = inversesqrt(max(dot(T, T), dot(B, B)));
    return mat3(T * invmax, B * invmax, normal);
}

void main() {
    frame_position = vs_position;

    #ifdef HAS_COLOR_TEXTURE
    frame_albedo = texture(color, vs_uv).rgb;
    #else
    frame_albedo = color;
    #endif

    #ifdef HAS_NORMAL_TEXTURE
    mat3 tbn = cotangent_frame(vs_position, vs_normal, vs_uv);
    vec3 tangent_map = -(texture(normal_map, vs_uv).xyz * 2. - 1.) * vec3(normal_amount, normal_amount, 1.);
    frame_normal = normalize(tbn * tangent_map);// <- world space
    #else
    frame_normal = vs_normal;
    #endif

    #if HAS_ROUGH_METAL_TEXTURE
    frame_rough_metal = texture(rough_metal, vs_uv).rg;
    #else
    frame_rough_metal = rough_metal.rg;
    #endif
}