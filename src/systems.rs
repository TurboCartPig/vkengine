use crate::{
    components::{Link, Transform, TransformMatrix},
    renderer::camera::ActiveCamera,
    resources::{DeltaTime, Keyboard, Mouse},
};
use float_duration::TimePoint;
use hibitset::BitSet;
use nalgebra::{UnitQuaternion, Vector3};
use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchyEvent, Parent};
use std::{mem, time::Instant};
use winit::VirtualKeyCode;

pub struct TimeSystem {
    first_frame: Instant,
    last_frame: Instant,
}

impl Default for TimeSystem {
    fn default() -> Self {
        TimeSystem {
            first_frame: Instant::now(),
            last_frame: Instant::now(),
        }
    }
}

impl<'a> System<'a> for TimeSystem {
    type SystemData = Write<'a, DeltaTime>;

    fn run(&mut self, mut delta_time: Self::SystemData) {
        let now = Instant::now();

        let delta = now.float_duration_since(self.last_frame).unwrap();
        delta_time.delta = delta.as_seconds();

        let first_frame = now.float_duration_since(self.first_frame).unwrap();
        delta_time.first_frame = first_frame.as_seconds();

        mem::replace(&mut self.last_frame, now);
    }
}

/// Syncs Transform and TransformMatrix per entity
///
/// For every Transform, whether relative or absolute, there should be a TransformMatrix
/// that contains the transform matrix for said Transform.
pub struct TransformSystem {
    dirty: BitSet,
    transform_reader_id: Option<ReaderId<ComponentEvent>>,
    hierarchy_reader_id: Option<ReaderId<HierarchyEvent>>,
}

impl<'a> System<'a> for TransformSystem {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, Hierarchy<Link>>,
        ReadStorage<'a, Link>,
        ReadStorage<'a, Transform>,
        WriteStorage<'a, TransformMatrix>,
    );

    // TODO We clone 2 bitsets here, that is not optimal
    fn run(&mut self, (entities, hierarchy, links, transforms, mut matrices): Self::SystemData) {
        
        // Add TransformMatrix component to all entities with Transforms
        (&entities, &transforms, !matrices.mask().clone())
            .join()
            .for_each(|(entity, transform, _)| {
                matrices
                    .insert(entity, TransformMatrix::from(transform.to_matrix()))
                    .unwrap();
                self.dirty.add(entity.id());
            });
        
        
        // Read events
        // Add new or modified entities to dirty bitset
        transforms
            .channel()
            .read(self.transform_reader_id.as_mut().unwrap())
            .for_each(|event| match *event {
                ComponentEvent::Removed(_) => (),
                ComponentEvent::Inserted(id) | ComponentEvent::Modified(id) => {
                    self.dirty.add(id);
                }
            });

        // If there are new or different parents, we need to resync
        hierarchy
            .changed()
            .read(self.hierarchy_reader_id.as_mut().unwrap())
            .for_each(|event| match *event {
                HierarchyEvent::Removed(entity) => {
                    let _ = entities.delete(entity);
                }
                HierarchyEvent::Modified(entity) => {
                    self.dirty.add(entity.id());
                }
            });

        // Children of dirty entities are also dirty and need their matrices synced
        (&entities, &transforms, &matrices, &self.dirty.clone())
            .join()
            .for_each(|(entity, _, _, _)| {
                let children = hierarchy.all_children(entity);
                self.dirty |= &children;
            });

        // Sync all dirty entities and their children
        (&entities, &transforms, &mut matrices, &self.dirty)
            .join()
            .for_each(|(entity, transform, matrix, _)| {
                matrix.mat = transform.to_matrix();

                let mut parent_entity = entity;
                while let Some(link) = links.get(parent_entity) {
                    parent_entity = link.parent_entity();
                    println!("Hei");
                    if let Some(p_trans) = transforms.get(parent_entity) {
                        matrix.mat = p_trans.to_matrix() * matrix.mat;
                        println!("Hei2");
                    }
                }
            });

        // Sync transforms without parents
        // We join on self.dirty so we only sync transforms that are out of sync
        // (&entities, &transforms, &mut matrices, &self.dirty, !&links)
        //     .join()
        //     .for_each(|(entity, transform, matrix, _, _)| {
        //         matrix.mat = transform.to_matrix();
        //     });

        // Sync transforms with parents
        // hierarchy.all().iter().for_each(|entity| {
        //     let self_dirty = self.dirty.contains(entity.id());

        // });

        // (&entities, &links, &transforms, &mut matrices, &self.dirty).join().for_each(|(entity, link, transform, matrix, _)| {

        // });

        // Reset
        self.dirty.clear();
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);

        let mut transforms = WriteStorage::<Transform>::fetch(res);
        let mut hierarchy = res.fetch_mut::<Hierarchy<Link>>();

        self.transform_reader_id = Some(transforms.register_reader());
        self.hierarchy_reader_id = Some(hierarchy.track());
    }
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self {
            dirty: BitSet::new(),
            transform_reader_id: None,
            hierarchy_reader_id: None,
        }
    }
}

/// Fly control system
pub struct FlyControlSystem;

impl<'a> System<'a> for FlyControlSystem {
    type SystemData = (
        Read<'a, Keyboard>,
        Write<'a, Mouse>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (keyboard, mut mouse, delta_time, active_camera, mut transform): Self::SystemData,
    ) {
        // If mouse is not grabbed, then the window is not focused, and we sould not handle input
        if !mouse.grabbed {
            return;
        }

        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        let (yaw, pitch) = mouse.move_delta;
        let (yaw, pitch) = (yaw as f32 * -0.001, pitch as f32 * -0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Reset mouse input
        mouse.clear_deltas();

        // Translation
        if keyboard.pressed(VirtualKeyCode::W) {
            camera_t.translate_forward(1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::S) {
            camera_t.translate_forward(-1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::A) {
            camera_t.translate_right(-1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::D) {
            camera_t.translate_right(1.0 * delta_time.delta as f32);
        }
    }
}
