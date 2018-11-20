use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::instance::{
    debug::{DebugCallback, MessageTypes},
    Instance,
};

/// Wrapper for Vulkan debug callback
///
/// Since _callback is never accessed through Debug we can impl Send + Sync  
pub struct Debug {
    _callback: DebugCallback,
}

unsafe impl Send for Debug {}
unsafe impl Sync for Debug {}

impl Debug {
    pub fn from_instance(instance: &Arc<Instance>) -> Self {
        let message_types = MessageTypes {
            error: true,
            warning: true,
            performance_warning: true,
            information: true,
            debug: true,
        };

        let _callback = DebugCallback::new(instance, message_types, |msg| {
            if msg.ty.error {
                error!("{}: {}", msg.layer_prefix, msg.description)
            } else if msg.ty.warning {
                warn!("{}: {}", msg.layer_prefix, msg.description)
            } else if msg.ty.performance_warning {
                warn!("{}: {}", msg.layer_prefix, msg.description)
            } else if msg.ty.information {
                info!("{}: {}", msg.layer_prefix, msg.description)
            } else if msg.ty.debug {
                debug!("{}: {}", msg.layer_prefix, msg.description)
            }
        })
        .expect("Failed to register debug callback");

        Self { _callback }
    }
}

