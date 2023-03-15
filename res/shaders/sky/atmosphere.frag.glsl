#include "../common/math.glsl"
#include "../common/uniforms/view.glsl"

const float MAX = 1e3;

layout(std140) uniform Atmosphere {
    vec3 center;
    float atmosphere_radius;
    float ground_radius;
    vec3 ground_albedo;
    vec3 sun_dir;
    vec3 sun_color;
} atmosphere;

in vec3 vs_position;
in vec2 vs_uv;
in vec3 vs_normal;

layout(location=0) out vec3 frame_position;
layout(location=1) out vec3 frame_albedo;
layout(location=2) out vec4 frame_normal;
layout(location=3) out vec2 frame_rough_metal;
layout(location=4) out vec3 frame_emission;

struct Ray {
    vec3 pos, dir;
};

vec3 ray_at(Ray ray, float t) {
    return ray.pos + t * ray.dir;
}

vec2 ray_sphere_isect(Ray ray, float radius) {
    float b = dot(ray.dir, ray.pos);
    float c = dot(ray.pos, ray.pos) - radius*radius;
    float d = b * b - c;
    if (d < 0) return vec2(MAX, -MAX);

    d = sqrt(d);
    return vec2(-b - d, - b + d);
}

// Mie
// g : ( -0.75, -0.999 )
//      3 * ( 1 - g^2 )               1 + c^2
// F = ----------------- * -------------------------------
//      8pi * ( 2 + g^2 )     ( 1 + g^2 - 2 * g * c )^(3/2)
float phase_mie(float g, float c, float cc) {
    float gg = g * g;

    float a = (1.0 - gg) * (1.0 + cc);

    float b = 1.0 + gg - 2.0 * g * c;
    b *= sqrt(b);
    b *= 2.0 + gg;

    return (3.0 / 8.0 / M_PI) * a / b;
}

// Rayleigh
// g : 0
// F = 3/16PI * ( 1 + c^2 )
float phase_ray(float cc) {
    return (3.0 / 16.0 / M_PI) * (1.0 + cc);
}

// TODO: Move to uniform
const int INSCATTER = 4;
const int OUTSCATTER = 4;

float density(vec3 p, float ph) {
    return exp(-max(length(p) - atmosphere.ground_radius, 0) / ph);
}

float accumulate_dentisy(vec3 a, vec3 b, float ph) {
    vec3 s = (b-a) / float(OUTSCATTER);

    vec3 v = a + s * 0.5;
    float sum = 0;
    for (int i = 0; i < OUTSCATTER; ++i) {
        sum += density(v, ph);
        v += s;
    }
    return sum;
}

vec3 in_scatter(Ray ray, vec2 e, vec3 light) {
    // TODO: Move to uniform
    const float ph_ray = 0.05 * 10;
    const float ph_mie = 0.02 * 10;

    const vec3 k_ray = vec3(3.8, 13.5, 33.1);
    const vec3 k_mie = vec3(21.0);
    const float k_mie_ex = 1.1;

    vec3 sum_ray = vec3(0.0);
    vec3 sum_mie = vec3(0.0);
    float n_ray0 = 0.0;
    float n_mie0 = 0.0;
    float len = (e.y - e.x) / float(INSCATTER);
    vec3 s = ray.dir * len;
    vec3 v = ray_at(ray, e.x + len * 0.5);
    for (int i = 0; i < INSCATTER; ++i) {
        float density_ray = density(v, ph_ray) * len;
        float density_mie = density(v, ph_mie) * len;
        n_ray0 += density_ray;
        n_mie0 += density_mie;

        Ray out_ray = Ray(v, light);
        vec2 f = ray_sphere_isect(out_ray, atmosphere.ground_radius);
        vec3 u = ray_at(out_ray, f.y);
        float n_ray1 = accumulate_dentisy(v, u, ph_ray);
        float n_mie1 = accumulate_dentisy(v, u, ph_mie);

        vec3 attenuation = exp(-(n_ray0 + n_ray1) * k_ray - (n_mie0 + n_mie1) * k_mie * k_mie_ex);
        sum_ray += attenuation * density_ray;
        sum_mie += attenuation * density_mie;
        v += s;
    }

    float lambert_k = dot(ray.dir, -light);
    float l2 = lambert_k * lambert_k;
    vec3 scatter = sum_ray * k_ray * phase_ray(l2)
    + sum_mie * k_mie * phase_mie(-0.78, lambert_k, l2);
    return 10 * scatter;
}

vec3 get_ray_dir() {
    vec2 v_uv = (gl_FragCoord.xy - view.viewport.xy) / view.viewport.zw;
    vec4 ray_clip = vec4(v_uv * 2 - 1, -1, 1);
    vec4 ray_eye = view.inv_proj * ray_clip;
    ray_eye.zw = vec2(-1, 0);
    vec3 ray_world = (view.inv_view * ray_eye).xyz;
    return normalize(ray_world);
}

vec3 get_ray_pos() {
    vec4 pos = view.inv_view * vec4(0, 0, 0, 1);
    pos.xyz /= pos.w;
    return pos.xyz;
}

Ray ray_primary() {
    vec3 dir = get_ray_dir();
    //    vec3 pos = view.camera_pos - atmosphere.center;
    vec3 pos = get_ray_pos() - atmosphere.center;
    return Ray(pos, dir);
}

vec3 get_atmosphere() {
    Ray primary = ray_primary();
    vec2 e = ray_sphere_isect(primary, atmosphere.atmosphere_radius);
    if (e.x > e.y) return vec3(0);

    vec2 f = ray_sphere_isect(primary, atmosphere.ground_radius);
    e.y = min(e.y, f.x);
    return in_scatter(primary, e, atmosphere.sun_dir);
}

void main() {
    frame_position = vs_position;
    frame_normal = vec4(-vs_normal, 1);
    frame_albedo = vec3(0);
    frame_rough_metal = vec2(1, 0);
    frame_emission = get_atmosphere();
}