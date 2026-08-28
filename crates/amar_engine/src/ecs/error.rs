use core::fmt;
use std::any::TypeId;

use crate::ecs::types::{ArchetypeId, EntityId};
 

#[derive(Debug)]
pub enum ECSError {
    ArchetypeNotFound(ArchetypeId),
    ComponentExistInArchetype(TypeId, ArchetypeId),
    ComponentAbscentFromArchetype(TypeId, ArchetypeId),
    /// (_,column_idx,column_type_id, component_type_id)
    ColumnComponentTypeMismatch(ArchetypeId, u32, TypeId, TypeId),
    /// (_, bundle_size, archetype_column_count)
    BundleArchetypeSizeMismatch(ArchetypeId, u32, u32),
    InvalidEntityId(EntityId),
    /// (_,column_idx, row)
    ColumnRowNotExistant(ArchetypeId, u32, u32),
    ArchetypeRowNotExistant(ArchetypeId, u32),
    ComponentNotYetRegistered(TypeId),
}

// used internaly in Column methods because they don't have access to archetype_id
pub(super) enum ColumnError {
    // ( column_internal_type_id , component_type_id )
    ColumnComponentTypeMismatch(TypeId, TypeId),
    ColumnRowNotExistant(u32),
    ColumnGivenRowOutOfBounds(u32),
}

// 1. User-facing error message string formatting
impl fmt::Display for ECSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchetypeNotFound(archetype_id) => {
                write!(f, "Archetype {archetype_id:?} not found")
            }
            Self::ComponentExistInArchetype(component_id, archetype_id) => write!(
                f,
                "Component {component_id:?} already exist in archetype {archetype_id:?}"
            ),
            Self::ComponentAbscentFromArchetype(component_id, archetype_id) => write!(
                f,
                "Component {component_id:?} is already abscent from archetype {archetype_id:?}"
            ),
            Self::ColumnComponentTypeMismatch(
                archetype_id,
                column_idx,
                column_element_type_id,
                component_id,
            ) => write!(
                f,
                "type_id {column_element_type_id:?} of column {column_idx:?} of archetype {archetype_id:?} isn't compatible with component_id {component_id:?}"
            ),
            Self::BundleArchetypeSizeMismatch(archetype_id, archetype_length, bundle_size) => {
                write!(
                    f,
                    "archetype {archetype_id:?} number of columns {archetype_length} doesn't match bundle size {bundle_size}"
                )
            }
            Self::InvalidEntityId(entity_id) => {
                write!(f, "invalid entity {entity_id:?}")
            }
            Self::ColumnRowNotExistant(archetype_id, column_idx, row) => {
                write!(
                    f,
                    " row {row} is invalid in column {column_idx} of archetype : {archetype_id:?}"
                )
            }
            Self::ArchetypeRowNotExistant(archetype_id, row) => {
                write!(f, " row {row} is invalid in archetype : {archetype_id:?}")
            }
            Self::ComponentNotYetRegistered(component_id) => {
                write!(f, "component {component_id:?} not yet registered")
            }
        }
    }
}
