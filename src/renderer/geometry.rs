use genmesh::{
    generators::{Cube, IndexedPolygon, Plane, SharedVertex, SphereUv},
    EmitTriangles, MapToVertices, MapVertex, Quad, Triangle, Triangulate, Vertex as GenMeshVertex,
    Vertices,
};
use na::{Matrix4, Rotation3, Translation3, Vector3};
use renderer::Vertex;
use specs::prelude::*;
use std::{fmt::Debug, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

#[derive(Component, Debug)]
pub struct MeshComponent {
    pub vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub index_buffer: Arc<CpuAccessibleBuffer<[u16]>>,
}

/// Primitive shapes
#[derive(Clone, Debug)]
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

pub struct ShapedMeshBuilder {}

impl ShapedMeshBuilder {
    pub fn new(device: Arc<Device>, shape: Shape) -> MeshComponent {
        let (vertex_buffer, index_buffer) = {
            let (vertex_data, index_data) = match shape {
                Shape::Sphere(u, v) => generate_vi(SphereUv::new(u, v)),
                Shape::Cube => generate_vi(Cube::new()),
                _ => unreachable!(),
            };

            let vertex_buffer = CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                vertex_data.iter().cloned(),
            ).expect("Failed to create vertex buffer");

            let index_buffer = CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                index_data.iter().cloned(),
            ).expect("Failed to create index buffer");

            (vertex_buffer, index_buffer)
        };

        MeshComponent {
            vertex_buffer,
            index_buffer,
        }
    }
}

// Generates vertecies based on shape generate
fn generate_v<F, P, G>(generator: G) -> Vec<Vertex>
where
    F: EmitTriangles<Vertex = GenMeshVertex>,
    F::Vertex: Clone + Copy + Debug + PartialEq,
    P: EmitTriangles<Vertex = usize>,
    G: SharedVertex<F::Vertex> + IndexedPolygon<P> + Iterator<Item = F>,
{
    let vertices: Vec<_> = generator.shared_vertex_iter().collect();

    let vertices: Vec<_> = generator
        .indexed_polygon_iter()
        .triangulate()
        // Get the vertex
        .map(|f| {
            f.map_vertex(|u| {
                let vertex = vertices[u];

                (vertex, u)
            })
        }).vertices()
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

    vertices
}

// FIXME This requires optimization
// Generates vertecies and indecies based on shape generator
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
        .map(|Triangle { x, y, z }| [x, y, z])
        .collect::<Vec<_>>();

    let mut new_indexed_polygons = Vec::with_capacity(indexed_polygons.len());

    // FIXME Find a differnt way to turn Vec<[u16; 3]> into Vec<u16>
    for [x, y, z] in indexed_polygons {
        new_indexed_polygons.push(x as u16);
        new_indexed_polygons.push(y as u16);
        new_indexed_polygons.push(z as u16);
    }

    let shared_vertecies = shared_vertecies
        .iter()
        .map(|v| Vertex {
            position: v.pos.into(),
            normal: v.normal.into(),
        }).collect::<Vec<_>>();

    (shared_vertecies, new_indexed_polygons)
}
