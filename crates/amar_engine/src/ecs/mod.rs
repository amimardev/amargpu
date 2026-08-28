mod archetypes;
mod error;

use crate::ecs::archetypes::{Archetype, ArchetypeComponents, ArchetypeId, Bundle, ComponentInfo};
use crate::ecs::error::ECSError;
use std::alloc::{Layout, alloc};
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::ptr::NonNull;

#[derive(Eq, Hash, PartialEq, Clone, Debug, Copy, Default)]
pub struct EntityId(u32);

impl EntityId {
    fn next(&mut self) -> Self {
        let old = *self;
        self.0 += 1;
        return old;
    }
}

pub type ArchetypeSet = HashSet<ArchetypeId>;

pub trait Resource: 'static {}
pub trait Component: 'static {}

#[derive(Debug, Clone)]
pub struct EntityInfo {
    archetype_id: ArchetypeId,
    row: u32,
}
pub struct ArchetypeRecord {
    column: u32,
}

#[derive(Default)]
pub struct World {
    // Find the archetype for an entity
    entity_index: HashMap<EntityId, EntityInfo>,
    resources: HashMap<TypeId, Box<dyn Resource>>,
    // List of archetype
    archetypes: HashMap<ArchetypeId, Archetype>,
    // Find an archetype by its list of component ids
    archetype_index: HashMap<ArchetypeComponents, ArchetypeId>,
    // Find the archetypes for a component
    // used in has_component()
    component_index: HashMap<TypeId, HashMap<ArchetypeId, ArchetypeRecord>>,
    current_arch_id: ArchetypeId,
    current_entity_id: EntityId,
}

impl World {
    fn spawn<B: Bundle>(&mut self, bundle: B) -> Result<(), ECSError> {
        let bundle_ids = B::component_ids();
        match self.archetype_index.get(bundle_ids) {
            Some(&archetype_id) => {
                let archetype = self.archetypes.get_mut(&archetype_id).unwrap();

                let entity_row = archetype.insert_bundle(bundle)?;
                self.entity_index.insert(
                    self.current_entity_id.next(),
                    EntityInfo {
                        archetype_id,
                        row: entity_row,
                    },
                );
            }
            None => {
                let bundle_infos = B::component_infos();
                let new_archetype_id = self.current_arch_id.next();

                // update self.archetype_index
                self.archetype_index
                    .insert(bundle_ids.clone(), new_archetype_id);

                // update self.components_index
                for (column_index, info) in bundle_infos.iter().enumerate() {
                    self.component_index
                        .entry(info.type_id())
                        .or_default()
                        .insert(
                            new_archetype_id,
                            ArchetypeRecord {
                                column: column_index as u32,
                            },
                        );
                }

                let mut new_archetype = Archetype::new(new_archetype_id, bundle_infos.clone());

                let entity_row = new_archetype.insert_bundle(bundle)?;
                self.entity_index.insert(
                    self.current_entity_id.next(),
                    EntityInfo {
                        archetype_id: new_archetype_id,
                        row: entity_row,
                    },
                );

                self.archetypes.insert(new_archetype.id, new_archetype);
            }
        }

        Ok(())
    }

    fn despawn(&mut self, entity_id: EntityId) -> Result<(), ECSError> {
        match self.entity_index.remove(&entity_id) {
            Some(entity_info) => {
                let Some(archetype) = self.archetypes.get_mut(&entity_info.archetype_id) else {
                    return Err(ECSError::ArchetypeNotFound(entity_info.archetype_id));
                };
                archetype.remove_row(entity_info.row)?;
                Ok(())
            }
            None => Err(ECSError::InvalidEntityId(entity_id)),
        }
    }

    fn has_component<Comp: Component>(&self, entity: EntityId) -> bool {
        self.entity_index
            .get(&entity)
            .zip(self.component_index.get(&TypeId::of::<Comp>()))
            .is_some_and(|(info, map)| map.contains_key(&info.archetype_id))
    }

    fn get_component<Comp: Component>(&self, entity: EntityId) -> Result<&Comp, ECSError> {
        let Some(entity_info) = self.entity_index.get(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        let Some(archetype) = self.archetypes.get(&entity_info.archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(entity_info.archetype_id));
        };
        archetype.get_component::<Comp>(entity_info.row)
    }

    fn get_component_mut<Comp: Component>(
        &mut self,
        entity: EntityId,
    ) -> Result<&mut Comp, ECSError> {
        let Some(entity_info) = self.entity_index.get(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        let Some(archetype) = self.archetypes.get_mut(&entity_info.archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(entity_info.archetype_id));
        };
        archetype.get_component_mut::<Comp>(entity_info.row)
    }

    pub fn add_to_archetype<Comp: Component>(
        &mut self,
        src_id: ArchetypeId,
    ) -> Result<ArchetypeId, ECSError> {
        let new_component_id = TypeId::of::<Comp>();

        let Some(src) = self.archetypes.get_mut(&src_id) else {
            return Err(ECSError::ArchetypeNotFound(src_id));
        };

        // check in cache
        if let Some(&arch_id) = src.add_archetypes.get(&new_component_id) {
            return Ok(arch_id);
        }
        let (new_archetype, insert_component_idx) =
            src.new_from_add(self.current_arch_id.next(), ComponentInfo::new::<Comp>())?;

        // region: update indexes
        self.archetype_index
            .insert(new_archetype.component_ids.clone(), new_archetype.id);

        // 1. Insert or update component_index entry for the new component
        self.component_index
            .entry(new_component_id)
            .or_default()
            .insert(
                new_archetype.id,
                ArchetypeRecord {
                    column: insert_component_idx,
                },
            );

        // 2. Increment column index for remaining components
        for &comp_id in &src.component_ids[insert_component_idx as usize + 1..] {
            if let Some(record) = self
                .component_index
                .get_mut(&comp_id)
                .and_then(|map| map.get_mut(&new_archetype.id))
            {
                record.column += 1;
            }
        }
        // endregion

        // update cache
        src.add_archetypes
            .insert(new_component_id, new_archetype.id);

        let new_archetype_id = new_archetype.id;
        self.archetypes.insert(new_archetype.id, new_archetype);

        Ok(new_archetype_id)
    }

    pub fn remove_from_archetype<Comp: Component>(
        &mut self,
        src_id: ArchetypeId,
    ) -> Result<ArchetypeId, ECSError> {
        let remove_component_id = TypeId::of::<Comp>();

        let Some(src) = self.archetypes.get_mut(&src_id) else {
            return Err(ECSError::ArchetypeNotFound(src_id));
        };

        // check in cache
        if let Some(&arch_id) = src.remove_archetypes.get(&remove_component_id) {
            return Ok(arch_id);
        }

        let (new_archetype, deleted_component_idx) =
            src.new_from_delete(self.current_arch_id.next(), ComponentInfo::new::<Comp>())?;

        // region: update indexes
        self.archetype_index
            .insert(new_archetype.component_ids.clone(), new_archetype.id);

        // 3. Increment column index for remaining components
        for &comp_id in &src.component_ids[deleted_component_idx as usize..] {
            if let Some(record) = self
                .component_index
                .get_mut(&comp_id)
                .and_then(|map| map.get_mut(&new_archetype.id))
            {
                record.column -= 1;
            }
        }
        // endregion

        // update cache
        src.remove_archetypes
            .insert(remove_component_id, new_archetype.id);

        let new_archetype_id = new_archetype.id;
        self.archetypes.insert(new_archetype.id, new_archetype);

        Ok(new_archetype_id)
    }

    /// Inserts or overwrites a resource of type `R`.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// Checks if a resource of type `R` exists in the world.
    pub fn has_resource<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }

    /// Returns an immutable reference to the resource of type `R` if present.
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .and_then(|boxed| (boxed as &dyn Any).downcast_ref::<R>())
    }

    /// Returns a mutable reference to the resource of type `R` if present.
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|boxed| (boxed as &mut dyn Any).downcast_mut::<R>())
    }

    pub fn add_component<Comp: Component>(
        &mut self,
        entity: EntityId,
        mut component: Comp,
    ) -> Result<(), ECSError> {
        let Some(entity_info_copy) = self.entity_index.get_mut(&entity).cloned() else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        let new_archetype_id = self.add_to_archetype::<Comp>(entity_info_copy.archetype_id)?;

        let Some(old_archetype) = self.archetypes.get_mut(&entity_info_copy.archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let mut old_entity_data = old_archetype.remove_row_ptr(entity_info_copy.row)?;

        let new_component_index = old_entity_data
            .binary_search_by_key(&TypeId::of::<Comp>(), |e| e.1.type_id())
            .err()
            .unwrap();

        let component_ptr = NonNull::new(&mut component as *mut Comp as *mut u8).unwrap();
        old_entity_data.insert(
            new_component_index,
            (component_ptr, ComponentInfo::new::<Comp>()),
        );

        let Some(new_archetype) = self.archetypes.get_mut(&new_archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let Some(entity_info_ref) = self.entity_index.get_mut(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        entity_info_ref.row = new_archetype.insert_row_ptr(&old_entity_data)?;
        entity_info_ref.archetype_id = new_archetype_id;

        // free all pointer copies
        for (index, (ptr, info)) in old_entity_data.iter().enumerate() {
            if index == new_component_index {
                continue;
            }
            let size = info.size() as usize;
            if size > 0 {
                let layout =
                    std::alloc::Layout::from_size_align(size, info.align() as usize).unwrap();
                unsafe {
                    std::alloc::dealloc(ptr.as_ptr(), layout);
                }
            }
        }

        // component was freed with the pointers, so to avoid double free we do this
        std::mem::forget(component);

        Ok(())
    }
    pub fn remove_component<Comp: Component>(&mut self, entity: EntityId) -> Result<(), ECSError> {
        let Some(entity_info_copy) = self.entity_index.get_mut(&entity).cloned() else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        let new_archetype_id = self.remove_from_archetype::<Comp>(entity_info_copy.archetype_id)?;

        let Some(old_archetype) = self.archetypes.get_mut(&entity_info_copy.archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let mut old_entity_data = old_archetype.remove_row_ptr(entity_info_copy.row)?;

        let remove_component_index = old_entity_data
            .binary_search_by_key(&TypeId::of::<Comp>(), |e| e.1.type_id())
            .ok()
            .unwrap();

        let (to_free_component_ptr, to_free_component_info) =
            old_entity_data.remove(remove_component_index);

        let Some(new_archetype) = self.archetypes.get_mut(&new_archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let Some(entity_info_ref) = self.entity_index.get_mut(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        entity_info_ref.row = new_archetype.insert_row_ptr(&old_entity_data)?;
        entity_info_ref.archetype_id = new_archetype_id;

        // free all pointer copies
        for (ptr, info) in old_entity_data.iter() {
            let size = info.size() as usize;
            if size > 0 {
                let layout =
                    std::alloc::Layout::from_size_align(size, info.align() as usize).unwrap();
                unsafe {
                    std::alloc::dealloc(ptr.as_ptr(), layout);
                }
            }
        }

        // freeying component
        let size = to_free_component_info.size() as usize;
        if size != 0 {
            let layout =
                std::alloc::Layout::from_size_align(size, to_free_component_info.align() as usize)
                    .unwrap();
            unsafe {
                std::alloc::dealloc(to_free_component_ptr.as_ptr(), layout);
            }
        }

        Ok(())
    }
}
