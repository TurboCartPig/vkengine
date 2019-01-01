use crate::renderer::shaders::VertexInput;
use gltf;
use log::info;
use nalgebra::Vector3;
use ncollide3d::procedural;
use specs::{Component, DenseVecStorage, HashMapStorage};
use specs_derive::Component;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use vulkano::{
    buffer::{
        cpu_pool::{CpuBufferPool, CpuBufferPoolSubbuffer},
        BufferUsage, CpuAccessibleBuffer,
    },
    descriptor::descriptor_set::{DescriptorSet, FixedSizeDescriptorSetsPool},
    device::Device,
    impl_vertex,
    memory::pool::StdMemoryPool,
    pipeline::GraphicsPipelineAbstract,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl_vertex!(Vertex, position, normal);

/// Primitive shapes
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    /// Sphere, number of points around the equator, number of points pole to pole
    Sphere(u32, u32),
    /// Cone, number of subdivisions around the radius, must be > 1
    Cone(u32),
    /// Cube
    Cube,
    /// Cylinder, number of points across the radius
    Cylinder(u32),
    /// Plane, number of subdivisions along x and y axis if given
    Quad(u32, u32),
    /// Capsule, number of subdivides around and across the capsule
    Capsule(u32, u32),
}

/// MeshBuilder created by gameplay systems or from prefab and then built by the renderer
#[derive(Component, Default, Debug)]
#[storage(HashMapStorage)]
pub struct MeshBuilder {
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
        }
    }

    pub fn with_shape(mut self, shape: Shape) -> Self {
        let mut trimesh = match shape {
            Shape::Sphere(u, v) => procedural::sphere(1.0, u, v, false),
            Shape::Cone(u) => procedural::cone(1.0, 1.0, u),
            Shape::Cube => procedural::cuboid(&Vector3::new(1.0, 1.0, 1.0)),
            Shape::Cylinder(u) => procedural::cylinder(1.0, 1.0, u),
            Shape::Quad(u, v) => procedural::quad(1.0, 1.0, u as usize, v as usize),
            Shape::Capsule(u, v) => procedural::capsule(&1.0, &1.0, u, v),
        };

        trimesh.unify_index_buffer();
        trimesh.recompute_normals();

        self.index_data = trimesh.flat_indices();

        let vertex_iter = trimesh.coords.into_iter();
        let normal_iter = trimesh.normals.unwrap().into_iter();

        self.vertex_data = vertex_iter
            .zip(normal_iter)
            .map(|(position, normal)| Vertex {
                position: position.coords.into(),
                normal: normal.into(),
            })
            .collect::<Vec<_>>();

        self
    }

    pub fn with_gltf_file(mut self, file: &str) -> Self {
        let file = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("resources")
            .join(file);

        println!("Loading file: {:?}", file);

        let (gltf, buffers, _) = gltf::import(file).expect("Failed to import gltf document");

        println!("Parsing file");

        // Get the first scene
        let scene = gltf.scenes().next().unwrap();

        // FIXME Only supports one mesh
        // Go through the nodes and add the meshes to vertex_data
        scene.nodes().for_each(|node| {
            if let Some(mesh) = node.mesh() {
                println!("Node: {:?}, has a mesh", node.index());

                mesh.primitives().for_each(|primitive| {
                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                    if let (Some(positions), Some(normals)) =
                        (reader.read_positions(), reader.read_normals())
                    {
                        println!("Writing vertex and index data");

                        self.vertex_data = positions
                            .zip(normals)
                            .map(|(position, normal)| Vertex { position, normal })
                            .collect();

                        self.index_data = reader.read_indices().unwrap().into_u32().collect();
                    }
                });
            }
        });

        self
    }

    pub fn build(
        self,
        device: Arc<Device>,
        vertex_input_pool: &CpuBufferPool<VertexInput>,
        vertex_input: VertexInput,
        descriptor_set_pool: &mut FixedSizeDescriptorSetsPool<
            Arc<GraphicsPipelineAbstract + Send + Sync>,
        >,
    ) -> MeshComponent {
        info!(
            "Building mesh from: Vertices: {:?}, Indices: {:?}",
            self.vertex_data, self.index_data
        );

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::vertex_buffer(),
            self.vertex_data.into_iter(),
        )
        .expect("Failed to create vertex buffer");

        let index_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::index_buffer(),
            self.index_data.into_iter(),
        )
        .expect("Failed to create index buffer");

        let vertex_uniforms = Arc::new(vertex_input_pool.next(vertex_input).unwrap());

        let descriptor_set = Arc::new(
            descriptor_set_pool
                .next()
                .add_buffer(vertex_uniforms.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        MeshComponent {
            vertex_buffer,
            index_buffer,
            vertex_uniforms,
            descriptor_set,
        }
    }
}

/// Generic mesh component
#[derive(Component)]
pub struct MeshComponent {
    pub vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub index_buffer: Arc<CpuAccessibleBuffer<[u32]>>,
    pub vertex_uniforms: Arc<CpuBufferPoolSubbuffer<VertexInput, Arc<StdMemoryPool>>>,
    pub descriptor_set: Arc<DescriptorSet + Send + Sync>,
}
