use std::{any::TypeId, collections::HashMap};

use crate::registry::EntityId;

#[derive(Default)]
pub struct LabelIndex {
    // (TypeId, label) -> set of EntityIds with that label
    by_label: HashMap<(TypeId, String), Vec<EntityId>>,
    // reverse lookup: EntityId -> (TypeId, label), for cleanup on despawn
    by_id: HashMap<EntityId, (TypeId, String)>,
}

impl LabelIndex {
    pub(super) fn insert<T: 'static>(&mut self, label: impl Into<String>, id: EntityId) {
        let label = label.into();
        self.by_label
            .entry((TypeId::of::<T>(), label.clone()))
            .or_default()
            .push(id);
        self.by_id.insert(id, (TypeId::of::<T>(), label));
    }
    pub(super) fn get<T: 'static>(&self, label: &str) -> &[EntityId] {
        self.by_label
            .get(&(TypeId::of::<T>(), label.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
    pub(super) fn remove(&mut self, id: EntityId) {
        if let Some((type_id, label)) = self.by_id.remove(&id) {
            if let Some(ids) = self.by_label.get_mut(&(type_id, label.clone())) {
                ids.retain(|&x| x != id);
                if ids.is_empty() {
                    self.by_label.remove(&(type_id, label));
                }
            }
        }
    }
}
