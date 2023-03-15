use std::collections::HashMap;

use hecs::{CommandBuffer, Component, Entity, EntityBuilder, World};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use rose_core::transform::Transform;

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct Parent(pub Entity);

pub trait Hierarchical: Component {
    type Global: Component;
    fn make_root(&self) -> Self::Global;
    fn make_child(&self, parent: &Self::Global) -> Self::Global;
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchicalSystem;

impl HierarchicalSystem {
    #[tracing::instrument(skip_all)]
    pub fn update<H: Hierarchical>(&self, world: &World, command_buffer: &mut CommandBuffer) {
        let curspan = tracing::span::Span::current();
        let mut q = world.query::<Option<&Parent>>().with::<&H>();
        let hierarchy = q
            .iter_batched(32)
            .par_bridge()
            .fold(HashMap::new, |mut map, batch| {
                for (entity, opt_parent) in batch {
                    map.insert(entity, opt_parent.map(|Parent(parent)| *parent));
                }
                map
            })
            .reduce_with(|mut a, b| {
                a.extend(b);
                a
            })
            .unwrap_or_default();
        let mut q = world.query::<&H>();
        let v = q.view();
        let mut built_globals = hierarchy
            .par_iter()
            .filter_map(|(e, opt)| if opt.is_none() { Some(*e) } else { None })
            .map(|e| {
                tracing::debug!(parent: &curspan, "Building {:?} root", e);
                (e, v.get(e).unwrap().make_root())
            })
            .collect::<HashMap<_, _>>();

        let parents = hierarchy
            .into_iter()
            .filter_map(|(e, opt)| opt.map(|parent| (e, parent)))
            .collect::<HashMap<_, _>>();

        // for (entity, parent) in parents_it {
        // }

        let mut insert_len = 1;
        while insert_len > 0 {
            let mut to_insert = HashMap::new();
            for (entity, parent) in parents
                .iter()
                .filter(|(e, p)| built_globals.contains_key(p) && !built_globals.contains_key(e))
                .map(|(e, p)| /* This should be a call to copied */ (*e, *p))
            {
                tracing::debug!(parent: &curspan, "Building {:?} <== {:?}", parent, entity);
                let parent_gen = &built_globals[&parent];
                let gen = v.get(entity).unwrap().make_child(parent_gen);
                to_insert.insert(entity, gen);
            }
            insert_len = to_insert.len();
            tracing::debug!("To insert: {}", insert_len);
            built_globals.extend(to_insert);
        }

        for (e, global) in built_globals {
            command_buffer.insert_one(e, global);
        }

        for (entity, parent) in world.query::<&Parent>().iter() {
            if world.entity(parent.0).is_err() {
                command_buffer.remove_one::<Parent>(entity);
            }
        }
    }
}

pub trait MakeChild {
    type Ret;
    fn spawn_child(&mut self, parent: Entity, child: &mut EntityBuilder) -> Self::Ret;
}

impl MakeChild for World {
    type Ret = Entity;
    fn spawn_child(&mut self, parent: Entity, child: &mut EntityBuilder) -> Entity {
        self.spawn(child.add(Parent(parent)).build())
    }
}

impl MakeChild for CommandBuffer {
    type Ret = ();

    fn spawn_child(&mut self, parent: Entity, child: &mut EntityBuilder) -> Self::Ret {
        self.spawn(child.add(Parent(parent)).build());
    }
}

pub trait MakeChildren {
    fn spawn_children<'e, I>(&mut self, parent: Entity, entities: I) -> Vec<Entity>
    where
        I: IntoIterator<Item = &'e mut EntityBuilder>,
        I::IntoIter: ExactSizeIterator;
}

impl MakeChildren for World {
    fn spawn_children<'e, I>(&mut self, parent: Entity, entities: I) -> Vec<Entity>
    where
        I: IntoIterator<Item = &'e mut EntityBuilder>,
        I::IntoIter: ExactSizeIterator,
    {
        let entities_it = entities.into_iter();
        let entities = self
            .reserve_entities(entities_it.len() as _)
            .collect::<Vec<_>>();
        for (entity, builder) in entities.iter().copied().zip(entities_it) {
            self.insert(entity, builder.add(Parent(parent)).build())
                .unwrap();
        }
        entities
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[repr(transparent)]
pub struct GlobalTransform(pub Transform);

impl From<GlobalTransform> for Transform {
    fn from(val: GlobalTransform) -> Self {
        val.0
    }
}

impl<'a> From<&'a GlobalTransform> for Transform {
    fn from(val: &'a GlobalTransform) -> Self {
        val.0
    }
}

impl Hierarchical for Transform {
    type Global = GlobalTransform;

    fn make_root(&self) -> Self::Global {
        GlobalTransform(*self)
    }

    fn make_child(&self, parent: &Self::Global) -> Self::Global {
        GlobalTransform(parent.0 * *self)
    }
}
