use crate::components::{Link, Transform, TransformMatrix};
use hibitset::BitSet;
use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchyEvent, Parent};

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
                    if let Some(p_trans) = transforms.get(parent_entity) {
                        matrix.mat = p_trans.to_matrix() * matrix.mat;
                    }
                }
            });

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

#[cfg(test)]
mod test {
    use crate::{
        components::{Link, Transform, TransformMatrix},
        systems::TransformSystem,
    };
    use nalgebra::Vector3;
    use specs::prelude::*;
    use specs_hierarchy::HierarchySystem;

    fn world<'a, 'b>() -> (World, Dispatcher<'a, 'b>) {
        let mut world = World::new();
        let hierarchy_sys = HierarchySystem::<Link>::new();
        let transform_sys = TransformSystem::default();

        world.register::<Transform>();
        world.register::<TransformMatrix>();
        world.register::<Link>();

        let mut dispatcher = DispatcherBuilder::new()
            .with(hierarchy_sys, "hs", &[])
            .with(transform_sys, "ts", &["hs"])
            .build();

        dispatcher.setup(&mut world.res);

        (world, dispatcher)
    }

    // Test if TransformMatrix is inserted and synced
    #[test]
    fn basic() {
        let (mut world, mut dispatcher) = world();

        let tra = Transform::from(Vector3::new(5.9, 3.9, 1.0));
        let e1 = world.create_entity().with(tra.clone()).build();

        dispatcher.dispatch(&world.res);

        let sys_mat = world
            .read_storage::<TransformMatrix>()
            .get(e1)
            .unwrap()
            .clone()
            .mat;
        let tra_mat = tra.to_matrix();

        assert_eq!(sys_mat, tra_mat);
    }

    // Test if matrix is synced even if parent is after child
    #[test]
    fn complex() {
        let (mut world, mut dispatcher) = world();

        let tra = Transform::from(Vector3::new(5.9, 3.9, 1.0));
        let e1 = world.create_entity().with(tra.clone()).build();

        let e2 = world.create_entity().with(tra.clone()).build();

        {
            let mut links = world.write_storage::<Link>();
            links.insert(e1, Link::new(e2)).unwrap();
        }

        world.maintain();

        dispatcher.dispatch(&world.res);

        let abs_tra_e1 = world
            .read_storage::<TransformMatrix>()
            .get(e1)
            .unwrap()
            .mat
            .clone();
        let abs_tra_e2 = world
            .read_storage::<TransformMatrix>()
            .get(e2)
            .unwrap()
            .mat
            .clone();
        let abs_tra = tra.to_matrix() * tra.to_matrix();

        // The absolute matricies should no longer be equal
        assert_ne!(abs_tra_e1, abs_tra_e2);
        // Actual result should be the same as simulated result
        assert_eq!(abs_tra_e1, abs_tra);
    }
}
