float desaturate(vec3 color) {
    const vec3 to_luma = vec3(0.2125, 0.7154, 0.0721);
    return dot(color, to_luma);
}

