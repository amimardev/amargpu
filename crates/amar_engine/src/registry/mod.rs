mod fetch_macros;
mod label_index;
pub mod registry_view;
pub mod static_ptr;
pub mod sys_runner;

use winit::event_loop::ActiveEventLoop;
 
use crate::registry::label_index::LabelIndex;
use crate::registry::registry_view::PluginRegistryView;
use crate::registry::static_ptr::StaticRef;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(u64);

#[derive(Default)]
pub struct EntityIdAllocator(u64);

impl EntityIdAllocator {
    pub fn next(&mut self) -> EntityId {
        let id = EntityId(self.0);
        self.0 += 1;
        id
    }
}
pub type EventLoopRef = StaticRef<ActiveEventLoop>;
#[derive(Default)]
pub struct ResourceRegistry {
    stores: HashMap<TypeId, Box<dyn Any>>, // TypeId -> HashMap<EntityId, T>
    resources: HashMap<TypeId, Box<dyn Any>>, // unchan
    ids: EntityIdAllocator,
    labels: LabelIndex,
}

impl ResourceRegistry {
    pub fn view(&mut self) -> PluginRegistryView<'_> {
        PluginRegistryView { registry: self }
    }
    // region: CRUD entities
    pub fn spawn<T: 'static>(&mut self, label: Option<&str>, value: T) -> EntityId {
        let id = self.ids.next();
        self.stores
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<EntityId, T>::new()))
            .downcast_mut::<HashMap<EntityId, T>>()
            .unwrap()
            .insert(id, value);

        if let Some(label) = label {
            self.labels.insert::<T>(label, id);
        }
        id
    }

    pub fn get<T: 'static>(&self, id: EntityId) -> Option<&T> {
        self.stores
            .get(&TypeId::of::<T>())?
            .downcast_ref::<HashMap<EntityId, T>>()?
            .get(&id)
    }

    pub fn get_by_label<T: 'static>(&self, label: &str) -> Vec<&T> {
        let ids = self.labels.get::<T>(label);
        let Some(map) = self
            .stores
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<HashMap<EntityId, T>>())
        else {
            return Vec::new();
        };
        ids.iter().filter_map(|id| map.get(id)).collect()
    }

    pub fn get_by_label_mut<T: 'static>(&mut self, label: &str) -> Vec<&mut T> {
        let ids = self.labels.get::<T>(label).to_vec(); // clone ids out first — see note below
        let Some(map) = self
            .stores
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<HashMap<EntityId, T>>())
        else {
            return Vec::new();
        };
        // can't use filter_map here safely for multiple &mut — see note below
        ids.into_iter()
            .filter_map(|id| {
                // SAFETY: each id is distinct (Vec from LabelIndex never has duplicates
                // if insert() is only ever called once per (T, id) pair), so these
                // &mut borrows don't alias.
                map.get_mut(&id).map(|v| unsafe { &mut *(v as *mut T) })
            })
            .collect()
    }
    pub fn get_mut<T: 'static>(&mut self, id: EntityId) -> Option<&mut T> {
        self.stores
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<EntityId, T>>()?
            .get_mut(&id)
    }

    pub fn despawn<T: 'static>(&mut self, id: EntityId) -> Option<T> {
        let value = self
            .stores
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<EntityId, T>>()?
            .remove(&id)?;
        self.labels.remove(id);
        Some(value)
    }

    // endregion
    pub(super) fn remove_map<T: 'static>(&mut self) -> Option<HashMap<EntityId, T>> {
        Some(
            *self
                .stores
                .remove(&TypeId::of::<T>())?
                .downcast::<HashMap<EntityId, T>>()
                .unwrap(),
        )
    }
    pub(super) fn insert_map<T: 'static>(&mut self, map: HashMap<EntityId, T>) {
        self.stores.insert(TypeId::of::<T>(), Box::new(map));
    }
    // region: CRUD resources
    pub fn insert_res<T: 'static>(&mut self, value: T) {
        self.resources
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(value));
    }

    pub fn get_res<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_res_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }

    pub fn remove_res<T: 'static>(&mut self) -> Option<T> {
        let boxed = self.resources.remove(&TypeId::of::<T>())?;
        boxed.downcast::<T>().ok().map(|b| *b)
    }

    // endregion
}
