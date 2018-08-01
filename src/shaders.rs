#[allow(dead_code)]
pub mod vertex {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[path = "shaders/basic.vert"]
    struct Dummy;
}

#[allow(dead_code)]
pub mod fragment {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[path = "shaders/basic.frag"]
    struct Dummy;
}
