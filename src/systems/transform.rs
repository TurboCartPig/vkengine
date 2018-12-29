use crate::{
    components::{GlobalTransform, Link, Transform},
    resources::DirtyEntities,
};
use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchyEvent, Parent};

/// Syncs Transform and GobalTransform per entity
///
/// For every Transform, whether relative or absolute, there should be a GlobalTransform
/// that contains the global transform for said Transform.
pub struct TransformSystem {
    transform_reader_id: Option<ReaderId<ComponentEvent>>,
    hierarchy_reader_id: Option<ReaderId<HierarchyEvent>>,
}

impl TransformSystem {
    /// Add a GlobalTransform to any entity with a Transform component
    fn add_globals(
        entities: &Entities<'_>,
        transforms: &ReadStorage<'_, Transform>,
        globals: &mut WriteStorage<'_, GlobalTransform>,
        dirty_entities: &mut Write<'_, DirtyEntities>,
    ) {
        (entities, transforms, !globals.mask().clone())
            .join()
            .for_each(|(entity, transform, _)| {
                globals
                    .insert(entity, GlobalTransform::from(transform.clone()))
                    .unwrap();
                dirty_entities.dirty.add(entity.id());
            });
    }
}

impl<'a> System<'a> for TransformSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, DirtyEntities>,
        ReadExpect<'a, Hierarchy<Link>>,
        ReadStorage<'a, Link>,
        ReadStorage<'a, Transform>,
        WriteStorage<'a, GlobalTransform>,
    );

    fn run(
        &mut self,
        (entities, mut dirty_entities, hierarchy, links, transforms, mut globals): Self::SystemData,
    ) {
        // Add GlobalTransforms to entities with Transforms
        Self::add_globals(&entities, &transforms, &mut globals, &mut dirty_entities);

        // Read events
        // Add new or modified entities to dirty bitset
        transforms
            .channel()
            .read(self.transform_reader_id.as_mut().unwrap())
            .for_each(|event| match *event {
                ComponentEvent::Removed(id) => {
                    let _ = globals.remove(entities.entity(id));
                }
                ComponentEvent::Inserted(id) | ComponentEvent::Modified(id) => {
                    dirty_entities.dirty.add(id);
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
                    dirty_entities.dirty.add(entity.id());
                }
            });

        // Children of dirty entities are also dirty and need their transforms updated
        (
            &entities,
            &transforms,
            &globals,
            &dirty_entities.dirty.clone(),
        )
            .join()
            .for_each(|(entity, _, _, _)| {
                let children = hierarchy.all_children(entity);
                dirty_entities.dirty |= &children;
            });

        // Sync all dirty entities and their children
        (&entities, &transforms, &mut globals, &dirty_entities.dirty)
            .join()
            .for_each(|(entity, transform, global, _)| {
                global.global = transform.clone();

                let mut parent_entity = entity;
                while let Some(link) = links.get(parent_entity) {
                    parent_entity = link.parent_entity();
                    if let Some(p_trans) = transforms.get(parent_entity) {
                        global.global += p_trans.clone();
                    }
                }
            });
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);

        // Register readers
        {
            let mut transforms = WriteStorage::<Transform>::fetch(res);
            let mut hierarchy = res.fetch_mut::<Hierarchy<Link>>();

            self.transform_reader_id = Some(transforms.register_reader());
            self.hierarchy_reader_id = Some(hierarchy.track());
        }

        // Add GlobalTransforms
        {
            let entities = Entities::fetch(res);
            let transforms = ReadStorage::<Transform>::fetch(res);
            let mut globals = WriteStorage::<GlobalTransform>::fetch(res);
            let mut dirty_entities = Write::<DirtyEntities>::fetch(res);
            Self::add_globals(&entities, &transforms, &mut globals, &mut dirty_entities);
        }
    }
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self {
            transform_reader_id: None,
            hierarchy_reader_id: None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        components::{GlobalTransform, Link, Transform},
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
        world.register::<GlobalTransform>();
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
            .read_storage::<GlobalTransform>()
            .get(e1)
            .unwrap()
            .to_matrix();

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
            .read_storage::<GlobalTransform>()
            .get(e1)
            .unwrap()
            .to_matrix();

        let abs_tra_e2 = world
            .read_storage::<GlobalTransform>()
            .get(e2)
            .unwrap()
            .to_matrix();

        let abs_tra = tra.to_matrix() * tra.to_matrix();

        // The absolute matricies should no longer be equal
        assert_ne!(abs_tra_e1, abs_tra_e2);
        // Actual result should be the same as simulated result
        assert_eq!(abs_tra_e1, abs_tra);
    }
}
