use getset::CopyGetters;
use getset::Getters;

use crate::ecs::Component;
use crate::ecs::ECSError;
use crate::ecs::error::ColumnError;
use std::alloc::Layout;
use std::alloc::alloc;
use std::alloc::realloc;
use std::any::TypeId;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::OnceLock;
pub trait Bundle: BundleInternal {
    fn component_infos() -> &'static Vec<ComponentInfo>;
    fn component_ids() -> &'static Vec<TypeId>;
}

pub(crate) trait BundleInternal {
    // Write components into your archetype arrays / registers
    // returns entity_row if successful
    fn put_components(self, storage: &mut Archetype, row: u32) -> Result<(), ECSError>;
}

macro_rules! impl_bundle {
    ($($T:ident),+) => {
        impl<$($T: Component),+> BundleInternal for ($($T,)+) {
            fn put_components(self, storage: &mut Archetype,row : u32) -> Result<(),ECSError>{
                const BUNDLE_LEN: u32 = [$(stringify!($T)),+].len() as u32;
                if storage.component_ids.len() as u32 != BUNDLE_LEN {
                    return Err(ECSError::BundleArchetypeSizeMismatch(storage.id, storage.length,BUNDLE_LEN));
                }
                // the check for types is done by columns internaly.

                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                $(
                    storage.insert_component($T,row)?;

                )+
                Ok(())
            }
        }


        impl<$($T: Component),+> Bundle for ($($T,)+) {
            fn component_infos() -> &'static Vec<ComponentInfo> {

                // avoid sorting everytime
                // OneLock is needed for static because Sync is required on every static variable,
                // even when not using multi-threading
                static INFOS: OnceLock<Vec<ComponentInfo>> = OnceLock::new();
                INFOS.get_or_init(|| {
                    let mut infos = vec![$(ComponentInfo::new::<$T>()),+];
                    infos.sort_unstable_by_key(|info| info.type_id);
                    infos
                })

            }
            fn component_ids() -> &'static Vec<TypeId> {
                static IDS: OnceLock<Vec<TypeId>> = OnceLock::new();
                IDS.get_or_init(|| {
                    let mut types = vec![$(TypeId::of::<$T>()),+];
                    types.sort_unstable();
                    types
                })
            }
        }
    };
}
impl BundleInternal for () {
    fn put_components(self, storage: &mut Archetype, _: u32) -> Result<(), ECSError> {
        const BUNDLE_LEN: u32 = 0;
        if storage.component_ids.len() as u32 != BUNDLE_LEN {
            return Err(ECSError::BundleArchetypeSizeMismatch(
                storage.id,
                storage.length,
                BUNDLE_LEN,
            ));
        }
        Ok(())
    }
}

// 0-element tuple
impl Bundle for () {
    fn component_infos() -> &'static Vec<ComponentInfo> {
        static EMPTY: Vec<ComponentInfo> = Vec::new();
        &EMPTY
    }

    fn component_ids() -> &'static Vec<TypeId> {
        static EMPTY: Vec<TypeId> = Vec::new();
        &EMPTY
    }
}

// Explicit calls for each tuple size
impl_bundle!(T1);
impl_bundle!(T1, T2);
impl_bundle!(T1, T2, T3);
impl_bundle!(T1, T2, T3, T4);
impl_bundle!(T1, T2, T3, T4, T5);
impl_bundle!(T1, T2, T3, T4, T5, T6);

/// we can use Vec<TypeId> as index : \[Position] != \[Position,Health]
/// order matters in comparaison, so to eliminate order difference, we must keep the vec ordered by TypeId
pub type ArchetypeComponents = Vec<TypeId>;

#[derive(Debug, Clone, Copy,CopyGetters)]
pub struct ComponentInfo {
    #[getset(get_copy = "pub")]
    type_id: TypeId,
    #[getset(get_copy = "pub")]
    size: u32,
    /// smallest power of 2 bigger then self.size  
    /// used for column allocation
    align: u32,
    drop_fn: unsafe fn(*mut u8),
}
impl ComponentInfo {
    pub fn new<Comp: Component>() -> Self {
        Self {
            type_id: TypeId::of::<Comp>(),
            size: size_of::<Comp>() as u32,
            align: align_of::<Comp>() as u32,
            drop_fn: Self::drop_component::<Comp>,
        }
    } 
    unsafe fn drop_component<T>(ptr: *mut u8) {
        unsafe { std::ptr::drop_in_place(ptr.cast::<T>()) };
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug, Default)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    pub fn next(&mut self) -> Self {
        let old = *self;
        self.0 += 1;
        return old;
    }
}

pub struct Column {
    pub elements: NonNull<u8>, // Pointer to start of heap-allocated byte buffer
    element_info: ComponentInfo,
    pub count: u32,
    capacity: u32,
}

impl Column {
    pub fn new(element_info: ComponentInfo) -> Self {
        let (elements, capacity) = if element_info.size == 0 {
            (NonNull::dangling(), 0)
        } else {
            let layout =
                Layout::from_size_align(element_info.size as usize, element_info.align as usize)
                    .unwrap();
            let ptr = unsafe { alloc(layout) };

            (NonNull::new(ptr).expect("allocation failed"), 1)
        };

        Self {
            elements,
            element_info,
            count: 0,
            capacity,
        }
    }

    pub fn get<Comp>(&self, row: u32) -> Option<&Comp> {
        if row >= self.count {
            return None;
        }
        unsafe {
            Some(
                self.elements
                    .add((row * self.element_info.size) as usize)
                    .cast()
                    .as_ref(),
            )
        }
    }
    pub fn get_mut<Comp>(&mut self, row: u32) -> Option<&mut Comp> {
        if row >= self.count {
            return None;
        }
        unsafe {
            Some(
                self.elements
                    .add((row * self.element_info.size) as usize)
                    .cast()
                    .as_mut(),
            )
        }
    }

    pub fn get_ptr(&self, row: u32) -> Result<NonNull<u8>, ColumnError> {
        if row >= self.count {
            return Err(ColumnError::ColumnRowNotExistant(row));
        }
        unsafe { Ok(self.elements.add((row * self.element_info.size) as usize)) }
    }

    /// Inserts a component at the given row index or appends it to the end if `row` is `None`.
    pub fn insert<Comp: Component>(
        &mut self,
        component: Comp,
        row: u32,
    ) -> Result<(), ColumnError> {
        let component_id = TypeId::of::<Comp>();
        if self.element_info.type_id != component_id {
            return Err(ColumnError::ColumnComponentTypeMismatch(
                self.element_info.type_id,
                component_id,
            ));
        }
        if row == self.count {
            self.count += 1;
        }
        if row > self.count {
            return Err(ColumnError::ColumnGivenRowOutOfBounds(row));
        }
        if self.count >= self.capacity {
            self.grow();
        }

        let offset = (self.element_info.size * row) as usize;
        unsafe {
            let ptr = self.elements.add(offset).cast::<Comp>();

            // will have no memory effect if COMP is a ZST
            ptr.write(component);
        }

        Ok(())
    }

    /// executes the drop function on the component at the given row index.
    pub fn delete(&mut self, component_id: TypeId, row: u32) -> Result<(), ColumnError> {
        if self.element_info.type_id != component_id {
            return Err(ColumnError::ColumnComponentTypeMismatch(
                self.element_info.type_id,
                component_id,
            ));
        }

        if row >= self.count {
            return Err(ColumnError::ColumnGivenRowOutOfBounds(row));
        }

        let offset = (self.element_info.size * row) as usize;
        unsafe {
            let ptr = self.elements.add(offset).as_ptr();
            (self.element_info.drop_fn)(ptr);
        }

        Ok(())
    }

    /// checks if type and row are valid in this column
    pub fn is_valid(&mut self, component_id: TypeId, row: u32) -> Result<(), ColumnError> {
        if self.element_info.type_id != component_id {
            return Err(ColumnError::ColumnComponentTypeMismatch(
                self.element_info.type_id,
                component_id,
            ));
        }

        if self.count <= row {
            return Err(ColumnError::ColumnRowNotExistant(row));
        }
        Ok(())
    }

    fn grow(&mut self) {
        let new_capacity = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };

        let old_size = self.element_info.size * self.capacity as u32;
        let new_size = self.element_info.size * new_capacity as u32;
        if self.element_info.size > 0 {
            let ptr = unsafe {
                if self.capacity == 0 {
                    let layout = Layout::from_size_align(
                        new_size as usize,
                        self.element_info.align as usize,
                    )
                    .unwrap();

                    alloc(layout)
                } else {
                    let layout = Layout::from_size_align(
                        old_size as usize,
                        self.element_info.align as usize,
                    )
                    .unwrap();

                    realloc(self.elements.as_ptr(), layout, new_size as usize)
                }
            };

            self.elements = NonNull::new(ptr).expect("allocation failed");
        }

        self.capacity = new_capacity;
    }
}

/// the table containing the data
pub struct Archetype {
    pub id: ArchetypeId,
    length: u32,
    pub component_ids: ArchetypeComponents,
    columns: Vec<Column>,
    pub add_archetypes: HashMap<TypeId, ArchetypeId>,
    pub remove_archetypes: HashMap<TypeId, ArchetypeId>,
    /// should be manualy set and managed by world,
    /// because archetype can only insert one component at a time
    /// we can make each column have auto managed list, but would be N times more cost (N number of columns in that archetype)
    entity_remove_list: Vec<u32>,
    /// includes deleted
    current_entity_count: u32,
}

impl Archetype {
    pub(super) fn new(id: ArchetypeId, mut component_infos: Vec<ComponentInfo>) -> Self {
        component_infos.sort_by_key(|c_f| c_f.type_id);
        return Self {
            id,
            length: component_infos.len() as u32,
            component_ids: component_infos.iter().map(|c_f| c_f.type_id).collect(),
            columns: component_infos
                .into_iter()
                .map(|c_f| Column::new(c_f))
                .collect(),
            add_archetypes: HashMap::new(),
            remove_archetypes: HashMap::new(),
            entity_remove_list: Vec::new(),

            current_entity_count: 0,
        };
    }
    pub fn get_component<Comp: Component>(&self, row: u32) -> Result<&Comp, ECSError> {
        if row >= self.current_entity_count || self.entity_remove_list.contains(&row) {
            return Err(ECSError::ArchetypeRowNotExistant(self.id, row));
        }

        let component_id = TypeId::of::<Comp>();
        match self.component_ids.binary_search(&component_id) {
            Ok(column_idx) => {
                // if component_id exists in archetype then tis column is garanteed to exist
                let column = self.columns.get(column_idx).unwrap();
                match column.get::<Comp>(row) {
                    Some(c) => Ok(c),
                    None => Err(ECSError::ColumnRowNotExistant(
                        self.id,
                        column_idx as u32,
                        row,
                    )),
                }
            }
            Err(_) => Err(ECSError::ComponentAbscentFromArchetype(
                component_id,
                self.id,
            )),
        }
    }

    pub fn get_component_mut<Comp: Component>(&mut self, row: u32) -> Result<&mut Comp, ECSError> {
        if row >= self.current_entity_count || self.entity_remove_list.contains(&row) {
            return Err(ECSError::ArchetypeRowNotExistant(self.id, row));
        }

        let component_id = TypeId::of::<Comp>();
        match self.component_ids.binary_search(&component_id) {
            Ok(column_idx) => {
                // if component_id exists in archetype then tis column is garanteed to exist
                let column = self.columns.get_mut(column_idx).unwrap();
                match column.get_mut::<Comp>(row) {
                    Some(c) => Ok(c),
                    None => Err(ECSError::ColumnRowNotExistant(
                        self.id,
                        column_idx as u32,
                        row,
                    )),
                }
            }
            Err(_) => Err(ECSError::ComponentAbscentFromArchetype(
                component_id,
                self.id,
            )),
        }
    }

    /// return new archetype and insert_column_index
    pub fn new_from_add(
        &self,
        id: ArchetypeId,
        component_info: ComponentInfo,
    ) -> Result<(Self, u32), ECSError> {
        let Err(insert_component_idx) = self.component_ids.binary_search(&component_info.type_id)
        else {
            return Err(ECSError::ComponentExistInArchetype(
                component_info.type_id,
                self.id,
            ));
        };

        let mut new_component_ids = self.component_ids.clone();
        new_component_ids.insert(insert_component_idx, component_info.type_id);

        let mut columns: Vec<Column> = self
            .columns
            .iter()
            .clone()
            .map(|c| Column::new(c.element_info))
            .collect();

        columns.insert(insert_component_idx, Column::new(component_info));

        Ok((
            Archetype {
                id,
                length: new_component_ids.len() as u32,
                component_ids: new_component_ids,
                columns,
                add_archetypes: HashMap::default(),
                remove_archetypes: HashMap::from([(component_info.type_id, self.id)]),
                entity_remove_list: Vec::new(),
                current_entity_count: 0,
            },
            insert_component_idx as u32,
        ))
    }

    /// return new archetype and deleted_column_index
    pub fn new_from_delete(
        &self,
        id: ArchetypeId,
        component_info: ComponentInfo,
    ) -> Result<(Self, u32), ECSError> {
        let Ok(delete_component_idx) = self.component_ids.binary_search(&component_info.type_id)
        else {
            return Err(ECSError::ComponentAbscentFromArchetype(
                component_info.type_id,
                self.id,
            ));
        };

        let mut new_component_ids = self.component_ids.clone();
        new_component_ids.remove(delete_component_idx);

        let mut columns: Vec<Column> = self
            .columns
            .iter()
            .clone()
            .map(|c| Column::new(c.element_info))
            .collect();
        columns.remove(delete_component_idx);

        Ok((
            Archetype {
                id,
                length: new_component_ids.len() as u32,
                component_ids: new_component_ids,
                columns,
                add_archetypes: HashMap::from([(component_info.type_id, self.id)]),
                remove_archetypes: HashMap::default(),
                entity_remove_list: Vec::new(),
                current_entity_count: 0,
            },
            delete_component_idx as u32,
        ))
    }

    /// row is managed by world,
    /// if entity_delete_list is empty then row must be provided as None,
    /// if not then pop and give the given value to row
    fn insert_component<Comp: Component>(&mut self, value: Comp, row: u32) -> Result<(), ECSError> {
        let component_id = TypeId::of::<Comp>();
        let Ok(column_index) = self.component_ids.binary_search(&component_id) else {
            return Err(ECSError::ComponentAbscentFromArchetype(
                component_id,
                self.id,
            ));
        };
        let column = self.columns.get_mut(column_index).unwrap();
        column.insert(value, row).or_else(|e| match e {
            ColumnError::ColumnComponentTypeMismatch(col_id, comp_id) => Err(
                ECSError::ColumnComponentTypeMismatch(self.id, row, col_id, comp_id),
            ),
            ColumnError::ColumnRowNotExistant(r) => Err(ECSError::ColumnRowNotExistant(
                self.id,
                column_index as u32,
                r,
            )),
            _ => Ok(()),
        })
    }

    fn remove_component(&mut self, component_id: TypeId, row: u32) -> Result<(), ECSError> {
        let Ok(column_index) = self.component_ids.binary_search(&component_id) else {
            return Err(ECSError::ComponentAbscentFromArchetype(
                component_id,
                self.id,
            ));
        };

        let column = self.columns.get_mut(column_index).unwrap();

        column.delete(component_id, row).or_else(|e| match e {
            ColumnError::ColumnComponentTypeMismatch(col_id, comp_id) => Err(
                ECSError::ColumnComponentTypeMismatch(self.id, row, col_id, comp_id),
            ),
            ColumnError::ColumnRowNotExistant(r) => Err(ECSError::ColumnRowNotExistant(
                self.id,
                column_index as u32,
                r,
            )),
            _ => Ok(()),
        })
    }
    pub(super) fn remove_row(&mut self, row: u32) -> Result<(), ECSError> {
        if self.entity_remove_list.contains(&row) {
            return Err(ECSError::ArchetypeRowNotExistant(self.id, row as u32));
        }

        for i in 0..self.component_ids.len() {
            let component_id = self.component_ids[i];
            self.remove_component(component_id, row as u32)?;
        }

        self.entity_remove_list.push(row);

        Ok(())
    }

    // return the row of insertion
    pub(super) fn insert_bundle<B: Bundle>(&mut self, bundle: B) -> Result<u32, ECSError> {
        let row = match self.entity_remove_list.pop() {
            Some(row) => row,
            None => {
                let insert_row = self.current_entity_count;
                self.current_entity_count += 1;
                insert_row
            }
        };

        bundle.put_components(self, row)?;
        Ok(row)
    }

    /// Retrieves raw memory pointers and metadata for all components at a specific row.
    pub fn get_row(&self, row: u32) -> Result<Vec<(NonNull<u8>, ComponentInfo)>, ECSError> {
        // Validate row boundaries and tombstone state
        if row >= self.current_entity_count || self.entity_remove_list.contains(&row) {
            return Err(ECSError::ArchetypeRowNotExistant(self.id, row));
        }

        let mut row_components = Vec::with_capacity(self.columns.len());

        for (column_idx, column) in self.columns.iter().enumerate() {
            match column.get_ptr(row) {
                Ok(ptr) => {
                    row_components.push((ptr, column.element_info));
                }
                Err(ColumnError::ColumnRowNotExistant(row)) => {
                    return Err(ECSError::ColumnRowNotExistant(
                        self.id,
                        column_idx as u32,
                        row,
                    ));
                }
                _ => {}
            }
        }

        Ok(row_components)
    }
    /*
    /// put raw memory pointers and metadata for all components of the given entity_data.
    pub fn insert_row(
        &self,
        components: Vec<(NonNull<u8>, ComponentInfo)>,
    ) -> Result<u32, ECSError> {
        // Validate row boundaries and tombstone state
        if row >= self.current_entity_count || self.entity_remove_list.contains(&row) {
            return Err(ECSError::ArchetypeRowNotExistant(self.id, row));
        }

        let mut row_components = Vec::with_capacity(self.columns.len());

        for (column_idx, column) in self.columns.iter().enumerate() {
            match column.get_ptr(row) {
                Ok(ptr) => {
                    row_components.push((ptr, column.element_info));
                }
                Err(ColumnError::ColumnRowNotExistant(row)) => {
                    return Err(ECSError::ColumnRowNotExistant(
                        self.id,
                        column_idx as u32,
                        row,
                    ));
                }
                _ => {}
            }
        }

        Ok(row_components)
    }
    */
}
