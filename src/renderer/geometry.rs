use crate::renderer::shaders::{VertexInput, FragInput};
use genmesh::{
    generators::{
        Circle, Cone, Cube, Cylinder, IcoSphere, IndexedPolygon, Plane, SharedVertex, SphereUv,
        Torus,
    },
    EmitTriangles, MapVertex, Triangle, Triangulate, Vertex as GenMeshVertex, Vertices,
};
use log::info;
use specs::{Component, DenseVecStorage, HashMapStorage};
use specs_derive::Component;
use std::{fmt::Debug, sync::Arc};
use vulkano::{
    buffer::{
        cpu_pool::CpuBufferPool, cpu_pool::CpuBufferPoolSubbuffer, BufferUsage, CpuAccessibleBuffer,
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
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Shape {
    /// Sphere, number of points around the equator, number of points pole to pole
    Sphere(usize, usize),
    /// Cone, number of subdivisions around the radius, must be > 1
    Cone(usize),
    /// Cube
    Cube,
    /// Cylinder, number of points across the radius, optional subdivides along the height
    Cylinder(usize, Option<usize>),
    /// Torus, radius from origin to center of tubular, tubular radius from toridal to surface,
    /// number of tube segments >= 3, number of segments around the tube
    Torus(f32, f32, usize, usize),
    /// Icosahedral sphere, number of subdivisions > 0 if given
    IcoSphere(Option<usize>),
    /// Plane, number of subdivisions along x and y axis if given
    Plane(Option<(usize, usize)>),
    /// Circle, number of points around the circle
    Circle(usize),
}

/// MeshBuilder created by gameplay systems or from prefab and then built by the renderer
#[derive(Component)]
#[storage(HashMapStorage)]
pub struct MeshBuilder {
    shape: Shape,
}

impl MeshBuilder {
    pub fn from_shape(shape: Shape) -> Self {
        Self {
            shape,
        }
    }

    pub fn build(
        &mut self,
        device: Arc<Device>,
        // uniform_pool: &CpuBufferPool<VertexInput>,
        vertex_input_pool: &CpuBufferPool<VertexInput>,
        frag_input_pool: &CpuBufferPool<FragInput>,
        vertex_input: VertexInput,
        frag_input: FragInput,
        descriptor_set_pool: &mut FixedSizeDescriptorSetsPool<
            Arc<GraphicsPipelineAbstract + Send + Sync>,
        >,
    ) -> MeshComponent {
        let vertex_data = match self.shape {
            Shape::Sphere(u, v) => generate_v(SphereUv::new(u, v)),
            Shape::Cone(u) => generate_v(Cone::new(u)),
            Shape::Cube => generate_v(Cube::new()),
            Shape::Cylinder(u, h) => {
                if let Some(h) = h {
                    generate_v(Cylinder::subdivide(u, h))
                } else {
                    generate_v(Cylinder::new(u))
                }
            }
            Shape::Torus(radius, tubular_radius, redial_segments, tubular_segments) => generate_v(
                Torus::new(radius, tubular_radius, redial_segments, tubular_segments),
            ),
            Shape::IcoSphere(subdivisions) => {
                if let Some(subdivisions) = subdivisions {
                    generate_v(IcoSphere::subdivide(subdivisions))
                } else {
                    generate_v(IcoSphere::new())
                }
            }
            Shape::Plane(subdivisions) => {
                if let Some((x, y)) = subdivisions {
                    generate_v(Plane::subdivide(x, y))
                } else {
                    generate_v(Plane::new())
                }
            }
            Shape::Circle(u) => generate_v(Circle::new(u)),
        };

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::vertex_buffer(),
            vertex_data.iter().cloned(),
        )
        .expect("Failed to create vertex buffer");

        // let uniform_buffer = Arc::new(uniform_pool.next(self.uniforms.unwrap()).unwrap());

        let vertex_uniforms = Arc::new(vertex_input_pool.next(vertex_input).unwrap());
        let frag_uniforms = Arc::new(frag_input_pool.next(frag_input).unwrap());

        let descriptor_set = Arc::new(
            descriptor_set_pool
                .next()
                // .add_buffer(uniform_buffer.clone())
                .add_buffer(vertex_uniforms.clone())
                .unwrap()
                .add_buffer(frag_uniforms.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        MeshComponent {
            vertex_buffer,
            // uniform_buffer,
            vertex_uniforms,
            frag_uniforms,
            descriptor_set,
        }
    }
}

/// Generic mesh component
#[derive(Component)]
pub struct MeshComponent {
    pub vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    // pub index_buffer: Arc<CpuAccessibleBuffer<[u16]>>,
    // pub uniform_buffer: Arc<CpuBufferPoolSubbuffer<VertexInput, Arc<StdMemoryPool>>>,
    pub vertex_uniforms: Arc<CpuBufferPoolSubbuffer<VertexInput, Arc<StdMemoryPool>>>,
    pub frag_uniforms: Arc<CpuBufferPoolSubbuffer<FragInput, Arc<StdMemoryPool>>>,
    pub descriptor_set: Arc<DescriptorSet + Send + Sync>,
}

// Generates vertices based on shape generate
#[allow(dead_code)]
fn generate_v<F, P, G>(generator: G) -> Vec<Vertex>
where
    F: EmitTriangles<Vertex = GenMeshVertex>,
    F::Vertex: Clone + Copy + Debug + PartialEq,
    P: EmitTriangles<Vertex = usize>,
    G: SharedVertex<F::Vertex> + IndexedPolygon<P> + Iterator<Item = F>,
{
    let vertices: Vec<_> = generator.shared_vertex_iter().collect();

    info!("Shared vertices: {:?}", vertices.len());

    let vertices: Vec<_> = generator
        .indexed_polygon_iter()
        .triangulate()
        // Get the vertex
        .map(|f| {
            f.map_vertex(|u| {
                let vertex = vertices[u];

                vertex
            })
        })
        .vertices()
        // Turn GenMeshVertex into renderer::Vertex
        .map(|v| Vertex {
            position: v.pos.into(),
            normal: v.normal.into(),
        })
        .collect();

    info!("Shared vertices: {:?}", vertices.len());

    vertices
}

// FIXME This requires optimization
// Generates vertecies and indecies based on shape generator
#[allow(dead_code)]
fn generate_vi<F, P, G>(generator: G) -> (Vec<Vertex>, Vec<u16>)
where
    F: EmitTriangles<Vertex = GenMeshVertex>,
    F::Vertex: Clone + Copy + Debug + PartialEq,
    P: EmitTriangles<Vertex = usize>,
    G: SharedVertex<F::Vertex> + IndexedPolygon<P> + Iterator<Item = F>,
{
    let indexed_polygons = generator
        .indexed_polygon_iter()
        .triangulate()
        .map(|Triangle { x, y, z }| (x, y, z))
        .collect::<Vec<_>>();

    let mut indecies = Vec::with_capacity(indexed_polygons.len() * 3);

    // FIXME Find a different way to turn Vec<[u16; 3]> into Vec<u16>
    for (x, y, z) in indexed_polygons {
        indecies.push(x as u16);
        indecies.push(y as u16);
        indecies.push(z as u16);
    }

    let shared_vertecies = generator.shared_vertex_iter().collect::<Vec<_>>();

    let shared_vertecies = shared_vertecies
        .iter()
        .map(|v| Vertex {
            position: v.pos.into(),
            normal: v.normal.into(),
        })
        .collect::<Vec<_>>();

    (shared_vertecies, indecies)
}
