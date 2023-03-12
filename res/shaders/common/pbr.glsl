#include "math.glsl"

struct LightSource {
    vec3 dir, specular, diffuse;
    float distance, dist_sqr;
};

struct LightingMaterial {
    float metallic, roughness, rough_sqr;
};

struct Lighting {
    LightSource source;
    LightingMaterial mat;
    vec3 V, H, N, albedo;
    float NdotV, NdotL, NdotH, HdotV;
};

const vec3 F0 = vec3(0.04);

LightSource create_light_source(vec3 dir, vec3 color, float distance) {
    LightSource src;
    src.dir = dir;
    src.diffuse = src.specular = color;
    src.distance = distance;
    src.dist_sqr = distance * distance;
    return src;
}

LightingMaterial create_material(float metallic, float roughness) {
    roughness = clamp(roughness, 0, 1);
    LightingMaterial mat;
    mat.metallic = metallic;
    mat.roughness = roughness;
    mat.rough_sqr = roughness*roughness;
    return mat;
}

Lighting create_lighting(LightSource source, LightingMaterial material, vec3 view, vec3 normal, vec3 albedo) {
    Lighting l;
    l.source = source;
    l.mat = material;
    l.albedo = albedo;
    l.V = -view;
    l.N = normal;
    l.H = normalize(l.V + l.N);
    l.NdotV = dot(l.N, l.V);
    l.NdotL = dot(l.N, l.source.dir);
    l.NdotH = dot(l.N, l.H);
    l.HdotV = dot(l.H, l.V);
    return l;
}

vec3 fresnel_roughness(float cos_theta, vec3 F0, float roughness)
{
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

vec3 fresnel(float cos_theta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

float ggx_dist(Lighting l) {
    float a      = l.mat.rough_sqr;
    float a2     = a*a;
    float NdotH  = max(l.NdotH, 0.0);
    float NdotH2 = NdotH*NdotH;

    float num   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = max(M_PI * denom * denom, 1e-4);

    return num / denom;
}

float ggx_geom(float cos_theta, float roughness) {
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num   = cos_theta;
    float denom = max(cos_theta * (1.0 - k) + k, 1e-4);

    return num / denom;
}

vec3 ggx_importance_sample(vec2 xi, vec3 normal, float roughness) {
    float a = roughness*roughness;

    float phi = M_TAU * xi.x;
    float cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a*a - 1.0) * xi.y));
    float sin_theta = sqrt(1.0 - cos_theta*cos_theta);

    // from spherical coordinates to cartesian coordinates
    vec3 H;
    H.x = cos(phi) * sin_theta;
    H.y = sin(phi) * sin_theta;
    H.z = cos_theta;

    // from tangent-space vector to world-space sample vector
    vec3 up        = abs(normal.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
    vec3 tangent   = normalize(cross(up, normal));
    vec3 bitangent = cross(normal, tangent);

    vec3 sampleVec = tangent * H.x + bitangent * H.y + normal * H.z;
    return normalize(sampleVec);
}

float smith(Lighting l) {
    float NdotV = max(l.NdotV, 0);
    float NdotL = max(l.NdotL, 0);
    float ggx2  = ggx_geom(NdotV, l.mat.roughness);
    float ggx1  = ggx_geom(NdotL, l.mat.roughness);

    return ggx1 * ggx2;
}

vec3 diffuse_brdf(LightSource src) {
    float attenuation = 1.0 / src.dist_sqr;
    return src.diffuse * attenuation;
}

vec3 specular_brdf(Lighting l) {
    float NDF = ggx_dist(l);
    float G = smith(l);
    float NdotV = max(0, l.NdotV);
    float NdotL = max(0, l.NdotL);
    vec3 F = fresnel_roughness(NdotV, F0, l.mat.roughness);
    vec3 num = vec3(NDF * G) * F;
    float denominator = max(4.0 * NdotV * NdotL * l.source.distance, 1e-4);
    return l.source.specular * num/denominator;
}

vec3 get_lighting(Lighting l) {
    vec3 kS = fresnel_roughness(max(0.0, l.HdotV), F0, l.mat.metallic);
    vec3 kD = (vec3(1.0) - kS) * (1.0 - l.mat.metallic);
    vec3 specular = specular_brdf(l);
    vec3 radiance = diffuse_brdf(l.source);
    float NdotL = max(0.0, l.NdotL);
    return (kD * l.albedo / M_PI + specular) * radiance * NdotL;
}
