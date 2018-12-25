#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 v_frag_pos;
layout(location = 2) out vec3 v_view_pos;

layout(set = 0, binding = 0) uniform Data {
	mat4 model;
	mat4 view;
	mat4 proj;
	vec3 view_pos;
} uniforms;

void main() {
	// TODO Crate the normal matrix on the cpu
    v_normal = mat3(transpose(inverse(uniforms.model))) * normal;
	v_frag_pos = vec3(uniforms.model * vec4(position, 1.0));
	v_view_pos = uniforms.view_pos;

    gl_Position = uniforms.proj * uniforms.view * uniforms.model * vec4(position, 1.0);
}
