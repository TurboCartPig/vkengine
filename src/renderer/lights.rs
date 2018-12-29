use crate::renderer::shaders::{DirectionalLight, PointLight};
use nalgebra::Vector3;
use specs::prelude::*;

#[derive(Debug)]
pub struct DirectionalLightRes {
    // The direction of the light
    direction: Vector3<f32>,
    // The color of the light
    ambient: Vector3<f32>,
    diffuse: Vector3<f32>,
    specular: Vector3<f32>,
}

impl Default for DirectionalLightRes {
    fn default() -> Self {
        Self {
            // Down
            direction: Vector3::new(0.0, 1.0, 0.0),
            // White
            ambient: Vector3::new(1.0, 1.0, 1.0),
            diffuse: Vector3::new(1.0, 1.0, 1.0),
            specular: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl DirectionalLightRes {
    pub fn new(direction: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self {
            direction,
            ambient: color * 0.2,
            diffuse: color,
            specular: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn to_directional_light(&self) -> DirectionalLight {
        DirectionalLight {
            direction: self.direction.into(),
            _dummy0: [0; 4],
            ambient: self.ambient.into(),
            _dummy1: [0; 4],
            diffuse: self.diffuse.into(),
            _dummy2: [0; 4],
            specular: self.specular.into(),
        }
    }
}

#[derive(Debug)]
pub struct PointLightComponent {
    // These determine the distance multiplier
    constant: f32,
    linear: f32,
    quadratic: f32,
    // The color of the light
    ambient: Vector3<f32>,
    diffuse: Vector3<f32>,
    specular: Vector3<f32>,
}

impl Component for PointLightComponent {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

impl PointLightComponent {
    pub fn from_color(color: Vector3<f32>) -> Self {
        Self {
            // Distance of 50
            constant: 1.0,
            linear: 0.09,
            quadratic: 0.032,
            // Scale the diffuse color for ambient
            ambient: color * 0.2,
            diffuse: color,
            specular: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn to_point_light(&self, position: Vector3<f32>) -> PointLight {
        PointLight {
            position: position.into(),
            constant: self.constant,
            linear: self.linear,
            quadratic: self.quadratic,
            _dummy0: [0; 8],
            ambient: self.ambient.into(),
            diffuse: self.diffuse.into(),
            specular: self.specular.into(),
            _dummy1: [0; 4],
            hh: 0,
            _dummy2: [0; 4],
        }
    }
}
