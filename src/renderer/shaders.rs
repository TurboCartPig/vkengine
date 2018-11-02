use std::sync::Arc;
use vulkano::device::Device;
use vulkano_shaders::vulkano_shader;

/// export the uniform input of the vertex shader
pub use renderer::shaders::vertex::ty::Data as VertexInput;

pub struct ShaderSet {
    pub vertex: vertex::Shader,
    pub fragment: fragment::Shader,
}

impl ShaderSet {
    pub fn new(device: Arc<Device>) -> Self {
        let vertex = vertex::Shader::load(device.clone()).expect("Failed to create shader module");
        let fragment =
            fragment::Shader::load(device.clone()).expect("Failed to create shader module");

        Self { vertex, fragment }
    }
}

vulkano_shader!{
    mod_name: vertex,
    ty: "vertex",
    path: "shaders/basic.vert"
}

vulkano_shader!{
    mod_name: fragment,
    ty: "fragment",
    path: "shaders/basic.frag"
}
