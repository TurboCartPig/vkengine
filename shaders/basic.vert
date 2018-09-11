#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 v_normal;

layout(set = 0, binding = 0) uniform Data {
  //mat4 world;
  mat4 view;
  mat4 proj;
  mat4 model;
} uniforms;

void main() {
    //mat4 worldview = uniforms.world * uniforms.view;
    mat4 worldview = uniforms.model * uniforms.view;
    v_normal = transpose(inverse(mat3(worldview))) * normal;

    mat4 space = uniforms.proj * worldview;
    //mat4 space uniforms.proj * worldview * uniforms.model;

    gl_Position = space * vec4(position, 1.0);
}
