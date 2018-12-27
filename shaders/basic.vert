#version 450
#include <common.glsl>

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec3 v_frag_pos;
layout(location = 2) out vec3 v_view_pos;

layout(push_constant) uniform PushConstants {
	mat4 view;
	mat4 proj;
} pc;

layout(set = 0, binding = 0) uniform MVP {
	mat4 model;
} mvp;

void main() {
	// TODO Crate the normal matrix on the cpu
    v_normal = mat3(transpose(inverse(mvp.model))) * normal;
	// Get the position of the fragment
	v_frag_pos = vec3(mvp.model * vec4(position, 1.0));
	// Get the position of the camera
	v_view_pos = pc.view[3].xyz;

    gl_Position = pc.proj * pc.view * mvp.model * vec4(position, 1.0);
}
