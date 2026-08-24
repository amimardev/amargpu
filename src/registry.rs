use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Default)]
pub struct ResourceRegistry {
    stores: HashMap<TypeId, Box<dyn Any>>, // TypeId -> HashMap<String, T>
    resources: HashMap<TypeId, Box<dyn Any>>, // TypeId -> T
}

impl ResourceRegistry {
    pub fn insert<T: 'static>(&mut self, key: impl Into<String>, value: T) {
        self.stores
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<String, T>::new()))
            .downcast_mut::<HashMap<String, T>>()
            .unwrap()
            .insert(key.into(), value);
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.stores
            .get(&TypeId::of::<T>())?
            .downcast_ref::<HashMap<String, T>>()?
            .get(key)
    }

    pub fn get_mut<T: 'static>(&mut self, key: &str) -> Option<&mut T> {
        self.stores
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<String, T>>()?
            .get_mut(key)
    }

    pub fn remove<T: 'static>(&mut self, key: &str) -> Option<T> {
        self.stores
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<String, T>>()?
            .remove(key)
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
    fn remove_map<T: 'static>(&mut self) -> Option<HashMap<String, T>> {
        Some(
            *self
                .stores
                .remove(&TypeId::of::<T>())?
                .downcast::<HashMap<String, T>>()
                .unwrap(),
        )
    }
    fn insert_map<T: 'static>(&mut self, map: HashMap<String, T>) {
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
            type Maps = ($(HashMap<String, $t>,)+);

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
