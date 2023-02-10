#version 330

in vec2 v_uv;
uniform sampler2D in_position;
uniform sampler2D in_albedo;
uniform sampler2D in_normal;
uniform sampler2D in_metal_rough;
uniform float exposure;

out vec4 out_color;

layout(std140) uniform Light {
    uint kind;
    vec3 pos_dir;// <- world space
    vec3 color;
} light;

const uint LIGHT_KIND_POINT = 0u;
const uint LIGHT_KIND_DIRECTIONAL = 1u;
const uint LIGHT_KIND_AMBIENT = 2u;

const float M_PI = 3.141592653589793238462;
const vec3 F0 = vec3(0.04);

vec3 fresnel(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

float ggx_dist(vec3 N, vec3 H, float roughness) {
    float a      = roughness*roughness;
    float a2     = a*a;
    float NdotH  = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;

    float num   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = M_PI * denom * denom;

    return num / denom;
}

float ggx_geom(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num   = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / denom;
}

float smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2  = ggx_geom(NdotV, roughness);
    float ggx1  = ggx_geom(NdotL, roughness);

    return ggx1 * ggx2;
}

vec3 diffuse_brdf(float distance) {
    float attenuation = 1.0 / (distance * distance);
    return light.color * attenuation;
}

vec3 specular_brdf(vec3 V, vec3 H, vec3 L, vec3 N, float distance, float roughness) {
    float NDF = ggx_dist(N, H, roughness);
    float G = smith(N, V, L, roughness);
    float NdotV = max(0.0, dot(N, V));
    float NdotL = max(0.0, dot(N, L));
    vec3 F = fresnel(NdotV, F0);
    vec3 num = NDF * G * F;
    float denominator = 4.0 * NdotV * NdotL * distance + 1e-4;
    return num/denominator;
}

vec3 get_lighting(vec3 V, vec3 L, vec3 N, float distance, float roughness, float metallic) {
    vec3 H = normalize(V+L);
    vec3 kS = fresnel(max(0.0, dot(H, V)), F0);
    vec3 kD = (vec3(1.0) - kS) * (1.0 - metallic);
    vec3 specular = specular_brdf(V, H, L, N, distance, roughness);
    vec3 radiance = diffuse_brdf(distance);
    float NdotL = max(0.0, dot(N, L));
    return radiance * (1 - metallic) * NdotL + specular;
}

mat3 cotangent_frame(vec3 normal, vec3 pos, vec2 uv) {
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

float desaturate(vec3 col) {
    return dot(col, vec3(0.2126, 0.7152, 0.0722));
}

vec3 reinhard(vec3 col) {
    return col / (1.0 + desaturate(col));
}

void main() {
    vec3 pos = texture(in_position, v_uv).rgb;
    vec3 albedo = texture(in_albedo, v_uv).rgb;
    vec3 normal = texture(in_normal, v_uv).rgb;
    vec2 mr = texture(in_metal_rough, v_uv).rg;
    float roughness = mr.r;
    float metallic = mr.g;

    vec3 c = texture(in_color, v_uv).rgb;
    out_color = vec4(reinhard(exposure * c), 1.0);
}
