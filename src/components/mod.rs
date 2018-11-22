mod transform;

pub use crate::components::transform::{Transform, TransformMatrix};

use specs::prelude::*;
use specs_hierarchy::Parent;

/// Component defining a link in a hierarchy of components
#[derive(Debug, Copy, Clone)]
pub struct Link {
    parent: Entity,
}

impl Link {
    pub fn new(parent: Entity) -> Self {
        Self { parent }
    }
}

impl Component for Link {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl Parent for Link {
    fn parent_entity(&self) -> Entity {
        self.parent
    }
}
