use std::sync::Arc;
use vulkano::device::Device;

/// export the uniform input of the vertex shader
pub use self::vertex::ty::Data as VertexInput;
pub use self::vertex::SpecializationConstants as VertexSC;
pub use self::fragment::SpecializationConstants as FragSC;

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

mod vertex {
    use vulkano_shaders::shader;

    shader! {
        ty: "vertex",
        path: "shaders/basic.vert"
    }
}

mod fragment {
    use vulkano_shaders::shader;

    shader! {
        ty: "fragment",
        path: "shaders/basic.frag"
    }
}
