use vulkano::{device::Queue, instance::QueueFamiliesIter};

use std::sync::Arc;

use renderer::Surface;

// TODO Find a more dynamic way of storing queues
// The point is to be able to have the same code work regardless of how many queues we actually vulkano_shader_derive
// For example, on Intel, these all refer to the same queue
pub struct Queues {
    pub general: Arc<Queue>,
    pub compute: Arc<Queue>,
    pub graphics: Arc<Queue>,
    pub present: Arc<Queue>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum QueueFamilyTypes {
    General,
    Compute,
    Graphics,
    Present,
}

#[derive(Debug, Clone)]
pub struct QueueFamilyIds {
    pub general: Option<u32>,
    pub compute: Option<u32>,
    pub graphics: Option<u32>,
    pub present: Option<u32>,
}

impl QueueFamilyIds {
    pub fn none() -> Self {
        Self {
            general: None,
            compute: None,
            graphics: None,
            present: None,
        }
    }

    pub fn from_queue_families_iter(iter: QueueFamiliesIter, surface: Surface) -> Self {
        let mut ids = Self::none();

        // We assume all queue families support transfers
        for qf in iter {
            if qf.supports_compute()
                && qf.supports_graphics()
                && surface.is_supported(qf).unwrap_or(false)
            {
                ids.general = Some(qf.id());
            } else if qf.supports_compute() {
                ids.compute = Some(qf.id());
            } else if qf.supports_graphics() && surface.is_supported(qf).unwrap_or(false) {
                ids.graphics = Some(qf.id());
            } else if surface.is_supported(qf).unwrap_or(false) {
                ids.present = Some(qf.id());
            }
        }

        ids
    }
}
