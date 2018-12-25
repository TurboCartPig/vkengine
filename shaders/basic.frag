#version 450

layout(constant_id = 0) const float gamma = 2.2;

layout(location = 0) in vec3 v_normal;

layout(location = 0) out vec4 f_color;

const vec3 LIGHT_DIR = vec3(1.0, 1.0, 1.0);
const vec3 DARK_COLOR = vec3(0.1, 0.1, 0.2);
const vec3 REGULAR_COLOR= vec3(1.0, 1.0, 1.0);

void main() {
    float brightness = abs(dot(normalize(v_normal), normalize(LIGHT_DIR)));

    f_color = vec4(mix(DARK_COLOR, REGULAR_COLOR, brightness), 1.0);
}
