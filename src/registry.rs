use std::any::{Any, TypeId};
use std::collections::HashMap;

type Tag<'a> = &'a str;

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

#[derive(Default)]
pub struct LabelIndex {
    // (TypeId, label) -> set of EntityIds with that label
    by_label: HashMap<(TypeId, String), Vec<EntityId>>,
    // reverse lookup: EntityId -> (TypeId, label), for cleanup on despawn
    by_id: HashMap<EntityId, (TypeId, String)>,
}

impl LabelIndex {
    fn insert<T: 'static>(&mut self, label: impl Into<String>, id: EntityId) {
        let label = label.into();
        self.by_label
            .entry((TypeId::of::<T>(), label.clone()))
            .or_default()
            .push(id);
        self.by_id.insert(id, (TypeId::of::<T>(), label));
    }
    fn get<T: 'static>(&self, label: &str) -> &[EntityId] {
        self.by_label
            .get(&(TypeId::of::<T>(), label.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
    fn remove(&mut self, id: EntityId) {
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

#[derive(Default)]
pub struct ResourceRegistry {
    stores: HashMap<TypeId, Box<dyn Any>>, // TypeId -> HashMap<EntityId, T>
    resources: HashMap<TypeId, Box<dyn Any>>, // unchanged, singleton resources
    ids: EntityIdAllocator,
    labels: LabelIndex,
}

impl ResourceRegistry {
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

    pub fn with_res_mut<Tup: FetchRes, R>(
        &mut self,
        f: impl FnOnce(&mut Tup, &mut Self) -> R,
    ) -> Option<R> {
        let mut tup = Tup::take(self)?;
        let result = f(&mut tup, self);
        tup.put_back(self);
        Some(result)
    }
    fn remove_map<T: 'static>(&mut self) -> Option<HashMap<EntityId, T>> {
        Some(
            *self
                .stores
                .remove(&TypeId::of::<T>())?
                .downcast::<HashMap<EntityId, T>>()
                .unwrap(),
        )
    }
    fn insert_map<T: 'static>(&mut self, map: HashMap<EntityId, T>) {
        self.stores.insert(TypeId::of::<T>(), Box::new(map));
    }

    pub fn with_maps_mut<Tup: FetchEntity, R>(
        &mut self,
        f: impl FnOnce(&mut Tup::Maps, &mut Self) -> R,
    ) -> Option<R> {
        let mut maps = Tup::take(self)?;
        let result = f(&mut maps, self);
        Tup::put_back(maps, self);
        Some(result)
    }
}

macro_rules! impl_fetch_res {
    ($($t:ident),+) => {
        impl<$($t: 'static),+> FetchRes for ($($t,)+) {
            fn take(registry: &mut ResourceRegistry) -> Option<Self> {
                Some(($(registry.remove_res::<$t>()?,)+))
            }
            fn put_back(self, registry: &mut ResourceRegistry) {
                #[allow(non_snake_case)]
                let ($($t,)+) = self;
                $(registry.insert_res($t);)+
            }
        }
    };
}
pub trait FetchRes: Sized {
    fn take(registry: &mut ResourceRegistry) -> Option<Self>;
    fn put_back(self, registry: &mut ResourceRegistry);
}

impl_fetch_res!(A);
impl_fetch_res!(A, B);
impl_fetch_res!(A, B, C);
impl_fetch_res!(A, B, C, D);
impl_fetch_res!(A, B, C, D, E);

pub trait FetchEntity: Sized {
    type Maps;
    fn take(registry: &mut ResourceRegistry) -> Option<Self::Maps>;
    fn put_back(maps: Self::Maps, registry: &mut ResourceRegistry);
}

macro_rules! impl_fetch_entity {
    ($($t:ident),+) => {
        impl<$($t: 'static),+> FetchEntity for ($($t,)+) {
            type Maps = ($(HashMap<EntityId, $t>,)+);

            fn take(registry: &mut ResourceRegistry) -> Option<Self::Maps> {
                Some(($(registry.remove_map::<$t>()?,)+))
            }

            fn put_back(maps: Self::Maps, registry: &mut ResourceRegistry) {
                #[allow(non_snake_case)]
                let ($($t,)+) = maps;
                $(registry.insert_map($t);)+
            }
        }
    };
}

impl_fetch_entity!(A);
impl_fetch_entity!(A, B);
impl_fetch_entity!(A, B, C);
impl_fetch_entity!(A, B, C, D);
impl_fetch_entity!(A, B, C, D, E);
