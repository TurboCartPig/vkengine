#version 450
#include <common.glsl>

layout(constant_id = 0) const float gamma = 2.2;

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_frag_pos;
layout(location = 2) in vec3 v_view_pos;

layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0) uniform Lights {
	DirectionalLight dir_light;
} lights;

layout(set = 1, binding = 1) readonly buffer PointLights {
	PointLight lights[];
} point_lights;

const float AMBIENT_STRENGHT = 0.2;

const Material MATERIAL = Material(
	vec3(1.0, 1.0, 1.0),	// Diffuse
	vec3(1.0),				// Specular
	64.0					// Shininess
);

vec3 calc_directional_light(DirectionalLight light, vec3 normal, vec3 view_dir) {
	vec3 light_dir = normalize(light.direction);

	// Diffuse
    float brightness = max(dot(normal, light_dir), 0.0);

	// Specular
	vec3 reflect_dir = reflect(-light_dir, normal);
	float spec = pow(max(dot(view_dir, reflect_dir), 0.0), MATERIAL.shininess);

	vec3 ambient = light.ambient * AMBIENT_STRENGHT * MATERIAL.diffuse;
	vec3 diffuse = light.diffuse * brightness * MATERIAL.diffuse;
	vec3 specular = light.specular * spec * MATERIAL.specular;

	return (ambient + diffuse + specular);
}

vec3 calc_point_light(PointLight light, vec3 normal, vec3 view_dir, vec3 frag_pos) {
	vec3 light_dir = normalize(light.position - frag_pos);

	// Diffuse
	float brightness = max(dot(normal, light_dir), 0.0);

	// Specular
	vec3 reflect_dir = reflect(-light_dir, normal);
	float spec = pow(max(dot(view_dir, reflect_dir), 0.0), MATERIAL.shininess);

	// Attenuation
	float dist = length(light.position - frag_pos);
	float attenuation = 1.0 / (light.constant + light.linear * dist + light.quadratic * (dist * dist));

	vec3 ambient = light.ambient * AMBIENT_STRENGHT * MATERIAL.diffuse * attenuation;
	vec3 diffuse = light.diffuse * brightness * MATERIAL.diffuse * attenuation;
	vec3 specular = light.specular * spec * MATERIAL.specular * attenuation;

	return (ambient + diffuse + specular);
}

void main() {
	vec3 view_dir = normalize(v_view_pos - v_frag_pos);
	vec3 normal = normalize(v_normal);

	vec3 color = vec3(0.0);

	// Directinal light
	color += calc_directional_light(lights.dir_light, normal, view_dir);

	// Point lights
	int num_point_lights = point_lights.lights.length();
	for (int i = 0; i < num_point_lights; i++)
		color += calc_point_light(point_lights.lights[i], normal, view_dir, v_frag_pos);

	f_color = vec4(color, 1.0);
}
