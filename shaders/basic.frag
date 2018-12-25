#version 450

layout(constant_id = 0) const float gamma = 2.2;

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_frag_pos;
layout(location = 2) in vec3 v_view_pos;

layout(location = 0) out vec4 f_color;

const vec3 LIGHT_POS = vec3(1.0, 1.0, 1.0);
const vec3 LIGHT_COLOR = vec3(1.0, 0.0, 0.0);

const float SPEC_STRENGHT = 0.5;

const vec3 MODEL_COLOR = vec3(0.5, 0.5, 0.5);
const int SHININESS = 64;

void main() {
	vec3 ambient = 0.2 * LIGHT_COLOR;

	vec3 normal = normalize(v_normal);
	vec3 light_dir = normalize(LIGHT_POS - v_frag_pos);
    float brightness = max(dot(v_normal, light_dir), 0.0);
	vec3 diffuse = brightness * LIGHT_COLOR;

	vec3 view_dir = normalize(v_view_pos - v_frag_pos);
	vec3 reflect_dir = reflect(-light_dir, normal);
	float spec = pow(max(dot(view_dir, reflect_dir), 0.0), SHININESS);
	vec3 specular = SPEC_STRENGHT * spec * LIGHT_COLOR;

	vec3 color = (ambient + diffuse + specular) * MODEL_COLOR;
	f_color = vec4(color, 1.0);
}
