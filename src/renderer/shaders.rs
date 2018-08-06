pub struct ShaderSet {
    pub vertex: vertex::Shader,
    pub fragment: fragment::Shader,
}

#[allow(dead_code)]
pub mod vertex {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[path = "shaders/basic.vert"]
    pub struct Dummy;
}

#[allow(dead_code)]
pub mod fragment {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[path = "shaders/basic.frag"]
    pub struct Dummy;
}
