#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 v_frag_pos;

layout(set = 0, binding = 0) uniform MVP {
	mat4 model;
	mat4 view;
	mat4 proj;
} mvp;

void main() {
	// TODO Crate the normal matrix on the cpu
    v_normal = mat3(transpose(inverse(mvp.model))) * normal;
	v_frag_pos = vec3(mvp.model * vec4(position, 1.0));

    gl_Position = mvp.proj * mvp.view * mvp.model * vec4(position, 1.0);
}
