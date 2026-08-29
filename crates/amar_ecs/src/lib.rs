mod archetypes;
mod error;
mod types;
mod query;


use crate::archetypes::{Archetype, Bundle};
use crate::error::ECSError;
use crate::types::{
    ArchetypeComponents, ArchetypeId, ArchetypeRecord, Component, ComponentInfo, EntityId,
    EntityInfo, RawComponent, Resource,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct World {
    // Find the archetype for an entity
    entity_index: HashMap<EntityId, EntityInfo>,
    resources: HashMap<TypeId, Box<dyn Any>>,
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
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Result<EntityId, ECSError> {
        let new_entity_id: EntityId;
        let bundle_ids = B::component_ids();
        match self.archetype_index.get(bundle_ids) {
            Some(&archetype_id) => {
                let archetype = self.archetypes.get_mut(&archetype_id).unwrap();

                let entity_row = archetype.insert_bundle(bundle)?;
                new_entity_id = self.current_entity_id.next();
                self.entity_index.insert(
                    new_entity_id,
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
                new_entity_id = self.current_entity_id.next();
                self.entity_index.insert(
                    new_entity_id,
                    EntityInfo {
                        archetype_id: new_archetype_id,
                        row: entity_row,
                    },
                );

                self.archetypes.insert(new_archetype.id, new_archetype);
            }
        }

        Ok(new_entity_id)
    }

    pub fn despawn(&mut self, entity_id: EntityId) -> Result<(), ECSError> {
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

    pub fn has_component<Comp: Component>(&self, entity: EntityId) -> bool {
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

    pub fn get_component_mut<Comp: Component>(
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

    fn add_to_archetype<Comp: Component>(
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
        let (new_archetype, _insert_component_idx) =
            src.new_from_add(self.current_arch_id.next(), ComponentInfo::new::<Comp>())?;

        // region: update indexes
        self.archetype_index
            .insert(new_archetype.component_ids.clone(), new_archetype.id);

        // Register every component in the new archetype with its new column.
        for (column, &component_id) in new_archetype.component_ids.iter().enumerate() {
            self.component_index
                .entry(component_id)
                .or_default()
                .insert(
                    new_archetype.id,
                    ArchetypeRecord {
                        column: column as u32,
                    },
                );
        }
        // endregion

        // update cache
        src.add_archetypes
            .insert(new_component_id, new_archetype.id);

        let new_archetype_id = new_archetype.id;
        self.archetypes.insert(new_archetype.id, new_archetype);

        Ok(new_archetype_id)
    }

    fn remove_from_archetype<Comp: Component>(
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

        let (new_archetype, _deleted_component_idx) =
            src.new_from_delete(self.current_arch_id.next(), ComponentInfo::new::<Comp>())?;

        // region: update indexes
        self.archetype_index
            .insert(new_archetype.component_ids.clone(), new_archetype.id);

        // Register every component remaining in the new archetype with its column.
        for (column, &component_id) in new_archetype.component_ids.iter().enumerate() {
            self.component_index
                .entry(component_id)
                .or_default()
                .insert(
                    new_archetype.id,
                    ArchetypeRecord {
                        column: column as u32,
                    },
                );
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
            .and_then(|boxed| boxed.downcast_ref::<R>())
    }

    /// Returns a mutable reference to the resource of type `R` if present.
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|boxed| boxed.downcast_mut::<R>())
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

        let mut old_entity_data = old_archetype.remove_raw_component(entity_info_copy.row)?;

        let new_component_index = old_entity_data
            .binary_search_by_key(&TypeId::of::<Comp>(), |e| e.info.type_id())
            .err()
            .unwrap();

        old_entity_data.insert(new_component_index, RawComponent::new(component));

        let Some(new_archetype) = self.archetypes.get_mut(&new_archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let Some(entity_info_ref) = self.entity_index.get_mut(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        entity_info_ref.row = new_archetype.insert_row_ptr(old_entity_data)?;
        entity_info_ref.archetype_id = new_archetype_id;

        Ok(())
    }
    pub fn remove_component<Comp: Component>(
        &mut self,
        entity: EntityId,
    ) -> Result<Comp, ECSError> {
        let Some(entity_info_copy) = self.entity_index.get_mut(&entity).cloned() else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        let new_archetype_id = self.remove_from_archetype::<Comp>(entity_info_copy.archetype_id)?;

        let Some(old_archetype) = self.archetypes.get_mut(&entity_info_copy.archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let mut old_entity_data = old_archetype.remove_raw_component(entity_info_copy.row)?;

        let remove_component_index = old_entity_data
            .binary_search_by_key(&TypeId::of::<Comp>(), |e| e.info.type_id())
            .ok()
            .unwrap();

        let raw_component = old_entity_data.remove(remove_component_index);

        let Some(new_archetype) = self.archetypes.get_mut(&new_archetype_id) else {
            return Err(ECSError::ArchetypeNotFound(new_archetype_id));
        };

        let Some(entity_info_ref) = self.entity_index.get_mut(&entity) else {
            return Err(ECSError::InvalidEntityId(entity));
        };
        entity_info_ref.row = new_archetype.insert_row_ptr(old_entity_data)?;
        entity_info_ref.archetype_id = new_archetype_id;
        Ok(raw_component.into_cast())
    }
}

#[cfg(test)]
mod tests {
    use super::{Component, ECSError, EntityId, Resource, World};

    #[derive(Debug, PartialEq)]
    struct Position(i32, i32);
    impl Component for Position {}

    #[derive(Debug, PartialEq)]
    struct Health(u32);
    impl Component for Health {}

    struct Score(u32);
    impl Resource for Score {}

    struct Missing;
    impl Component for Missing {}

    fn entity_at(index: u32) -> EntityId {
        let mut entity = EntityId::default();
        for _ in 0..index {
            entity.next();
        }
        entity
    }

    #[test]
    fn spawn_read_and_mutate_components() {
        let mut world = World::default();
        let entity = world.spawn((Position(1, 2), Health(100))).unwrap();

        assert_eq!(
            world.get_component::<Position>(entity).unwrap(),
            &Position(1, 2)
        );
        assert_eq!(world.get_component::<Health>(entity).unwrap(), &Health(100));
        assert!(world.has_component::<Position>(entity));
        assert!(!world.has_component::<Missing>(entity));

        world.get_component_mut::<Position>(entity).unwrap().0 = 9;
        assert_eq!(
            world.get_component::<Position>(entity).unwrap(),
            &Position(9, 2)
        );
    }

    #[test]
    fn adding_and_removing_component_preserves_existing_data() {
        let mut world = World::default();

        let entity = world.spawn((Position(3, 4),)).unwrap();
        world.add_component(entity, Health(75)).unwrap();
        assert_eq!(
            world.get_component::<Position>(entity).unwrap(),
            &Position(3, 4)
        );
        assert_eq!(world.get_component::<Health>(entity).unwrap(), &Health(75));

        let removed = world.remove_component::<Health>(entity).unwrap();
        assert_eq!(removed, Health(75));
        assert!(world.has_component::<Position>(entity));
        assert!(!world.has_component::<Health>(entity));
    }

    #[test]
    fn despawn_rejects_entity_and_reuses_storage_without_aliasing() {
        let mut world = World::default();

        let first = world.spawn((Position(1, 1),)).unwrap();
        let second = world.spawn((Position(2, 2),)).unwrap();

        world.despawn(first).unwrap();
        assert!(!world.has_component::<Position>(first));
        assert_eq!(
            world.get_component::<Position>(second).unwrap(),
            &Position(2, 2)
        );
        assert!(matches!(
            world.despawn(first),
            Err(ECSError::InvalidEntityId(_))
        ));

        world.spawn((Position(3, 3),)).unwrap();
        let reused_storage_entity = entity_at(2);
        assert_eq!(
            world
                .get_component::<Position>(reused_storage_entity)
                .unwrap(),
            &Position(3, 3)
        );
        assert_eq!(
            world.get_component::<Position>(second).unwrap(),
            &Position(2, 2)
        );
    }

    #[test]
    fn resources_can_be_inserted_overwritten_and_mutated() {
        let mut world = World::default();
        assert!(!world.has_resource::<Score>());
        assert!(world.get_resource::<Score>().is_none());

        world.insert_resource(Score(10));
        assert!(world.has_resource::<Score>());
        assert_eq!(world.get_resource::<Score>().unwrap().0, 10);

        world.get_resource_mut::<Score>().unwrap().0 = 20;
        world.insert_resource(Score(30));
        assert_eq!(world.get_resource::<Score>().unwrap().0, 30);
    }
}
