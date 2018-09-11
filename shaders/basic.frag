#version 450

layout(location = 0) in vec3 v_normal;

layout(location = 0) out vec4 f_color;

const vec3 LIGHT = vec3(0.0, 0.0, 1.0);
const vec3 DARK_COLOR = vec3(0.6, 0.0, 0.0);
const vec3 REGULAR_COLOR= vec3(1.0, 0.0, 0.0);

void main() {
    float brightness = dot(normalize(v_normal), normalize(LIGHT));

    f_color = vec4(mix(DARK_COLOR, REGULAR_COLOR, brightness), 1.0);
}
