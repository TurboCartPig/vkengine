use std::sync::Arc;
use vulkano::device::Device;

/// export the uniform input of the vertex shader
pub use self::vertex::ty::MVP as VertexInput;
// Structs from the fragment shader
pub use self::fragment::ty::{DirectionalLight, PointLight};
// Uniforms from the fragment shader
pub use self::fragment::ty::{Lights, PointLights};
// pub use self::fragment::ty::Material;

pub use self::vertex::ty::PushConstants;

pub use self::{
    fragment::SpecializationConstants as FragSC, vertex::SpecializationConstants as VertexSC,
};

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
        include: ["shaders"],
        path: "shaders/basic.vert",
    }
}

mod fragment {
    use vulkano_shaders::shader;

    shader! {
        ty: "fragment",
        include: ["shaders"],
        path: "shaders/basic.frag",
    }
}
