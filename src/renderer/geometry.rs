use genmesh::{
    generators::{
        Circle, Cone, Cube, Cylinder, IcoSphere, IndexedPolygon, Plane, SharedVertex, SphereUv,
        Torus,
    },
    EmitTriangles, MapVertex, Triangle, Triangulate, Vertex as GenMeshVertex, Vertices,
};
use renderer::Vertex;
use specs::DenseVecStorage;
use std::{
    fmt::Debug,
    sync::Arc,
};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

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
    /// Plane, located in the XY plane, number of subdivisions along x and y axis if given
    Plane(Option<(usize, usize)>),
    /// Circle, located in the XY plane, number of points around the circle
    Circle(usize),
}

/// Generic mesh component
#[derive(Component)]
pub struct MeshComponent {
    pub vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    // pub index_buffer: Arc<CpuAccessibleBuffer<[u16]>>,
}

impl MeshComponent {
    pub fn from_shape(device: Arc<Device>, shape: Shape) -> Self {
        let vertex_buffer = {
            let vertex_data = match shape {
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
                Shape::Torus(radius, tubular_radius, redial_segments, tubular_segments) => {
                    generate_v(Torus::new(
                        radius,
                        tubular_radius,
                        redial_segments,
                        tubular_segments,
                    ))
                }
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
                BufferUsage::all(),
                vertex_data.iter().cloned(),
            )
            .expect("Failed to create vertex buffer");

            vertex_buffer
        };

        Self {
            vertex_buffer,
        }
    }
}

// Generates vertecies based on shape generate
#[allow(dead_code)]
fn generate_v<F, P, G>(generator: G) -> Vec<Vertex>
where
    F: EmitTriangles<Vertex = GenMeshVertex>,
    F::Vertex: Clone + Copy + Debug + PartialEq,
    P: EmitTriangles<Vertex = usize>,
    G: SharedVertex<F::Vertex> + IndexedPolygon<P> + Iterator<Item = F>,
{
    let vertices: Vec<_> = generator.shared_vertex_iter().collect();
    println!("Shared vertices: {:?}", vertices.len());

    let vertices: Vec<_> = generator
        .indexed_polygon_iter()
        .triangulate()
        // Get the vertex
        .map(|f| {
            f.map_vertex(|u| {
                let vertex = vertices[u];

                (vertex, u)
            })
        })
        .vertices()
        // Turn GenMeshVertex into renderer::Vertex
        .map(|(v, u)| {
            let vertex = Vertex {
                position: v.pos.into(),
                normal: v.normal.into(),
            };

            (vertex, u)
        })
        // Drop indecies
        .map(|(v, _)| v)
        .collect();

    println!("Shared vertices: {:?}", vertices.len());
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
     let shared_vertecies = generator.shared_vertex_iter().collect::<Vec<_>>();

     let indexed_polygons = generator
         .indexed_polygon_iter()
         .triangulate()
         // .vertecies() might do what I want
         .map(|Triangle { x, y, z }| (x, y, z))
         .collect::<Vec<_>>();

     let mut indecies = Vec::with_capacity(indexed_polygons.len());

     // FIXME Find a differnt way to turn Vec<[u16; 3]> into Vec<u16>
     for (x, y, z) in indexed_polygons {
         indecies.push(x as u16);
         indecies.push(y as u16);
         indecies.push(z as u16);
     }

     let shared_vertecies = shared_vertecies
         .iter()
         .map(|v| Vertex {
             position: v.pos.into(),
             normal: v.normal.into(),
         })
         .collect::<Vec<_>>();

     // println!("Shared Vertecies: {:?}", shared_vertecies);

     (shared_vertecies, indecies)
}
