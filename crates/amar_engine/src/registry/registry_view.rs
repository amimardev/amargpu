use crate::{
    input_handler::InputHandler,
    registry::{EntityId, EventLoopRef, ResourceRegistry},
    state::GlobalState,
};
use std::any::TypeId;

/// View of `ResourceRegistry` handed to plugins. Wraps everything the
/// full registry offers, but forbids inserting/removing a fixed set of
/// "protected" resource types that only the engine itself is allowed
/// to manage.
pub struct PluginRegistryView<'a> {
    pub(super) registry: &'a mut ResourceRegistry,
}

/// Marks resource types plugins may never insert or remove.
/// (They can still *read* them via `get_res`/`get_res_mut` if you want —
/// see note below.)
fn is_protected(type_id: TypeId) -> bool {
    type_id == TypeId::of::<InputHandler>()
        || type_id == TypeId::of::<EventLoopRef>() // StaticRef<ActiveEventLoop>
        || type_id == TypeId::of::<GlobalState>()
}

impl<'a> PluginRegistryView<'a> {
    pub fn glb(&self) -> &GlobalState {
        self.registry.get_res::<GlobalState>().unwrap()
    }
    pub fn glb_mut(&mut self) -> &mut GlobalState {
        self.registry.get_res_mut::<GlobalState>().unwrap()
    }
    // --- entity CRUD: fully passed through, unrestricted ---
    pub fn spawn<T: 'static>(&mut self, label: Option<&str>, value: T) -> EntityId {
        self.registry.spawn(label, value)
    }
    pub fn get<T: 'static>(&self, id: EntityId) -> Option<&T> {
        self.registry.get(id)
    }
    pub fn get_mut<T: 'static>(&mut self, id: EntityId) -> Option<&mut T> {
        self.registry.get_mut(id)
    }
    pub fn get_by_label<T: 'static>(&self, label: &str) -> Vec<&T> {
        self.registry.get_by_label(label)
    }
    pub fn get_by_label_mut<T: 'static>(&mut self, label: &str) -> Vec<&mut T> {
        self.registry.get_by_label_mut(label)
    }
    pub fn despawn<T: 'static>(&mut self, id: EntityId) -> Option<T> {
        self.registry.despawn(id)
    }

    // --- resource reads: unrestricted, plugins can still *see* protected resources ---
    pub fn get_res<T: 'static>(&self) -> Option<&T> {
        self.registry.get_res::<T>()
    }
    pub fn get_res_mut<T: 'static>(&mut self) -> Option<&mut T> {
        // Note: if even *mutating in place* the protected resources should be
        // forbidden, gate this the same way as insert/remove below instead.
        self.registry.get_res_mut::<T>()
    }

    // --- resource writes: guarded ---
    pub fn insert_res<T: 'static>(&mut self, value: T) {
        if is_protected(TypeId::of::<T>()) {
            panic!(
                "plugin attempted to insert protected resource `{}`",
                std::any::type_name::<T>()
            );
        }
        self.registry.insert_res(value);
    }

    pub fn remove_res<T: 'static>(&mut self) -> Option<T> {
        if is_protected(TypeId::of::<T>()) {
            panic!(
                "plugin attempted to remove protected resource `{}`",
                std::any::type_name::<T>()
            );
        }
        self.registry.remove_res::<T>()
    }
}
