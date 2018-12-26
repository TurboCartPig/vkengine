#version 450
#include <common.glsl>

layout(constant_id = 0) const float gamma = 2.2;

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_frag_pos;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 1) uniform Input {
	vec3 view_pos;
} stuff;

const DirectionalLight LIGHT = DirectionalLight(
		vec3(1.0), 				// Direction
		vec3(0.1), 				// Ambient
		vec3(0.5, 1.0, 0.5), 	// Diffuse
		vec3(1.0) 				// Specular
	);

const Material MATERIAL = Material(
		vec3(0.0, 0.0, 1.0),	// Diffuse
		vec3(1.0),				// Specular
		64.0					// Shininess
	);

vec3 directional_light(DirectionalLight light, vec3 normal, vec3 view_dir, Material mat) {
	normal = normalize(normal);
	view_dir = normalize(view_dir);
	vec3 light_dir = normalize(-light.direction);

	// Diffuse
    float brightness = max(dot(normal, light_dir), 0.0);

	// Specular
	vec3 reflect_dir = reflect(-light_dir, normal);
	float spec = pow(max(dot(view_dir, reflect_dir), 0.0), mat.shininess);

	vec3 ambient = light.ambient * mat.diffuse;
	vec3 diffuse = light.diffuse * mat.diffuse * brightness;
	vec3 specular = light.specular * mat.specular * spec;

	return (ambient + diffuse + specular);
}

void main() {
	vec3 view_dir = normalize(stuff.view_pos - v_frag_pos);
	vec3 color = directional_light(LIGHT, v_normal, view_dir, MATERIAL);
	f_color = vec4(color, 1.0);
}
